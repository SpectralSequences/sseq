//! Benchmarks for the C-motivic Steenrod algebra engine.
//!
//! Three levels, from the kernel outwards:
//!
//!  - `motivic_product` — a single [`multiply_closed`], the Kong–Lin Theorem 5.1 product. This is
//!    the arithmetic the whole layer is built on.
//!  - `motivic_block` — [`MotivicMilnorAlgebra::fill_block`], the batch unit a resolution actually
//!    asks for: every structure constant for one pair of topological degrees. Throughput is in
//!    structure constants, so the numbers are comparable across degrees.
//!  - `motivic_basis` — [`enum_basis`], the basis enumeration each new degree pays once.
//!
//! The `motivic_block` group is the one to watch when changing the coefficient representation:
//! it is the only group that exercises the `DualElement` map, the index lookup, and the product
//! together, in the proportion a resolution hits them.

use algebra::{
    MotivicMilnorAlgebra,
    motivic::milnor::{Monomial, enum_basis, multiply_closed},
};
use criterion::{
    BenchmarkGroup, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main,
    measurement::WallTime,
};
use pprof::criterion::{Output, PProfProfiler};

/// `Q(E)P(R)` from the `Q` indices and the ξ exponents, in the paper's indexing where `R[0]`
/// belongs to ξ_0 = 1 and is skipped.
fn elt(q: &[u32], xi: &[u32]) -> Monomial {
    let mut r = vec![0];
    r.extend_from_slice(xi);
    Monomial::from_paper(q.iter().map(|i| 1 << i).sum(), &r).unwrap()
}

fn bench_product(g: &mut BenchmarkGroup<WallTime>, name: &str, a: Monomial, b: Monomial) {
    g.bench_function(name, |bench| {
        bench.iter(|| std::hint::black_box(multiply_closed(a, b)));
    });
}

fn product(c: &mut Criterion) {
    let mut g = c.benchmark_group("motivic_product");

    // Pure ξ: the classical Milnor-matrix part of the formula, with no Y enumeration.
    bench_product(&mut g, "xi/small", elt(&[], &[2]), elt(&[], &[2]));
    bench_product(&mut g, "xi/medium", elt(&[], &[4, 1]), elt(&[], &[2, 1]));
    bench_product(&mut g, "xi/large", elt(&[], &[6, 2, 1]), elt(&[], &[4, 1]));

    // With a Q-part, which is what turns on the second (`Y`) matrix and the τ-rewriting.
    bench_product(&mut g, "q/small", elt(&[0], &[1]), elt(&[1], &[1]));
    bench_product(&mut g, "q/medium", elt(&[0, 1], &[2]), elt(&[2], &[1, 1]));
    bench_product(&mut g, "q/large", elt(&[0, 2], &[3, 1]), elt(&[1], &[2, 1]));

    g.finish();
}

fn block(c: &mut Criterion) {
    let mut g = c.benchmark_group("motivic_block");

    for t in [12, 16, 20] {
        // Size the throughput by the number of structure constants in the block, so the
        // per-product cost is comparable across degrees.
        let dims = {
            let alg = MotivicMilnorAlgebra::new();
            alg.compute_basis(t);
            alg.dimension(t)
        };
        g.throughput(Throughput::Elements((dims * dims) as u64));
        g.bench_with_input(BenchmarkId::from_parameter(t), &t, |bench, &t| {
            // A fresh algebra per iteration: `fill_block` is memoized, so reusing one would
            // measure the cache rather than the product.
            bench.iter_batched(
                MotivicMilnorAlgebra::new,
                |alg| alg.fill_block(t, t),
                criterion::BatchSize::SmallInput,
            );
        });
    }

    g.finish();
}

fn basis(c: &mut Criterion) {
    let mut g = c.benchmark_group("motivic_basis");

    for t in [20, 30, 40] {
        g.bench_with_input(BenchmarkId::from_parameter(t), &t, |bench, &t| {
            bench.iter(|| std::hint::black_box(enum_basis(t)));
        });
    }

    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(std::time::Duration::from_secs(3))
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = product, block, basis
}
criterion_main!(benches);
