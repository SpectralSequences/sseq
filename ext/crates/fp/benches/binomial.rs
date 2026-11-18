//! Throughput of `binomial4`, for comparing implementations with criterion baselines.

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use fp::prime::Binomial;
use pprof::criterion::{Output, PProfProfiler};
use rand::{Rng, SeedableRng, rngs::StdRng};

/// Number of `(n, k)` pairs evaluated per iteration.
///
/// Large enough that the branch predictor cannot learn the batch, which otherwise flatters
/// branchy implementations on inputs with mixed outcomes.
const BATCH: usize = 1 << 18;

/// Exclusive upper bound on `n`, so that every input fits in every implementor of [`Binomial`].
const MAX_N: u32 = u16::MAX as u32 + 1;

/// Draws the input regimes: small `n` with mixed outcomes, uniform `k <= n` for large `n` (mostly
/// even coefficients), and `k` a bit-subset of `n` (always odd).
fn inputs() -> [(&'static str, Vec<(u32, u32)>); 3] {
    let mut rng = StdRng::seed_from_u64(0x5eed);
    let mut draw = |f: &dyn Fn(&mut StdRng) -> (u32, u32)| -> Vec<(u32, u32)> {
        (0..BATCH).map(|_| f(&mut rng)).collect()
    };
    [
        (
            "small",
            draw(&|r| {
                let n = r.random_range(0..50);
                (n, r.random_range(0..=n))
            }),
        ),
        (
            "large_uniform",
            draw(&|r| {
                let n = r.random_range(1 << 10..MAX_N);
                (n, r.random_range(0..=n))
            }),
        ),
        (
            "large_odd",
            draw(&|r| {
                let n = r.random_range(1 << 10..MAX_N);
                (n, n & r.random::<u32>())
            }),
        ),
    ]
}

/// Benchmarks [`Binomial::binomial4`] for `T` over every input regime.
fn bench_binomial4<T: Binomial + Copy + TryFrom<u32>>(c: &mut Criterion, type_name: &str) {
    let mut group = c.benchmark_group(format!("binomial4/{type_name}"));
    group.throughput(Throughput::Elements(BATCH as u64));
    for (name, pairs) in inputs() {
        let pairs: Vec<(T, T)> = pairs
            .into_iter()
            .map(|(n, k)| match (T::try_from(n), T::try_from(k)) {
                (Ok(n), Ok(k)) => (n, k),
                _ => unreachable!("inputs are below MAX_N"),
            })
            .collect();
        group.bench_function(name, |b| {
            b.iter(|| {
                for &(n, k) in &pairs {
                    black_box(T::binomial4(black_box(n), black_box(k)));
                }
            })
        });
    }
    group.finish();
}

/// Benchmarks [`Binomial::binomial4`] for every implementor.
fn bench_binomial4_all(c: &mut Criterion) {
    bench_binomial4::<u16>(c, "u16");
    bench_binomial4::<u32>(c, "u32");
    bench_binomial4::<i32>(c, "i32");
}

criterion_group! {
    name = binomial;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_binomial4_all
}

criterion_main!(binomial);
