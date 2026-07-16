//! GPU dispatch for F₂ matrix multiplication (Hopper `wgmma.b1`).

use std::sync::OnceLock;

use fp_cuda::GpuContext;

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

/// The process-wide GPU context, created lazily on first use.
///
/// `None` if no usable device is present (no driver, no Hopper GPU, or the kernel PTX is the
/// nvcc-absent build stub), or if `FP_CUDA_DISABLE` is set.
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

/// Row-major, K-major `u64` limbs — the exact layout `fp_cuda::matmul_b1_raw` expects.
///
/// That is `rows × columns.div_ceil(64)` limbs with no inter-row padding. Uses `Matrix::to_bytes`,
/// which already strips the physical row stride.
fn to_limbs(m: &Matrix) -> Vec<u64> {
    let stride = m.columns().div_ceil(64);
    let mut bytes = Vec::with_capacity(m.rows() * stride * 8);
    m.to_bytes(&mut bytes).expect("Vec writes never fail");
    let (chunks, _) = bytes.as_chunks::<8>();
    chunks.iter().map(|&c| u64::from_le_bytes(c)).collect()
}

/// Try to compute `a · b` on the GPU.
///
/// This is consulted by `<&Matrix as Mul>::mul` before the CPU BLAS path: for large enough
/// `p = 2` products it converts the operands to the raw row-major limb layout `fp-cuda` expects,
/// runs the kernel, and rebuilds a [`Matrix`]. Anything that makes the GPU path unavailable or
/// unsuitable — no device, a launch error, or a below-threshold size — returns `None`.
///
/// Assumes `a.prime() == b.prime() == 2` and `a.columns() == b.rows()`.
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

/// Try to row-reduce `m` to RREF on the GPU, in place. Returns `Some(rank)` and
/// leaves `m` in the same canonical reduced form `Matrix::row_reduce` produces
/// (pivot rows at the top in column order, zeros below, `pivots` set); returns
/// `None` — and the caller uses the CPU M4RI path — if the GPU is unavailable,
/// below threshold, or a launch fails. The result is bit-identical to the CPU
/// path (validated in `fp-cuda`'s `row_reduce_demo`).
///
/// Assumes `m.prime() == 2` (the caller has checked).
pub(crate) fn try_row_reduce(m: &mut Matrix) -> Option<usize> {
    debug_assert_eq!(m.prime(), TWO);
    let (rows, cols) = (m.rows(), m.columns());
    let t = threshold();
    if rows < t || cols < t {
        return None;
    }
    let ctx = context()?;

    let stride = cols.div_ceil(64);
    let limbs = to_limbs(m);

    // Lock-free, like `try_mul`: every submission goes through the calling thread's own stream
    // with per-call device buffers, so concurrent callers do not interfere (see [`context`]).
    let mut dm = ctx.upload(&limbs, rows, cols).ok()?;
    let (perm, r, pivot_cols) = ctx.row_reduce_dev(&mut dm).ok()?;
    let dev_limbs = ctx.download(&dm).ok()?;
    let perm = ctx.download_u32(&perm).ok()?;

    // Materialize the canonical RREF: pivot k (column pivot_cols[k], ascending)
    // at row k, taken from device row perm[k]; rows [r, rows) zero.
    let mut out = vec![0u64; rows * stride];
    for k in 0..r {
        let src = perm[k] as usize * stride;
        out[k * stride..k * stride + stride].copy_from_slice(&dev_limbs[src..src + stride]);
    }
    *m = Matrix::from_data(TWO, rows, cols, out);
    m.initialize_pivots();
    let piv = m.pivots_mut();
    for (k, &q) in pivot_cols.iter().enumerate() {
        piv[q] = k as isize;
    }
    Some(r)
}
