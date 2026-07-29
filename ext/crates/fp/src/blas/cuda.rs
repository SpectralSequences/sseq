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

/// Smallest `min(m, k, n)` for which we attempt the GPU matmul. Below this the
/// host marshalling (bit-repack into TMA tiles + copies) costs more than it saves.
const DEFAULT_THRESHOLD: usize = 2048;

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

fn threshold() -> usize {
    std::env::var("FP_CUDA_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_THRESHOLD)
}

fn rr_threshold() -> usize {
    std::env::var("FP_CUDA_RR_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_RR_THRESHOLD)
}

/// The process-wide GPU context, created lazily on first use. `None` if no
/// usable device is present (no driver, no Hopper GPU, or the kernel PTX is the
/// nvcc-absent build stub). Shared as `&'static` — no lock: `GpuContext` is
/// `Send + Sync` (its cudarc handles are), and every submission goes through a
/// per-thread stream ([`GpuContext::stream`]), so concurrent workers run on
/// independent streams (overlapping transfers + kernels) instead of serializing.
/// Buffers are per-call and thread-local, so there is no shared device state to guard.
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
    let limbs = to_limbs(m);

    let (dev_limbs, perm, r, pivot_cols) = {
        // Lock-free, per-thread stream (see [`context`]): the row-reduce is composable (no
        // cooperative launch) and allocates device buffers per call, so concurrent workers reduce
        // different matrices on independent streams instead of serializing.
        let mut dm = ctx.upload(&limbs, rows, cols).ok()?;
        let (perm, r, pivot_cols) = ctx.row_reduce_dev(&mut dm).ok()?;
        let dev_limbs = ctx.download(&dm).ok()?;
        let perm = ctx.download_u32(&perm).ok()?;
        (dev_limbs, perm, r, pivot_cols)
    };

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
