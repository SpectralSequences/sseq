use ext::{chain_complex::ChainComplex, utils::construct};
use sseq::coordinates::Bidegree;

fn benchmark(module_name: &str, max_degree: i32, algebra: &str, n_times: u128) {
    let max = Bidegree::s_t(max_degree as u32, max_degree);
    for _ in 0..n_times {
        let res = construct((module_name, algebra), None).unwrap();
        res.compute_through_bidegree(max);
        assert!(
            res.module(max_degree as u32)
                .number_of_gens_in_degree(max_degree)
                < 1000
        );
    }
}

fn resolve_adem_2_60() {
    benchmark("S_2", 60, "adem", 1);
}

fn resolve_milnor_2_60() {
    benchmark("S_2", 60, "milnor", 1);
}

fn resolve_adem_3_120() {
    benchmark("S_3", 120, "adem", 1);
}

fn resolve_milnor_3_120() {
    benchmark("S_3", 120, "milnor", 1);
}

fn resolve_adem_5_200() {
    benchmark("S_5", 200, "adem", 1);
}

fn resolve_milnor_5_200() {
    benchmark("S_5", 200, "milnor", 1);
}

iai::main!(
    resolve_adem_2_60,
    resolve_milnor_2_60,
    resolve_adem_3_120,
    resolve_milnor_3_120,
    resolve_adem_5_200,
    resolve_milnor_5_200
);
