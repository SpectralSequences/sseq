//! End-to-end timing harness for Nassau's algorithm on the sphere `S_2` at p = 2.
//!
//! Resolves `S_2` via [`construct_nassau`] and [`Resolution::compute_through_stem`] up to a given
//! `(stem, filtration)`, reporting the fastest wall time over several fresh runs. This exercises the
//! whole resolution — the Milnor multiplication kernel *and* the F₂ linear algebra — so it is the
//! right tool for judging whether a change to `MilnorAlgebra` multiplication actually moves the
//! needle in practice (unlike a micro-benchmark of the multiply alone).
//!
//! Usage:
//! ```text
//! cargo run --release --example nassau_e2e -- [stem=80] [filtration=42] [runs=3]
//! # for the parallel driver on a big case, enable rayon:
//! cargo run --release --features concurrent --example nassau_e2e -- 100 55 3
//! ```
//!
//! To A/B a multiplication change: build/run on the change, then `git stash`/checkout the baseline
//! and run again; compare the `best=` figures (min over runs is the most throttling-robust).
//!
//! To see *where* the time goes (e.g. multiply vs. linear algebra), build with the `flamegraph`
//! feature and set `FLAME` to an output path — a sampling flamegraph of every run is written there:
//! ```text
//! FLAME=/tmp/nassau.svg cargo run --release --features flamegraph --example nassau_e2e -- 60 32 1
//! ```

use std::time::Instant;

use ext::{chain_complex::ChainComplex, utils::construct_nassau};
use sseq::coordinates::Bidegree;

fn main() {
    let mut args = std::env::args().skip(1);
    let n: i32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(80);
    let s: i32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(42);
    let runs: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(3);

    // When built `--features flamegraph` and `FLAME=<path>` is set, sample the whole run and write a
    // flamegraph there. Kept alive until after the timing loop so it captures every resolution.
    #[cfg(feature = "flamegraph")]
    let flame = std::env::var("FLAME").ok().map(|path| {
        (
            path,
            pprof::ProfilerGuard::new(1000).expect("failed to start profiler"),
        )
    });

    let target = Bidegree::n_s(n, s);
    let mut best = f64::INFINITY;
    for run in 1..=runs {
        // Each run builds a fresh resolution: the resolution caches results, so it cannot be reused.
        let start = Instant::now();
        let res = construct_nassau(("S_2", "milnor"), None).unwrap();
        res.compute_through_stem(target);
        let secs = start.elapsed().as_secs_f64();
        best = best.min(secs);
        // Touch the result so nothing is optimized away (the compute has side effects regardless).
        eprintln!(
            "  run {run}/{runs}: {secs:.2} s  (hom degrees resolved: {})",
            res.next_homological_degree()
        );
    }
    println!("S_2 @ p=2  stem={n}  filtration={s}  runs={runs}  best={best:.2}s");

    // Prints Milnor-multiply counters when built with `MILNOR_PROFILE=1`, else a one-line note.
    algebra::milnor_algebra::profile::report();

    #[cfg(feature = "flamegraph")]
    if let Some((path, guard)) = flame {
        let report = guard
            .report()
            .build()
            .expect("failed to build profiler report");
        let file = std::fs::File::create(&path).expect("failed to create flamegraph file");
        report.flamegraph(file).expect("failed to write flamegraph");
        eprintln!("wrote flamegraph to {path}");
    }
}
