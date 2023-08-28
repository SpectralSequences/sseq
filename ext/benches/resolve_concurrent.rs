#[cfg(feature = "concurrent")]
fn benchmark(algebra: &str) {
    use ext::chain_complex::ChainComplex;
    use ext::utils::construct;
    use sseq::coordinates::Bidegree;
    use std::io::Write;
    use std::time::Instant;

    let resolution = construct(("S_2", algebra), None).unwrap();

    print!("benchmark  {:6}  S_2  80:    ", algebra,);
    std::io::stdout().flush().unwrap();

    let start = Instant::now();
    resolution.compute_through_bidegree(Bidegree::s_t(80, 80));
    let dur = start.elapsed();

    assert!(resolution.module(80).number_of_gens_in_degree(80) < 1000);

    println!("{} ms / iter", dur.as_millis());
}

#[cfg(not(feature = "concurrent"))]
fn benchmark(_algebra: &str) {}

fn main() {
    benchmark("adem");
    benchmark("milnor");
}
