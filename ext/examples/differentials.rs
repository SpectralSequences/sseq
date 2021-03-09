/// This is a simple script to print all the differentials in the resolution.
use ext::load_s_2;

const MAX_S: u32 = 6;
const MAX_T: i32 = 70;

fn main() {
    load_s_2!(resolution, "milnor", "resolution.save");

    resolution.resolve_through_bidegree(MAX_S, MAX_T);

    for f in 0..=MAX_T {
        for s in 0..=MAX_S {
            let t = f + s as i32;
            if t > MAX_T {
                break;
            }
            for i in 0..resolution.module(s).number_of_gens_in_degree(t) {
                println!(
                    "d x_{{{},{},{}}} = {}",
                    f,
                    s,
                    i,
                    resolution.inner.cocycle_string(s, t, i)
                );
            }
        }
    }
}
