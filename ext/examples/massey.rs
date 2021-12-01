//! Computes the triple Massey product up to a sign

use algebra::module::homomorphism::ModuleHomomorphism;
use algebra::module::{FDModule, Module};
use ext::chain_complex::{ChainComplex, ChainHomotopy, FiniteChainComplex};
use ext::resolution::Resolution;
use ext::resolution_homomorphism::ResolutionHomomorphism;
use fp::matrix::{AugmentedMatrix, Matrix};
use std::sync::Arc;

fn parse_vec(s: &str) -> Result<Vec<u32>, core::num::ParseIntError> {
    s[1..s.len() - 1]
        .split(',')
        .map(|x| x.trim().parse())
        .collect::<Result<Vec<_>, _>>()
}

fn main() -> anyhow::Result<()> {
    let resolution = Arc::new(ext::utils::query_module(None, true)?.resolution);
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

    let a_n: i32 = query::with_default("n", "0", str::parse);
    let a_s: u32 = query::with_default("s", "1", |v| match v.parse::<u32>() {
        Ok(0) => Err(String::from("Must be positive filtration class")),
        Ok(x) => Ok(x),
        Err(e) => Err(e.to_string()),
    });
    let a_t = a_n + a_s as i32;
    let a_class = query::with_default("class", "[1]", parse_vec);

    eprintln!("\nEnter b:");

    let b_n: i32 = query::with_default("n", "1", str::parse);
    let b_s: u32 = query::with_default("s", "1", |v| match v.parse::<u32>() {
        Ok(0) => Err(String::from("Must be positive filtration class")),
        Ok(x) => Ok(x),
        Err(e) => Err(e.to_string()),
    });
    let b_t = b_n + b_s as i32;
    let b_class = query::with_default("class", "[1]", parse_vec);

    // The Massey product shifts the bidegree by this amount
    let shift_s = a_s + b_s - 1;
    let shift_t = a_t + b_t;
    let shift_n = shift_t - shift_s as i32;

    if !is_unit {
        unit.compute_through_stem(shift_s, shift_n);
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

    b_hom.extend_through_stem(shift_s, shift_n);

    let offset_a = unit.module(a_s).generator_offset(a_t, a_t, 0);
    for (s, n, t) in resolution.iter_stem() {
        if !resolution.has_computed_bidegree(s + shift_s, t + shift_t) {
            continue;
        }

        let tot_s = s + shift_s;
        let tot_t = t + shift_t;
        let tot_n = n + shift_n;

        let num_gens = resolution.module(s).number_of_gens_in_degree(t);
        let product_num_gens = resolution.module(s + b_s).number_of_gens_in_degree(t + b_t);
        let target_num_gens = resolution.module(tot_s).number_of_gens_in_degree(tot_t);
        if num_gens == 0 || target_num_gens == 0 {
            continue;
        }

        let mut answers = vec![vec![0; target_num_gens]; num_gens];
        let mut product = AugmentedMatrix::<2>::new(p, num_gens, [product_num_gens, num_gens]);
        product.segment(1, 1).add_identity();

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

            hom.extend_through_stem(tot_s, tot_n);

            let homotopy = ChainHomotopy::new(
                &*resolution,
                &*unit,
                s + b_s,
                t + b_t,
                |source_s, source_t, idx, row| {
                    let mid_s = source_s - s;

                    hom.get_map(source_s)
                        .compose(b_hom.get_map(mid_s))
                        .apply_to_basis_element(row.as_slice_mut(), 1, source_t, idx)
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
            ext::utils::print_element(row.as_slice(), n, s);
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
