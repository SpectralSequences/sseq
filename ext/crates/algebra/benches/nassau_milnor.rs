//! Benchmarks the Milnor multiplication kernel in the exact regime Nassau's algorithm
//! exercises when resolving the sphere `S` at the prime 2.
//!
//! ## What Nassau actually calls
//!
//! Nassau (`ext/src/nassau.rs`) resolves over `FreeModule<MilnorAlgebra>`, which is a *stable*
//! (`U = false`) free module built on `MilnorAlgebra::new(TWO, false)`. Building the masked
//! differential matrices (`get_partial_matrix` → `FreeModuleHomomorphism::apply_to_basis_element`
//! → `FreeModule::act`) issues, per generator block, one
//!
//! ```text
//! Algebra::multiply_basis_element_by_element(result, 1, op_degree, op_index, elt_degree, elt)
//! ```
//!
//! — a fixed *operation* basis element times a general algebra *element*. Because the free module
//! is `U = false`, the `MuAlgebra<false>` shim discards `excess` and routes to the plain **stable**
//! multiply; every basis-by-basis product funnels through `MilnorAlgebra::multiply_with_allocation`
//! (`excess = i32::MAX`, no instability truncation) into the `PPartMultiplier::<false>` sweep — the
//! innermost kernel. The `*_unstable` entry points are never taken.
//!
//! ## Where the regime comes from
//!
//! The `(operation_degree, element_degree)` pairs in [`REGIME`] were captured by instrumenting
//! `multiply_with_allocation` during a real `construct_nassau(("S_2", "milnor"), None)` resolved
//! through bidegree (70, 70): ~3.77M basis-by-basis products over ~2200 distinct degree pairs. The
//! cost mass sits in target degrees `op + elt ≈ 40..52`; within that, products land heavily on
//! degree-24 and degree-32 element components, alongside a frequent "large operation × small
//! element" band (e.g. a degree-40+ operation applied to a degree-1 element). The table samples
//! those regions plus a couple of smaller pairs for the cheap end of the cost curve. To regenerate
//! after algorithm changes, re-instrument `multiply_with_allocation` to tally `(m1.degree,
//! m2.degree)` and rerun the resolution (see the plan/commit history).

use std::hint::black_box;

use algebra::{Algebra, MilnorAlgebra};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use fp::{prime::TWO, vector::FpVector};
use pprof::criterion::{Output, PProfProfiler};

/// `(operation_degree, element_degree)` pairs sampled from Nassau's `S_2` @ p=2 regime.
const REGIME: &[(i32, i32)] = &[
    // cheap end of the curve
    (8, 8),
    (16, 16),
    // operation × degree-24 element component (a hot band)
    (8, 24),
    (20, 24),
    (24, 24),
    (26, 24),
    // operation × degree-32 element component
    (16, 32),
    (24, 32),
    // operation applied to a smaller element
    (24, 16),
    (32, 8),
    // large operation × small element (the frequent Sq^1-style band)
    (40, 1),
    (46, 1),
];

/// A dense element of the given length. At p=2 every present coordinate is 1, so this is the
/// all-ones vector — the densest element, giving a stable upper bound on the inner-loop cost.
fn dense_element(len: usize) -> FpVector {
    let mut v = FpVector::new(TWO, len);
    for i in 0..len {
        v.set_entry(i, 1);
    }
    v
}

fn nassau_milnor(c: &mut Criterion) {
    // Exactly the algebra Nassau uses: the full Milnor algebra at p=2, stable (not unstable).
    let algebra = MilnorAlgebra::new(TWO, false);
    let mut g = c.benchmark_group("nassau_milnor");

    for &(op_degree, elt_degree) in REGIME {
        let out_degree = op_degree + elt_degree;
        // `compute_basis` is cumulative, so this readies every degree up to `out_degree`.
        algebra.compute_basis(out_degree);
        let op_dim = algebra.dimension(op_degree);
        let elt_dim = algebra.dimension(elt_degree);
        let out_dim = algebra.dimension(out_degree);
        if op_dim == 0 || elt_dim == 0 {
            continue;
        }

        // A dense element sends every basis product to the kernel: op_dim * elt_dim of them.
        g.throughput(Throughput::Elements((op_dim * elt_dim) as u64));

        let elt = dense_element(elt_dim);
        let mut scratch = FpVector::new(TWO, out_dim);
        g.bench_function(
            format!("basis_by_element/op{op_degree}xel{elt_degree}"),
            |b| {
                b.iter(|| {
                    scratch.set_to_zero();
                    // Sweep every operation basis element against the dense element — exactly the
                    // `multiply_basis_element_by_element` call `FreeModule::act` makes in Nassau.
                    for i in 0..op_dim {
                        algebra.multiply_basis_element_by_element(
                            scratch.as_slice_mut(),
                            1,
                            op_degree,
                            i,
                            elt_degree,
                            elt.as_slice(),
                        );
                    }
                    black_box(&scratch);
                });
            },
        );
    }

    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(std::time::Duration::from_secs(3))
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = nassau_milnor
}
criterion_main!(benches);
