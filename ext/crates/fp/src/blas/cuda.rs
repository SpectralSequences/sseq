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
        // `FP_CUDA_DEVICE` puts the row reduction on its own GPU. On a single device the two CUDA
        // consumers contend, which is why [`crate::gpu_lock`] exists; separate devices remove the
        // contention by construction, so the lock's shared side becomes a no-op (see
        // [`crate::gpu_lock::set_devices_shared`]).
        let device = std::env::var("FP_CUDA_DEVICE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let mult_devices = multiply_devices();
        let shared = device < mult_devices;
        crate::gpu_lock::set_devices_shared(shared);
        // Log it: whether arbitration is live decides whether the reduction's thousands of tiny
        // launches overlap the multiply's saturating ones, and getting it wrong is invisible in a
        // normal run until something fails far away. `[batch-stats] lock=` alone cannot distinguish
        // "arbitration off" from "arbitration on but uncontended".
        eprintln!(
            "[fp-cuda] row reduction on device {device}; multiply spans devices \
             0..{mult_devices}; multiply yields to reductions: {}; reductions serialize against \
             each other: always",
            if shared {
                "yes"
            } else {
                "no (separate devices)"
            }
        );
        GpuContext::new(device).ok()
    })
    .as_ref()
}

/// How many GPUs the cubecl Milnor multiply spreads over — it shards across ALL visible devices, so
/// the row reduction shares a device with it whenever `FP_CUDA_DEVICE < multiply_devices()`.
///
/// This must count every device the multiply can land on, not the one it starts on: concluding
/// "separate devices" while the multiply is in fact sharing the reduction's GPU silently discards
/// the arbitration [`crate::gpu_lock::exclusive`] exists to provide.
///
/// Mirrors `algebra::algebra::milnor_gpu::gpu_count` — `fp` cannot call it (`algebra` depends on
/// `fp`, not the reverse), so the two must be kept in step. Both honour `CUDA_VISIBLE_DEVICES`,
/// since CUDA renumbers the visible subset to `0..n`.
fn multiply_devices() -> usize {
    const MAX_GPUS: usize = 8;
    let physical = std::fs::read_dir("/proc/driver/nvidia/gpus")
        .map(|d| d.filter_map(|e| e.ok()).count())
        .unwrap_or(0)
        .max(1);
    let visible = std::env::var("CUDA_VISIBLE_DEVICES").ok().map(|v| {
        v.split(',')
            .take_while(|e| {
                e.trim()
                    .parse::<usize>()
                    .is_ok_and(|ord| ord < physical.max(MAX_GPUS))
            })
            .count()
    });
    std::env::var("NASSAU_GPU_DEVICES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or_else(|| visible.unwrap_or(physical).max(1))
        .clamp(1, MAX_GPUS)
}

/// The single thread every `fp-cuda` submission goes through, so this process has exactly one
/// owner of the reduction GPU.
///
/// # Why a thread and not a lock
///
/// Both `fp-cuda` entry points launch persistent grids sized to fill the machine (`num_ctas =
/// occupancy x SMs`): the row reduction's trailing GEMM and the standalone [`try_mul`]. These are
/// ordinary grids, so two of them co-scheduled do not fail — they *queue*, and that is the problem.
/// The reduction is a chain of thousands of tiny sequential relaunches, each one a dependency of
/// the next, so every relaunch that waits behind a saturating GEMM adds its full queueing delay to
/// a serial critical path (see [`crate::gpu_lock::exclusive`]). The cooperative reduction path
/// (`FP_CUDA_RR_COOP`) fails harder still, spinning forever at a grid-wide barrier for CTAs that
/// were never scheduled, which is why it is off by default.
///
/// A lock would serialize only the call sites that remember to take it, and *both* entry points
/// have to be covered or the device has two independent machine-filling consumers again. Routing
/// them through one thread makes single ownership structural instead of a discipline.
///
/// # Completion, not submission
///
/// The job runs to completion on this thread, and both jobs end in a device-to-host download, which
/// synchronizes. That is the property that matters: serializing *submission* is not enough, because
/// kernels outlive the call that launched them — the mistake `gpu_lock::shared` still makes on the
/// multiply side (it is taken inside the submit closure and dropped when submission returns, which
/// is why `[batch-stats] lock=` reads 0.0s in every run).
///
/// # Not yet done
///
/// Transfers are serialized along with compute. They need not be: copy engines do not consume SMs,
/// so the next job's H2D upload could overlap the current job's kernels without touching
/// co-residency. That requires splitting each job into upload / compute / download stages on
/// separate streams and pipelining them here; the correctness property above does not depend on it.
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
                        // NO `gpu_lock::exclusive()` here. Serialization among fp-cuda jobs is
                        // already structural — this is the only thread that submits them — so the
                        // guard would be redundant, and taking it deadlocked the run: it waits for
                        // the multiply's readers to drain while worker threads block on `run`
                        // waiting for this loop. Yielding to the multiply on a SHARED device has to
                        // be arranged without a guard held across a blocking job.
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

    // Lock-free, per-thread stream (see [`context`]): the default row-reduce is composable (no
    // cooperative launch) and allocates its device buffers per call, so concurrent rayon workers
    // reduce different matrices on independent streams — overlapping instead of serializing.
    //
    // Composability (no cooperative launch) means this path *can* overlap other GPU work without
    // deadlocking — not that it should. Take the device exclusively for the duration; see
    // [`crate::gpu_lock::exclusive`].
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
