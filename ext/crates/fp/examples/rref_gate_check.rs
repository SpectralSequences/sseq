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
//! ```sh
//! cargo run --release -p fp --features gpu,concurrent --example rref_gate_check
//! ```

use fp::{matrix::Matrix, prime::TWO};
use rand::Rng;

/// The dispatch gate under test, mirrored from `fp`'s `DEFAULT_RR_MIN_WORK`.
///
/// The predicate is crate-private, so an example cannot call it. Keep the two in step.
const GATE_WORK: u64 = 100_000_000_000;

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

/// The elimination work the gate weighs, `rank² · cols` with the rank estimated as the caller's.
fn work(rows: usize, cols: usize) -> u64 {
    let rank = (rows.min(cols) / 2) as u64;
    rank * rank * cols as u64
}

/// Reduce each shape both ways and compare rank and full reduced form.
fn main() {
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
        let admitted = work(rows, cols) >= GATE_WORK;
        let base = half_rank(rows, cols);

        let mut viadispatch = base.clone();
        let r1 = viadispatch.row_reduce();

        let mut reference = base.clone();
        let r2 = reference.row_reduce_cpu();

        let rank_ok = r1 == r2;
        let form_ok = viadispatch == reference;
        let gate_ok = admitted == expect_gpu;
        let ok = rank_ok && form_ok && gate_ok;
        if !ok {
            failures += 1;
        }

        println!(
            "  {rows:>5} x {cols:<9} work {:>8.1e}  gate={:<8} rank {r1:>6}=={r2:<6} {}  form {}  \
             {}",
            work(rows, cols) as f64,
            if admitted { "ADMIT" } else { "reject" },
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
