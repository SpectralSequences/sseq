//! This prints all the differentials in the resolution.

use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    utils::query_module,
};
use sseq::coordinates::BidegreeGenerator;

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging();

    let resolution = query_module(None, false)?;

    for b in resolution.iter_stem() {
        for i in 0..resolution.number_of_gens_in_bidegree(b) {
            let g = BidegreeGenerator::new(b, i);
            let boundary = resolution.boundary_string(g, true);
            println!("d x_{g:#} = {boundary}");
        }
    }
    Ok(())
}
