use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkGroup, Criterion};
use fp::{
    matrix::{
        blas::{self, MatrixBlock, MatrixBlockMut},
        Matrix,
    },
    prime::TWO,
};
use pprof::criterion::{Output, PProfProfiler};
use rand::Rng;

fn muls(c: &mut Criterion) {
    // TODO: Add more benchmarks for different sizes.
    bench_gemm_block(c);
    // bench_mkn(64, 64, 64, c);
}

fn bench_gemm_block(c: &mut Criterion) {
    let mut g = c.benchmark_group("gemm_block");
    bench_individual_gemm(&mut g, "gemm_block_naive", blas::naive::gemm_block_naive);
    bench_individual_gemm(&mut g, "gemm_block_scalar", blas::scalar::gemm_block_scalar);
    bench_individual_gemm(&mut g, "gemm_block_avx512", blas::avx512::gemm_block_avx512);
    bench_individual_gemm(
        &mut g,
        "gemm_block_avx512_unrolled",
        blas::avx512::gemm_block_avx512_unrolled,
    );
    g.finish();
}

fn bench_individual_gemm(
    g: &mut BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    gemm_fn: fn(bool, MatrixBlock, MatrixBlock, bool, &mut MatrixBlockMut),
) {
    g.bench_function(name, |b| {
        b.iter_batched(
            || {
                (
                    random_matrix(64, 64),
                    random_matrix(64, 64),
                    random_matrix(64, 64),
                )
            },
            |(a, b, mut c)| {
                gemm_fn(
                    true,
                    a.block_at(0, 0),
                    b.block_at(0, 0),
                    true,
                    &mut c.block_mut_at(0, 0),
                );
            },
            BatchSize::SmallInput,
        );
    });
}

// fn bench_mkn(m: usize, k: usize, n: usize, c: &mut Criterion) {
//     let mut g = c.benchmark_group(format!("{m}x{k} * {k}x{n}"));
//     g.bench_function("default_matmul", |b| {
//         b.iter_batched(
//             || random_matrix_pair(m, k, n),
//             |(a, b)| (&a) * (&b),
//             BatchSize::SmallInput,
//         );
//     });
//     g.bench_function("fast_matmul_scalar", |b| {
//         b.iter_batched(
//             || random_matrix_pair(m, k, n),
//             |(a, b)| a.fast_mul::<false, false>(&b),
//             BatchSize::SmallInput,
//         );
//     });
//     g.bench_function("fast_matmul_simd_looped", |b| {
//         b.iter_batched(
//             || random_matrix_pair(m, k, n),
//             |(a, b)| a.fast_mul::<true, false>(&b),
//             BatchSize::SmallInput,
//         );
//     });
//     g.bench_function("fast_matmul_simd_unrolled", |b| {
//         b.iter_batched(
//             || random_matrix_pair(m, k, n),
//             |(a, b)| a.fast_mul::<true, true>(&b),
//             BatchSize::SmallInput,
//         );
//     });
//     g.finish();
// }

fn random_matrix_pair(rows: usize, inner: usize, cols: usize) -> (Matrix, Matrix) {
    (random_matrix(rows, inner), random_matrix(inner, cols))
}

fn random_matrix(rows: usize, cols: usize) -> Matrix {
    let mut rng = rand::thread_rng();
    let mut data = Vec::new();
    let data_len = rows * (cols + 63) / 64;
    for _ in 0..data_len {
        data.push(rng.gen());
    }
    Matrix::from_data(TWO, rows, cols, data)
}

criterion_group! {
    name = mul;
    config = Criterion::default()
        .measurement_time(std::time::Duration::from_secs(10))
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = muls
}

criterion_main!(mul);
