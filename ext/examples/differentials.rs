use ext::chain_complex::ChainComplex;
/// This is a simple script to print all the differentials in the resolution.
use ext::utils::{construct, iter_stems};

fn main() {
    let resolution = query::with_default("Module", "S_2", |name: String| {
        construct(&*name, None).map_err(|e| e.to_string())
    });

    let max_s = query::with_default("Max s", "15", Ok);
    let max_t = query::with_default("Max t", "30", Ok);

    resolution.compute_through_bidegree(max_s, max_t);

    for (s, f, t) in iter_stems(max_s, max_t) {
        for i in 0..resolution.module(s).number_of_gens_in_degree(t) {
            let cocycle = resolution.cocycle_string(s, t, i);
            println!("d x_{{{},{},{}}} = {}", f, s, i, cocycle);
        }
    }
}
