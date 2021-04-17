#[cfg(feature = "concurrent")]
fn benchmark(algebra: &str) {
    use ext::chain_complex::ChainComplex;
    use ext::utils::construct_s_2;
    use std::io::Write;
    use std::time::Instant;
    use thread_token::TokenBucket;

    let resolution = construct_s_2::<&str>(algebra, None);
    let bucket = TokenBucket::new(3);

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
