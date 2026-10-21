//! GPU-dispatch correctness for `<&Matrix as Mul>::mul` under the `gpu` feature.
#![cfg(feature = "gpu")]

use fp::{
    matrix::{Matrix, arbitrary::MatrixArbParams},
    prime::TWO,
};
use proptest::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

/// A `rows × cols` matrix over F₂ built from `data`, one `u64` limb per 64 entries of a row.
fn from_limbs(rows: usize, cols: usize, mut data: Vec<u64>) -> Matrix {
    let limbs = cols.div_ceil(64);
    // `Matrix` keeps the bits past the last column zero.
    if !cols.is_multiple_of(64) {
        let mask = (1u64 << (cols % 64)) - 1;
        for row in data.chunks_exact_mut(limbs) {
            row[limbs - 1] &= mask;
        }
    }
    Matrix::from_data(TWO, rows, cols, data)
}

/// An arbitrary `rows × cols` matrix over F₂.
fn arb_matrix(rows: usize, cols: usize) -> BoxedStrategy<Matrix> {
    Matrix::arbitrary_with(MatrixArbParams {
        p: Some(TWO),
        rows: Just(rows).boxed(),
        columns: Just(cols).boxed(),
    })
}

/// A pseudorandom `rows × cols` matrix over F₂, for the concurrency test.
fn random_matrix(rows: usize, cols: usize, seed: u64) -> Matrix {
    let mut rng = StdRng::seed_from_u64(seed);
    let data = (0..rows * cols.div_ceil(64))
        .map(|_| rng.random())
        .collect();
    from_limbs(rows, cols, data)
}

/// Mirrors the private `blas::cuda::gemm::threshold`, which an integration test cannot reach.
fn threshold() -> usize {
    std::env::var("FP_CUDA_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2048)
}

/// Multipliable operands, every dimension at or above the dispatch threshold and reaching past one
/// K tile of it.
fn arb_operands() -> impl Strategy<Value = (Matrix, Matrix)> {
    let t = threshold();
    let min_dim = t.max(32);
    let dim = min_dim..=min_dim + 1100;
    (dim.clone(), dim.clone(), dim).prop_flat_map(|(m, k, n)| (arb_matrix(m, k), arb_matrix(k, n)))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    /// The dispatched product must be bit-identical to the CPU BLAS kernel.
    #[test]
    fn gpu_dispatch_matches_cpu((a, b) in arb_operands()) {
        prop_assert_eq!(&a * &b, a.fast_mul_concurrent(&b));
    }
}

/// Many threads matmul-ing on the GPU at once must each stay bit-identical to the CPU.
///
/// The submissions queue on the driver thread, each with its own buffers; shared device state
/// would corrupt results or fail the launch.
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
                    let seed = (t * ITERS + i) as u64;
                    let a = random_matrix(m, k, 2 * seed);
                    let b = random_matrix(k, n, 2 * seed + 1);
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

/// A well-formed random 0/1 matrix, of rank at most `rank` when `rank > 0`.
fn rank_matrix(rows: usize, cols: usize, rank: usize, seed: u64) -> Matrix {
    if rank == 0 {
        return random_matrix(rows, cols, seed);
    }
    &random_matrix(rows, rank, 2 * seed) * &random_matrix(rank, cols, 2 * seed + 1)
}

/// Records the `path` field of every `fp::rr` `row_reduce` event.
///
/// Without this the GPU tests are vacuous: `row_reduce` falls back to the CPU whenever the gate
/// declines or no device is present, and comparing that against `row_reduce_cpu` then passes while
/// touching no GPU at all. Asserting on the emitted path is what makes the test require the device.
#[derive(Clone, Default)]
struct PathLog(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

impl PathLog {
    /// Whether any reduction reported `path="gpu"`.
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
    (log, tracing::Dispatch::new(sub))
}

/// The dispatched `row_reduce` must be bit-identical to `row_reduce_cpu`: RREF, rank and pivots.
///
/// The shapes clear the production gate as it ships, rather than lowering it through the
/// environment: an override that reached the device would be testing a configuration nobody runs.
/// They are correspondingly slow.
#[test]
fn gpu_row_reduce_matches_cpu() {
    let (paths, dispatch) = watch_paths();
    tracing::dispatcher::with_default(&dispatch, || {
        for (i, &(rows, cols, rank)) in [
            (8192, 8192, 0),
            (8192, 12288, 0),
            (1131, 611_461, 0), // wide, the shape family the resolution produces
            (8192, 8192, 500),  // rank-deficient
        ]
        .iter()
        .enumerate()
        {
            let base = rank_matrix(rows, cols, rank, 1000 + i as u64);
            let mut gpu = base.clone();
            let rank_gpu = gpu.row_reduce();
            let mut cpu = base.clone();
            let rank_cpu = cpu.row_reduce_cpu();
            assert_eq!(rank_gpu, rank_cpu, "rank mismatch at {rows}x{cols}");
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

/// Many threads row-reducing on the GPU at once must each stay bit-identical to the CPU.
#[test]
fn gpu_row_reduce_concurrent() {
    let (paths, dispatch) = watch_paths();
    // Few, large reductions: each has to clear the production gate to reach the device at all.
    const THREADS: usize = 8;
    const ITERS: usize = 2;
    std::thread::scope(|s| {
        for t in 0..THREADS {
            let dispatch = dispatch.clone();
            s.spawn(move || {
                tracing::dispatcher::with_default(&dispatch, || {
                    for i in 0..ITERS {
                        let rows = 8192 + 256 * (t % 4);
                        let cols = 8192 + 256 * (i % 3);
                        let base = random_matrix(rows, cols, (100 * t + i) as u64);
                        let mut gpu = base.clone();
                        let rank_gpu = gpu.row_reduce();
                        let mut cpu = base.clone();
                        let rank_cpu = cpu.row_reduce_cpu();
                        assert_eq!(
                            rank_gpu, rank_cpu,
                            "rank mismatch {rows}x{cols} (t{t} i{i})"
                        );
                        assert_eq!(gpu, cpu, "RREF mismatch {rows}x{cols} (t{t} i{i})");
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
