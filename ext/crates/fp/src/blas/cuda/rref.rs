//! The F₂ row reduction to reduced row echelon form, dispatched to the device.

use super::{context, fill_limbs};
use crate::{matrix::Matrix, prime::TWO};

/// Smallest `min(rows, cols)` for which we attempt the GPU row reduction. Higher
/// than the matmul threshold: a full reduction is many dependent panel steps, not
/// one GEMM, so its CPU crossover is later. Measured on an H200 (random square,
/// device incl. upload/download vs multi-threaded M4RI `row_reduce`): the GPU
/// first wins at n≈8192 (n=4096 still ~1.7× slower), so 8192 is the crossover.
/// Override with `FP_CUDA_RR_THRESHOLD`.
const DEFAULT_RR_THRESHOLD: usize = 8192;

/// The row-reduction threshold in use, overridable via `FP_CUDA_RR_THRESHOLD`.
fn rr_threshold() -> usize {
    std::env::var("FP_CUDA_RR_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_RR_THRESHOLD)
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
    let t = rr_threshold();
    if rows < t || cols < t {
        return None;
    }
    let ctx = context()?;

    let stride = cols.div_ceil(64);
    let mut limbs = Vec::new();
    fill_limbs(m, &mut limbs);

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

