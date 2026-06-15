//! Isolate *why* kernel-only throughput drops past N=16384: is it total size, or
//! the B operand spilling out of the 50 MB L2?
//!
//! Each B column-panel is reused across every M-tile, so the L2-reuse condition
//! is simply whether the whole B matrix (K*N/8 bytes) fits in L2. These shapes
//! hold FLOPs fixed while flipping "B fits in L2", which a pure size/occupancy
//! story cannot explain.
//!
//! Run: `cargo run --release -p fp-cuda --example bench_shapes`.

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::{GpuContext, matmul_b1_timed};
use rand::Rng;

fn binary_tops(m: usize, k: usize, n: usize, secs: f64) -> f64 {
    2.0 * (m as f64) * (n as f64) * (k as f64) / secs / 1e12
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    let l2_mb = 50.0; // H100 NVL L2
    println!("GPU L2 ~= {l2_mb} MB. B in L2 (bytes = K*N/8) governs cross-M-tile reuse.\n");

    let mut rng = rand::rng();
    let mut make = |rows: usize, cols: usize| {
        let data: Vec<u64> = (0..rows * cols.div_ceil(64))
            .map(|_| rng.random())
            .collect();
        Matrix::from_data(TWO, rows, cols, data)
    };

    // (M, K, N, iters, note)
    let shapes = [
        (16384usize, 16384, 16384, 10, "cube, B fits"),
        (32768, 32768, 32768, 5, "cube, B spills"),
        // Same FLOPs (1.76e13), only B-in-L2 differs:
        (
            65536,
            16384,
            16384,
            10,
            "tall: huge M, B FITS  (=16384^3 x4 FLOPs)",
        ),
        (
            16384,
            16384,
            65536,
            5,
            "wide: huge N, B SPILLS (same FLOPs as above)",
        ),
        (
            16384,
            65536,
            16384,
            5,
            "deep: huge K, B SPILLS (same FLOPs as above)",
        ),
        // B fits even at 2x the tall-case FLOPs:
        (
            131072,
            16384,
            16384,
            5,
            "taller: M=128K, B FITS (=16384^3 x8 FLOPs)",
        ),
    ];

    println!(
        "{:>7} {:>7} {:>7} | {:>9} {:>6} | {:>9} | {}",
        "M", "K", "N", "B (MB)", "fits", "TOPS", "note"
    );
    for &(m, k, n, iters, note) in &shapes {
        let b_mb = (k as f64) * (n as f64) / 8.0 / 1e6;
        let a = make(m, k);
        let b = make(k, n);
        let (_, secs) = matmul_b1_timed(&gpu, &a, &b, iters)?;
        println!(
            "{m:>7} {k:>7} {n:>7} | {b_mb:>9.1} {:>6} | {:>9.1} | {note}",
            if b_mb <= l2_mb { "yes" } else { "NO" },
            binary_tops(m, k, n, secs),
        );
    }
    Ok(())
}
