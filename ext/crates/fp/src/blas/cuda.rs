//! GPU dispatch for F₂ matrix multiplication (Hopper `wgmma.b1`).

use std::sync::OnceLock;

use fp_cuda::GpuContext;

use crate::{matrix::Matrix, prime::TWO};

/// Smallest `min(m, k, n)` for which we attempt the GPU.
///
/// Below this the host marshalling (bit-repack into TMA tiles + copies) costs more than it saves.
const DEFAULT_THRESHOLD: usize = 2048;

/// Smallest `min(rows, cols)` for which we attempt the GPU row reduction.
///
/// Higher than [`DEFAULT_THRESHOLD`]: a full reduction is many dependent panel steps, not one GEMM,
/// so its CPU crossover is later. This is a floor rather than a fitted optimum — see
/// `crates/fp-cuda/EXPERIMENTS.md`. Override with `FP_CUDA_RR_THRESHOLD`.
const DEFAULT_RR_THRESHOLD: usize = 8192;

/// The matmul threshold in use, overridable via the `FP_CUDA_THRESHOLD` environment variable.
fn threshold() -> usize {
    std::env::var("FP_CUDA_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_THRESHOLD)
}

/// The row-reduction threshold in use, overridable via `FP_CUDA_RR_THRESHOLD`.
fn rr_threshold() -> usize {
    std::env::var("FP_CUDA_RR_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_RR_THRESHOLD)
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
        // `FP_CUDA_DEVICE` selects the GPU the row reduction runs on.
        let device = std::env::var("FP_CUDA_DEVICE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        GpuContext::new(device).ok()
    })
    .as_ref()
}

/// The single thread every `fp-cuda` submission goes through.
///
/// This is what gives the process exactly one owner of the reduction GPU.
///
/// Both entry points — the row reduction's trailing GEMM and the standalone [`try_mul`] — launch
/// grids sized to fill the machine. Co-scheduled they do not fail, they *queue*, and the reduction
/// is a chain of thousands of dependent relaunches, so that queueing lands on a serial critical
/// path. A lock would cover only the call sites that remember to take it; one thread makes single
/// ownership structural.
///
/// Jobs run here to *completion*, not just submission: kernels outlive the call that launched them,
/// and both jobs end in a synchronizing device-to-host download.
mod driver {
    use std::sync::{Mutex, OnceLock, mpsc};

    type Job = Box<dyn FnOnce() + Send + 'static>;

    /// The driver thread's job channel, spawning the thread on first use.
    fn sender() -> &'static Mutex<mpsc::Sender<Job>> {
        static TX: OnceLock<Mutex<mpsc::Sender<Job>>> = OnceLock::new();
        TX.get_or_init(|| {
            let (tx, rx) = mpsc::channel::<Job>();
            std::thread::Builder::new()
                .name("fp-cuda-driver".into())
                .spawn(move || {
                    for job in rx {
                        job();
                    }
                })
                .expect("failed to spawn the fp-cuda driver thread");
            Mutex::new(tx)
        })
    }

    /// Run `f` on the driver thread and block for its result.
    ///
    /// `f` owns everything it touches (both call sites have already marshalled to owned limb
    /// buffers), so nothing borrows across threads.
    pub(super) fn run<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
        let (tx, rx) = mpsc::channel();
        sender()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .send(Box::new(move || {
                // A send failure means the caller gave up; the job still ran, so just drop it.
                let _ = tx.send(f());
            }))
            .expect("the fp-cuda driver thread died");
        rx.recv().expect("the fp-cuda driver thread dropped a job")
    }
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

    // Through the driver: this is a persistent whole-device grid, so "concurrent callers do not
    // interfere" was wrong — two at once cannot both be placed (see [`driver`]).
    // `.ok()` inside the closure: the error is a `Box<dyn Error>`, which is not `Send`, so it
    // cannot cross back from the driver thread. The caller only distinguishes success from
    // fall-back-to-CPU anyway.
    let c = driver::run(move || fp_cuda::matmul_b1_raw(ctx, &a_limbs, m, k, &b_limbs, n).ok())?;
    Some(Matrix::from_data(TWO, m, n, c))
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
    let t = rr_threshold();
    if rows < t || cols < t {
        return None;
    }
    let ctx = context()?;

    let stride = cols.div_ceil(64);
    let limbs = to_limbs(m);

    // The default row-reduce is composable (no cooperative launch) and allocates its device
    // buffers per call, so it needs no exclusion of its own; [`driver`] is what keeps this process
    // to a single GPU owner.
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
