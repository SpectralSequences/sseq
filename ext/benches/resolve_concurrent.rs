#[cfg(feature = "concurrent")]
fn benchmark(algebra: &str) {
    use std::{io::Write, time::Instant};

    use ext::{
        chain_complex::{ChainComplex, FreeChainComplex},
        utils::construct,
    };
    use sseq::coordinates::Bidegree;

    let resolution = construct(("S_2", algebra), None).unwrap();
    let b = Bidegree::s_t(80, 80);

    print!("benchmark  {:6}  S_2  80:    ", algebra,);
    std::io::stdout().flush().unwrap();

    let start = Instant::now();
    resolution.compute_through_bidegree(b);
    let dur = start.elapsed();

    assert!(resolution.number_of_gens_in_bidegree(b) < 1000);

    println!("{} ms / iter", dur.as_millis());
}

#[cfg(not(feature = "concurrent"))]
fn benchmark(_algebra: &str) {}

fn main() {
    benchmark("adem");
    benchmark("milnor");
}
