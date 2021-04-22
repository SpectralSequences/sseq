//! Computes the triple Massey product up to a sign

use algebra::module::homomorphism::ModuleHomomorphism;
use algebra::module::{FDModule, Module};
use ext::chain_complex::{ChainComplex, ChainHomotopy, FiniteChainComplex};
use ext::resolution::Resolution;
use ext::resolution_homomorphism::ResolutionHomomorphism;
use fp::matrix::{AugmentedMatrix, Matrix};
use std::sync::Arc;

fn parse_vec(s: String) -> Result<Vec<u32>, String> {
    s[1..s.len() - 1]
        .split(',')
        .map(|x| x.trim().parse())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|x: core::num::ParseIntError| x.to_string())
}

fn main() -> error::Result<()> {
    let resolution = Arc::new(ext::utils::query_module(None)?.resolution);
    let p = resolution.prime();

    let (is_unit, unit) = if resolution.complex().module(0).is_unit() {
        (true, Arc::clone(&resolution))
    } else {
        let module = Arc::new(
            FDModule::new(
                resolution.algebra(),
                format!("S_{}", p),
                bivec::BiVec::from_vec(0, vec![1]),
            )
            .into(),
        );
        let ccdz = Arc::new(FiniteChainComplex::ccdz(module));
        (false, Arc::new(Resolution::new(ccdz)))
    };

    eprintln!("\nComputing Massey products <a, b, ->");
    eprintln!("\nEnter a:");

    let a_f: i32 = query::with_default("f", "0", Ok);
    let a_s = query::with_default("s", "1", |v| {
        if v == 0 {
            Err("Must be positive filtration class".into())
        } else {
            Ok(v)
        }
    });
    let a_t = a_f + a_s as i32;
    let a_class = query::with_default("class", "[1]", parse_vec);

    eprintln!("\nEnter b:");

    let b_f: i32 = query::with_default("f", "1", Ok);
    let b_s = query::with_default("s", "1", |v| {
        if v == 0 {
            Err("Must be positive filtration class".into())
        } else {
            Ok(v)
        }
    });
    let b_t = b_f + b_s as i32;
    let b_class = query::with_default("class", "[1]", parse_vec);

    // The Massey product shifts the bidegree by this amount
    let shift_s = a_s + b_s - 1;
    let shift_t = a_t + b_t;
    let shift_f = shift_t - shift_s as i32;

    if !is_unit {
        unit.compute_through_stem(shift_s, shift_f);
    }

    if !resolution.has_computed_bidegree(shift_s, shift_t + resolution.min_degree()) {
        eprintln!("No computable bidegrees");
        return Ok(());
    }

    let b_hom = ResolutionHomomorphism::from_class(
        "b".into(),
        Arc::clone(&unit),
        Arc::clone(&unit),
        b_s,
        b_t,
        &b_class,
    );

    b_hom.extend_through_stem(shift_s, shift_f);

    let offset_a = unit.module(a_s).generator_offset(a_t, a_t, 0);
    for (s, f, t) in resolution.iter_stem() {
        if !resolution.has_computed_bidegree(s + shift_s, t + shift_t) {
            continue;
        }

        let tot_s = s + shift_s;
        let tot_t = t + shift_t;
        let tot_f = f + shift_f;

        let num_gens = resolution.module(s).number_of_gens_in_degree(t);
        let product_num_gens = resolution.module(s + b_s).number_of_gens_in_degree(t + b_t);
        let target_num_gens = resolution.module(tot_s).number_of_gens_in_degree(tot_t);
        if num_gens == 0 || target_num_gens == 0 {
            continue;
        }

        let mut answers = vec![vec![0; target_num_gens]; num_gens];
        let mut product = AugmentedMatrix::<2>::new(p, num_gens, [product_num_gens, num_gens]);
        product.segment(1, 1).add_identity(num_gens, 0, 0);

        let mut matrix = Matrix::new(p, num_gens, 1);
        for idx in 0..num_gens {
            let hom = ResolutionHomomorphism::new(
                "c".into(),
                Arc::clone(&resolution),
                Arc::clone(&unit),
                s,
                t,
            );

            matrix[idx].set_entry(0, 1);
            hom.extend_step(s, t, Some(&matrix));
            matrix[idx].set_entry(0, 0);

            hom.extend_through_stem(tot_s, tot_f);

            let homotopy = ChainHomotopy::new(
                Arc::clone(&resolution),
                Arc::clone(&resolution),
                s + b_s,
                t + b_t,
                |source_s, source_t, idx, row| {
                    let mid_s = source_s - s;
                    let mid_t = source_t - t;
                    let last_s = mid_s - b_s;

                    b_hom.get_map(last_s).apply(
                        row,
                        1,
                        mid_t,
                        hom.get_map(mid_s).output(source_t, idx).as_slice(),
                    );
                },
            );

            homotopy.extend(tot_s, tot_t);

            let last = homotopy.homotopy(tot_s);
            for i in 0..target_num_gens {
                let output = last.output(tot_t, i);
                for (k, &v) in a_class.iter().enumerate() {
                    if v != 0 {
                        answers[idx][i] += v * output.entry(offset_a + k);
                    }
                }
            }

            for (k, &v) in b_class.iter().enumerate() {
                if v != 0 {
                    hom.act(product[idx].slice_mut(0, product_num_gens), v, b_s, b_t, k);
                }
            }
        }
        product.row_reduce();
        let kernel = product.compute_kernel();

        for row in &**kernel {
            print!("<a, b, ");
            // print element name
            let mut first = true;
            for (i, v) in row.iter().enumerate() {
                if v == 0 {
                    continue;
                }
                if !first {
                    print!("+");
                }
                if v != 1 {
                    print!("{}", v);
                }
                print!("x_({}, {}, {})", f, s, i);
                first = false;
            }
            print!("> = [");
            for i in 0..target_num_gens {
                let mut entry = 0;
                for (j, v) in row.iter().enumerate() {
                    entry += v * answers[j][i];
                }
                if i != 0 {
                    print!(", ");
                }
                print!("{}", entry % *p);
            }
            println!("]");
        }
    }

    Ok(())
}
