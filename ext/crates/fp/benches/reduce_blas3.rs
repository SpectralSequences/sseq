//! M4RI (`row_reduce`) vs blocked GEMM-based (`row_reduce_blas3`) row reduction
//! over F₂, on full-rank and (the target workload) rank-deficient matrices, plus
//! a panel-width sweep to inform the default block.
//!
//! Note: on CPU the GEMM path is the AVX-512 tiled kernel (or the scalar
//! fallback if the host lacks AVX-512), so the crossover measured here is a
//! lower bound on the eventual GPU (`wgmma.b1`) speedup.

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use fp::{matrix::Matrix, prime::TWO};
use pprof::criterion::{Output, PProfProfiler};
use rand::Rng;

fn random_matrix(rows: usize, cols: usize) -> Matrix {
    let mut rng = rand::rng();
    Matrix::from_vec(
        TWO,
        &(0..rows)
            .map(|_| (0..cols).map(|_| rng.random_range(0..2)).collect())
            .collect::<Vec<Vec<u32>>>(),
    )
}

/// An `n × n` F₂ matrix of rank at most `rank`, as the product of a random
/// `n × rank` and `rank × n` matrix (built with the fast GEMM). This models the
/// highly rank-deficient inputs the algorithm targets.
fn rank_deficient_matrix(n: usize, rank: usize) -> Matrix {
    let a = random_matrix(n, rank);
    let b = random_matrix(rank, n);
    &a * &b
}

fn full_rank(c: &mut Criterion) {
    let mut group = c.benchmark_group("row_reduce_f2_full_rank");
    for n in [512usize, 1024, 2048] {
        group.bench_with_input(BenchmarkId::new("m4ri", n), &n, |bch, &n| {
            bch.iter_batched_ref(
                || random_matrix(n, n),
                |m| m.row_reduce(),
                BatchSize::LargeInput,
            )
        });
        group.bench_with_input(BenchmarkId::new("blas3", n), &n, |bch, &n| {
            bch.iter_batched_ref(
                || random_matrix(n, n),
                |m| m.row_reduce_blas3(),
                BatchSize::LargeInput,
            )
        });
    }
    group.finish();
}

fn rank_deficient(c: &mut Criterion) {
    let mut group = c.benchmark_group("row_reduce_f2_half_rank");
    for n in [512usize, 1024, 2048, 4096] {
        let rank = n / 2;
        group.bench_with_input(BenchmarkId::new("m4ri", n), &n, |bch, &n| {
            bch.iter_batched_ref(
                || rank_deficient_matrix(n, rank),
                |m| m.row_reduce(),
                BatchSize::LargeInput,
            )
        });
        group.bench_with_input(BenchmarkId::new("blas3", n), &n, |bch, &n| {
            bch.iter_batched_ref(
                || rank_deficient_matrix(n, rank),
                |m| m.row_reduce_blas3(),
                BatchSize::LargeInput,
            )
        });
    }
    group.finish();
}

/// Panel-width sweep at a fixed rank-deficient size, to pick the default block.
fn block_sweep(c: &mut Criterion) {
    let mut group = c.benchmark_group("row_reduce_f2_block_sweep_2048");
    let n = 2048;
    let rank = n / 2;
    for block in [64usize, 128, 256, 512, 1024] {
        group.bench_with_input(BenchmarkId::from_parameter(block), &block, |bch, &block| {
            bch.iter_batched_ref(
                || rank_deficient_matrix(n, rank),
                |m| m.row_reduce_blas3_block(block),
                BatchSize::LargeInput,
            )
        });
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = full_rank, rank_deficient, block_sweep
}

criterion_main!(benches);
