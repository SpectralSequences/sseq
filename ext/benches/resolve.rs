use ext::chain_complex::ChainComplex;
use ext::utils::construct;
use sseq::coordinates::Bidegree;
use std::io::Write;
use std::time::Instant;

fn benchmark(module_name: &str, max_degree: i32, algebra: &str, n_times: u128) {
    print!("benchmark  {algebra:6}  {module_name}  {max_degree}:    ");
    std::io::stdout().flush().unwrap();

    let max = Bidegree::s_t(max_degree as u32, max_degree);
    let start = Instant::now();
    for _ in 0..n_times {
        let res = construct((module_name, algebra), None).unwrap();
        res.compute_through_bidegree(max);
        assert!(
            res.module(max_degree as u32)
                .number_of_gens_in_degree(max_degree)
                < 1000
        );
    }
    let dur = start.elapsed();

    println!("{} ms / iter", dur.as_millis() / n_times);
}

fn benchmark_pair(module_name: &str, max_degree: i32, n_times: u128) {
    println!();
    benchmark(module_name, max_degree, "adem", n_times);
    benchmark(module_name, max_degree, "milnor", n_times);
}

fn main() {
    benchmark_pair("S_2", 60, 3);
    benchmark_pair("S_2", 70, 1);

    #[cfg(feature = "odd-primes")]
    benchmark_pair("S_3", 120, 3);

    #[cfg(feature = "odd-primes")]
    benchmark_pair("S_5", 200, 3);
}
