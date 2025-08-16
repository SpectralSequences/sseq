use aligned_vec::AVec;
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkGroup, Criterion};
use fp::{
    matrix::{
        blas::{self, MatrixBlock, MatrixBlockSliceMut},
        Matrix,
    },
    prime::TWO,
};
use pprof::criterion::{Output, PProfProfiler};
use rand::Rng;

fn muls(c: &mut Criterion) {
    // TODO: Add more benchmarks for different sizes.
    bench_gemm_block(c);
    for m in [64, 128, 256, 512, 1024] {
        for k in [64, 128, 256, 512, 1024] {
            for n in [64, 128, 256, 512, 1024] {
                bench_mkn(m, k, n, c);
            }
        }
    }
    bench_mkn(2048, 2048, 2048, c);
    bench_mkn(4096, 4096, 4096, c);
    bench_mkn(8192, 8192, 8192, c);
}

fn bench_gemm_block(c: &mut Criterion) {
    let mut g = c.benchmark_group("gemm_block");
    bench_individual_gemm(&mut g, "gemm_block_naive", blas::naive::gemm_block_naive);
    bench_individual_gemm(&mut g, "gemm_block_scalar", blas::scalar::gemm_block_scalar);
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
    gemm_fn: fn(bool, MatrixBlock, MatrixBlock, bool, &mut MatrixBlockSliceMut),
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
                    a.block_at(0, 0).gather_block(),
                    b.block_at(0, 0).gather_block(),
                    true,
                    &mut c.block_mut_at(0, 0),
                );
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_mkn(m: usize, k: usize, n: usize, c: &mut Criterion) {
    let mut g = c.benchmark_group(format!("{m}x{k} * {k}x{n}"));
    g.throughput(criterion::Throughput::Elements((2 * m * k * n) as u64));
    let orderings: [(fn(&Matrix, &Matrix) -> Matrix, &str); 6] = [
        (Matrix::fast_mul_sequential_cir, "cir"),
        (Matrix::fast_mul_sequential_cri, "cri"),
        (Matrix::fast_mul_sequential_icr, "icr"),
        (Matrix::fast_mul_sequential_irc, "irc"),
        (Matrix::fast_mul_sequential_rci, "rci"),
        (Matrix::fast_mul_sequential_ric, "ric"),
    ];
    for (func, name) in orderings.iter() {
        g.bench_function(format!("matmul_sequential_{name}"), |b| {
            b.iter_batched(
                || random_matrix_pair(m, k, n),
                |(a, b)| func(&a, &b),
                BatchSize::SmallInput,
            );
        });
    }
    // g.bench_function("matmul_sequential_", |b| {
    //     b.iter_batched(
    //         || random_matrix_pair(m, k, n),
    //         |(a, b)| a.fast_mul_sequential(&b),
    //         BatchSize::SmallInput,
    //     );
    // });
    g.bench_function("matmul_concurrent", |b| {
        b.iter_batched(
            || random_matrix_pair(m, k, n),
            |(a, b)| a.fast_mul_concurrent(&b),
            BatchSize::SmallInput,
        );
    });
    g.finish();
}

fn random_matrix_pair(rows: usize, inner: usize, cols: usize) -> (Matrix, Matrix) {
    (random_matrix(rows, inner), random_matrix(inner, cols))
}

fn random_matrix(rows: usize, cols: usize) -> Matrix {
    let mut rng = rand::thread_rng();
    let mut data = AVec::new(0);
    let data_len = rows * (cols + 63) / 64;
    for _ in 0..data_len {
        data.push(rng.gen());
    }
    Matrix::from_data(TWO, rows, cols, data)
}

criterion_group! {
    name = mul;
    config = Criterion::default()
        .measurement_time(std::time::Duration::from_secs(15))
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = muls
}

criterion_main!(mul);
