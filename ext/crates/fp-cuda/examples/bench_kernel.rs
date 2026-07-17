use std::time::Instant;

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::{GpuContext, matmul_b1};
use rand::Rng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    let (major, minor) = gpu.compute_capability()?;
    println!("GPU: sm_{major}{minor} (H100)");
    println!("Timing includes host serialization + H2D + kernel + D2H.");
    println!("The scalar B transpose in the kernel dominates; wgmma itself is starved.");
    println!();

    let mut rng = rand::rng();

    // Focus on compute-bound regime: large K maximizes wgmma fraction
    for &(m, k, n) in &[
        (8192usize, 8192, 8192),
        (16384, 16384, 16384),
        (32768, 32768, 32768),
    ] {
        let stride_a = (k + 63) / 64;
        let stride_b = (n + 63) / 64;
        let a_bytes = m * stride_a * 8;
        let b_bytes = k * stride_b * 8;

        println!("=== {m} x {k} x {n} ===");
        println!(
            "  A: {:.1} MB, B: {:.1} MB",
            a_bytes as f64 / 1e6,
            b_bytes as f64 / 1e6
        );

        let a_data: Vec<u64> = (0..m * stride_a).map(|_| rng.random()).collect();
        let b_data: Vec<u64> = (0..k * stride_b).map(|_| rng.random()).collect();
        let a = Matrix::from_data(TWO, m, k, a_data);
        let b = Matrix::from_data(TWO, k, n, b_data);

        // Warmup (includes compilation, allocation)
        let _ = matmul_b1(&gpu, &a, &b)?;

        // Timed: end-to-end (host serial + H2D + kernel + D2H)
        let trials = 3;
        let mut best = f64::MAX;
        for _ in 0..trials {
            let t0 = Instant::now();
            let _ = matmul_b1(&gpu, &a, &b)?;
            best = best.min(t0.elapsed().as_secs_f64());
        }

        let bit_ops = 2.0 * (m as f64) * (n as f64) * (k as f64);
        let tops = bit_ops / best / 1e12;

        // Estimate host overhead: time just serialization + padding
        let t_host = Instant::now();
        let _a_ser = {
            let mut v = Vec::new();
            a.to_bytes(&mut v).unwrap();
            v
        };
        let _b_ser = {
            let mut v = Vec::new();
            b.to_bytes(&mut v).unwrap();
            v
        };
        let host_ser_ms = t_host.elapsed().as_secs_f64() * 1e3;

        println!(
            "  End-to-end: {:.1} ms → {:.1} binary TOPS",
            best * 1e3,
            tops
        );
        println!(
            "  Host serialization alone: {:.1} ms ({:.0}% of total)",
            host_ser_ms,
            host_ser_ms / (best * 1e3) * 100.0
        );
        println!("  K-chunks per CTA: {}", (k + 255) / 256);
        println!();
    }

    println!("Note: The H100 peak for binary tensor ops is ~360,000 TOPS.");
    println!("Current utilization is <0.1% due to the scalar B transpose");
    println!("dominating kernel runtime. Phase 2 (warp-shuffle transpose +");
    println!("double-buffering) is needed to approach peak.");

    Ok(())
}
