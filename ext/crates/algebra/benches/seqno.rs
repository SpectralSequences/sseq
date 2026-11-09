//! A/B benchmark: the hash-free table index against the basis hashmap.

use algebra::{Algebra, MilnorAlgebra};
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use fp::prime::TWO;
use rustc_hash::FxHashMap;

/// Degrees to sample, spanning the point where the table index overtakes the hashmap (see
/// `EXPERIMENTS.md`).
///
/// `compute_basis` builds every degree below the maximum, so raising the top of this range costs
/// memory as well as time.
const DEGREES: &[i32] = &[32, 64, 128, 192, 256, 300, 340];

/// Time both indices over every basis element of each degree in [`DEGREES`].
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
        // Snapshot the basis so neither index pays to walk the algebra's storage while timed.
        let basis: Vec<_> = (0..dim)
            .map(|i| algebra.basis_element_from_index(degree, i))
            .collect();

        // Build a local hashmap to measure actual hashmap performance, not seqno routing.
        let map: FxHashMap<_, usize> = basis.iter().enumerate().map(|(i, e)| (*e, i)).collect();

        g.throughput(Throughput::Elements(dim as u64));

        g.bench_function(format!("hashmap/deg{degree}"), |b| {
            b.iter(|| {
                for elt in &basis {
                    black_box(map[elt]);
                }
            });
        });

        g.bench_function(format!("seqno/deg{degree}"), |b| {
            let tables = algebra.seqno_tables();
            b.iter(|| {
                for elt in &basis {
                    black_box(tables.rank(elt.p_part, degree));
                }
            });
        });

        // `seqno` hoists the table guard out of the loop as a hot caller would; the gap against
        // `seqno_naive`, which re-acquires it per call, is what that hoisting is worth.
        g.bench_function(format!("seqno_naive/deg{degree}"), |b| {
            b.iter(|| {
                for elt in &basis {
                    black_box(algebra.seqno(elt.p_part, degree));
                }
            });
        });
    }

    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = seqno
}
criterion_main!(benches);
