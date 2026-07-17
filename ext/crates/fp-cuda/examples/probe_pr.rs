//! P0-a — flat-K microbench (Pass 0 of the optimization plan).
//!
//! The trailing GEMM of the row reduction is `C = L(m×pr) · U(pr×T)` where
//! `pr` = pivots-per-panel ≤ 64. But `matmul_b1_dev` pads the contraction
//! dimension to `TILE_K = 1024` (`k.next_multiple_of(TILE_K)`), so the tensor
//! cores issue a full K=1024 GEMM regardless of `pr`. This probe measures
//! **kernel-only** wall time as a function of `pr` at fixed `m, T`, using the
//! same `matmul_b1_raw_timed` path (host pack, but the timed loop is GPU GEMM
//! only). If wall time is flat from pr=64 to pr=1024, Loss 1 (16× K-padding) is
//! confirmed: ~15/16 of tensor-core issue is multiply-by-zero, and widening the
//! panel (raising `pr` toward `b`) is worth up to ~16× on the trailing GEMM.
//!
//! The effective-throughput column divides by the *real* work `2·m·T·pr`, so a
//! flat wall time shows Tbop/s climbing ~linearly with `pr` — exactly the
//! headroom Pass 3 reclaims.
//!
//! Run with `cargo run --release -p fp-cuda --example probe_pr`.

use fp_cuda::{GpuContext, matmul_b1_raw_timed};
use rand::Rng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    let mut rng = rand::rng();

    // Fixed trailing-update shape: tall-skinny GEMM like the real workload at
    // n≈32768 (m rows still active, T trailing columns).
    let m: usize = std::env::var("PROBE_M")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(32768);
    let t: usize = std::env::var("PROBE_T")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(16384);
    let iters: usize = 20;
    let t_lim = t.div_ceil(64);

    println!("=== P0-a flat-K probe: C = L(m×pr)·U(pr×T), m={m} T={t}, {iters} iters ===");
    println!(
        "{:>6}  {:>12}  {:>12}  {:>10}",
        "pr", "kernel(ms)", "Tbop/s", "vs pr=64"
    );

    // Small pr (≤1024) all pad to TILE_K=1024, so time is flat — the K-padding
    // signature. Large pr (≥1024) uses nchunks = pr/1024 real K-tiles: if kernel
    // time then scales ~linearly with pr, the kernel is K-work bound (each K-tile
    // costs a fixed load+compute), which means a native small-TILE_K kernel would
    // cut time proportionally — the payoff that justifies the fork. If large-pr
    // time is instead sublinear/flat, a fixed per-launch overhead dominates and
    // the fork would not help.
    let pr_list = std::env::var("PROBE_LARGE")
        .map(|_| vec![1024usize, 2048, 3072])
        .unwrap_or_else(|_| vec![64, 128, 256, 512, 1024]);
    let mut base_ms = 0.0f64;
    for (i, &pr) in pr_list.iter().enumerate() {
        let a_lim = pr.div_ceil(64);
        let a: Vec<u64> = (0..m * a_lim).map(|_| rng.random()).collect();
        let b: Vec<u64> = (0..pr * t_lim).map(|_| rng.random()).collect();
        let (_c, secs) = matmul_b1_raw_timed(&gpu, &a, m, pr, &b, t, iters)?;
        let ms = secs * 1e3;
        if i == 0 {
            base_ms = ms;
        }
        let bops = 2.0 * m as f64 * t as f64 * pr as f64;
        let tbops = bops / secs / 1e12;
        println!("{pr:>6}  {ms:>12.3}  {tbops:>12.1}  {:>9.2}x", ms / base_ms);
    }
    println!(
        "\nFlat kernel(ms) across pr ⇒ Loss 1 confirmed (K padded to {}).",
        1024
    );
    Ok(())
}
