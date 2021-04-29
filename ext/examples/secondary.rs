//! This computes d₂ differentials in the Adams spectral sequence. This only works for fairly
//! specific modules, but tends to cover most cases of interest.
//!
//! In general, the set of possible d₂'s is a torsor over Ext^{2, 1}(M, M); the action of χ ∈
//! Ext^{2, 1}(M, M) is given by adding χ-multiplication to the d₂ map. This algorithm computes one
//! possible set of d₂'s. If Ext^{2, 1}(M, M) is non-zero, some differentials will have to be
//! calculated by hand to determine the actual set of d₂'s.
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

use ext::chain_complex::ChainComplex;

use ext::secondary::*;
use ext::utils::query_module;

fn main() -> error::Result {
    let data = query_module(Some(algebra::AlgebraType::Milnor))?;
    let resolution = data.resolution;

    #[cfg(feature = "concurrent")]
    let del_save_file: Option<String> = query::optional("C save file", str::parse);

    if !can_compute(&resolution) {
        eprintln!(
            "Cannot compute d2 for the module {}",
            resolution.complex().module(0)
        );
        return Ok(());
    }

    #[cfg(not(feature = "concurrent"))]
    let deltas = compute_delta(&resolution);

    #[cfg(feature = "concurrent")]
    let deltas = compute_delta_concurrent(&resolution, &data.bucket, del_save_file);

    // Iterate through target of the d2
    for (s, n, t) in resolution.iter_stem() {
        if s < 3 {
            continue;
        }
        if resolution.module(s).number_of_gens_in_degree(t) == 0 {
            continue;
        }
        let delta = &deltas[s as usize - 3];
        if t >= delta.next_degree() {
            continue;
        }
        let d = delta.hom_k(t - 1);

        for (i, entry) in d.into_iter().enumerate() {
            println!("d_2 x_({}, {}, {}) = {:?}", n + 1, s - 2, i, entry);
        }
    }
    Ok(())
}
