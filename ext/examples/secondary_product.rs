//! Computes products in $\Mod_{C\lambda^2}$.
//!
//! # Usage
//! The program asks for a module $M$ and an element $x \in \Ext^{\*, \*}(M, k)$. It then computes
//! the secondary product of the standard lift of $x$ with all (standard lifts of) elements in
//! $\Ext^{\*, \*}(M, k)$ that survive $d_2$.
//!
//! These products are computed for all elements whose product with $x$ lies in the specified
//! bidegree of $M$, and $k$ is resolved as far as necessary to support this computation.
//!
//! Running this program requires computing the secondary resolution of both $M$ and $k$, i.e. the
//! calculations performed by [`secondary`](../secondary/index.html). The user is encouraged to make
//! use of a save file to reuse these calculations for different products. (When $M$ is not equal to
//! $k$, the user will be prompted for the save directory of $k$)
//!
//! # Output
//! This prints the corresponding products in $\Mod_{C\lambda^2}$. In particular, $x$ multiplies on
//! the left, and the sign twist of $(-1)^{s't}$ is inserted.
//!
//! # Notes
//! The program verifies that $x$ is indeed permanent.

use std::sync::Arc;

use algebra::module::Module;
use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    resolution_homomorphism::ResolutionHomomorphism,
    secondary::*,
    utils::query_module,
};
use fp::{matrix::Matrix, vector::FpVector};
use itertools::Itertools;
use sseq::coordinates::{Bidegree, BidegreeElement, BidegreeGenerator};

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging();

    let resolution = Arc::new(query_module(Some(algebra::AlgebraType::Milnor), true)?);

    let (is_unit, unit) = ext::utils::get_unit(Arc::clone(&resolution))?;

    let p = resolution.prime();

    let name: String = query::raw("Name of product", str::parse);

    let shift = Bidegree::n_s(
        query::raw(&format!("n of Ext class {name}"), str::parse),
        query::raw(&format!("s of Ext class {name}"), str::parse),
    );

    let hom = ResolutionHomomorphism::new(name, Arc::clone(&resolution), Arc::clone(&unit), shift);

    let mut matrix = Matrix::new(p, hom.source.number_of_gens_in_bidegree(shift), 1);

    if matrix.rows() == 0 || matrix.columns() == 0 {
        panic!("No classes in this bidegree");
    }
    let v: Vec<u32> = query::vector("Input ext class", matrix.rows());
    for (i, &x) in v.iter().enumerate() {
        matrix[i].set_entry(0, x);
    }

    if !is_unit {
        let res_max = Bidegree::n_s(
            resolution.module(0).max_computed_degree(),
            resolution.next_homological_degree() - 1,
        );
        unit.compute_through_stem(res_max - shift);
    }

    hom.extend_step(shift, Some(&matrix));
    hom.extend_all();

    let res_lift = SecondaryResolution::new(Arc::clone(&resolution));
    res_lift.extend_all();

    // Check that class survives to E3.
    {
        let m = res_lift.homotopy(shift.s() + 2).homotopies.hom_k(shift.t());
        assert_eq!(m.len(), v.len());
        let mut sum = vec![0; m[0].len()];
        for (x, d2) in v.iter().zip_eq(&m) {
            sum.iter_mut().zip_eq(d2).for_each(|(a, b)| *a += x * b);
        }
        assert!(
            sum.iter().all(|x| *x % p == 0),
            "Class supports a non-zero d2"
        );
    }
    let res_lift = Arc::new(res_lift);

    let unit_lift = if is_unit {
        Arc::clone(&res_lift)
    } else {
        let lift = SecondaryResolution::new(Arc::clone(&unit));
        lift.extend_all();
        Arc::new(lift)
    };

    let hom = Arc::new(hom);
    let hom_lift = SecondaryResolutionHomomorphism::new(
        Arc::clone(&res_lift),
        Arc::clone(&unit_lift),
        Arc::clone(&hom),
    );

    if let Some(s) = ext::utils::secondary_job() {
        hom_lift.compute_partial(s);
        return Ok(());
    }

    hom_lift.extend_all();

    // Compute E3 page
    let res_sseq = Arc::new(res_lift.e3_page());
    let unit_sseq = if is_unit {
        Arc::clone(&res_sseq)
    } else {
        Arc::new(unit_lift.e3_page())
    };

    fn get_page_data(sseq: &sseq::Sseq<sseq::Adams>, b: Bidegree) -> &fp::matrix::Subquotient {
        let d = sseq.page_data(b.n(), b.s() as i32);
        &d[std::cmp::min(3, d.len() - 1)]
    }

    let name = hom_lift.name();
    // Iterate through the multiplicand
    for b in unit.iter_nonzero_stem() {
        // The potential target has to be hit, and we need to have computed (the data need for) the
        // d2 that hits the potential target.
        if !resolution.has_computed_bidegree(b + shift + LAMBDA_BIDEGREE) {
            continue;
        }
        if !resolution.has_computed_bidegree(b + shift - Bidegree::s_t(1, 0)) {
            continue;
        }

        let page_data = get_page_data(unit_sseq.as_ref(), b);

        let target_num_gens = resolution.number_of_gens_in_bidegree(b + shift);
        let lambda_num_gens = resolution.number_of_gens_in_bidegree(b + shift + LAMBDA_BIDEGREE);

        if target_num_gens == 0 && lambda_num_gens == 0 {
            continue;
        }

        // First print the products with non-surviving classes
        if target_num_gens > 0 {
            let hom_k = hom.get_map((b + shift).s()).hom_k(b.t());
            for i in page_data.complement_pivots() {
                let gen = BidegreeGenerator::new(b, i);
                println!("{name} λ x_{gen} = λ {:?}", &hom_k[i]);
            }
        }

        // Now print the secondary products
        if page_data.subspace_dimension() == 0 {
            continue;
        }

        let mut outputs = vec![
            FpVector::new(p, target_num_gens + lambda_num_gens);
            page_data.subspace_dimension()
        ];

        hom_lift.hom_k(
            Some(&res_sseq),
            b,
            page_data.subspace_gens(),
            outputs.iter_mut().map(FpVector::as_slice_mut),
        );
        for (gen, output) in page_data.subspace_gens().zip_eq(outputs) {
            println!(
                "{name} [{basis_string}] = {} + λ {}",
                output.slice(0, target_num_gens),
                output.slice(target_num_gens, target_num_gens + lambda_num_gens),
                basis_string = BidegreeElement::new(b, gen.to_owned()).to_basis_string(),
            );
        }
    }
    Ok(())
}
