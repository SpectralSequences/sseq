//! GPU-dispatch correctness for `<&Matrix as Mul>::mul` under the `gpu` feature.
//!
//! Run with `FP_CUDA_DEBUG=1` to see the `[fp-cuda]` launch line and confirm the GPU path was
//! taken.
#![cfg(feature = "gpu")]

use fp::{matrix::Matrix, prime::TWO};
use rand::Rng;

fn random_matrix(rows: usize, cols: usize) -> Matrix {
    let mut rng = rand::rng();
    let limbs = cols.div_ceil(64);
    let data: Vec<u64> = (0..rows * limbs).map(|_| rng.random()).collect();
    Matrix::from_data(TWO, rows, cols, data)
}

/// Well-formed 0/1 matrix (bits past the last column masked), of rank ≤ `rank`
/// when `rank > 0`, else full-random.
fn clean_matrix(rows: usize, cols: usize, rank: usize) -> Matrix {
    let mut rng = rand::rng();
    let mut rand_vec = |r: usize, c: usize| -> Matrix {
        let v: Vec<Vec<u32>> = (0..r)
            .map(|_| (0..c).map(|_| rng.random::<bool>() as u32).collect())
            .collect();
        Matrix::from_vec(TWO, &v)
    };
    if rank == 0 {
        rand_vec(rows, cols)
    } else {
        &rand_vec(rows, rank) * &rand_vec(rank, cols)
    }
}

