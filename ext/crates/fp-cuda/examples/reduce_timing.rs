//! Rough wall-clock comparison of the device-resident reduction against the CPU
//! M4RI reducer, on half-rank square matrices (the target regime shape). Not a
//! rigorous benchmark — a single timed run per size to see whether the device
//! path is in the right ballpark and where the time goes. Correctness is still
//! asserted (device RREF == CPU) so a fast-but-wrong result can't slip through.
//!
//! Run with `cargo run --release -p fp-cuda --example reduce_timing`.

use std::time::Instant;

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::GpuContext;

mod common;
use common::{half_rank, upload_matrix};

fn main() -> anyhow::Result<()> {
    let gpu = GpuContext::new(0)?;
    println!("=== device vs CPU M4RI reduction (half-rank square) ===");

    for &n in &[1024usize, 2048, 4096, 8192] {
        let mm = half_rank(n, n);

        // Device: time upload + full reduce + sync (excludes download).
        let t0 = Instant::now();
        let mut dm = upload_matrix(&gpu, &mm)?;
        let (perm, r, _piv) = gpu.row_reduce_dev(&mut dm)?;
        let dev_secs = t0.elapsed().as_secs_f64();

        // CPU M4RI.
        let mut cpu = mm.clone();
        let t1 = Instant::now();
        let cpu_rank = cpu.row_reduce_cpu();
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
            "  n={n:5} rank={r:5}: device {dev_secs:7.3}s  cpu-m4ri {cpu_secs:7.3}s  speedup \
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
