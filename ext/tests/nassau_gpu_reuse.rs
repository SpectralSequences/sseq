//! End-to-end validation of the GPU compute-once-reuse path in `step_resolution`.
//!
//! Resolves `S_2` (Nassau) twice — once with the CPU per-signature `build_partial_matrix`
//! and once with `NASSAU_GPU` set, which computes the full differential matrix once per
//! bidegree on the GPU and slices each signature's rows out of it — and asserts the two
//! resolutions agree bidegree-by-bidegree. This exercises the `reuse_full_matrix` /
//! `select_rows` restructuring (and the `dx == 0` correction invariant inside the
//! signature loop), which the per-matrix `nassau_gpu` test does not touch.
//!
//! Requires a live CUDA device + toolkit env (run under the `gpu` dev shell, unsandboxed);
//! `gpu`-gated so it is skipped in ordinary CI. Isolated in its own test binary because it
//! toggles the `NASSAU_GPU` process env var, which would race parallel tests.
#![cfg(feature = "gpu")]

use ext::{chain_complex::FreeChainComplex, utils::construct_nassau};
use sseq::coordinates::Bidegree;

fn betti(stem: i32, filt: i32) -> Vec<(i32, i32, usize)> {
    let res = construct_nassau(("S_2", "milnor"), None).unwrap();
    res.compute_through_stem(Bidegree::n_s(stem, filt));
    let mut out = Vec::new();
    for s in 0..=filt {
        for n in 0..=stem {
            let b = Bidegree::n_s(n, s);
            out.push((n, s, res.number_of_gens_in_bidegree(b)));
        }
    }
    out
}

#[test]
fn gpu_reuse_resolution_matches_cpu() {
    let stem: i32 = std::env::var("STEM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let filt: i32 = std::env::var("FILT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(17);

    // SAFETY: single-threaded test (own binary); the env var only gates build dispatch.
    unsafe { std::env::remove_var("NASSAU_GPU") };
    let cpu = betti(stem, filt);

    unsafe { std::env::set_var("NASSAU_GPU", "1") };
    let gpu = betti(stem, filt);

    assert_eq!(
        cpu, gpu,
        "GPU compute-once-reuse resolution disagrees with the CPU resolution"
    );
    let total: usize = cpu.iter().map(|&(_, _, g)| g).sum();
    eprintln!("matched {total} generators across {} bidegrees", cpu.len());
}
