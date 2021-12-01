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
use std::sync::Arc;

use ext::chain_complex::ChainComplex;
use ext::secondary::*;
use ext::utils::query_module;

fn main() -> anyhow::Result<()> {
    let data = query_module(Some(algebra::AlgebraType::Milnor), false)?;
    let resolution = Arc::new(data.resolution);

    if !can_compute(&resolution) {
        eprintln!(
            "Cannot compute d2 for the module {}",
            resolution.complex().module(0)
        );
        return Ok(());
    }

    let start = std::time::Instant::now();

    let lift = SecondaryLift::new(Arc::clone(&resolution));
    lift.initialize_homotopies();
    lift.compute_composites();
    lift.compute_intermediates();

    #[cfg(feature = "concurrent")]
    lift.compute_homotopies_concurrent(&data.bucket);

    #[cfg(not(feature = "concurrent"))]
    lift.compute_homotopies();

    eprintln!("Time spent: {:?}", start.elapsed());

    // Iterate through target of the d2
    for (s, n, t) in lift.chain_complex.iter_stem() {
        if s < 3 {
            continue;
        }

        if t - 1 > resolution.module(s - 2).max_computed_degree() {
            continue;
        }
        if resolution.module(s).number_of_gens_in_degree(t) == 0 {
            continue;
        }
        let homotopy = lift.homotopy(s);
        let m = homotopy.homotopies.hom_k(t - 1);

        for (i, entry) in m.into_iter().enumerate() {
            println!("d_2 x_({}, {}, {}) = {:?}", n + 1, s - 2, i, entry);
        }
    }

    Ok(())
}
