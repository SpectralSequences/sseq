//! Compares [`PPartRanker`] against the hash map lookup it would replace.
//!
//! `MilnorAlgebra::basis_element_to_index` is called once per term of every product, so it is one
//! of the hottest operations in a resolution. It is currently a hash map from the (packed) basis
//! element to its position in `basis_table`. The ranker computes that position arithmetically
//! instead, from a table that covers every degree at once.
//!
//! The two are compared on the same workload: recover the index of every basis element of a
//! degree. Note that they do not agree on *which* index — see [`PPartRanker`] — so this measures
//! the cost of the two strategies, not a drop-in substitution.
//!
//! Each is measured under two access orders, because the choice flatters the map:
//!
//! - **sequential** — sweep the basis in order. This is the map's insertion order, so every probe
//!   walks memory linearly and prefetches perfectly. Flattering, and not what callers do.
//! - **scattered** — the same elements in a fixed pseudo-random permutation. This is closer to
//!   real use, where `basis_element_to_index` is called on multiplication *outputs*, which arrive
//!   in no particular order. It matters because the two structures scale differently: the map
//!   stores an entry per basis element and leaves cache as the basis grows (~60 KiB in degree 120
//!   alone), whereas the ranker's table is a few KiB covering every degree at once.
//!
//! [`PPartRanker`]: algebra::milnor_rank::PPartRanker

use std::hint::black_box;

use algebra::{Algebra, MilnorAlgebra, milnor_rank::PPartRanker};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use fp::prime::TWO;
use pprof::criterion::{Output, PProfProfiler};

/// Degrees to sweep.
///
/// The range matters more than it looks, because the two structures live in different parts of the
/// memory hierarchy and the crossover is inside this range. A lookup probes only its own degree's
/// map, which is ~0.1 MB in degree 120 (L2-resident) but ~3 MB in degree 300 and ~12 MB in degree
/// 400 — well past L3, so every probe is a DRAM miss. The ranker's table is ~35 KB for *all*
/// degrees and stays in L1 throughout. Measuring only the small degrees answers a question nobody
/// is asking; the large ones are where expanding the algebra actually hurts.
const DEGREES: &[i32] = &[120, 300, 400, 500];

/// A fixed permutation of `0..n`, from a Fisher-Yates shuffle driven by a small LCG. Deterministic
/// so the two variants see exactly the same access order.
fn scattered(n: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..n).collect();
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    for i in (1..n).rev() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        order.swap(i, (state >> 33) as usize % (i + 1));
    }
    order
}

fn milnor_rank(c: &mut Criterion) {
    let algebra = MilnorAlgebra::new(TWO, false);
    let max_degree = *DEGREES.iter().max().unwrap();
    algebra.compute_basis(max_degree);
    let ranker = PPartRanker::new(TWO, max_degree);

    let mut g = c.benchmark_group("milnor_rank");
    for &degree in DEGREES {
        let dim = algebra.dimension(degree);
        g.throughput(Throughput::Elements(dim as u64));

        // Collect the elements once so neither variant pays for the table walk itself.
        let elements: Vec<_> = (0..dim)
            .map(|i| algebra.basis_element_from_index(degree, i))
            .collect();
        let shuffled: Vec<_> = scattered(dim).into_iter().map(|i| elements[i]).collect();

        for (order, elements) in [("seq", &elements), ("scattered", &shuffled)] {
            g.bench_function(format!("hashmap_{order}/deg{degree}"), |b| {
                b.iter(|| {
                    for elt in elements {
                        black_box(algebra.basis_element_to_index(elt));
                    }
                });
            });

            g.bench_function(format!("ranker_{order}/deg{degree}"), |b| {
                b.iter(|| {
                    for elt in elements {
                        black_box(ranker.rank(elt.p_part, degree));
                    }
                });
            });
        }
    }
    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(std::time::Duration::from_secs(3))
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = milnor_rank
}
criterion_main!(benches);
