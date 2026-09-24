//! Kernel-only throughput of the GPU matmul: the number to quote.

use fp::{blas::cuda::GpuContext, matrix::Matrix, prime::TWO};
use rand::Rng;

/// Binary TOPS of an `m × k` by `k × n` product that took `secs`.
fn binary_tops(m: usize, k: usize, n: usize, secs: f64) -> f64 {
    2.0 * (m as f64) * (n as f64) * (k as f64) / secs / 1e12
}

/// Time back-to-back launches at a few cube sizes, and check each product against the CPU.
///
/// Host marshalling and the H2D/D2H copies happen once per size and are not timed; see
/// `Matrix::cuda_mul_timed`.
fn main() -> anyhow::Result<()> {
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

        let (c, secs) = a.cuda_mul_timed(&gpu, &b, iters)?;
        let ok = c == &a * &b;
        println!(
            "  {m:>6} x {k:>6} x {n:>6}: {:>7.1} binary TOPS  ({:>8.3} ms/launch, {iters} iters)  \
             correct={ok}",
            binary_tops(m, k, n, secs),
            secs * 1e3,
        );
        if !ok {
            eprintln!("    CORRECTNESS FAILURE at {m}x{k}x{n}");
            std::process::exit(1);
        }
    }

    println!("\nH100 binary tensor-op peak is ~360,000 TOPS.");
    Ok(())
}
