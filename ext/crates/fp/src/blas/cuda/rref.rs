//! The F₂ row reduction to reduced row echelon form, dispatched to the device.

use super::{context, driver, fill_limbs};
use crate::{matrix::Matrix, prime::TWO};

/// Smallest `min(rows, cols)` for which we attempt the GPU row reduction. Higher
/// than the matmul threshold: a full reduction is many dependent panel steps, not
/// one GEMM, so its CPU crossover is later. Re-validated on an H200 post-
/// optimization (half-rank square, device incl. upload/reduce vs M4RI
/// `row_reduce`): GPU is 0.57× at n=4096 (a loss) and 1.57× at n=8192 (a win),
/// so the crossover sits just below 8192. The small-n crossover is bound by fixed
/// launch/transfer overhead, not the trailing GEMM, so the recent throughput wins
/// (which scale with n²) did not move it. Measured against single-thread M4RI;
/// the concurrent CPU path is faster, which only pushes the crossover up — so
/// 8192 is the safe floor. Override with `FP_CUDA_RR_THRESHOLD`.
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

    // Lock-free, per-thread stream (see [`context`]): the default row-reduce is composable (no
    // cooperative launch) and allocates its device buffers per call, so concurrent rayon workers
    // reduce different matrices on independent streams — overlapping instead of serializing.
    //
    // The claim that this "needs no cross-runtime exclusion against the cubecl multiply" is exactly
    // backwards. Composability (no cooperative launch) means this path *can* overlap other GPU work
    // without deadlocking — not that it should. This reduction is a chain of thousands of tiny
    // sequential per-column relaunches, so overlapping it with the multiply's saturating kernels
    // makes every launch queue: 1.8–9.7 ms standalone becomes 8.6–96.8 s co-running. Take the
    // device exclusively for the duration; see [`fp::gpu_lock`] for the measurements and the cost
    // (~5 s of multiply pause across a whole stem-200 resolution).
    // The exclusive guard now lives on the driver thread, which holds it for the whole job — see
    // [`driver`]. Taking it here as well would deadlock: the driver would wait on a guard this
    // thread holds while this thread waits on the driver.
    let (dev_limbs, perm, r, pivot_cols) = driver::run(move || {
        let mut dm = ctx.upload(&limbs, rows, cols).ok()?;
        let (perm, r, pivot_cols) = ctx.row_reduce_dev(&mut dm).ok()?;
        let dev_limbs = ctx.download(&dm).ok()?;
        let perm = ctx.download_u32(&perm).ok()?;
        Some((dev_limbs, perm, r, pivot_cols))
    })?;

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

