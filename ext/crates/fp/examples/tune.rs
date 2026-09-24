//! Fast tuning target: kernel-only TOPS at a couple of sizes.
//!
//! No CPU cross-check beyond a single correctness spot-check at 4096, so a knob configuration can
//! be compared quickly:
//!
//! `cargo run --release -p fp --features gpu --example tune`

use fp::{blas::cuda::GpuContext, matrix::Matrix, prime::TWO};
use rand::Rng;

fn binary_tops(m: usize, k: usize, n: usize, secs: f64) -> f64 {
    2.0 * (m as f64) * (n as f64) * (k as f64) / secs / 1e12
}

fn main() -> anyhow::Result<()> {
    let gpu = GpuContext::new(0)?;
    let mut rng = rand::rng();
    let mut make = |rows: usize, cols: usize| {
        let data: Vec<u64> = (0..rows * cols.div_ceil(64))
            .map(|_| rng.random())
            .collect();
        Matrix::from_data(TWO, rows, cols, data)
    };

    // One cheap correctness spot-check so a broken config is caught fast.
    {
        let a = make(4096, 4096);
        let b = make(4096, 4096);
        let cpu = &a * &b;
        let gpu_ref = a.cuda_mul(&gpu, &b)?;
        if cpu != gpu_ref {
            eprintln!("CORRECTNESS FAILURE at 4096");
            std::process::exit(1);
        }
    }

    for &(m, k, n, iters) in &[
        (8192usize, 8192, 8192, 30),
        (16384, 16384, 16384, 15),
        (32768, 32768, 32768, 8),
    ] {
        let a = make(m, k);
        let b = make(k, n);
        let (_, secs) = a.cuda_mul_timed(&gpu, &b, iters)?;
        println!(
            "  {m:>6} x {k:>6} x {n:>6}: {:>7.1} TOPS  ({:>8.3} ms)",
            binary_tops(m, k, n, secs),
            secs * 1e3,
        );
    }
    Ok(())
}
