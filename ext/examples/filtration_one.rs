//! This computes all available filtration one products for a module. This only works at the prime
//! 2 for the moment.
//!
//! We omit outputs where the target bidegree is zero (or not yet computed)

use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils::query_module;

fn main() -> anyhow::Result<()> {
    let resolution = query_module(None, false)?.resolution;
    assert_eq!(*resolution.prime(), 2);

    for (s, n, t) in resolution.iter_stem() {
        let mut i = 0;
        while let Some(products) = resolution.filtration_one_product(1 << i, 0, s + 1, t + (1 << i))
        {
            for (idx, row) in products.into_iter().enumerate() {
                if !row.is_empty() {
                    println!("h_{} x_({}, {}, {}) = {:?}", i, n, s, idx, row);
                }
            }
            i += 1;
        }
    }
    Ok(())
}
