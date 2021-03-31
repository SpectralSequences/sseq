use ext::chain_complex::ChainComplex;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;
use std::time::Instant;

use algebra::module::homomorphism::ModuleHomomorphism;
use ext::resolution::Resolution;
use ext::utils::construct_s_2;

use query::*;
use saveload::Load;

#[cfg(feature = "concurrent")]
use thread_token::TokenBucket;

fn main() -> error::Result<()> {
    let mut resolution = construct_s_2("milnor");

    let max_s = query_with_default("Max s", "7", Ok);
    let max_t = query_with_default("Max t", "30", Ok);

    let res_save_file: Option<String> = query_optional("Resolution save file", Ok);
    #[cfg(feature = "concurrent")]
    let del_save_file: Option<String> = query_optional("Delta save file", Ok);

    #[cfg(feature = "concurrent")]
    let num_threads = query_with_default("Number of threads", "2", Ok);

    if let Some(p) = res_save_file {
        if Path::new(&*p).exists() {
            print!("Loading saved resolution: ");
            let start = Instant::now();
            let f = File::open(&*p)?;
            let mut f = BufReader::new(f);
            resolution = Resolution::load(&mut f, &resolution.complex())?;
            println!("{:.2?}", start.elapsed());
        }
    }

    let should_resolve = !resolution.has_computed_bidegree(max_s, max_t);

    #[cfg(not(feature = "concurrent"))]
    let deltas = {
        if should_resolve {
            print!("Resolving module: ");
            let start = Instant::now();
            resolution.resolve_through_bidegree(max_s, max_t);
            println!("{:.2?}", start.elapsed());
        }

        ext::secondary::compute_delta(&resolution, max_s, max_t)
    };

    #[cfg(feature = "concurrent")]
    let deltas = {
        let bucket = TokenBucket::new(num_threads);

        if should_resolve {
            print!("Resolving module: ");
            let start = Instant::now();
            resolution.resolve_through_bidegree_concurrent(max_s, max_t, &bucket);
            println!("{:.2?}", start.elapsed());
        }

        ext::secondary::compute_delta_concurrent(&resolution, max_s, max_t, &bucket, del_save_file)
    };

    let mut filename = String::from("d2");
    while Path::new(&filename).exists() {
        filename.push('_');
    }
    let mut output = File::create(&filename).unwrap();

    for f in 1..max_t {
        for s in 1..(max_s - 1) {
            let t = s as i32 + f;
            if t >= max_t {
                break;
            }
            let delta = &deltas[s as usize - 1];

            if delta.source().number_of_gens_in_degree(t + 1) == 0 {
                continue;
            }
            let d = delta.hom_k(t);

            for (i, entry) in d.into_iter().enumerate() {
                writeln!(output, "d_2 x_({}, {}, {}) = {:?}", f, s, i, entry).unwrap();
            }
        }
    }
    Ok(())
}
