use std::time::Instant;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::{GpuContext, matmul_b1};
use rand::Rng;

const SIZES: &[usize] = &[128, 256, 512, 1024, 2048, 4096, 8192];

fn random_matrix(rows: usize, cols: usize) -> Matrix {
    let mut rng = rand::rng();
    let data_len = rows * cols.div_ceil(64);
    let data: Vec<u64> = (0..data_len).map(|_| rng.random()).collect();
    Matrix::from_data(TWO, rows, cols, data)
}

fn assert_bit_equal(cpu: &Matrix, gpu: &Matrix) {
    assert_eq!(cpu.rows(), gpu.rows(), "row count mismatch");
    assert_eq!(cpu.columns(), gpu.columns(), "column count mismatch");
    assert!(
        cpu == gpu,
        "CPU and GPU F_2 matmul results disagree at {}x{}",
        cpu.rows(),
        cpu.columns(),
    );
}

fn binary_tops(m: usize, k: usize, n: usize, secs: f64) -> f64 {
    // 2 * M * N * K binary ops (one AND + one XOR per inner-product step).
    2.0 * (m as f64) * (n as f64) * (k as f64) / secs / 1e12
}

fn bench_square(c: &mut Criterion, gpu: &GpuContext, size: usize) {
    let mut group = c.benchmark_group(format!("matmul_b1_{size}x{size}"));
    group.throughput(criterion::Throughput::Elements(
        (2 * size * size * size) as u64,
    ));

    // One-shot correctness check (outside the criterion timing loop) before benching.
    let (a, b) = (random_matrix(size, size), random_matrix(size, size));
    let cpu_ref = &a * &b;
    let gpu_ref = matmul_b1(gpu, &a, &b).expect("GPU matmul launch failed");
    assert_bit_equal(&cpu_ref, &gpu_ref);

    group.bench_function("cpu_fast_mul_concurrent", |bencher| {
        bencher.iter_batched(
            || (random_matrix(size, size), random_matrix(size, size)),
            |(a, b)| &a * &b,
            BatchSize::SmallInput,
        );
    });

    group.bench_function("gpu_matmul_b1", |bencher| {
        bencher.iter_batched(
            || (random_matrix(size, size), random_matrix(size, size)),
            |(a, b)| matmul_b1(gpu, &a, &b).expect("GPU matmul launch failed"),
            BatchSize::SmallInput,
        );
    });

    group.finish();

    // Coarse manual TFLOPS report (criterion has its own throughput, but explicit
    // logging makes binary-op throughput easy to grep from the bench output).
    let runs = 5;
    let start = Instant::now();
    for _ in 0..runs {
        let _ = matmul_b1(gpu, &a, &b).expect("GPU matmul launch failed");
    }
    let gpu_avg = start.elapsed().as_secs_f64() / runs as f64;

    let start = Instant::now();
    for _ in 0..runs {
        let _ = &a * &b;
    }
    let cpu_avg = start.elapsed().as_secs_f64() / runs as f64;

    println!(
        "[matmul_b1_{size}x{size}] CPU {:.2} TOPS ({:.2} ms)  GPU {:.2} TOPS ({:.2} ms)  speedup \
         {:.2}x",
        binary_tops(size, size, size, cpu_avg),
        cpu_avg * 1e3,
        binary_tops(size, size, size, gpu_avg),
        gpu_avg * 1e3,
        cpu_avg / gpu_avg,
    );
}

fn bench_all(c: &mut Criterion) {
    let gpu = GpuContext::new(0).expect("failed to initialise GpuContext");
    let (major, minor) = gpu
        .compute_capability()
        .expect("failed to query compute capability");
    println!("fp-cuda bench running on sm_{major}{minor}");

    for &size in SIZES {
        bench_square(c, &gpu, size);
    }
}

criterion_group! {
    name = matmul_b1_bench;
    config = Criterion::default().measurement_time(std::time::Duration::from_secs(3));
    targets = bench_all
}
criterion_main!(matmul_b1_bench);
