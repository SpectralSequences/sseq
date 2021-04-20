use ext::chain_complex::ChainComplex;
use std::time::Instant;

use ext::secondary::*;
use ext::utils::construct;

fn main() -> error::Result<()> {
    let module_file_name: String = query::with_default("Module", "S_2", Ok);

    let max_s = query::with_default("Max s", "7", Ok);
    let max_t = query::with_default("Max t", "30", Ok);

    let res_save_file: Option<String> = query::optional("Resolution save file", Ok);
    #[cfg(feature = "concurrent")]
    let del_save_file: Option<String> = query::optional("Delta save file", Ok);

    #[cfg(feature = "concurrent")]
    let bucket = {
        let num_threads = query::with_default("Number of threads", "2", Ok);
        thread_token::TokenBucket::new(num_threads)
    };

    let resolution = construct(
        (&*module_file_name, algebra::AlgebraType::Milnor),
        res_save_file.as_deref(),
    )?;

    if !resolution.has_computed_bidegree(max_s, max_t) {
        eprint!("Resolving module: ");
        let start = Instant::now();

        #[cfg(not(feature = "concurrent"))]
        resolution.compute_through_bidegree(max_s, max_t);

        #[cfg(feature = "concurrent")]
        resolution.compute_through_bidegree_concurrent(max_s, max_t, &bucket);

        eprintln!("{:.2?}", start.elapsed());
    }

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
    let deltas = compute_delta_concurrent(&resolution, &bucket, del_save_file);

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
