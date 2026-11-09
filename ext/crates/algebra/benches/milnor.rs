//! Benchmarks for the low-level Milnor `PPartMultiplier` kernel.

use algebra::milnor_algebra::{PPart, PPartAllocation, PPartMultiplier};
use criterion::{
    BenchmarkGroup, Criterion, criterion_group, criterion_main, measurement::WallTime,
};
use fp::prime::ValidPrime;
use pprof::criterion::{Output, PProfProfiler};

fn bench_ppart<const MOD4: bool>(
    g: &mut BenchmarkGroup<WallTime>,
    name: &str,
    p: u32,
    r: PPart,
    s: PPart,
) {
    let p = ValidPrime::new(p);
    g.bench_function(name, |bench| {
        // Hoist the allocation into the (untimed) setup so only the multiplier's iteration is
        // measured; `black_box` the yielded coefficients so the loop can't be optimized away.
        bench.iter_batched(
            PPartAllocation::default,
            |alloc| {
                let m = PPartMultiplier::<MOD4>::new_from_allocation(p, r, s, alloc, 0, 0);
                for c in m {
                    std::hint::black_box(c);
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn ppart(c: &mut Criterion) {
    let mut g = c.benchmark_group("milnor_ppart");

    bench_ppart::<false>(
        &mut g,
        "ppart_2/a",
        2,
        PPart::from_slice(&[60, 30, 8, 2, 1]),
        PPart::from_slice(&[20, 30, 20, 4, 1, 2]),
    );
    bench_ppart::<false>(
        &mut g,
        "ppart_2/b",
        2,
        PPart::from_slice(&[35, 12, 20, 14, 1, 3]),
        PPart::from_slice(&[60, 30, 0, 2, 1]),
    );

    bench_ppart::<true>(
        &mut g,
        "ppart_4/a",
        2,
        PPart::from_slice(&[60, 30, 8, 2, 1]),
        PPart::from_slice(&[20, 30, 20, 4, 1, 2]),
    );
    bench_ppart::<true>(
        &mut g,
        "ppart_4/b",
        2,
        PPart::from_slice(&[35, 12, 20, 14, 1, 3]),
        PPart::from_slice(&[60, 30, 0, 2, 1]),
    );

    #[cfg(feature = "odd-primes")]
    {
        bench_ppart::<false>(
            &mut g,
            "ppart_3/a",
            3,
            PPart::from_slice(&[120, 70, 40, 2]),
            PPart::from_slice(&[60, 35, 21, 6]),
        );
        bench_ppart::<false>(
            &mut g,
            "ppart_3/b",
            3,
            PPart::from_slice(&[30, 12, 35, 24]),
            PPart::from_slice(&[100, 80, 16, 2, 3]),
        );
    }

    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(std::time::Duration::from_secs(3))
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = ppart
}
criterion_main!(benches);
