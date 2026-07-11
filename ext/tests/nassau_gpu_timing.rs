//! End-to-end timing of the GPU wire-in: resolve `S_2` (Nassau) with and without the
//! GPU `get_partial_matrix` path and print wall times. `#[ignore]` by default (it is a
//! benchmark, not a correctness check); run explicitly under the `gpu` dev shell:
//! `cargo test --test nassau_gpu_timing --features gpu -- --ignored --nocapture --test-threads=1`.
//! `--test-threads=1` is required: both tests toggle the process-wide `NASSAU_GPU` env var, so
//! running them concurrently would race (and `set_var`/`remove_var` are themselves unsound under
//! concurrency).
#![cfg(feature = "gpu")]

use std::time::Instant;

use ext::utils::construct_nassau;
use sseq::coordinates::Bidegree;

fn resolve_time(stem: i32, filt: i32) -> f64 {
    let res = construct_nassau(("S_2", "milnor"), None).unwrap();
    let t0 = Instant::now();
    res.compute_through_stem(Bidegree::n_s(stem, filt));
    t0.elapsed().as_secs_f64()
}

/// GPU-only resolution (no CPU baseline) — for iterating on the GPU path at large stems
/// without paying the multi-minute CPU run each time. `STEM`/`FILT` env, default 100/55.
#[test]
#[ignore = "GPU-only benchmark; run explicitly with --ignored"]
fn gpu_only_resolution_time() {
    let stem: i32 = std::env::var("STEM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);
    let filt: i32 = std::env::var("FILT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(55);
    // SAFETY: single-threaded test setup; the env var only gates build_partial_matrix.
    unsafe { std::env::set_var("NASSAU_GPU", "1") };
    let _ = algebra::milnor_gpu::take_batch_stats();
    let gpu = resolve_time(stem, filt);
    let (calls, marshal_us, device_us, pairs) = algebra::milnor_gpu::take_batch_stats();
    eprintln!("GPU-only (stem {stem}, filt {filt}): {gpu:.1}s");
    eprintln!(
        "GPU launches: {calls}  |  host marshal {:.2}s  device {:.2}s  |  avg {:.0} pairs/launch",
        marshal_us as f64 / 1e6,
        device_us as f64 / 1e6,
        pairs as f64 / calls.max(1) as f64,
    );
}

#[test]
#[ignore = "timing benchmark; run explicitly with --ignored"]
fn gpu_vs_cpu_resolution_time() {
    let stem: i32 = std::env::var("STEM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);
    let filt: i32 = std::env::var("FILT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(27);

    // SAFETY: single-threaded test setup; the env var only gates build_partial_matrix.
    unsafe { std::env::remove_var("NASSAU_GPU") };
    let cpu = resolve_time(stem, filt);
    eprintln!("CPU  (stem {stem}, filt {filt}): {cpu:.1}s");

    let _ = algebra::milnor_gpu::take_batch_stats(); // reset before the GPU run
    unsafe { std::env::set_var("NASSAU_GPU", "1") };
    let gpu = resolve_time(stem, filt);
    eprintln!("GPU  (stem {stem}, filt {filt}): {gpu:.1}s");

    let (calls, marshal_us, device_us, pairs) = algebra::milnor_gpu::take_batch_stats();
    eprintln!(
        "GPU launches: {calls}  |  host marshal {:.2}s  device {:.2}s  |  avg {:.0} pairs/launch",
        marshal_us as f64 / 1e6,
        device_us as f64 / 1e6,
        pairs as f64 / calls.max(1) as f64,
    );
    eprintln!("speedup (CPU / GPU): {:.2}×", cpu / gpu);
}
