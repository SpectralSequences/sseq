//! The F₂ matrix product, on Hopper `wgmma.b1`.

use super::{context, driver};
use crate::{matrix::Matrix, prime::TWO};

/// Smallest `min(m, k, n)` for which we attempt the GPU.
///
/// Below this the host marshalling (bit-repack into TMA tiles + copies) costs more than it saves.
const DEFAULT_THRESHOLD: usize = 2048;

/// The threshold in use, overridable via the `FP_CUDA_THRESHOLD` environment variable.
fn threshold() -> usize {
    std::env::var("FP_CUDA_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_THRESHOLD)
}

/// Try to compute `a · b` on the GPU.
///
/// `None` if there is no device, the product is below the threshold, or the launch fails.
pub(in crate::blas) fn try_mul(a: &Matrix, b: &Matrix) -> Option<Matrix> {
    debug_assert_eq!(a.prime(), TWO);
    debug_assert_eq!(b.prime(), TWO);
    debug_assert_eq!(a.columns(), b.rows());

    let t = threshold();
    if a.rows() < t || a.columns() < t || b.columns() < t {
        return None;
    }
    let gpu = context()?;
    driver::run(|| a.cuda_mul(gpu, b).ok())
}
