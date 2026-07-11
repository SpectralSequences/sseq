//! A/B benchmark: the hash-free table-based Milnor index ([`MilnorAlgebra::seqno`]) versus the
//! `FxHashMap`-backed [`MilnorAlgebra::basis_element_to_index`], over the exact operation of
//! turning a basis element back into its index.
//!
//! This is the lookup that a resolution performs on *every* output term of a multiply — Nassau's
//! `S_2` @ p=2 run issues ~1.3B of them — so a lookup that is even a little cheaper is worth having,
//! and a GPU kernel (which cannot carry a hashmap) *needs* the table-based form regardless.
//!
//! The `seqno` group here reads the flat [`arc_swap`]-backed table; compare its `basis_to_index/*`
//! ids against the `hashmap/*` ids at the same degree to see which wins on the CPU.

use std::hint::black_box;

use algebra::{Algebra, MilnorAlgebra};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use fp::prime::TWO;
use pprof::criterion::{Output, PProfProfiler};

/// Degrees to sweep. Chosen to bracket the range a real `S_2` resolution reaches, from the cheap
/// low-dimensional end up to degrees whose basis has thousands of elements.
const DEGREES: &[i32] = &[16, 24, 32, 40, 48, 56, 64];

fn seqno(c: &mut Criterion) {
    let algebra = MilnorAlgebra::new(TWO, false);
    let max_degree = *DEGREES.iter().max().unwrap();
    algebra.compute_basis(max_degree);
    algebra.compute_seqno_tables(max_degree);

    let mut g = c.benchmark_group("seqno");

    for &degree in DEGREES {
        let dim = algebra.dimension(degree);
        if dim == 0 {
            continue;
        }
        // Snapshot the basis so neither method pays to walk the algebra's storage during timing.
        let basis: Vec<_> = (0..dim)
            .map(|i| algebra.basis_element_from_index(degree, i).clone())
            .collect();

        g.throughput(Throughput::Elements(dim as u64));

        g.bench_function(format!("hashmap/deg{degree}"), |b| {
            b.iter(|| {
                for elt in &basis {
                    black_box(algebra.basis_element_to_index(elt));
                }
            });
        });

        g.bench_function(format!("basis_to_index/deg{degree}"), |b| {
            b.iter(|| {
                for elt in &basis {
                    black_box(algebra.seqno(&elt.p_part));
                }
            });
        });
    }

    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(std::time::Duration::from_secs(3))
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = seqno
}
criterion_main!(benches);
