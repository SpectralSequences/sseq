//! Minimal profiling target: exactly one `matmul_b1` kernel launch at each of
//! two sizes (16384, 32768), no CPU cross-check. Built for `ncu --launch-count`
//! so the L2-cliff hypothesis can be checked with hardware counters.
//!
//! `ncu --set basic --launch-count 2 target/release/examples/prof_sizes`

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::{GpuContext, matmul_b1};
use rand::Rng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    let mut rng = rand::rng();
    let mut make = |rows: usize, cols: usize| {
        let data: Vec<u64> = (0..rows * cols.div_ceil(64))
            .map(|_| rng.random())
            .collect();
        Matrix::from_data(TWO, rows, cols, data)
    };

    for &n in &[16384usize, 32768] {
        let a = make(n, n);
        let b = make(n, n);
        let _ = matmul_b1(&gpu, &a, &b)?;
        eprintln!("launched {n}");
    }
    Ok(())
}
