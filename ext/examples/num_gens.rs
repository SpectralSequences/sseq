//! This prints the number of generators in each $\Ext^{s, n + s}$ in the format `n,s,num_gens`.
use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    utils::query_module,
};

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let resolution = query_module(None, false)?;

    for b in resolution.iter_stem() {
        println!(
            "{},{},{}",
            b.n(),
            b.s(),
            resolution.number_of_gens_in_bidegree(b)
        );
    }
    Ok(())
}
