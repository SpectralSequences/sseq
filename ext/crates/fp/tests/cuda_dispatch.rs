//! GPU-dispatch correctness for `<&Matrix as Mul>::mul` under the `cuda`
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

/// The dispatched product must be bit-identical to the CPU BLAS kernel. Sizes
/// are chosen above the default 2048 threshold so the GPU path is attempted.
#[test]
fn gpu_dispatch_matches_cpu() {
    for &(m, k, n) in &[(2048, 2048, 2048), (4096, 2048, 3072), (3072, 4096, 2048)] {
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
