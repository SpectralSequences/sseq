use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use fp::{
    matrix::Matrix,
    prime::{ValidPrime, TWO},
};
use pprof::criterion::{Output, PProfProfiler};
use rand::Rng;

fn muls(c: &mut Criterion) {
    // TODO: Add more benchmarks for different sizes.
    bench_mkn(64, 64, 64, c);
}

fn bench_mkn(m: usize, k: usize, n: usize, c: &mut Criterion) {
    let mut g = c.benchmark_group(format!("{m}x{k} * {k}x{n}"));
    g.bench_function("default_matmul", |b| {
        b.iter_batched(
            || random_matrix_pair(TWO, m, k, n),
            |(a, b)| (&a) * (&b),
            BatchSize::SmallInput,
        );
    });
    g.bench_function("fast_matmul", |b| {
        b.iter_batched(
            || random_matrix_pair(TWO, m, k, n),
            |(a, b)| a.fast_mul(&b),
            BatchSize::SmallInput,
        );
    });
    g.bench_function("fast_2_matmul", |b| {
        b.iter_batched(
            || random_matrix_pair(TWO, m, k, n),
            |(a, b)| a.fast_mul_2(&b),
            BatchSize::SmallInput,
        );
    });
    g.bench_function("fast_3_matmul", |b| {
        b.iter_batched(
            || random_matrix_pair(TWO, m, k, n),
            |(a, b)| a.fast_mul_3(&b),
            BatchSize::SmallInput,
        );
    });
    g.bench_function("fast_4_matmul", |b| {
        b.iter_batched(
            || random_matrix_pair(TWO, m, k, n),
            |(a, b)| a.fast_mul_4(&b),
            BatchSize::SmallInput,
        );
    });
    g.bench_function("fast_5_matmul", |b| {
        b.iter_batched(
            || random_matrix_pair(TWO, m, k, n),
            |(a, b)| a.fast_mul_5(&b),
            BatchSize::SmallInput,
        );
    });
    g.bench_function("fast_6_matmul", |b| {
        b.iter_batched(
            || random_matrix_pair(TWO, m, k, n),
            |(a, b)| a.fast_mul_6(&b),
            BatchSize::SmallInput,
        );
    });
    g.bench_function("fast_7_matmul", |b| {
        b.iter_batched(
            || random_matrix_pair(TWO, m, k, n),
            |(a, b)| a.fast_mul_7(&b),
            BatchSize::SmallInput,
        );
    });
    g.finish();
}

fn random_matrix_pair(p: ValidPrime, rows: usize, inner: usize, cols: usize) -> (Matrix, Matrix) {
    (random_matrix(p, rows, inner), random_matrix(p, inner, cols))
}

fn random_matrix(p: ValidPrime, rows: usize, cols: usize) -> Matrix {
    Matrix::from_vec(
        p,
        &(0..rows)
            .map(|_| random_vector(p, cols))
            .collect::<Vec<_>>(),
    )
}

fn random_vector(p: ValidPrime, dimension: usize) -> Vec<u32> {
    let mut result = Vec::with_capacity(dimension);
    let mut rng = rand::thread_rng();
    result.resize_with(dimension, || rng.gen::<u32>() % p);
    result
}

criterion_group! {
    name = mul;
    config = Criterion::default()
        .measurement_time(std::time::Duration::from_secs(10))
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = muls
}

criterion_main!(mul);
