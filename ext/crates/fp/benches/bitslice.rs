//! PHASE 0 PROTOTYPE bench — to be removed in Phase 5.
//!
//! Compares the bit-sliced `add`/`scale` kernels (generic, and the F3 fast path) against
//! the existing packed `FpVector` implementation, across a few representative primes and
//! vector lengths. This is the go/no-go gate for the bit-slicing project: it tells us
//! where bit-slicing actually wins before committing to the larger refactor.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use fp::{
    bitslice_proto::BitSlicedVec,
    prime::{Prime, ValidPrime},
    vector::FpVector,
};
use rand::Rng;

const PRIMES: [u32; 4] = [3, 5, 7, 251];
const LENGTHS: [usize; 4] = [100, 1000, 10_000, 100_000];
/// A representative non-unit, non-negation scalar to exercise the full multiply path.
const SCALAR: u32 = 2;

fn random_data(p: u32, len: usize) -> Vec<u32> {
    let mut rng = rand::rng();
    (0..len).map(|_| rng.random_range(0..p)).collect()
}

fn bench_add(c: &mut Criterion) {
    for p in PRIMES {
        let vp = ValidPrime::new(p);
        let mut group = c.benchmark_group(format!("add_p{p}"));
        for len in LENGTHS {
            let a = random_data(p, len);
            let b = random_data(p, len);

            // Packed reference (existing implementation).
            let packed_a = FpVector::from_slice(vp, &a);
            let packed_b = FpVector::from_slice(vp, &b);
            group.bench_with_input(BenchmarkId::new("packed", len), &len, |bench, _| {
                bench.iter_batched_ref(
                    || packed_a.clone(),
                    |va| va.add(&packed_b, SCALAR),
                    criterion::BatchSize::SmallInput,
                )
            });

            // Bit-sliced generic kernel.
            let bs_a = BitSlicedVec::from_u32(p, &a);
            let bs_b = BitSlicedVec::from_u32(p, &b);
            group.bench_with_input(BenchmarkId::new("bitsliced_generic", len), &len, |bench, _| {
                bench.iter_batched_ref(
                    || bs_a.clone(),
                    |va| va.add_generic(&bs_b, SCALAR),
                    criterion::BatchSize::SmallInput,
                )
            });

            // Bit-sliced F3 fast circuit.
            if p == 3 {
                group.bench_with_input(BenchmarkId::new("bitsliced_f3", len), &len, |bench, _| {
                    bench.iter_batched_ref(
                        || bs_a.clone(),
                        |va| va.add_f3(&bs_b, SCALAR),
                        criterion::BatchSize::SmallInput,
                    )
                });
            }
        }
        group.finish();
    }
}

fn bench_scale(c: &mut Criterion) {
    for p in PRIMES {
        let vp = ValidPrime::new(p);
        let mut group = c.benchmark_group(format!("scale_p{p}"));
        for len in LENGTHS {
            let a = random_data(p, len);

            let packed_a = FpVector::from_slice(vp, &a);
            group.bench_with_input(BenchmarkId::new("packed", len), &len, |bench, _| {
                bench.iter_batched_ref(
                    || packed_a.clone(),
                    |va| va.scale(SCALAR),
                    criterion::BatchSize::SmallInput,
                )
            });

            let bs_a = BitSlicedVec::from_u32(p, &a);
            group.bench_with_input(BenchmarkId::new("bitsliced_generic", len), &len, |bench, _| {
                bench.iter_batched_ref(
                    || bs_a.clone(),
                    |va| va.scale_generic(SCALAR),
                    criterion::BatchSize::SmallInput,
                )
            });

            if p == 3 {
                group.bench_with_input(BenchmarkId::new("bitsliced_f3", len), &len, |bench, _| {
                    bench.iter_batched_ref(
                        || bs_a.clone(),
                        |va| va.scale_f3(SCALAR),
                        criterion::BatchSize::SmallInput,
                    )
                });
            }
        }
        group.finish();
    }
}

criterion_group!(benches, bench_add, bench_scale);
criterion_main!(benches);
