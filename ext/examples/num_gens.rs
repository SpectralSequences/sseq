//! This prints the number of generators in each $\Ext^{s, n + s}$ in the format `n,s,num_gens`.
//!
use ext::chain_complex::ChainComplex;
use ext::utils::query_module;

fn main() -> anyhow::Result<()> {
    let resolution = query_module(None, false)?;

    for b in resolution.iter_stem() {
        println!(
            "{},{},{}",
            b.n(),
            b.s(),
            resolution.module(b.s()).number_of_gens_in_degree(b.t())
        );
    }
    Ok(())
}
