//! GPU dispatch for F₂ linear algebra on Hopper.

use std::sync::OnceLock;

pub(crate) mod gemm;
mod kernel;
mod params;
mod row_reduce;
pub(crate) mod rref;

pub use kernel::{
    DEVICE_LIMB_MULTIPLE, DeviceMatrix, GpuContext, UPLOAD_CHUNK_BYTES, compile_kernel,
    device_stride, product_fallbacks, tma_xor_supported,
};
pub use row_reduce::{RrOptions, bs_slab_rows, compile_row_reduce};

/// The process-wide GPU context, created lazily on first use.
///
/// `None` if no usable device is present (no driver, no Hopper GPU, or no NVRTC to compile the
/// kernel with), or if `FP_CUDA_DISABLE` is set.
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
        // `cudarc` dlopen's the driver on first use and *panics* if there is none, rather than
        // returning an error, so `None` has to be caught out of a panic here. The hook is left
        // installed on purpose: the driver's message says why the GPU is unavailable, and
        // silencing it would also swallow an unrelated thread's panic.
        std::panic::catch_unwind(|| GpuContext::new(device).ok()).unwrap_or(None)
    })
    .as_ref()
}

/// The single thread every GPU submission goes through.
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
    /// `f` may borrow from the caller's stack, so a job reads its operands in place rather than
    /// from a copy.
    pub(super) fn run<'a, T: Send + 'a>(f: impl FnOnce() -> T + Send + 'a) -> T {
        let (tx, rx) = mpsc::channel();
        let job: Box<dyn FnOnce() + Send + 'a> = Box::new(move || {
            // A send failure means the caller gave up; the job still ran, so just drop it.
            let _ = tx.send(f());
        });
        // SAFETY: the job outlives `'a` only in its type. This call does not return until `rx`
        // yields, which happens either when the job has sent its result, or when its sender is
        // dropped: by the job returning or unwinding, or by the driver dropping the job unrun.
        // In every case the job has finished with its borrows by then, so nothing it borrows is
        // touched after `'a` ends.
        let job: Job = unsafe { std::mem::transmute(job) };
        sender()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .send(job)
            .expect("the fp-cuda driver thread died");
        rx.recv().expect("the fp-cuda driver thread dropped a job")
    }
}
