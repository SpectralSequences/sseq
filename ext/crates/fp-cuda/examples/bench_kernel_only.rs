//! Kernel-only throughput for `matmul_b1`.
//!
//! Unlike `bench_kernel` (end-to-end, host-serialization-bound) and the
//! `cargo bench` criterion harness (also end-to-end), this isolates the GPU
//! kernel: all host (de)serialization, the TMA-layout pre-arrangement, and the
//! H2D/D2H copies happen once, then only back-to-back kernel launches are
//! timed (see `matmul_b1_timed`). This is the apples-to-apples number to
//! compare against the ~100-binary-TOPS pre-swizzle kernel baseline.
//!
//! Run: `cargo run --release -p fp-cuda --example bench_kernel_only`.

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::{GpuContext, matmul_b1, matmul_b1_timed};
use rand::Rng;

fn binary_tops(m: usize, k: usize, n: usize, secs: f64) -> f64 {
    2.0 * (m as f64) * (n as f64) * (k as f64) / secs / 1e12
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    let (major, minor) = gpu.compute_capability()?;
    println!("GPU: sm_{major}{minor}");
    println!("Kernel-only binary TOPS (host setup + H2D/D2H excluded):\n");

    let mut rng = rand::rng();
    let mut make = |rows: usize, cols: usize| {
        let data: Vec<u64> = (0..rows * cols.div_ceil(64))
            .map(|_| rng.random())
            .collect();
        Matrix::from_data(TWO, rows, cols, data)
    };

    for &(m, k, n, iters) in &[
        (4096usize, 4096, 4096, 50),
        (8192, 8192, 8192, 30),
        (16384, 16384, 16384, 10),
        (32768, 32768, 32768, 5),
    ] {
        let a = make(m, k);
        let b = make(k, n);

        // Bit-exact correctness check against the CPU path once per shape.
        let cpu = &a * &b;
        let (gpu_ref, _) = matmul_b1_timed(&gpu, &a, &b, 1)?;
        let ok = cpu == gpu_ref;
        // matmul_b1 (single launch) must agree with the timed multi-launch path.
        let single = matmul_b1(&gpu, &a, &b)?;
        let idempotent = single == gpu_ref;

        let (_, secs) = matmul_b1_timed(&gpu, &a, &b, iters)?;
        println!(
            "  {m:>6} x {k:>6} x {n:>6}: {:>7.1} binary TOPS  ({:>8.3} ms/launch, {iters} iters)  \
             correct={ok} idempotent={idempotent}",
            binary_tops(m, k, n, secs),
            secs * 1e3,
        );
        if !ok || !idempotent {
            eprintln!("    CORRECTNESS FAILURE at {m}x{k}x{n}");
            std::process::exit(1);
        }
    }

    println!("\nH100 binary tensor-op peak is ~360,000 TOPS.");
    Ok(())
}
