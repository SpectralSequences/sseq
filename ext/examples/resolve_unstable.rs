//! Resolves an unstable module up to an $(n, s)$ and prints an ASCII depiction of the Ext groups:

use ext::chain_complex::FreeChainComplex;
use sseq::coordinates::Bidegree;

fn main() -> anyhow::Result<()> {
    let res = ext::utils::query_unstable_module(false)?;

    let max = Bidegree::n_s(
        query::raw("Max n", str::parse),
        query::raw("Max s", str::parse),
    );

    res.compute_through_stem(max);

    println!("{}", res.graded_dimension_string());

    Ok(())
}
