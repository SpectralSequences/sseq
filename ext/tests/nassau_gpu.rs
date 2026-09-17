//! End-to-end validation of the GPU `get_partial_matrix` wire-in against the CPU path.
//!
//! Resolves `S_2` (Nassau) over a modest region and, for every non-trivial differential
//! bidegree, asserts [`ext::nassau_gpu::get_partial_matrix`] reproduces the CPU
//! [`ModuleHomomorphism::get_partial_matrix`] bit-for-bit. Requires a live CUDA device +
//! the toolkit env (run under the `gpu` dev shell, unsandboxed); the whole test is
//! `gpu`-gated so it is skipped in ordinary CI.
#![cfg(feature = "gpu")]

use algebra::module::{Module, homomorphism::ModuleHomomorphism};
use ext::{chain_complex::ChainComplex, nassau_gpu, utils::construct_nassau};
use sseq::coordinates::Bidegree;

#[test]
fn gpu_partial_matrix_matches_cpu() {
    let stem = 30;
    let filt = 17;
    let res = construct_nassau(("S_2", "milnor"), None).unwrap();
    res.compute_through_stem(Bidegree::n_s(stem, filt));

    let mut checked_bidegrees = 0usize;
    let mut checked_rows = 0usize;
    for s in 1..=filt {
        let diff = res.differential(s);
        for t in s..=(s + stem) {
            let source_dim = diff.source().dimension(t);
            let target_dim = diff.target().dimension(t);
            if source_dim == 0 || target_dim == 0 {
                continue;
            }
            // Full launch: every source basis element as an input, like the resolution's
            // `get_partial_matrix` call over a signature mask (here the trivial mask).
            let inputs: Vec<usize> = (0..source_dim).collect();
            let gpu = nassau_gpu::get_partial_matrix(&diff, t, &inputs);
            let cpu = diff.get_partial_matrix(t, &inputs);
            for row in 0..source_dim {
                let g: Vec<usize> = gpu.row(row).iter_nonzero().map(|(i, _)| i).collect();
                let c: Vec<usize> = cpu.row(row).iter_nonzero().map(|(i, _)| i).collect();
                assert_eq!(g, c, "mismatch at (s={s}, t={t}), row {row}");
            }
            checked_bidegrees += 1;
            checked_rows += source_dim;
        }
    }
    assert!(
        checked_bidegrees > 0,
        "no non-trivial bidegrees were checked"
    );
    eprintln!("verified {checked_rows} rows across {checked_bidegrees} bidegrees");
}
