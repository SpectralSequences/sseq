//! Phase 5 gate (BLAS3-GPU-HANDOFF §5, the decisive one): the full device
//! reduction `row_reduce_dev` (forward pass + back-substitution over one
//! persistent buffer) must equal `fp::Matrix::row_reduce` bit-for-bit — the RREF
//! matrix *and* the pivot list.
//!
//! The device leaves the RREF pivot rows at perm positions [0, r) in ascending
//! column order; we materialize them into the canonical layout (pivot k at row
//! k, zero rows below) and compare to the CPU reducer's output. Also checks
//! agreement with `row_reduce_blas3`, the CPU BLAS3 oracle this port mirrors.
//!
//! Run with `cargo run -p fp-cuda --example row_reduce_demo`.

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::GpuContext;

mod common;
use common::upload_matrix;
use rand::Rng;

fn pivot_columns(m: &Matrix) -> Vec<usize> {
    (0..m.columns()).filter(|&q| m.pivots()[q] >= 0).collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    println!("=== fp-cuda row_reduce_dev (Phase 5, full RREF) demo ===");

    let mut rng = rand::rng();
    let mut any_fail = false;
    // (rows, cols, rank): rank=0 → full-rank random; rank>0 → rank-deficient.
    let cases: &[(usize, usize, usize)] = &[
        (1, 1, 0),
        (64, 64, 0),
        (100, 130, 0),
        (200, 200, 0),
        (130, 130, 40),
        (300, 130, 30),
        (500, 320, 50),
        (1000, 256, 0),
        (1000, 500, 100),
        (2000, 640, 200),
        (64, 300, 0),
        (777, 333, 111),
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

        // CPU oracles.
        let mut reference = mm.clone();
        let ref_rank = reference.row_reduce();
        let ref_pivots = pivot_columns(&reference);
        let mut blas3 = mm.clone();
        blas3.row_reduce_blas3();

        // Device full reduction.
        let mut dm = upload_matrix(&gpu, &mm)?;
        let (perm, r, mut pivcols) = gpu.row_reduce_dev(&mut dm)?;
        let dev_limbs = gpu.download(&dm)?;
        let perm_host = gpu.download_u32(&perm)?;

        // Materialize the canonical RREF: pivot k at row k (perm[k]), zeros below.
        let stride = cols.div_ceil(64);
        let mut e_limbs = vec![0u64; rows * stride];
        for k in 0..r {
            let src = perm_host[k] as usize * stride;
            e_limbs[k * stride..k * stride + stride].copy_from_slice(&dev_limbs[src..src + stride]);
        }
        let e = Matrix::from_data(TWO, rows, cols, e_limbs);

        pivcols.sort_unstable();
        let ok_rank = r == ref_rank;
        let ok_piv = pivcols == ref_pivots;
        let ok_rref = e == reference;
        let ok_blas3 = e == blas3;
        let ok = ok_rank && ok_piv && ok_rref && ok_blas3;
        println!(
            "  rows={rows} cols={cols} rank={ref_rank}: {} (rref {} vs-blas3 {} rank {} piv {})",
            if ok { "OK" } else { "MISMATCH" },
            yn(ok_rref),
            yn(ok_blas3),
            yn(ok_rank),
            yn(ok_piv),
        );
        if !ok {
            any_fail = true;
        }
    }

    if any_fail {
        println!("Some cases mismatched.");
        std::process::exit(1);
    }
    println!("All cases matched (device row_reduce_dev == CPU row_reduce, bit-exact).");
    Ok(())
}

fn yn(b: bool) -> &'static str {
    if b { "ok" } else { "BAD" }
}
