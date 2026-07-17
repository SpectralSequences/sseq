//! Scaling comparison of the device-resident reduction against the CPU M4RI reducer on random
//! square F₂ matrices of size 2^n, n = 10..=15. Random square matrices over F₂ are
//! essentially full rank, so this is the full-rank regime (work ∝ rank = n);
//! the target workload is half-rank, where the device lead is larger.
//!
//! The CPU baseline runs multi-threaded (the example's `fp` enables
//! `concurrent`); the device path is reached directly, never through fp's GPU
//! dispatch. Correctness is asserted each size (device RREF == M4RI).
//!
//! Sizes are multiples of 64, so `from_data` with random limbs is well-formed.
//!
//! Run with `cargo run --release -p fp-cuda --example reduce_pow2`.

use std::time::Instant;

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::GpuContext;

mod common;
use common::upload_matrix;
use rand::Rng;

fn main() -> anyhow::Result<()> {
    let gpu = GpuContext::new(0)?;
    let max_n: u32 = std::env::var("REDUCE_POW2_MAX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(15);
    println!("=== device vs CPU M4RI — random square 2^n, n=10..={max_n} ===");
    println!(
        "{:>7}  {:>10}  {:>10}   {:>9}  {:>9}",
        "n", "device(s)", "m4ri(s)", "vs-m4ri", "Tbop/s"
    );

    let mut rng = rand::rng();
    for e in 10..=max_n {
        let n = 1usize << e;
        let stride = n / 64; // n is a multiple of 64
        let data: Vec<u64> = (0..n * stride).map(|_| rng.random()).collect();
        let mm = Matrix::from_data(TWO, n, n, data);

        // Device: upload + full reduce + sync (excludes download).
        let t0 = Instant::now();
        let mut dm = upload_matrix(&gpu, &mm)?;
        let (perm, r, _piv) = gpu.row_reduce_dev(&mut dm)?;
        let dev_secs = t0.elapsed().as_secs_f64();

        // CPU M4RI (pure CPU, multi-threaded).
        let mut m4ri = mm.clone();
        let t1 = Instant::now();
        let m4ri_rank = m4ri.row_reduce();
        let m4ri_secs = t1.elapsed().as_secs_f64();

        // Correctness: materialize device RREF (pivot k at row k via perm) and
        // compare to the M4RI result.
        let dev_limbs = gpu.download(&dm)?;
        let perm_host = gpu.download_u32(&perm)?;
        let mut e_limbs = vec![0u64; n * stride];
        for k in 0..r {
            let src = perm_host[k] as usize * stride;
            e_limbs[k * stride..k * stride + stride].copy_from_slice(&dev_limbs[src..src + stride]);
        }
        let dev_rref = Matrix::from_data(TWO, n, n, e_limbs);
        let ok = r == m4ri_rank && dev_rref == m4ri;

        // Effective binary-op throughput: leading-order elimination work
        // 2·m·n·R (bit-MAC = AND + accumulate = 2 ops). Tbop/s = 10^12 ops/s.
        let tbops = 2.0 * n as f64 * n as f64 * r as f64 / dev_secs / 1e12;
        println!(
            "{:>7}  {:>10.3}  {:>10.3}   {:>8.2}x  {:>9.1}   rank={r} [{}]",
            n,
            dev_secs,
            m4ri_secs,
            m4ri_secs / dev_secs,
            tbops,
            if ok { "ok" } else { "WRONG" },
        );
        if !ok {
            std::process::exit(1);
        }
    }
    Ok(())
}