/// Records the `path` field of every `fp::rr` `row_reduce` event.
///
/// Without this the GPU tests are vacuous: `row_reduce` falls back to the CPU whenever the gate
/// declines or no device is present, and comparing that against `row_reduce_cpu` then passes while
/// touching no GPU at all. Asserting on the emitted path is what makes the test require the device.
#[derive(Clone, Default)]
struct PathLog(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

impl PathLog {
    fn saw_gpu(&self) -> bool {
        self.0.lock().unwrap().iter().any(|p| p == "gpu")
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for PathLog {
    fn on_event(&self, ev: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        if ev.metadata().target() != "fp::rr" {
            return;
        }
        struct Grab(Option<String>);
        impl tracing::field::Visit for Grab {
            fn record_debug(&mut self, f: &tracing::field::Field, v: &dyn std::fmt::Debug) {
                if f.name() == "path" {
                    self.0 = Some(format!("{v:?}").trim_matches('"').to_string());
                }
            }
        }
        let mut g = Grab(None);
        ev.record(&mut g);
        if let Some(p) = g.0 {
            self.0.lock().unwrap().push(p);
        }
    }
}

/// A recorder, and the dispatcher that feeds it.
///
/// The dispatcher is returned rather than installed because `set_default` is thread-local: a test
/// that spawns threads has to hand each one the same `Dispatch`, or their events go unrecorded and
/// the assertion fires on a run that did use the device.
fn watch_paths() -> (PathLog, tracing::Dispatch) {
    use tracing_subscriber::layer::SubscriberExt;
    let log = PathLog::default();
    let sub = tracing_subscriber::registry().with(log.clone());
    (log.clone(), tracing::Dispatch::new(sub))
}

/// Mirrors the private `blas::cuda::threshold`, which an integration test cannot reach.
fn threshold() -> usize {
    std::env::var("FP_CUDA_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2048)
}

/// The dispatched product must be bit-identical to the CPU BLAS kernel. Sizes
/// are chosen above the default 2048 threshold so the GPU path is attempted.
#[test]
fn gpu_dispatch_matches_cpu() {
    let t = threshold();
    for &(m, k, n) in &[
        (2048, 2048, 2048),
        (4096, 2048, 3072),
        (3072, 4096, 2048),
        // Non-tile-aligned dims (not multiples of the kernel's 192/128/1024
        // tiles; rows still pad to a multiple of 64 so dispatch fires):
        // exercise edge masks, partial limbs, and raster tails.
        (2049, 2051, 2053),
        (3000, 2112, 4097),
    ] {
        assert!(
            m >= t && k >= t && n >= t,
            "{m}x{k} * {k}x{n} is below the threshold {t}, so the GPU path is not attempted"
        );
        let a = random_matrix(m, k);
        let b = random_matrix(k, n);

        let dispatched = &a * &b;
        let reference = a.fast_mul_concurrent(&b);

        assert_eq!(
            dispatched, reference,
            "GPU/CPU mismatch at {m}x{k} * {k}x{n}"
        );
    }
}

/// Many threads matmul-ing on the GPU at once must each stay bit-identical to the CPU.
///
/// This is the concurrency the per-thread-stream refactor enables. If concurrent matmuls shared
/// device state this would corrupt results or fail the launch; independent per-stream buffers make
/// it pass.
#[test]
fn gpu_matmul_concurrent() {
    const THREADS: usize = 16;
    const ITERS: usize = 6;
    std::thread::scope(|s| {
        for t in 0..THREADS {
            s.spawn(move || {
                for i in 0..ITERS {
                    let m = 2048 + 256 * (t % 6);
                    let k = 2048 + 256 * (i % 5);
                    let n = 2048 + 128 * ((t + i) % 6);
                    let a = random_matrix(m, k);
                    let b = random_matrix(k, n);
                    let dispatched = &a * &b;
                    let reference = a.fast_mul_concurrent(&b);
                    assert_eq!(
                        dispatched, reference,
                        "concurrent matmul mismatch {m}x{k}*{k}x{n} (t{t} i{i})"
                    );
                }
            });
        }
    });
}

/// The dispatched `row_reduce` (GPU, feature on) must be bit-identical to `row_reduce_cpu` — RREF,
/// rank, and pivots.
///
/// The shapes are chosen to clear the production gate as it ships, rather than lowering it through
/// the environment: the gate weighs elimination work, so the small shapes this test used to carry
/// no longer reach the device at all, and an override that did reach it would be testing a
/// configuration nobody runs. They are correspondingly slow.
#[test]
fn gpu_row_reduce_matches_cpu() {
    let (paths, dispatch) = watch_paths();
    tracing::dispatcher::with_default(&dispatch, || {
        for &(rows, cols, rank) in &[
            (8192, 8192, 0),
            (8192, 12288, 0),
            (1131, 611_461, 0), // wide, the shape family the resolution produces
            (8192, 8192, 500),  // rank-deficient
        ] {
            let base = clean_matrix(rows, cols, rank);

            let mut gpu = base.clone();
            let rank_gpu = gpu.row_reduce(); // GPU dispatch (feature on, above threshold)
            let mut cpu = base.clone();
            let rank_cpu = cpu.row_reduce_cpu(); // CPU oracle, never dispatches

            assert_eq!(
                rank_gpu, rank_cpu,
                "rank mismatch at {rows}x{cols} rank={rank}"
            );
            assert_eq!(
                gpu.pivots(),
                cpu.pivots(),
                "pivot mismatch at {rows}x{cols}"
            );
            assert_eq!(gpu, cpu, "RREF mismatch at {rows}x{cols} rank={rank}");
        }
    });
    assert!(
        paths.saw_gpu(),
        "no reduction reported path=\"gpu\": the gate declined every shape, or no device was \
         present. The comparison above would pass either way, so it proves nothing."
    );
}

/// Many threads row-reducing on the GPU AT ONCE must each stay bit-identical to the CPU — the
/// concurrency the per-thread-stream refactor enables. Isolates the GPU RREF path from the cubecl
/// multiply: if concurrent reductions share any device state (a `__device__` global, a fixed
/// scratch), this corrupts or LAUNCH_FAILEDs; if they're truly independent per-stream, it passes.
#[test]
fn gpu_row_reduce_concurrent() {
    let (paths, dispatch) = watch_paths();
    // Fewer, larger reductions than the shape-coverage test: each has to clear the production gate
    // to reach the device at all, which makes it expensive.
    const THREADS: usize = 8;
    const ITERS: usize = 2;
    std::thread::scope(|s| {
        for t in 0..THREADS {
            let dispatch = dispatch.clone();
            s.spawn(move || {
                tracing::dispatcher::with_default(&dispatch, || {
                    for i in 0..ITERS {
                        // Vary shapes per thread/iter so streams don't run identical work in lockstep.
                        let rows = 8192 + 256 * (t % 4);
                        let cols = 8192 + 256 * (i % 3);
                        let base = clean_matrix(rows, cols, 0);
                        let mut gpu = base.clone();
                        let rank_gpu = gpu.row_reduce();
                        let mut cpu = base.clone();
                        let rank_cpu = cpu.row_reduce_cpu();
                        assert_eq!(
                            rank_gpu, rank_cpu,
                            "concurrent rank mismatch {rows}x{cols} (t{t} i{i})"
                        );
                        assert_eq!(
                            gpu, cpu,
                            "concurrent RREF mismatch {rows}x{cols} (t{t} i{i})"
                        );
                    }
                });
            });
        }
    });
    assert!(
        paths.saw_gpu(),
        "no reduction reported path=\"gpu\"; the comparisons above would pass on the CPU alone."
    );
}
