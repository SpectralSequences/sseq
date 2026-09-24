//! GPU dispatch for F₂ matrix multiplication (Hopper `wgmma.b1`).

use std::sync::OnceLock;

use crate::{matrix::Matrix, prime::TWO};

mod kernel;
mod params;

pub use kernel::{GpuContext, compile_kernel};

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

/// The process-wide GPU context, created lazily on first use.
///
/// `None` if no usable device is present or if `FP_CUDA_DISABLE` is set.
///
/// Shared as `&'static` with no lock: `GpuContext` is `Send + Sync`, every submission goes through
/// a per-thread stream ([`GpuContext::stream`]) so concurrent callers overlap instead of
/// serializing, and device buffers are per-call, so there is no shared state to guard.
fn context() -> Option<&'static GpuContext> {
    static GPU: OnceLock<Option<GpuContext>> = OnceLock::new();
    GPU.get_or_init(|| {
        if std::env::var_os("FP_CUDA_DISABLE").is_some() {
            return None;
        }
        GpuContext::new(0).ok()
    })
    .as_ref()
}

/// Try to compute `a · b` on the GPU.
///
/// This is consulted by `<&Matrix as Mul>::mul` before the CPU BLAS path. Anything that makes the
/// GPU path unavailable or unsuitable — no device, a launch error, or a below-threshold size —
/// returns `None`.
pub(super) fn try_mul(a: &Matrix, b: &Matrix) -> Option<Matrix> {
    debug_assert_eq!(a.prime(), TWO);
    debug_assert_eq!(b.prime(), TWO);
    debug_assert_eq!(a.columns(), b.rows());

    let t = threshold();
    if a.rows() < t || a.columns() < t || b.columns() < t {
        return None;
    }
    a.cuda_mul(context()?, b).ok()
}
