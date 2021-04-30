//! This prints the number of generators in each $\Ext^{s, n + s}$ in the format `n,s,num_gens`.
//!
use ext::chain_complex::ChainComplex;
use ext::utils::query_module;

fn main() -> error::Result {
    let resolution = query_module(None)?.resolution;

    for (s, n, t) in resolution.iter_stem() {
        println!(
            "{},{},{}",
            n,
            s,
            resolution.module(s).number_of_gens_in_degree(t)
        );
    }
    Ok(())
}
