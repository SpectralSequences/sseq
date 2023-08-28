//! This prints all the differentials in the resolution.

use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils::query_module;
use sseq::coordinates::BidegreeGenerator;

fn main() -> anyhow::Result<()> {
    let resolution = query_module(None, false)?;

    for b in resolution.iter_stem() {
        for i in 0..resolution.module(b.s()).number_of_gens_in_degree(b.t()) {
            let gen = BidegreeGenerator::new(b, i);
            let cocycle = resolution.cocycle_string(gen, true);
            println!("d x_{gen:#} = {cocycle}");
        }
    }
    Ok(())
}
