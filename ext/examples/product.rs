//! Computes products in Ext by left-multiplication by a fixed class.
//!
//! The program asks for a module `M` and a class `x ∈ Ext(M, k)`. It then prints the products of
//! `x` with every basis class of `Ext(k, k)` that lands in a computed bidegree.
//!
//! This is the primary (i.e. non-secondary) analogue of [`secondary_product`](../secondary_product),
//! written against the [`ExtModule`] abstraction so the plumbing stays out of the way.

use std::sync::Arc;

use algebra::module::Module;
use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    ext_algebra::ExtModule,
    utils::query_module,
};
use sseq::coordinates::Bidegree;

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let resolution = Arc::new(query_module(None, true)?);
    let e2 = ExtModule::from_resolution(Arc::clone(&resolution))?;

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

    // `query_module` resolves only `M`; a separate unit must be resolved far enough to support
    // the products.
    if !e2.is_unit() {
        let res_max = Bidegree::n_s(
            resolution.module(0).max_computed_degree(),
            resolution.next_homological_degree() - 1,
        );
        e2.algebra()
            .resolution()
            .compute_through_stem(res_max - shift);
    }

    for b in e2.algebra().resolution().iter_nonzero_stem() {
        // `None` means `b + shift` is out of the computed range, so skip it.
        let Some(rows) = e2.multiply_into(&x, b) else {
            continue;
        };
        for (g, row) in e2.algebra().basis(b).into_iter().zip(rows.iter()) {
            let coords: Vec<u32> = row.iter().collect();
            if coords.iter().any(|&c| c != 0) {
                println!("x · x_{g} = {coords:?}");
            }
        }
    }
    Ok(())
}
