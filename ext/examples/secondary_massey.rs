//! Computes massey products in $\Mod_{C\lambda^2}$.
//!
//! # Usage
//! This computes all Massey products of the form $\langle -, b, a\rangle$, where $a \in \Ext^{\*,
//! \*}(M, k)$ and $b, (-) \in \Ext^{\*, \*}(k, k)$. It does not verify that the Massey product is
//! valid, i.e. $a$ and $b$ both lift to $\Mod_{C\lambda^2}$ and have trivial product.
//!
//! Since we must choose $a$ and $b$ to have trivial product, it is necessary to be able to specify
//! the $\lambda$ part of them, and not insist that they are standard lifts of the $\Ext$ classes.
//! Thus, the user is first prompted for the $\Ext$ part, then the $\lambda$ part of each class. To
//! set a part to zero, supply an empty name. Note that if the bidegree right above the class is
//! empty, the user is not prompted for the $\lambda$ part.
//!
//! # Output
//! This computes the Massey products up to a sign. We write our output in the category
//! $\Mod_{C\lambda^2}$, so the format is $\langle a, b, -\rangle$ instead of $\langle -, b,
//! a\rangle$. Brave souls are encouraged to figure out the correct sign for the products.
//!
//! The heavy lifting is done by [`SecondaryExtModule::secondary_massey`]; this program is just the
//! interactive front-end (querying the classes and formatting the output).

use std::sync::Arc;

use algebra::module::Module;
use ext::{
    chain_complex::ChainComplex,
    ext_algebra::{
        ExtModule,
        secondary::{SecondaryClass, SecondaryExtModule},
    },
    secondary::LAMBDA_BIDEGREE,
    utils::query_module,
};
use fp::vector::FpVector;
use itertools::Itertools;
use sseq::coordinates::{Bidegree, BidegreeElement};

/// Query the $\Ext$ and $\lambda$ coordinates of a $\Mod_{C\lambda^2}$ class at bidegree `degree`,
/// returning the class together with its display name. `ext_dim` / `lambda_dim` are the dimensions
/// of $\Ext$ at `degree` and `degree + LAMBDA_BIDEGREE`.
fn query_class(
    name: &str,
    degree: Bidegree,
    ext_dim: usize,
    lambda_dim: usize,
    p: fp::prime::ValidPrime,
) -> (SecondaryClass, String) {
    let ext_name: String = query::raw(&format!("Name of Ext part of {name}"), str::parse);
    let mut ext = FpVector::new(p, ext_dim);
    if !ext_name.is_empty() {
        if ext_dim == 0 {
            eprintln!("No classes in this bidegree");
        } else {
            let v: Vec<u32> = query::vector(&format!("Input Ext class {ext_name}"), ext_dim);
            for (i, &x) in v.iter().enumerate() {
                ext.set_entry(i, x);
            }
        }
    }

    let mut lambda = FpVector::new(p, lambda_dim);
    let lambda_name = if lambda_dim > 0 {
        let ln: String = query::raw(&format!("Name of λ part of {name}"), str::parse);
        if !ln.is_empty() {
            let v: Vec<u32> = query::vector(&format!("Input Ext class {ln}"), lambda_dim);
            for (i, &x) in v.iter().enumerate() {
                lambda.set_entry(i, x);
            }
        }
        ln
    } else {
        String::new()
    };

    let display = match (&*ext_name, &*lambda_name) {
        ("", "") => panic!("Do not compute zero Massey product"),
        ("", x) => format!("λ{x}"),
        (x, "") => format!("[{x}]"),
        (x, y) => format!("[{x}] + λ{y}"),
    };

    (SecondaryClass::new(degree, ext, lambda), display)
}

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    eprintln!(
        "We are going to compute <-, b, a> for all (-), where a is an element in Ext(M, k) and b \
         and (-) are elements in Ext(k, k)."
    );

    let resolution = Arc::new(query_module(Some(algebra::AlgebraType::Milnor), true)?);
    let module = Arc::new(ExtModule::from_resolution(Arc::clone(&resolution))?);
    let p = resolution.prime();

    // `a ∈ Ext(M, k)`.
    let a_deg = Bidegree::n_s(
        query::raw("n of a", str::parse),
        query::raw("s of a", str::parse),
    );
    module
        .resolution()
        .compute_through_stem(a_deg + LAMBDA_BIDEGREE);
    let (a, a_name) = query_class(
        "a",
        a_deg,
        module.dimension(a_deg),
        module.dimension(a_deg + LAMBDA_BIDEGREE),
        p,
    );

    // `b ∈ Ext(k, k)`.
    let b_deg = Bidegree::n_s(
        query::raw("n of b", str::parse),
        query::raw("s of b", str::parse),
    );
    module
        .algebra()
        .resolution()
        .compute_through_stem(b_deg + LAMBDA_BIDEGREE);
    let (b, b_name) = query_class(
        "b",
        b_deg,
        module.algebra().dimension(b_deg),
        module.algebra().dimension(b_deg + LAMBDA_BIDEGREE),
        p,
    );

    // Ensure the unit is resolved far enough to support the brackets.
    if !module.is_unit() {
        let res_max = Bidegree::n_s(
            resolution.module(0).max_computed_degree(),
            resolution.next_homological_degree() - 1,
        );
        module
            .algebra()
            .resolution()
            .compute_through_stem(res_max - a_deg);
    }

    let sec = SecondaryExtModule::from_module(Arc::clone(&module));
    sec.extend_all();

    let results = sec.secondary_massey(&a, &b, ext::utils::secondary_job());

    for r in results {
        print!("<{a_name}, {b_name}, ");

        let has_ext = !r.multiplicand.is_zero();
        if has_ext {
            print!(
                "[{basis_string}]",
                basis_string =
                    BidegreeElement::new(r.degree, r.multiplicand.clone()).to_basis_string()
            );
        }

        let num_lambda = r.multiplicand_lambda.iter_nonzero().count();
        if num_lambda > 0 {
            if has_ext {
                print!(" + ");
            }
            print!("λ");
            let basis_string =
                BidegreeElement::new(r.degree + LAMBDA_BIDEGREE, r.multiplicand_lambda.clone())
                    .to_basis_string();
            if num_lambda == 1 {
                print!("{basis_string}");
            } else {
                print!("({basis_string})");
            }
        }

        print!("> = ±");
        print!("[{}]", r.ext_part.iter().format(", "));
        println!(" + λ{}", r.lambda_part);
    }

    Ok(())
}
