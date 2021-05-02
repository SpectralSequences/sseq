#[cfg(feature = "concurrent")]
fn benchmark(algebra: &str) {
    use core::num::NonZeroUsize;
    use ext::chain_complex::ChainComplex;
    use ext::utils::construct;
    use std::io::Write;
    use std::time::Instant;
    use thread_token::TokenBucket;

    let resolution = construct(("S_2", algebra), None).unwrap();
    let bucket = TokenBucket::new(NonZeroUsize::new(3).unwrap());

    print!("benchmark  {:6}  S_2  80:    ", algebra,);
    std::io::stdout().flush().unwrap();

    let start = Instant::now();
    resolution.compute_through_bidegree_concurrent(80, 80, &bucket);
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
