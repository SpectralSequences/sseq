//! GPU-dispatch correctness for `<&Matrix as Mul>::mul` under the `gpu`
//! feature. Only compiled when the feature is on; if no usable GPU is present
//! the dispatch transparently falls back to the CPU kernel, so this test still
//! passes (it just doesn't exercise the device). Run with `FP_CUDA_DEBUG=1` to
//! see the `[fp-cuda]` launch line and confirm the GPU path was taken.
#![cfg(feature = "gpu")]

use fp::{matrix::Matrix, prime::TWO};
use rand::Rng;

fn random_matrix(rows: usize, cols: usize) -> Matrix {
    let mut rng = rand::rng();
    let limbs = cols.div_ceil(64);
    let data: Vec<u64> = (0..rows * limbs).map(|_| rng.random()).collect();
    Matrix::from_data(TWO, rows, cols, data)
}

/// Well-formed 0/1 matrix (bits past the last column masked), of rank ≤ `rank`
/// when `rank > 0`, else full-random.
fn clean_matrix(rows: usize, cols: usize, rank: usize) -> Matrix {
    let mut rng = rand::rng();
    let mut rand_vec = |r: usize, c: usize| -> Matrix {
        let v: Vec<Vec<u32>> = (0..r)
            .map(|_| (0..c).map(|_| rng.random::<bool>() as u32).collect())
            .collect();
        Matrix::from_vec(TWO, &v)
    };
    if rank == 0 {
        rand_vec(rows, cols)
    } else {
        &rand_vec(rows, rank) * &rand_vec(rank, cols)
    }
}

/// The dispatched product must be bit-identical to the CPU BLAS kernel. Sizes
/// are chosen above the default 2048 threshold so the GPU path is attempted.
#[test]
fn gpu_dispatch_matches_cpu() {
    for &(m, k, n) in &[
        (2048, 2048, 2048),
        (4096, 2048, 3072),
        (3072, 4096, 2048),
        // Non-tile-aligned dims (not multiples of the kernel's 192/128/1024
        // tiles; rows still pad to a multiple of 64 so dispatch fires):
        // exercise edge masks, partial limbs, and raster tails.
        (2049, 2051, 2053),
        (3000, 2112, 4097),
    ] {
        let a = random_matrix(m, k);
        let b = random_matrix(k, n);

        // `&a * &b` consults the GPU (feature on); `fast_mul_concurrent` is the
        // reference CPU kernel. They must agree bit-for-bit.
        let dispatched = &a * &b;
        let reference = a.fast_mul_concurrent(&b);

        assert_eq!(
            dispatched, reference,
            "GPU/CPU mismatch at {m}x{k} * {k}x{n}"
        );
    }
}

/// Many threads matmul-ing on the GPU AT ONCE must each stay bit-identical to the CPU — the
/// concurrency the per-thread-stream refactor enables. If concurrent matmuls shared device state this
/// would corrupt or LAUNCH_FAILED; independent per-stream buffers make it pass.
#[test]
fn gpu_matmul_concurrent() {
    const THREADS: usize = 16;
    const ITERS: usize = 6;
    std::thread::scope(|s| {
        for t in 0..THREADS {
            s.spawn(move || {
                for i in 0..ITERS {
                    let m = 2048 + 256 * (t % 6);
                    let k = 2048 + 256 * (i % 5);
                    let n = 2048 + 128 * ((t + i) % 6);
                    let a = random_matrix(m, k);
                    let b = random_matrix(k, n);
                    let dispatched = &a * &b;
                    let reference = a.fast_mul_concurrent(&b);
                    assert_eq!(
                        dispatched, reference,
                        "concurrent matmul mismatch {m}x{k}*{k}x{n} (t{t} i{i})"
                    );
                }
            });
        }
    });
}

/// The dispatched `row_reduce` (GPU, feature on) must be bit-identical to the CPU
/// BLAS3 reducer — RREF, rank, and pivots. The production row-reduce threshold is
/// 8192 (below that the CPU wins); force it down here so these small, fast test
/// shapes still exercise the GPU path. This test uses `FP_CUDA_RR_THRESHOLD`,
/// distinct from the matmul test's `FP_CUDA_THRESHOLD`, so the two don't collide.
#[test]
fn gpu_row_reduce_matches_cpu() {
    // SAFETY: set once at the start of the test, before any threshold() read.
    unsafe { std::env::set_var("FP_CUDA_RR_THRESHOLD", "2048") };
    for &(rows, cols, rank) in &[
        (2048, 2048, 0),
        (4096, 2560, 0),
        (2048, 3000, 0),
        (3000, 2048, 500), // rank-deficient
    ] {
        let base = clean_matrix(rows, cols, rank);

        let mut gpu = base.clone();
        let rank_gpu = gpu.row_reduce(); // GPU dispatch (feature on, above threshold)
        let mut cpu = base.clone();
        let rank_cpu = cpu.row_reduce_blas3(); // CPU oracle, never dispatches

        assert_eq!(
            rank_gpu, rank_cpu,
            "rank mismatch at {rows}x{cols} rank={rank}"
        );
        assert_eq!(
            gpu.pivots(),
            cpu.pivots(),
            "pivot mismatch at {rows}x{cols}"
        );
        assert_eq!(gpu, cpu, "RREF mismatch at {rows}x{cols} rank={rank}");
    }
}
