//! This example uses secondary.rs to compute d_2 on x_{65, 4}. The code is similar to that in the
//! secondary command, but with hardcoded values. I also use this for performance benchmarking.

use ext::chain_complex::ChainComplex;
use ext::secondary::*;
use ext::utils::construct;
use sseq::coordinates::Bidegree;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    // Attempt to load a resolution of S_2 from resolution_milnor.save, and generates one from
    // scratch if it isn't available.
    let save_file = std::path::PathBuf::from("S_2_milnor");
    let save_file = if save_file.exists() {
        Some(save_file)
    } else {
        None
    };
    let resolution = construct("S_2@milnor", save_file).unwrap();

    // Compute the minimal resolution R_{s, t}
    resolution.compute_through_bidegree(Bidegree::s_t(6, 70));

    let start = Instant::now();
    let lift = SecondaryResolution::new(Arc::new(resolution));
    lift.initialize_homotopies();
    lift.compute_composites();
    lift.compute_homotopies();

    println!("Time elapsed: {:.2?}", start.elapsed());

    // We can now get the d_2 starting at (65, 4).
    let output = lift.homotopy(6).homotopies.hom_k(69);

    println!("d_2 x_(65, 4) = {:?}", output[0]);
}
