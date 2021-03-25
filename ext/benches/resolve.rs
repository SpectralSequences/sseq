use ext::utils::{construct, Config};
use std::io::Write;
use std::time::Instant;

fn benchmark(module_name: &str, max_degree: i32, algebra: &str, n_times: u128) {
    let path = std::path::PathBuf::from("steenrod_modules");
    let cfg = Config {
        module_paths: vec![path],
        module_file_name: module_name.to_string(),
        max_degree,
        algebra_name: String::from(algebra),
    };

    print!(
        "benchmark  {:6}  {}  {}:    ",
        algebra, module_name, max_degree
    );
    std::io::stdout().flush().unwrap();

    let start = Instant::now();
    for _ in 0..n_times {
        let res = construct(&cfg).unwrap();
        res.resolve_through_degree(max_degree);
        assert!(
            res.module(max_degree as u32)
                .number_of_gens_in_degree(max_degree)
                < 1000
        );
    }
    let dur = start.elapsed();

    println!("{} ms / iter", dur.as_millis() / n_times as u128);
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
