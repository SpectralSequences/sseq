//! Rough wall-clock comparison of the device-resident reduction against the CPU
//! BLAS3 reducer, on half-rank square matrices (the target regime shape). Not a
//! rigorous benchmark — a single timed run per size to see whether the device
//! path is in the right ballpark and where the time goes. Correctness is still
//! asserted (device RREF == CPU) so a fast-but-wrong result can't slip through.
//!
//! Run with `cargo run --release -p fp-cuda --example reduce_timing`.

use std::time::Instant;

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::GpuContext;

mod common;
use common::upload_matrix;
use rand::Rng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    println!("=== device vs CPU BLAS3 reduction (half-rank square) ===");

    let mut rng = rand::rng();
    for &n in &[1024usize, 2048, 4096, 8192] {
        let rank = n / 2;
        // Half-rank n×n via A(n×rank)·B(rank×n).
        let a: Vec<Vec<u32>> = (0..n)
            .map(|_| (0..rank).map(|_| rng.random::<bool>() as u32).collect())
            .collect();
        let b: Vec<Vec<u32>> = (0..rank)
            .map(|_| (0..n).map(|_| rng.random::<bool>() as u32).collect())
            .collect();
        let mm = &Matrix::from_vec(TWO, &a) * &Matrix::from_vec(TWO, &b);

        // Device: time upload + full reduce + sync (excludes download).
        let t0 = Instant::now();
        let mut dm = upload_matrix(&gpu, &mm)?;
        let (perm, r, _piv) = gpu.row_reduce_dev(&mut dm)?;
        let dev_secs = t0.elapsed().as_secs_f64();

        // CPU BLAS3.
        let mut cpu = mm.clone();
        let t1 = Instant::now();
        let cpu_rank = cpu.row_reduce_blas3();
        let cpu_secs = t1.elapsed().as_secs_f64();

        // Correctness: materialize device RREF and compare to CPU.
        let stride = n.div_ceil(64);
        let dev_limbs = gpu.download(&dm)?;
        let perm_host = gpu.download_u32(&perm)?;
        let mut e_limbs = vec![0u64; n * stride];
        for k in 0..r {
            let src = perm_host[k] as usize * stride;
            e_limbs[k * stride..k * stride + stride].copy_from_slice(&dev_limbs[src..src + stride]);
        }
        let e = Matrix::from_data(TWO, n, n, e_limbs);
        let ok = r == cpu_rank && e == cpu;

        println!(
            "  n={n:5} rank={r:5}: device {dev_secs:7.3}s  cpu-blas3 {cpu_secs:7.3}s  speedup \
             {:5.2}x  [{}]",
            cpu_secs / dev_secs,
            if ok { "correct" } else { "WRONG" }
        );
        if !ok {
            std::process::exit(1);
        }
    }
    Ok(())
}
