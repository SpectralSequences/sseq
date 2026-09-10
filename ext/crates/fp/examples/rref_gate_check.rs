//! Correctness of the reductions the size-based gate NEWLY admits.
//!
//! Moving the gate from a short-side floor to `DEFAULT_RR_MIN_BITS` admits a large class of
//! wide-and-short matrices that previously always went to CPU M4RI. The `cuda_dispatch` tests only
//! cover shapes that were already admitted, and `reduce_shapes` checks rank agreement, which a
//! wrong-but-same-rank reduction would pass.
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

/// The dispatch gate under test, mirrored from `fp`'s `DEFAULT_RR_MIN_BITS`.
///
/// The predicate is crate-private, so an example cannot call it. Keep the two in step.
const GATE_BITS: u64 = 1 << 22;

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

/// Reduce each shape both ways and compare rank and full reduced form.
fn main() {
    // Every one of these was rejected by the old short-side floor and is admitted by the size
    // gate. The last two sit below the size gate and must still be rejected — a gate that admits
    // everything is not a gate.
    let shapes: &[(usize, usize, bool)] = &[
        (16, 1_600_000, true),
        (64, 1_600_000, true),
        (587, 1_524_934, true),  // b=(287,5)
        (1131, 611_461, true),   // b=(255,9)
        (1676, 1_686_395, true), // b=(266,7)
        (2877, 1_622_037, true), // b=(253,10)
        (2048, 2048, true),      // square, just above the gate
        (512, 512, false),       // below the gate, stays on CPU
        (1024, 1024, false),     // below the gate, stays on CPU
    ];

    let mut failures = 0;
    for &(rows, cols, expect_gpu) in shapes {
        let bits = rows as u64 * cols as u64;
        let admitted = bits >= GATE_BITS;
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
            "  {rows:>5} x {cols:<9} {:>8.2} MB  gate={:<8} rank {r1:>6}=={r2:<6} {}  form {}  {}",
            bits as f64 / 8.0 / (1 << 20) as f64,
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
