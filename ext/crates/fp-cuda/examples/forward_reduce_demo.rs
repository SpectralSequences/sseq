//! Phase 5a gate (BLAS3-GPU-HANDOFF §5): the device forward pass
//! (`forward_reduce`) must produce a correct row-echelon form over the
//! persistent buffer. Validated against an independent oracle — `row_reduce`:
//!
//!  1. the forward pass applies only elementary row operations, so it preserves
//!     row space: `row_reduce(device_M) == row_reduce(original)`;
//!  2. rank matches `row_reduce`'s rank, and the reported pivot columns match;
//!  3. the non-pivot rows (perm positions `[r, m)`) are zeroed.
//!
//! No hand-written mirror reference — the check is against the ground-truth CPU
//! reducer. (Phase 5b adds back-substitution and validates full RREF equality.)
//!
//! Run with `cargo run -p fp-cuda --example forward_reduce_demo`.

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::GpuContext;

mod common;
use common::{download_matrix, upload_matrix};
use rand::Rng;

fn pivot_columns(m: &Matrix) -> Vec<usize> {
    (0..m.columns()).filter(|&q| m.pivots()[q] >= 0).collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    println!("=== fp-cuda forward_reduce (Phase 5a) demo ===");

    let mut rng = rand::rng();
    let mut any_fail = false;
    // (rows, cols, rank): rank=0 → full-rank random; rank>0 → rank-deficient.
    let cases: &[(usize, usize, usize)] = &[
        (64, 64, 0),
        (100, 130, 0),
        (200, 200, 0),
        (300, 130, 30), // rank-deficient
        (500, 320, 50), // multiple panels, rank-deficient
        (1000, 256, 0),
        (1000, 500, 100), // rank ≈ m/5, several panels
        (2000, 640, 200),
        (64, 300, 0), // wide (many panels), full row rank
    ];
    for &(rows, cols, rank) in cases {
        let mm = if rank == 0 {
            let v: Vec<Vec<u32>> = (0..rows)
                .map(|_| (0..cols).map(|_| rng.random::<bool>() as u32).collect())
                .collect();
            Matrix::from_vec(TWO, &v)
        } else {
            let a: Vec<Vec<u32>> = (0..rows)
                .map(|_| (0..rank).map(|_| rng.random::<bool>() as u32).collect())
                .collect();
            let b: Vec<Vec<u32>> = (0..rank)
                .map(|_| (0..cols).map(|_| rng.random::<bool>() as u32).collect())
                .collect();
            &Matrix::from_vec(TWO, &a) * &Matrix::from_vec(TWO, &b)
        };

        // Reference: row_reduce of the original.
        let mut reference = mm.clone();
        let ref_rank = reference.row_reduce();
        let ref_pivots = pivot_columns(&reference);

        // Device forward pass.
        let mut dm = upload_matrix(&gpu, &mm)?;
        let (perm, r, mut pivcols) = gpu.forward_reduce(&mut dm)?;
        let dev_limbs = gpu.download(&dm)?;
        let perm_host = gpu.download_u32(&perm)?;
        let dev_m = download_matrix(&gpu, &dm)?;

        // (1) row-space preserved: row_reduce(device) == row_reduce(original).
        let mut dev_rr = dev_m.clone();
        let dev_rr_rank = dev_rr.row_reduce();
        let ok_space = dev_rr == reference && dev_rr_rank == ref_rank;
        // (2) rank + pivot columns.
        pivcols.sort_unstable();
        let ok_rank = r == ref_rank;
        let ok_piv = pivcols == ref_pivots;
        // (3) non-pivot rows (perm[r..]) are zero.
        let stride = cols.div_ceil(64);
        let ok_zero = perm_host[r..].iter().all(|&row| {
            let base = row as usize * stride;
            dev_limbs[base..base + stride].iter().all(|&w| w == 0)
        });

        let ok = ok_space && ok_rank && ok_piv && ok_zero;
        println!(
            "  rows={rows} cols={cols} rank={ref_rank}: {} (space {} rank {} piv {} zero {})",
            if ok { "OK" } else { "MISMATCH" },
            yn(ok_space),
            yn(ok_rank),
            yn(ok_piv),
            yn(ok_zero),
        );
        if !ok {
            any_fail = true;
        }
    }

    if any_fail {
        println!("Some cases mismatched.");
        std::process::exit(1);
    }
    println!("All cases matched (device forward pass == CPU row-echelon oracle).");
    Ok(())
}

fn yn(b: bool) -> &'static str {
    if b { "ok" } else { "BAD" }
}
