use ext::chain_complex::ChainComplex;
/// This is a simple script to print all the differentials in the resolution.
use ext::{load_s_2, utils::iter_stems};

const MAX_S: u32 = 6;
const MAX_T: i32 = 70;

fn main() {
    load_s_2!(resolution, "milnor", "resolution.save");

    resolution.resolve_through_bidegree(MAX_S, MAX_T);

    for (s, f, t) in iter_stems(MAX_S, MAX_T) {
        for i in 0..resolution.module(s).number_of_gens_in_degree(t) {
            let cocycle = resolution.cocycle_string(s, t, i);
            println!("d x_{{{},{},{}}} = {}", f, s, i, cocycle);
        }
    }
}
