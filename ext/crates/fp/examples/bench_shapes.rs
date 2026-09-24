//! Kernel-only throughput at equal-work shapes where B does and does not fit in L2.

use fp::{blas::cuda::GpuContext, matrix::Matrix, prime::TWO};
use rand::Rng;

/// Binary TOPS of an `m × k` by `k × n` product that took `secs`.
fn binary_tops(m: usize, k: usize, n: usize, secs: f64) -> f64 {
    2.0 * (m as f64) * (n as f64) * (k as f64) / secs / 1e12
}

/// Time shapes that hold the work fixed while flipping whether B fits in L2.
///
/// Each B column panel is reused across every M tile, so L2 reuse depends only on whether the whole
/// of B fits; a size or occupancy effect would not track that.
fn main() -> anyhow::Result<()> {
    let gpu = GpuContext::new(0)?;
    // The H100/H200 NVL L2 capacity, assumed rather than read from the device; change it when
    // profiling another card.
    let l2_mb = 50.0;
    println!(
        "Assumed GPU L2 ~= {l2_mb} MB (H100/H200 NVL). B in L2 (bytes = K*N/8) governs \
         cross-M-tile reuse.\n"
    );

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
        "{:>7} {:>7} {:>7} | {:>9} {:>6} | {:>9} | note",
        "M", "K", "N", "B (MB)", "fits", "kern TOPS"
    );
    for &(m, k, n, iters, note) in &shapes {
        let b_mb = (k as f64) * (n as f64) / 8.0 / 1e6;
        let a = make(m, k);
        let b = make(k, n);
        let (_, secs) = a.cuda_mul_timed(&gpu, &b, iters)?;
        println!(
            "{m:>7} {k:>7} {n:>7} | {b_mb:>9.1} {:>6} | {:>9.1} | {note}",
            if b_mb <= l2_mb { "yes" } else { "NO" },
            binary_tops(m, k, n, secs),
        );
    }
    Ok(())
}
