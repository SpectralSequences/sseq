//! Benchmarks for the low-level Milnor `PPartMultiplier` kernel.

use algebra::milnor_algebra::{PPartAllocation, PPartEntry, PPartMultiplier};
use criterion::{
    BenchmarkGroup, Criterion, criterion_group, criterion_main, measurement::WallTime,
};
use fp::prime::{Prime, ValidPrime};
use pprof::criterion::{Output, PProfProfiler};

fn bench_ppart<const MOD4: bool>(
    g: &mut BenchmarkGroup<WallTime>,
    name: &str,
    p: u32,
    r: Vec<PPartEntry>,
    s: Vec<PPartEntry>,
) {
    let p = ValidPrime::new(p);
    g.bench_function(name, |bench| {
        bench.iter(|| {
            let m = PPartMultiplier::<MOD4>::new_from_allocation(
                p,
                &r,
                &s,
                PPartAllocation::default(),
                0,
                0,
            );

            for c in m {
                if MOD4 {
                    assert!(c < 4);
                } else {
                    assert!(c < p.as_u32());
                }
            }
        });
    });
}

fn ppart(c: &mut Criterion) {
    let mut g = c.benchmark_group("milnor_ppart");

    bench_ppart::<false>(
        &mut g,
        "ppart_2/a",
        2,
        vec![60, 30, 8, 2, 1],
        vec![20, 30, 20, 4, 1, 2],
    );
    bench_ppart::<false>(
        &mut g,
        "ppart_2/b",
        2,
        vec![35, 12, 20, 14, 1, 3],
        vec![60, 30, 0, 2, 1],
    );

    bench_ppart::<true>(
        &mut g,
        "ppart_4/a",
        2,
        vec![60, 30, 8, 2, 1],
        vec![20, 30, 20, 4, 1, 2],
    );
    bench_ppart::<true>(
        &mut g,
        "ppart_4/b",
        2,
        vec![35, 12, 20, 14, 1, 3],
        vec![60, 30, 0, 2, 1],
    );

    #[cfg(feature = "odd-primes")]
    {
        bench_ppart::<false>(
            &mut g,
            "ppart_3/a",
            3,
            vec![120, 70, 40, 2],
            vec![60, 35, 21, 6],
        );
        bench_ppart::<false>(
            &mut g,
            "ppart_3/b",
            3,
            vec![30, 12, 35, 24],
            vec![100, 80, 16, 2, 3],
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
