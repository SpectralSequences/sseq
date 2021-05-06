use chart::{Backend as _, TikzBackend as Backend};
use ext::{chain_complex::ChainComplex, utils::construct};
use ext_webserver::actions::SseqChoice;
use ext_webserver::sseq::Sseq;
use fp::{prime::ValidPrime, vector::FpVector};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

const TWO: ValidPrime = ValidPrime::new(2);

fn main() -> error::Result<()> {
    let module_file_name: String = query::with_default("Module", "S_2", str::parse);

    let max_s: u32 = query::with_default("Max s", "7", str::parse);
    let max_t: i32 = query::with_default("Max t", "30", str::parse);

    // Clippy false positive
    #[allow(clippy::redundant_closure)]
    let save_file: Option<File> = query::optional("Resolution save file", |s| File::open(s));

    #[cfg(feature = "concurrent")]
    let bucket = ext::utils::query_bucket();

    let resolution = construct(
        (&*module_file_name, algebra::AlgebraType::Milnor),
        save_file,
    )?;

    if !resolution.has_computed_bidegree(max_s, max_t) {
        print!("Resolving module: ");
        let start = Instant::now();

        #[cfg(not(feature = "concurrent"))]
        resolution.compute_through_bidegree(max_s, max_t);

        #[cfg(feature = "concurrent")]
        resolution.compute_through_bidegree_concurrent(max_s, max_t, &bucket);

        println!("{:.2?}", start.elapsed());
    }

    let mut sseq = Sseq::new(TWO, SseqChoice::Main, 0, 0, None);

    for i in 0..3 {
        sseq.add_product_type(&format!("h{}", i), (1 << i) - 1, 1, true, true);
    }

    for (s, n, t) in resolution.iter_stem() {
        let num_gens = resolution.module(s).number_of_gens_in_degree(t);
        sseq.set_class(n, s as i32, num_gens);

        for i in 0..3 {
            if let Some(products) = resolution.filtration_one_product(1 << i, 0, s, t) {
                sseq.add_product(
                    &format!("h{}", i),
                    n - (1 << i) + 1,
                    s as i32 - 1,
                    (1 << i) - 1,
                    1,
                    false,
                    &products,
                );
            }
        }
    }

    let f = BufReader::new(File::open(
        Path::new(file!())
            .parent()
            .unwrap()
            .join(format!("d2_{}", module_file_name)),
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

    let write = |path, page, diff, prod| {
        const EXT: &str = Backend::<File>::EXT;
        let backend = Backend::new(File::create(format!(
            "{}_{}.{}",
            path, module_file_name, EXT
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
