use algebra::module::OperationGeneratorPair;
use chart::{Backend as _, TikzBackend as Backend};
use ext::{
    chain_complex::ChainComplex,
    resolution::Resolution,
    utils::{construct, get_config, iter_stems},
};
use ext_webserver::actions::SseqChoice;
use ext_webserver::sseq::Sseq;
use fp::{prime::ValidPrime, vector::FpVector};
use saveload::Load;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

const TWO: ValidPrime = ValidPrime::new(2);

fn main() -> error::Result<()> {
    let mut config = get_config();
    config.algebra_name = String::from("milnor");

    let max_s = query::with_default("Max s", "7", Ok);
    let max_t = query::with_default("Max t", "30", Ok);

    let save_file: Option<String> = query::optional("Resolution save file", Ok);
    let mut resolution = construct(&config)?;

    #[cfg(feature = "concurrent")]
    let bucket = {
        let num_threads = query::with_default("Number of threads", "2", Ok);
        thread_token::TokenBucket::new(num_threads)
    };

    if let Some(path) = save_file {
        let f = File::open(path).unwrap();
        let mut f = std::io::BufReader::new(f);
        resolution = Resolution::load(&mut f, &resolution.complex())?;
    }

    if !resolution.has_computed_bidegree(max_s, max_t) {
        print!("Resolving module: ");
        let start = Instant::now();

        #[cfg(not(feature = "concurrent"))]
        resolution.resolve_through_bidegree(max_s, max_t);

        #[cfg(feature = "concurrent")]
        resolution.resolve_through_bidegree_concurrent(max_s, max_t, &bucket);

        println!("{:.2?}", start.elapsed());
    }

    let mut sseq = Sseq::new(TWO, SseqChoice::Main, 0, 0, None);

    for i in 0..3 {
        sseq.add_product_type(&format!("h{}", i), (1 << i) - 1, 1, true, true);
    }

    for (s, f, t) in iter_stems(max_s, max_t) {
        let num_gens = resolution.module(s).number_of_gens_in_degree(t);
        sseq.set_class(f, s as i32, num_gens);

        if s == 0 {
            continue;
        }

        let source = resolution.module(s - 1);
        let d = resolution.differential(s);

        for i in 0..3 {
            if f < (1 << i) - 1 {
                continue;
            }
            let source_num_gens = source.number_of_gens_in_degree(t - (1 << i));
            let mut matrix = vec![vec![0; num_gens]; source_num_gens];

            for k in 0..num_gens {
                let dg = d.output(t, k);

                #[allow(clippy::needless_range_loop)]
                for l in 0..source_num_gens {
                    let elt = source.operation_generator_pair_to_idx(&OperationGeneratorPair {
                        operation_index: 0,
                        operation_degree: 1 << i,
                        generator_index: l,
                        generator_degree: t - (1 << i),
                    });
                    if dg.entry(elt) != 0 {
                        matrix[l][k] = 1;
                    }
                }
            }
            sseq.add_product(
                &format!("h{}", i),
                f - (1 << i) + 1,
                s as i32 - 1,
                (1 << i) - 1,
                1,
                false,
                &matrix,
            );
        }
    }

    let f = BufReader::new(File::open(
        Path::new(file!())
            .parent()
            .unwrap()
            .join(format!("d2_{}", config.module_file_name)),
    )?);
    let mut v = FpVector::new(TWO, 0);
    for line in f.lines() {
        let data: Vec<u32> = line?
            .trim()
            .split(',')
            .map(|x| x.parse().unwrap())
            .collect();
        let source_x = data[0];
        let source_y = data[1];
        let source_idx = data[2];

        let target = &data[3..];
        if !target.iter().any(|&x| x != 0) {
            continue;
        }
        let target = FpVector::from_slice(TWO, target);

        v.set_scratch_vector_size(
            resolution
                .module(source_y as u32)
                .number_of_gens_in_degree((source_x + source_y) as i32),
        );
        v.add_basis_element(source_idx as usize, 1);

        sseq.add_differential(2, source_x as i32, source_y as i32, &v, &target);
    }

    sseq.refresh_all();

    let mut write = |path, page, diff, prod| {
        const EXT: &str = Backend::<File>::EXT;
        let backend = Backend::new(File::create(format!(
            "{}_{}.{}",
            path, config.module_file_name, EXT
        ))?);
        sseq.write_to_graph(backend, page, diff, prod)?;
        <Result<(), std::io::Error>>::Ok(())
    };

    write("e2", 2, false, &["h0", "h1", "h2"])?;
    write("e2_d2", 2, true, &["h0", "h1", "h2"])?;
    write("e3", 3, false, &["h0", "h1", "h2"])?;

    write("e2_clean", 2, false, &["h0", "h1"])?;
    write("e2_d2_clean", 2, true, &["h0", "h1"])?;
    write("e3_clean", 3, false, &["h0", "h1"])?;

    Ok(())
}
