use algebra::module::OperationGeneratorPair;
use ext::{chain_complex::ChainComplex, load_s_2, utils::iter_stems};
use ext_webserver::actions::SseqChoice;
use ext_webserver::sseq::Sseq;
use fp::{prime::ValidPrime, vector::FpVector};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

const TWO: ValidPrime = ValidPrime::new(2);
const MAX_T: i32 = 30;
const MAX_S: u32 = 12;

fn main() -> std::io::Result<()> {
    load_s_2!(resolution, "milnor", "resolution.save");
    resolution.resolve_through_bidegree(MAX_S, MAX_T);

    let mut sseq = Sseq::new(TWO, SseqChoice::Main, 0, 0, None);

    for i in 0..3 {
        sseq.add_product_type(&format!("h{}", i), (1 << i) - 1, 1, true, true);
    }

    for (s, f, t) in iter_stems(MAX_S, MAX_T) {
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

    let f = BufReader::new(File::open(Path::new(file!()).parent().unwrap().join("d2"))?);
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

    sseq.write_to_svg(File::create("e2.svg")?, 2, false, &["h0", "h1", "h2"])?;
    sseq.write_to_svg(File::create("e2_d2.svg")?, 2, true, &["h0", "h1", "h2"])?;
    sseq.write_to_svg(File::create("e3.svg")?, 3, false, &["h0", "h1", "h2"])?;
    Ok(())
}
