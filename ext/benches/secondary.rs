/// This example uses secondary.rs to compute d_2 on x_{65, 4}. The code is similar to that in the
/// secondary command, but with hardcoded values. I also use this for performance benchmarking.
use ext::load_s_2;
use ext::secondary::compute_delta;
use std::time::Instant;

fn main() {
    // This macro attempts to load a resolution of S_2 from resolution.save, and generates one from
    // scratch if it isn't available. The result is written to the variable `resolution`.
    load_s_2!(resolution, "milnor", "resolution.save");

    // Compute the minimal resolution R_{s, t}
    resolution.resolve_through_bidegree(6, 70);

    let start = Instant::now();
    // deltas is a vector of FreeModuleHomomorphisms R_{s, t} -> R_{s - 2, t - 1} that is dual to
    // the d_2 map. The vector is indexed by s with the first entry being s = 3.
    let deltas = compute_delta(&resolution, 6, 70);
    println!("Time elapsed: {:.2?}", start.elapsed());

    // We can now get the matrix of the d_2 starting at (65, 4).
    let output = deltas[6 - 3].hom_k(69);

    // dim R_{65, 4} = 1 and the generator is the last basis element.
    println!("d_2 x_{{65, 4}} = {}", output[0][0]);
}
