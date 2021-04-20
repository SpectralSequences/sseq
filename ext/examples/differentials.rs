/// This is a simple script to print all the differentials in the resolution.
use ext::chain_complex::ChainComplex;
use ext::utils::query_module;

fn main() -> error::Result<()> {
    let resolution = query_module(None)?.resolution;

    for (s, f, t) in resolution.iter_stem() {
        for i in 0..resolution.module(s).number_of_gens_in_degree(t) {
            let cocycle = resolution.cocycle_string(s, t, i);
            println!("d x_{{{},{},{}}} = {}", f, s, i, cocycle);
        }
    }
    Ok(())
}
