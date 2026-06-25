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
    ext_algebra::{ExtAlgebra, secondary::SecondaryExtAlgebra},
    secondary::{LAMBDA_BIDEGREE, SecondaryLift},
    utils::query_module,
};
use sseq::coordinates::{Bidegree, BidegreeGenerator};

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let resolution = Arc::new(query_module(Some(algebra::AlgebraType::Milnor), true)?);
    let e2 = Arc::new(ExtAlgebra::from_resolution(Arc::clone(&resolution))?);

    let name: String = query::raw("Name of product", str::parse);
    let shift = Bidegree::n_s(
        query::raw(&format!("n of Ext class {name}"), str::parse),
        query::raw(&format!("s of Ext class {name}"), str::parse),
    );

    let dim = e2.dimension(shift);
    if dim == 0 {
        panic!("No classes in this bidegree");
    }
    let v: Vec<u32> = query::vector("Input ext class", dim);
    let x = e2.element(shift, &v);

    // Ensure the unit is resolved far enough to support the products.
    if !e2.is_unit() {
        let res_max = Bidegree::n_s(
            resolution.module(0).max_computed_degree(),
            resolution.next_homological_degree() - 1,
        );
        e2.unit().compute_through_stem(res_max - shift);
    }

    let sec_e2 = Arc::new(SecondaryExtAlgebra::new(Arc::clone(&e2)));
    sec_e2.extend_all();

    // Check that the class survives to E3 (supports no d2).
    assert!(sec_e2.survives(&x), "Class supports a non-zero d2");

    let lift = sec_e2.secondary_product_lift(&x);

    if let Some(s) = ext::utils::secondary_job() {
        lift.underlying().extend_all();
        lift.compute_partial(s);
        return Ok(());
    }

    // `x` multiplies on the left; the printed name is bracketed as in the original output.
    let disp = format!("[{name}]");

    // Iterate through the multiplicand.
    for b in e2.unit().iter_nonzero_stem() {
        // The potential target has to be hit, and we need to have computed (the data needed for)
        // the d2 that hits the potential target.
        if !resolution.has_computed_bidegree(b + shift + LAMBDA_BIDEGREE) {
            continue;
        }
        if !resolution.has_computed_bidegree(b + shift - Bidegree::s_t(1, 0)) {
            continue;
        }

        let target_num_gens = e2.dimension(b + shift);
        let lambda_num_gens = e2.dimension(b + shift + LAMBDA_BIDEGREE);
        if target_num_gens == 0 && lambda_num_gens == 0 {
            continue;
        }

        let page = sec_e2.unit_page_data(b);

        // First the products with non-surviving classes: these are just λ times the (primary)
        // product, read off the multiplication map.
        if target_num_gens > 0
            && let Some(rows) = e2.multiply_into(&x, b)
        {
            for i in page.complement_pivots() {
                let g = BidegreeGenerator::new(b, i);
                let entry: Vec<u32> = rows.row(i).iter().collect();
                println!("{disp} λ x_{g} = λ {entry:?}");
            }
        }

        // Now the genuinely secondary products with surviving classes.
        for prod in sec_e2.secondary_multiply_into(&x, b) {
            println!(
                "{disp} [{basis}] = {ext} + λ {lambda}",
                basis = prod.source.to_basis_string(),
                ext = prod.ext_part.as_slice(),
                lambda = prod.lambda_part.as_slice(),
            );
        }
    }

    Ok(())
}
