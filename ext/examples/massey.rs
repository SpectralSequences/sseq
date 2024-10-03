//! Computes the triple Massey product up to a sign
//!
//! This is optimized to compute <a, b, -> for fixed a, b and all -, where a and b have small
//! degree.

use std::sync::Arc;

use ext::{
    chain_complex::{ChainComplex, ChainHomotopy, FreeChainComplex},
    resolution_homomorphism::ResolutionHomomorphism,
};
use fp::matrix::{AugmentedMatrix, Matrix};
use sseq::coordinates::{Bidegree, BidegreeElement, BidegreeGenerator};

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging();

    let resolution = Arc::new(ext::utils::query_module(None, true)?);
    let p = resolution.prime();

    let (is_unit, unit) = ext::utils::get_unit(Arc::clone(&resolution))?;

    eprintln!("\nComputing Massey products <a, b, ->");
    eprintln!("\nEnter a:");

    let a = Bidegree::n_s(
        query::raw("n of Ext class a", str::parse),
        query::raw("s of Ext class a", str::parse::<std::num::NonZeroU32>).get(),
    );

    unit.compute_through_stem(a);

    let a_class = query::vector("Input Ext class a", unit.number_of_gens_in_bidegree(a));

    eprintln!("\nEnter b:");

    let b = Bidegree::n_s(
        query::raw("n of Ext class b", str::parse),
        query::raw("s of Ext class b", str::parse::<std::num::NonZeroU32>).get(),
    );

    unit.compute_through_stem(b);

    let b_class = query::vector("Input Ext class b", unit.number_of_gens_in_bidegree(b));

    // The Massey product shifts the bidegree by this amount
    let shift = a + b - Bidegree::s_t(1, 0);

    if !is_unit {
        unit.compute_through_stem(shift);
    }

    if !resolution.has_computed_bidegree(shift + Bidegree::s_t(0, resolution.min_degree())) {
        eprintln!("No computable bidegrees");
        return Ok(());
    }

    let b_hom = Arc::new(ResolutionHomomorphism::from_class(
        String::new(),
        Arc::clone(&unit),
        Arc::clone(&unit),
        b,
        &b_class,
    ));

    b_hom.extend_through_stem(shift);

    let offset_a = unit.module(a.s()).generator_offset(a.t(), a.t(), 0);
    for c in resolution.iter_nonzero_stem() {
        if !resolution.has_computed_bidegree(c + shift) {
            continue;
        }

        let tot = c + shift;

        let num_gens = resolution.number_of_gens_in_bidegree(c);
        let product_num_gens = resolution.number_of_gens_in_bidegree(b + c);
        let target_num_gens = resolution.number_of_gens_in_bidegree(tot);
        if target_num_gens == 0 {
            continue;
        }

        let mut answers = vec![vec![0; target_num_gens]; num_gens];
        let mut product = AugmentedMatrix::<2>::new(p, num_gens, [product_num_gens, num_gens]);
        product.segment(1, 1).add_identity();

        let mut matrix = Matrix::new(p, num_gens, 1);
        for idx in 0..num_gens {
            let hom = Arc::new(ResolutionHomomorphism::new(
                String::new(),
                Arc::clone(&resolution),
                Arc::clone(&unit),
                c,
            ));

            matrix[idx].set_entry(0, 1);
            hom.extend_step(c, Some(&matrix));
            matrix[idx].set_entry(0, 0);

            hom.extend_through_stem(tot);

            let homotopy = ChainHomotopy::new(Arc::clone(&hom), Arc::clone(&b_hom));

            homotopy.extend(tot);

            let last = homotopy.homotopy(tot.s());
            for i in 0..target_num_gens {
                let output = last.output(tot.t(), i);
                for (k, &v) in a_class.iter().enumerate() {
                    if v != 0 {
                        answers[idx][i] += v * output.entry(offset_a + k);
                    }
                }
            }

            for (k, &v) in b_class.iter().enumerate() {
                if v != 0 {
                    let gen = BidegreeGenerator::new(b, k);
                    hom.act(product[idx].slice_mut(0, product_num_gens), v, gen);
                }
            }
        }
        product.row_reduce();
        let kernel = product.compute_kernel();

        for row in kernel.iter() {
            let c_element = BidegreeElement::new(c, row.to_owned());
            print!(
                "<a, b, {c_string}> = [",
                c_string = c_element.to_basis_string()
            );

            for i in 0..target_num_gens {
                let mut entry = 0;
                for (j, v) in row.iter().enumerate() {
                    entry += v * answers[j][i];
                }
                if i != 0 {
                    print!(", ");
                }
                print!("{}", entry % p);
            }
            println!("]");
        }
    }

    Ok(())
}
