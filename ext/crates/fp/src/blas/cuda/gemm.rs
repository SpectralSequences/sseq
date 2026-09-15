//! The F₂ matrix product, on Hopper `wgmma.b1`.

use super::{context, driver, fill_limbs};
use crate::{matrix::Matrix, prime::TWO};

/// Smallest `min(m, k, n)` for which we attempt the GPU.
///
/// Below this the host marshalling (bit-repack into TMA tiles + copies) costs more than it saves.
const DEFAULT_THRESHOLD: usize = 2048;

/// The matmul threshold in use, overridable via the `FP_CUDA_THRESHOLD` environment variable.
fn threshold() -> usize {
    std::env::var("FP_CUDA_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_THRESHOLD)
}

/// Try to compute `a · b` on the GPU.
///
/// This is consulted by `<&Matrix as Mul>::mul` before the CPU BLAS path: for large enough
/// `p = 2` products it converts the operands to the raw row-major limb layout `fp-cuda` expects,
/// runs the kernel, and rebuilds a [`Matrix`]. Anything that makes the GPU path unavailable or
/// unsuitable — no device, a launch error, or a below-threshold size — returns `None`.
///
/// Assumes `a.prime() == b.prime() == 2` and `a.columns() == b.rows()`.
pub(in crate::blas) fn try_mul(a: &Matrix, b: &Matrix) -> Option<Matrix> {
    debug_assert_eq!(a.prime(), TWO);
    debug_assert_eq!(b.prime(), TWO);
    debug_assert_eq!(a.columns(), b.rows());

    let (m, k, n) = (a.rows(), a.columns(), b.columns());
    let t = threshold();
    if m < t || k < t || n < t {
        return None;
    }

    let ctx = context()?;
    let mut a_limbs = Vec::new();
    fill_limbs(a, &mut a_limbs);
    let mut b_limbs = Vec::new();
    fill_limbs(b, &mut b_limbs);

    // Through the driver: this is a persistent whole-device grid, so "concurrent callers do not
    // interfere" was wrong — two at once cannot both be placed (see [`driver`]).
    // `.ok()` inside the closure: the error is a `Box<dyn Error>`, which is not `Send`, so it
    // cannot cross back from the driver thread. The caller only distinguishes success from
    // fall-back-to-CPU anyway.
    let c = driver::run(move || fp_cuda::matmul_b1_raw(ctx, &a_limbs, m, k, &b_limbs, n).ok())?;
    Some(Matrix::from_data(TWO, m, n, c))
}
