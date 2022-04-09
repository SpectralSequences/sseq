//! Resolves an unstable module up to an $(n, s)$ and prints an ASCII depiction of the Ext groups:

use ext::chain_complex::FreeChainComplex;

fn main() -> anyhow::Result<()> {
    let res = ext::utils::query_unstable_module(false)?;

    let max_n = query::raw("Max n", str::parse);
    let max_s = query::raw("Max s", str::parse);

    res.compute_through_stem(max_s, max_n);

    println!("{}", res.graded_dimension_string());

    Ok(())
}
