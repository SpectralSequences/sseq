use ext::chain_complex::ChainComplex;

use ext::secondary::*;
use ext::utils::query_module;

fn main() -> error::Result {
    let data = query_module(Some(algebra::AlgebraType::Milnor))?;
    let resolution = data.resolution;

    #[cfg(feature = "concurrent")]
    let del_save_file: Option<String> = query::optional("Delta save file", Ok);

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
    for (s, f, t) in resolution.iter_stem() {
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
            println!("d_2 x_({}, {}, {}) = {:?}", f + 1, s - 2, i, entry);
        }
    }
    Ok(())
}
