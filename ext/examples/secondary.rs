//! This computes $d_2$ differentials in the Adams spectral sequence. This only works for fairly
//! specific modules, but tends to cover most cases of interest.
//!
//! # Usage
//! This asks for a module in the usual way, and verifies that the module satisfies the conditions
//! necessary for the algorithm the work. It only works with the Milnor basis.
//!
//! # Output
//! We omit differentials if the target bidegree is zero.

use algebra::module::Module;
use std::sync::Arc;

use ext::chain_complex::{AugmentedChainComplex, ChainComplex};
use ext::secondary::*;
use ext::utils::query_module;

fn main() -> anyhow::Result<()> {
    let resolution = Arc::new(query_module(
        Some(algebra::AlgebraType::Milnor),
        ext::utils::LoadQuasiInverseOption::IfNoSave,
    )?);

    if !can_compute(&resolution) {
        eprintln!(
            "Cannot compute d2 for the module {}",
            resolution.target().module(0)
        );
        return Ok(());
    }

    let start = std::time::Instant::now();

    let lift = SecondaryResolution::new(Arc::clone(&resolution));
    lift.extend_all();

    eprintln!("Time spent: {:?}", start.elapsed());

    // Iterate through target of the d2
    for (s, n, t) in lift.underlying().iter_stem() {
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
