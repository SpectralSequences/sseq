//! This computes all available filtration one products for a module. This only works at the prime
//! 2 for the moment.
//!
//! We omit outputs where the target bidegree is zero (or not yet computed)

use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils::query_module;
use sseq::coordinates::{Bidegree, BidegreeGenerator};

fn main() -> anyhow::Result<()> {
    let resolution = query_module(None, false)?;
    assert_eq!(*resolution.prime(), 2);

    for b in resolution.iter_stem() {
        let mut i = 0;
        while resolution.has_computed_bidegree(b + Bidegree::s_t(1, 1 << i)) {
            // TODO: This doesn't work with the reordered Adams basis
            let products = resolution.filtration_one_product(1 << i, 0, b).unwrap();
            for (idx, row) in products.into_iter().enumerate() {
                let gen = BidegreeGenerator::new(b, idx);
                if !row.is_empty() {
                    println!("h_{i} x_{gen} = {row:?}");
                }
            }
            i += 1;
        }
    }
    Ok(())
}
