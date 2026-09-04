//! A/B benchmark: the hash-free table index against the basis hashmap.

use algebra::{Algebra, MilnorAlgebra};
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use fp::prime::TWO;

/// Degrees to sample.
///
/// The two indices scale differently, which is why this sweeps a range instead of sampling one
/// size. The hashmap is per-degree, so its working set grows with the dimension of that degree and
/// eventually falls out of cache; the `g` table is shared across degrees and grows only linearly in
/// the degree, so it stays resident. The range is chosen to span the point where that crosses over.
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
        // Snapshot the basis so neither index pays to walk the algebra's storage during timing.
        let basis: Vec<_> = (0..dim)
            .map(|i| algebra.basis_element_from_index(degree, i))
            .collect();

        g.throughput(Throughput::Elements(dim as u64));

        // Reads the degree straight off the basis element, then one hash and probe of the packed
        // key.
        g.bench_function(format!("hashmap/deg{degree}"), |b| {
            b.iter(|| {
                for elt in &basis {
                    black_box(algebra.basis_element_to_index(elt));
                }
            });
        });

        // The tables acquired once, the degree supplied by the caller — what a hot loop would do.
        g.bench_function(format!("seqno/deg{degree}"), |b| {
            let ranker = algebra.seqno_ranker();
            b.iter(|| {
                for elt in &basis {
                    black_box(ranker.rank(elt.p_part, degree));
                }
            });
        });

        // The convenience API, which re-acquires the tables on every call. The gap between this and
        // `seqno` is what hoisting the guard is worth.
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
