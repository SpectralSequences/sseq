//! Quick check of active-row compaction on a low-rank tall matrix: rank r ≪ m,
//! so once the first panels exhaust the rank every remaining below row is dead.
//! Compaction should drop the per-panel scan cost sharply. Compares device wall
//! time with compaction on vs off (set FP_CUDA_NO_COMPACT in the process to test
//! off — here we time the default-on path and print it; run twice to compare).

use std::time::Instant;

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::GpuContext;

mod common;
use common::upload_matrix;
use rand::Rng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    let mut rng = rand::rng();
    // rank-r m×n matrix = (m×r random) · (r×n random).
    let (m, n, r) = (65536usize, 16384usize, 1024usize);
    let mut build = |rows: usize, cols: usize| -> Matrix {
        let v: Vec<Vec<u32>> = (0..rows)
            .map(|_| (0..cols).map(|_| rng.random::<bool>() as u32).collect())
            .collect();
        Matrix::from_vec(TWO, &v)
    };
    let a = build(m, r);
    let b = build(r, n);
    let mm = &a * &b; // rank ≤ r

    let t0 = Instant::now();
    let mut dm = upload_matrix(&gpu, &mm)?;
    let (_perm, rank, _piv) = gpu.row_reduce_dev(&mut dm)?;
    let secs = t0.elapsed().as_secs_f64();
    let compact = if std::env::var("FP_CUDA_NO_COMPACT").is_ok() {
        "OFF"
    } else {
        "ON "
    };
    println!("m={m} n={n} target_rank={r}: compaction {compact}  device {secs:.3}s  rank={rank}");
    Ok(())
}
