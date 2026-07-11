//! Benchmarks for algebra multiplication.
//!
//! Exercises both the Adem and Milnor bases at primes 2, 3 and 5 (odd primes require the
//! `odd-primes` feature). See `common::bench_algebra_multiplication` for what is measured.

mod common;

use algebra::{AdemAlgebra, MilnorAlgebra};
use criterion::{Criterion, criterion_group, criterion_main};
use fp::prime::ValidPrime;
use pprof::criterion::{Output, PProfProfiler};

/// Benchmark both bases at prime `p` over the given `(r_degree, s_degree)` pairs. Degrees that turn
/// out to be empty for a basis are skipped by the helper.
fn bench_prime(c: &mut Criterion, p: u32, pairs: &[(i32, i32)]) {
    let prime = ValidPrime::new(p);

    let adem = AdemAlgebra::new(prime, false);
    let mut g = c.benchmark_group(format!("multiplication/p{p}"));
    common::bench_algebra_multiplication(&mut g, "adem", &adem, pairs);
    let milnor = MilnorAlgebra::new(prime, false);
    common::bench_algebra_multiplication(&mut g, "milnor", &milnor, pairs);
    g.finish();
}

fn multiplication(c: &mut Criterion) {
    bench_prime(c, 2, &[(8, 8), (16, 16), (24, 24), (16, 8)]);
    #[cfg(feature = "odd-primes")]
    {
        // Odd-prime degrees are spaced by q = 2(p - 1), so we pick multiples of q.
        bench_prime(c, 3, &[(8, 8), (12, 12), (16, 16), (12, 8)]);
        bench_prime(c, 5, &[(8, 8), (16, 16), (24, 24), (16, 8)]);
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(std::time::Duration::from_secs(3))
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = multiplication
}
criterion_main!(benches);
