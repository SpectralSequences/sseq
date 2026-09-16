//! GPU dispatch for the F₂ linear algebra `fp-cuda` provides.
//!
//! [`gemm`] is the one entry point today. What lives here is what any additional subprogram would
//! also need: one device context, one driver thread, and the host-side limb marshalling. Anything
//! specific to a subprogram, its dispatch threshold included, belongs in that subprogram's module.

pub(crate) mod gemm;

use std::sync::OnceLock;

use fp_cuda::GpuContext;

use crate::matrix::Matrix;

/// The process-wide GPU context, created lazily on first use.
///
/// `None` if no usable device is present (no driver, no Hopper GPU, or the kernel PTX is the
/// nvcc-absent build stub), or if `FP_CUDA_DISABLE` is set.
///
/// Shared as `&'static` with no lock: `GpuContext` is `Send + Sync`, each thread gets its own
/// stream ([`GpuContext::stream`]) rather than sharing one, and device buffers are per-call, so
/// there is no shared state to guard.
///
/// That is about there being nothing to protect, not about overlap. Submissions do not overlap:
/// every one of them goes through [`driver`], which runs them one at a time.
fn context() -> Option<&'static GpuContext> {
    static GPU: OnceLock<Option<GpuContext>> = OnceLock::new();
    GPU.get_or_init(|| {
        if std::env::var_os("FP_CUDA_DISABLE").is_some() {
            return None;
        }
        // `FP_CUDA_DEVICE` selects which GPU this process submits to.
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
/// This is what gives the process exactly one owner of the device.
///
/// Every submission launches a grid sized to fill the machine, so concurrent callers do not fail,
/// they *queue* — and a caller that issues many dependent launches has that queueing land on its
/// serial critical path. A lock would cover only the call sites that remember to take it; routing
/// through one thread makes single ownership structural, and keeps it that way as entry points are
/// added.
///
/// Jobs run here to *completion*, not just submission: kernels outlive the call that launched them,
/// and a job ends in a synchronizing device-to-host download.
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

/// Pack `m`'s rows into `buf` in the tight row-major layout every `fp-cuda` entry point expects.
///
/// Replaces a `u64` → little-endian bytes → `u64` round trip, which was two full-size copies to
/// perform the identity on a little-endian machine. When the matrix's own stride already matches
/// the packed one — the common case, since `Matrix::new` sets `columns_capacity == columns` — this
/// is a single `extend_from_slice` of the whole buffer.
fn fill_limbs(m: &Matrix, buf: &mut Vec<u64>) {
    let packed = m.columns().div_ceil(64);
    let ms = m.stride();
    let rows = m.rows();
    buf.clear();
    buf.reserve(rows * packed);
    let data = m.data();
    if ms == packed {
        buf.extend_from_slice(&data[..rows * packed]);
    } else {
        for i in 0..rows {
            buf.extend_from_slice(&data[i * ms..i * ms + packed]);
        }
    }
}
