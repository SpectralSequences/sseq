//! Computes products in Ext by left-multiplication by a fixed class.
//!
//! The program asks for a module `M` and a class `x ∈ Ext(M, k)`. It then prints the products of
//! `x` with every basis class of `Ext(k, k)` that lands in a computed bidegree.
//!
//! This is the primary (i.e. non-secondary) analogue of [`secondary_product`](../secondary_product),
//! written against the [`ExtAlgebra`] abstraction so the plumbing stays out of the way.

use std::sync::Arc;

use ext::{chain_complex::FreeChainComplex, ext_algebra::ExtAlgebra, utils::query_module};
use sseq::coordinates::Bidegree;

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let resolution = Arc::new(query_module(None, true)?);
    let e2 = ExtAlgebra::from_resolution(resolution)?;

    let shift = Bidegree::n_s(
        query::raw("n of Ext class", str::parse),
        query::raw("s of Ext class", str::parse),
    );

    let dim = e2.dimension(shift);
    if dim == 0 {
        panic!("No classes in bidegree {shift}");
    }
    let v: Vec<u32> = query::vector("Input Ext class", dim);
    let x = e2.element(shift, &v);

    for b in e2.unit().iter_nonzero_stem() {
        // `None` means `b + shift` is out of the computed range, so skip it.
        let Some(rows) = e2.multiply_into(&x, b) else {
            continue;
        };
        for (g, row) in e2.unit_basis(b).into_iter().zip(rows.iter()) {
            let coords: Vec<u32> = row.iter().collect();
            if coords.iter().any(|&c| c != 0) {
                println!("x · x_{g} = {coords:?}");
            }
        }
    }
    Ok(())
}
