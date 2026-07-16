//! Phase 4 gate (BLAS3-GPU-HANDOFF §4): the device `panel_factor` base kernel
//! (one 64-bit panel, forward-only, virtual perm) must match a CPU reference of
//! the identical panel-limb operation bit-for-bit — the reduced panel limb, the
//! `perm` ordering, the multiplier matrix `L`, and `(pr, pivcols)`.
//!
//! The CPU reference below is a direct transliteration of the CUDA kernel, so
//! this pins the kernel to an executable spec. (Phase 5 wires it, promotion, and
//! gemm_xor_into into the full row reduction and validates against `row_reduce`.)
//!
//! Run with `cargo run -p fp-cuda --example panel_factor_demo`.

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::GpuContext;

mod common;
use common::{download_matrix, to_limbs, upload_matrix};
use rand::Rng;

/// CPU mirror of the `panel_factor` CUDA kernel: forward factorization of the
/// single limb `plimb`, rows addressed through `perm`, starting at pivot row
/// `r`. Mutates `m_buf` (panel limb of below rows), `perm` (pivot swaps) and
/// `l_buf` (multiplier bits, by original row id). Returns `(pr, pivcols)`.
// The explicit `perm[p]` indexing deliberately mirrors the CUDA kernel.
#[allow(clippy::too_many_arguments, clippy::needless_range_loop)]
fn panel_factor_ref(
    m_buf: &mut [u64],
    perm: &mut [u32],
    l_buf: &mut [u64],
    plimb: usize,
    r: usize,
    n: usize,
    m: usize,
    stride: usize,
    l_stride: usize,
) -> (usize, Vec<u32>) {
    let mut pr = 0usize;
    let mut pivcols = Vec::new();
    for j in 0..64 {
        let q = plimb * 64 + j;
        if q >= n {
            break;
        }
        let mut pivpos = None;
        for p in (r + pr)..m {
            let row = perm[p] as usize;
            if (m_buf[row * stride + plimb] >> j) & 1 == 1 {
                pivpos = Some(p);
                break;
            }
        }
        let Some(pp) = pivpos else { continue };
        perm.swap(r + pr, pp);
        pivcols.push(q as u32);
        let pivrow = perm[r + pr] as usize;
        let pivword = m_buf[pivrow * stride + plimb];
        for p in (r + pr + 1)..m {
            let row = perm[p] as usize;
            if (m_buf[row * stride + plimb] >> j) & 1 == 1 {
                l_buf[row * l_stride + pr / 64] |= 1 << (pr % 64);
                m_buf[row * stride + plimb] ^= pivword;
            }
        }
        pr += 1;
    }
    (pr, pivcols)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    println!("=== fp-cuda panel_factor (Phase 4) demo ===");

    let mut rng = rand::rng();
    let mut any_fail = false;
    // (rows, cols, plimb, r, rank): panel is limb `plimb`; factor rows [r, m).
    // rank=0 → full-rank random; rank>0 → rank-deficient (free columns).
    let cases: &[(usize, usize, usize, usize, usize)] = &[
        (64, 64, 0, 0, 0),
        (200, 64, 0, 0, 0),
        (1000, 130, 1, 0, 0), // second panel, partial last limb
        (1000, 130, 1, 5, 0), // mid-sweep start row
        (5000, 256, 2, 0, 0),
        (5000, 256, 3, 40, 0), // last panel, r>0
        (100000, 64, 0, 0, 0), // large m
        (300, 40, 0, 0, 0),    // n < 64 (partial panel)
        (500, 64, 0, 0, 10),   // rank-deficient: ~10 pivots, free columns
        (2000, 128, 1, 0, 20), // rank-deficient, second panel
        (5000, 64, 0, 3, 5),   // rank-deficient, mid-sweep start
    ];
    for &(rows, cols, plimb, r, rank) in cases {
        let stride = cols.div_ceil(64);
        let mm = if rank == 0 {
            make(&mut rng, rows, cols)
        } else {
            make_low_rank(&mut rng, rows, cols, rank)
        };

        // CPU reference.
        let mut m_cpu = to_limbs(&mm);
        let mut perm_cpu: Vec<u32> = (0..rows as u32).collect();
        let mut l_cpu = vec![0u64; rows]; // l_stride = 1 (≤64 pivots)
        let (pr_cpu, piv_cpu) = panel_factor_ref(
            &mut m_cpu,
            &mut perm_cpu,
            &mut l_cpu,
            plimb,
            r,
            cols,
            rows,
            stride,
            1,
        );

        // Device.
        let mut dm = upload_matrix(&gpu, &mm)?;
        let mut perm = gpu.identity_perm(rows)?;
        let mut dl = upload_matrix(&gpu, &Matrix::new(TWO, rows, 64))?; // zeroed, l_stride=1
        let (pr_gpu, piv_gpu) = gpu.panel_factor(&mut dm, &mut perm, &mut dl, plimb, r)?;

        let m_gpu = to_limbs(&download_matrix(&gpu, &dm)?);
        let perm_gpu = gpu.download_u32(&perm)?;
        let l_gpu = gpu.download(&dl)?;

        let ok_pr = pr_cpu == pr_gpu;
        let ok_piv = piv_cpu == piv_gpu;
        let ok_m = m_cpu == m_gpu;
        let ok_perm = perm_cpu == perm_gpu;
        let ok_l = l_cpu == l_gpu;
        let ok = ok_pr && ok_piv && ok_m && ok_perm && ok_l;
        println!(
            "  rows={rows} cols={cols} plimb={plimb} r={r} pr={pr_cpu}: {} (M {} perm {} L {} pr \
             {} piv {})",
            if ok { "OK" } else { "MISMATCH" },
            yn(ok_m),
            yn(ok_perm),
            yn(ok_l),
            yn(ok_pr),
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
    println!("All cases matched (device panel_factor == CPU reference).");
    Ok(())
}

fn yn(b: bool) -> &'static str {
    if b { "ok" } else { "BAD" }
}

fn make(rng: &mut impl Rng, rows: usize, cols: usize) -> Matrix {
    let v: Vec<Vec<u32>> = (0..rows)
        .map(|_| (0..cols).map(|_| rng.random::<bool>() as u32).collect())
        .collect();
    Matrix::from_vec(TWO, &v)
}

/// A rank-deficient matrix (rank ≤ `rank`) so the panel has free columns and the
/// kernel's find-first "no pivot" branch is exercised.
fn make_low_rank(rng: &mut impl Rng, rows: usize, cols: usize, rank: usize) -> Matrix {
    let a = make(rng, rows, rank);
    let b = make(rng, rank, cols);
    &a * &b
}
