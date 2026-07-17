//! Probe the GEMM's correctness as a function of the contraction dimension `k`,
//! at fixed large `m`, `n`. Isolates the small-`k` mismatch seen in
//! `matmul_b1_dev_demo`. Uses explicit 0/1 matrices (no high-bit garbage past
//! the last real column) so a mismatch can only be the kernel, not test data.

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::GpuContext;

mod common;
use common::matmul_b1;
use rand::Rng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    let mut rng = rand::rng();
    // Build via from_vec with genuine 0/1 entries: Matrix masks unused bits.
    let mut make = |rows: usize, cols: usize| {
        let v: Vec<Vec<u32>> = (0..rows)
            .map(|_| (0..cols).map(|_| rng.random::<bool>() as u32).collect())
            .collect();
        Matrix::from_vec(TWO, &v)
    };
    let (m, n) = (4096usize, 256usize);
    println!("m={m} n={n}, sweeping k:");
    for k in [
        1, 2, 4, 8, 16, 32, 48, 56, 60, 62, 63, 64, 65, 66, 96, 128, 256,
    ] {
        let a = make(m, k);
        let b = make(k, n);
        let cpu = &a * &b;
        let gpu_out = matmul_b1(&gpu, &a, &b)?;
        println!(
            "  k={k:4}: {}",
            if cpu == gpu_out { "OK" } else { "MISMATCH" }
        );
    }
    Ok(())
}
