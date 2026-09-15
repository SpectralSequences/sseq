//! The F₂ row reduction to reduced row echelon form, dispatched to the device.

use super::{context, driver, fill_limbs};
use crate::{matrix::Matrix, prime::TWO};

/// Smallest problem size, in bits, for which we attempt the GPU row reduction.
///
/// Size rather than a short side, because that is what the crossover tracks: the device needs
/// enough total work, not a fat short side. This is the first size that wins outright. See
/// `crates/fp-cuda/EXPERIMENTS.md` for the shapes it was measured on, and for the short-side
/// floor it replaced. Override with `FP_CUDA_RR_MIN_BITS`.
const DEFAULT_RR_MIN_BITS: u64 = 1 << 22;

/// Legacy minimum on the short side, `FP_CUDA_RR_THRESHOLD`.
///
/// Inert at its default of 0: [`DEFAULT_RR_MIN_BITS`] decides. Kept so that scripts setting it keep
/// working, and so `FP_CUDA_RR_THRESHOLD=8192` restores the old behaviour exactly.
const DEFAULT_RR_THRESHOLD: usize = 0;

/// The legacy short-side floor in use, overridable via `FP_CUDA_RR_THRESHOLD`.
fn rr_threshold() -> usize {
    std::env::var("FP_CUDA_RR_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_RR_THRESHOLD)
}

/// The row-reduction size floor in use, overridable via `FP_CUDA_RR_MIN_BITS`.
fn rr_min_bits() -> u64 {
    std::env::var("FP_CUDA_RR_MIN_BITS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_RR_MIN_BITS)
}

/// Is this reduction worth the device?
///
/// Size decides; the legacy short-side floor applies only when explicitly set.
pub(crate) fn rr_worth_gpu(rows: usize, cols: usize) -> bool {
    let t = rr_threshold();
    if rows < t || cols < t {
        return false;
    }
    (rows as u64).saturating_mul(cols as u64) >= rr_min_bits()
}

/// A small pool of reusable host buffers for marshalling matrices to and from the device.
///
/// The reduction cannot borrow the matrix it is reducing: [`driver::run`] requires `'static`, so
/// the limbs have to be owned and moved into the closure. That copy is made on the *calling*
/// thread, so allocating one per call makes live memory scale with the driver's queue depth rather
/// than with device concurrency — which at frontier sizes dominated the process.
///
/// Buffers are taken here, moved in, and handed back by the closure whether or not the reduction
/// succeeded: dropping one inside would lose a permit permanently.
mod marshal {
    use std::{
        sync::{Condvar, LazyLock, Mutex},
        time::Duration,
    };

    struct Pool {
        free: Vec<Vec<u64>>,
        checked_out: usize,
    }

    static POOL: LazyLock<(Mutex<Pool>, Condvar)> = LazyLock::new(|| {
        (
            Mutex::new(Pool {
                free: Vec::new(),
                checked_out: 0,
            }),
            Condvar::new(),
        )
    });

    /// How many buffers may exist at once, overridable via `FP_CUDA_MARSHAL_BUFFERS`.
    ///
    /// Enough for one buffer being filled while another is in flight. The right bound is really
    /// bytes rather than a count — at frontier sizes a single buffer is many GiB — so raising the
    /// count is not the way to serve more concurrent marshalling.
    fn capacity() -> usize {
        static CAP: LazyLock<usize> = LazyLock::new(|| {
            std::env::var("FP_CUDA_MARSHAL_BUFFERS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2)
        });
        *CAP
    }

    /// How long to wait for a free buffer before allocating one instead.
    ///
    /// Short deliberately, because it is hit constantly rather than rarely:
    /// [`super::try_row_reduce`] acquires twice, so one reduction in flight consumes the whole pool
    /// while marshalling stays concurrent across every worker. Waiting bounds nothing either — the
    /// buffer is allocated on timeout regardless — so a long deadline can only ever lose.
    const ACQUIRE_WAIT: Duration = Duration::from_millis(200);

    /// Take a buffer from the pool, or a fresh one if none is free within [`ACQUIRE_WAIT`].
    pub(super) fn acquire() -> Vec<u64> {
        let (lock, cv) = &*POOL;
        let mut g = lock.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if let Some(b) = g.free.pop() {
                g.checked_out += 1;
                return b;
            }
            if g.checked_out < capacity() {
                g.checked_out += 1;
                return Vec::new();
            }
            let (ng, timeout) = cv
                .wait_timeout(g, ACQUIRE_WAIT)
                .unwrap_or_else(|e| e.into_inner());
            g = ng;
            if timeout.timed_out() {
                // Either the pool is simply busy — the common case — or a permit was lost when a
                // panicking closure failed to return its buffer. Both are handled the same way:
                // allocate, and let the pool self-heal as live buffers come back.
                g.checked_out += 1;
                return Vec::new();
            }
        }
    }

    /// Hand a buffer back, waking one waiter.
    pub(super) fn release(mut b: Vec<u64>) {
        let (lock, cv) = &*POOL;
        let mut g = lock.lock().unwrap_or_else(|e| e.into_inner());
        g.checked_out = g.checked_out.saturating_sub(1);
        if g.free.len() < capacity() {
            b.clear();
            g.free.push(b);
        }
        cv.notify_one();
    }
}

/// Try to row-reduce `m` to RREF on the GPU, in place.
///
/// Returns `Some(rank)`, leaving `m` in the same canonical reduced form `Matrix::row_reduce`
/// produces — pivot rows at the top in column order, zeros below, `pivots` set — and bit-identical
/// to it, which `fp-cuda`'s `row_reduce_demo` validates. Returns `None` if the GPU is unavailable,
/// the matrix is below threshold, or a launch fails; the caller then takes the CPU M4RI path.
pub(crate) fn try_row_reduce(m: &mut Matrix) -> Option<usize> {
    debug_assert_eq!(m.prime(), TWO);
    let (rows, cols) = (m.rows(), m.columns());
    if !rr_worth_gpu(rows, cols) {
        return None;
    }
    let ctx = context()?;

    let stride = cols.div_ceil(64);

    // The default row-reduce is composable (no cooperative launch) and allocates its device
    // buffers per call, so it needs no exclusion of its own; [`driver`] is what keeps this process
    // to a single GPU owner.
    //
    // Both buffers come from [`marshal`] and are handed back by the closure on every path, since
    // dropping one inside would lose a permit permanently.
    let mut in_buf = marshal::acquire();
    fill_limbs(m, &mut in_buf);
    let mut out_buf = marshal::acquire();
    out_buf.clear();
    out_buf.resize(rows * stride, 0);

    let (in_buf, out_buf, res) = driver::run(move || {
        let mut outcome = None;
        if let Ok(mut dm) = ctx.upload(&in_buf, rows, cols)
            && let Ok((perm_dev, r, pivot_cols)) = ctx.row_reduce_dev(&mut dm)
            && ctx.download_into(&dm, &mut out_buf).is_ok()
            && let Ok(perm) = ctx.download_u32(&perm_dev)
        {
            outcome = Some((perm, r, pivot_cols));
        }
        (in_buf, out_buf, outcome)
    });
    marshal::release(in_buf);
    let Some((perm, r, pivot_cols)) = res else {
        marshal::release(out_buf);
        return None;
    };

    // Materialize the canonical RREF in place: pivot k (column pivot_cols[k], ascending) at row k,
    // taken from device row perm[k]; rows [r, rows) zero. Writing into `m`'s existing storage saves
    // a full-size allocation and preserves `m`'s row and column capacity, which `Matrix::from_data`
    // silently discarded — callers such as `extend_image` then `add_row` into it.
    let ms = m.stride();
    {
        let data = m.data_mut();
        data.fill(0);
        for k in 0..r {
            let src = perm[k] as usize * stride;
            data[k * ms..k * ms + stride].copy_from_slice(&out_buf[src..src + stride]);
        }
    }
    marshal::release(out_buf);
    m.initialize_pivots();
    let piv = m.pivots_mut();
    for (k, &q) in pivot_cols.iter().enumerate() {
        piv[q] = k as isize;
    }
    Some(r)
}
