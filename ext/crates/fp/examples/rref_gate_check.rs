//! Correctness of the reductions the gate admits, and of its refusals.
//!
//! The gate decides on elimination work, so which matrices it admits differs in kind from what a
//! short-side floor admitted. The `cuda_dispatch` tests cover a narrow set of shapes, and
//! `reduce_shapes` checks rank agreement, which a wrong-but-same-rank reduction would pass.
//!
//! So compare the FULL reduced form, through the real dispatch: `row_reduce` consults the gate and
//! goes to the device, `row_reduce_cpu` is the CPU reference in the same process. (Toggling
//! `FP_CUDA_DISABLE` cannot work here — the context is a process-wide `OnceLock`.)
//!
//! Which path each shape took is read from the `fp::rr` `row_reduce` event rather than from a copy
//! of the threshold. A mirrored constant tests only that two numbers match, and goes stale the
//! moment the gate is retuned; the event is what the dispatch actually did, so it also catches a
//! shape the gate admits but the device then declines.
//!
//! ```sh
//! cargo run --release -p fp --features gpu,concurrent --example rref_gate_check
//! ```

use std::sync::{Arc, Mutex};

use fp::{matrix::Matrix, prime::TWO};
use rand::Rng;
use tracing_subscriber::layer::SubscriberExt;

/// Records the `path` field of every `fp::rr` `row_reduce` event.
///
/// The same observation the `cuda_dispatch` tests make; this one is resettable, because the
/// verdict here is per shape rather than per run.
#[derive(Clone, Default)]
struct PathLog(Arc<Mutex<Vec<String>>>);

impl PathLog {
    /// Take everything recorded since the last call.
    fn drain(&self) -> Vec<String> {
        std::mem::take(&mut *self.0.lock().unwrap())
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
        if let Some(path) = g.0 {
            self.0.lock().unwrap().push(path);
        }
    }
}

/// Random `rows × cols` built straight into limbs; going through `Vec<Vec<u32>>` costs a u32 per
/// bit and would need tens of GB at these widths.
fn random_matrix(rows: usize, cols: usize) -> Matrix {
    let stride = cols.div_ceil(64);
    let mut rng = rand::rng();
    let mut limbs = vec![0u64; rows * stride];
    for l in limbs.iter_mut() {
        *l = rng.random();
    }
    let tail = cols % 64;
    if tail != 0 {
        let mask = (1u64 << tail) - 1;
        for r in 0..rows {
            limbs[r * stride + stride - 1] &= mask;
        }
    }
    Matrix::from_data(TWO, rows, cols, limbs)
}

/// Half-rank `rows × cols`, so the reduction has a non-trivial kernel to find.
fn half_rank(rows: usize, cols: usize) -> Matrix {
    let rank = (rows / 2).max(1);
    &random_matrix(rows, rank) * &random_matrix(rank, cols)
}

/// The elimination work the gate weighs, `rank² · cols` with the rank estimated as the gate's.
///
/// Printed for context only — the verdict comes from the dispatch event, not from this.
fn work(rows: usize, cols: usize) -> u64 {
    let rank = (rows.min(cols) / 2) as u64;
    rank * rank * cols as u64
}

/// Reduce each shape both ways and compare rank and full reduced form.
fn main() {
    let paths = PathLog::default();
    let subscriber = tracing_subscriber::registry().with(paths.clone());
    let _guard = tracing::subscriber::set_default(subscriber);

    // Shapes on both sides of the gate, from both families: a gate that admits everything is not a
    // gate, and one tested only on wide inputs would not catch a bad square.
    let shapes: &[(usize, usize, bool)] = &[
        (587, 1_524_934, true),  // b=(287,5)
        (1131, 611_461, true),   // b=(255,9)
        (1676, 1_686_395, true), // b=(266,7)
        (2877, 1_622_037, true), // b=(253,10)
        (8192, 8192, true),      // square, just above the gate
        (6000, 6000, false),     // square, just below it
        (64, 1_600_000, false),  // wide, but far too little work
        (1024, 1024, false),     // small, stays on CPU
    ];

    let mut failures = 0;
    for &(rows, cols, expect_gpu) in shapes {
        let base = half_rank(rows, cols);

        paths.drain();
        let mut viadispatch = base.clone();
        let r1 = viadispatch.row_reduce();
        // One reduction, so one event; anything else means the dispatch changed shape.
        let took_gpu = paths.drain() == ["gpu"];

        let mut reference = base.clone();
        let r2 = reference.row_reduce_cpu();

        let rank_ok = r1 == r2;
        let form_ok = viadispatch == reference;
        let gate_ok = took_gpu == expect_gpu;
        let ok = rank_ok && form_ok && gate_ok;
        if !ok {
            failures += 1;
        }

        println!(
            "  {rows:>5} x {cols:<9} work {:>8.1e}  path={:<6} rank {r1:>6}=={r2:<6} {}  form {}  \
             {}",
            work(rows, cols) as f64,
            if took_gpu { "gpu" } else { "cpu" },
            if rank_ok { "ok" } else { "MISMATCH" },
            if form_ok { "ok" } else { "MISMATCH" },
            if ok { "" } else { "  <<< FAILED" }
        );
    }

    if failures > 0 {
        eprintln!("\n{failures} shape(s) FAILED");
        std::process::exit(1);
    }
    println!("\nall shapes agree with the CPU reference, and the gate admits/rejects as intended");
}
