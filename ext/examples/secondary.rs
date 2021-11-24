//! This computes $d_2$ differentials in the Adams spectral sequence. This only works for fairly
//! specific modules, but tends to cover most cases of interest.
//!
//! In general, the set of possible $d_2$'s is a torsor over $\Ext^{2, 1}(M, M)$; the
//! action of $\chi \in \Ext^{2, 1}(M, M)$ is given by adding $\chi$-multiplication
//! to the $d_2$ map. This algorithm computes one possible set of $d_2$'s. If $\Ext^{2, 1}(M, M)$
//! is non-zero, some differentials will have to be calculated by hand to determine the actual set
//! of $d_2$'s.
//!
//! # Usage
//! This asks for a module and a resolution in the usual way. It only works with the Milnor basis,
//! and the `@milnor` modifier can be omitted.
//!
//! If `concurrent` is enabled, it also asks for a save file for the C function. Computing the C
//! function is the most expensive part of the of the computation. If a save file is provided, we
//! read existing computations from the save file and write new ones into it. The same save file
//! can be reused for different ranges of the same module.
//!
//! # Output
//! We omit differentials if the target bidegree is zero

use algebra::module::Module;
use itertools::Itertools;
use std::sync::Arc;

use ext::chain_complex::ChainComplex;
use ext::secondary::*;
use ext::utils::query_module;

fn main() -> anyhow::Result<()> {
    let data = query_module(Some(algebra::AlgebraType::Milnor))?;
    let resolution = Arc::new(data.resolution);

    if !can_compute(&resolution) {
        eprintln!(
            "Cannot compute d2 for the module {}",
            resolution.complex().module(0)
        );
        return Ok(());
    }

    let mut lift = SecondaryLift::new(Arc::clone(&resolution));
    lift.initialize_homotopies();
    lift.compute_composites();
    lift.compute_homotopies();

    // Iterate through target of the d2
    for (s, n, t) in lift.chain_complex.iter_stem() {
        if s < 3 {
            continue;
        }

        let source = resolution.module(s);
        let target = resolution.module(s - 2);
        if t - 1 > target.max_computed_degree() {
            continue;
        }
        let source_num_gens = source.number_of_gens_in_degree(t);
        let target_num_gens = target.number_of_gens_in_degree(t - 1);
        if source_num_gens == 0 || target_num_gens == 0 {
            continue;
        }
        let homotopy = lift.homotopy(s);
        let mut entries = vec![vec![0; target_num_gens]; source_num_gens];

        let offset = target.generator_offset(t - 1, t - 1, 0);

        for (n, row) in entries.iter_mut().enumerate() {
            let dx = &homotopy.output(t, n).homotopy;

            for (k, entry) in row.iter_mut().enumerate() {
                *entry = dx.entry(offset + k);
            }
        }

        for k in 0..target_num_gens {
            println!(
                "d_2 x_({}, {}, {k}) = [{}]",
                n + 1,
                s - 2,
                (0..source_num_gens).map(|n| entries[n][k]).format(", ")
            )
        }
    }

    Ok(())
}
