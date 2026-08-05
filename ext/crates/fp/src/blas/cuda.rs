//! GPU dispatch for F₂ matrix multiplication (Hopper `wgmma.b1`).
//!
//! Compiled only under the `gpu` feature. [`try_mul`] is consulted by
//! `<&Matrix as Mul>::mul` before the CPU BLAS path: for large enough `p = 2`
//! products it converts the operands to the raw row-major limb layout
//! `fp-cuda` expects, runs the kernel, and rebuilds a [`Matrix`]. Anything that
//! makes the GPU path unavailable or unsuitable — no device, a launch error, or
//! a below-threshold size — returns `None`, and the caller falls back to the
//! (bit-identical) CPU kernel.
//!
//! Tuning knobs (environment variables, read once):
//! - `FP_CUDA_DISABLE` — set to any value to force the CPU path.
//! - `FP_CUDA_THRESHOLD` — minimum of `m`, `k`, `n` (in bits) below which the
//!   CPU path is used. Defaults to 2048; the GPU only wins once the kernel work
//!   dwarfs the H2D/D2H + TMA-layout marshalling, which dominates small sizes.

use std::sync::OnceLock;

use fp_cuda::GpuContext;

use crate::{matrix::Matrix, prime::TWO};

/// Smallest `min(m, k, n)` for which we attempt the GPU. Below this the host
/// marshalling (bit-repack into TMA tiles + copies) costs more than it saves.
const DEFAULT_THRESHOLD: usize = 2048;

fn threshold() -> usize {
    std::env::var("FP_CUDA_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_THRESHOLD)
}

/// The process-wide GPU context, created lazily on first use. `None` if no
/// usable device is present (no driver, no Hopper GPU, or the kernel PTX is the
/// nvcc-absent build stub). Shared as `&'static` — no lock: `GpuContext` is
/// `Send + Sync` (its cudarc handles are), and every submission goes through a
/// per-thread stream ([`GpuContext::stream`]), so concurrent callers run on
/// independent streams (overlapping transfers + kernels) instead of serializing.
/// Device buffers are per-call and thread-local, so there is no shared state to guard.
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

/// Row-major, K-major `u64` limbs — the exact layout `fp_cuda::matmul_b1_raw`
/// expects (`rows × columns.div_ceil(64)` limbs, no inter-row padding). Uses
/// `Matrix::to_bytes`, which already strips the physical row stride.
fn to_limbs(m: &Matrix) -> Vec<u64> {
    let stride = m.columns().div_ceil(64);
    let mut bytes = Vec::with_capacity(m.rows() * stride * 8);
    m.to_bytes(&mut bytes).expect("Vec writes never fail");
    let (chunks, _) = bytes.as_chunks::<8>();
    chunks.iter().map(|&c| u64::from_le_bytes(c)).collect()
}

/// Try to compute `a · b` on the GPU. Returns `None` (and the caller uses the
/// CPU path) if the GPU is unavailable, the product is below the size
/// threshold, or the launch fails. The result is bit-identical to the CPU path.
///
/// Assumes `a.prime() == b.prime() == 2` and `a.columns() == b.rows()` — the
/// same preconditions the caller has already checked.
pub(super) fn try_mul(a: &Matrix, b: &Matrix) -> Option<Matrix> {
    debug_assert_eq!(a.prime(), TWO);
    debug_assert_eq!(b.prime(), TWO);
    debug_assert_eq!(a.columns(), b.rows());

    let (m, k, n) = (a.rows(), a.columns(), b.columns());
    let t = threshold();
    if m < t || k < t || n < t {
        return None;
    }

    let ctx = context()?;
    let a_limbs = to_limbs(a);
    let b_limbs = to_limbs(b);

    // Lock-free: `matmul_b1_raw` submits on the calling thread's own stream with per-call device
    // buffers, so concurrent callers do not interfere (see [`context`]).
    let c = fp_cuda::matmul_b1_raw(ctx, &a_limbs, m, k, &b_limbs, n).ok()?;
    Some(Matrix::from_data(TWO, m, n, c))
}
