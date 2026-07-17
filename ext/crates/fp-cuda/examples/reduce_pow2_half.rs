//! Half-rank scaling comparison (the target workload): device-resident
//! reduction vs the CPU reducers on rank-(n/2) square F₂ matrices of size 2^n.
//! Extends to 2^18; the CPU baselines are skipped once they would take too long
//! (M4RI is ~size-driven and explodes; CPU BLAS3 is ∝ rank and stays feasible
//! longer), leaving the device GPU-only at the top end.
//!
//! Policy (override via env EXP_MIN/EXP_MAX, M4RI_MAX_EXP, BLAS3_MAX_EXP):
//!   device  : always
//!   M4RI    : exp ≤ M4RI_MAX_EXP  (default 16; ~48 min at 2^17, ~10 h at 2^18)
//!   BLAS3   : exp ≤ BLAS3_MAX_EXP (default 17; ~1 h at 2^18)
//!
//! Correctness oracle per size: device RREF == M4RI (when run), else == CPU
//! BLAS3 (when run), else — GPU-only — the known rank n/2 plus idempotence
//! (re-reducing the RREF is a fixed point). Bit-exactness vs the CPU is already
//! validated exhaustively at smaller sizes (`row_reduce_demo`, `cuda_dispatch`).
//!
//! The half-rank matrix is built cheaply in packed form: n/2 random basis rows,
//! then n/2 rows each the XOR of an adjacent basis pair — rank n/2, ~50% dense,
//! O(n·stride) to construct. Sizes are multiples of 64 so limbs are well-formed.
//!
//! Run with `cargo run --release -p fp-cuda --example reduce_pow2_half`.

use std::time::Instant;

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::GpuContext;

mod common;
use common::upload_matrix;
use rand::Rng;

fn env_exp(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Build a rank-(n/2) n×n F₂ matrix in packed form (n a multiple of 64).
fn half_rank(rng: &mut impl Rng, n: usize) -> Matrix {
    let stride = n / 64;
    let h = n / 2;
    let mut data = vec![0u64; n * stride];
    // Top half: random basis rows.
    for w in data[..h * stride].iter_mut() {
        *w = rng.random();
    }
    // Bottom half: row h+i = basis[i] XOR basis[(i+1) mod h].
    for i in 0..h {
        let a = i * stride;
        let b = ((i + 1) % h) * stride;
        let dst = (h + i) * stride;
        for c in 0..stride {
            data[dst + c] = data[a + c] ^ data[b + c];
        }
    }
    Matrix::from_data(TWO, n, n, data)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    let exp_min = env_exp("EXP_MIN", 10);
    let exp_max = env_exp("EXP_MAX", 18);
    let m4ri_max = env_exp("M4RI_MAX_EXP", 16);
    let blas3_max = env_exp("BLAS3_MAX_EXP", 17);

    println!("=== device vs CPU (M4RI, BLAS3) — half-rank square 2^n, n={exp_min}..={exp_max} ===");
    println!(
        "{:>7}  {:>10}  {:>10}  {:>10}   {:>9}  {:>9}  {:>8}  {:>9}  check",
        "n", "device(s)", "m4ri(s)", "blas3(s)", "vs-m4ri", "vs-blas3", "rank", "Tbop/s"
    );

    let mut rng = rand::rng();
    for exp in exp_min..=exp_max {
        let n = 1usize << exp;
        let stride = n / 64;
        let mm = half_rank(&mut rng, n);

        // Device: upload + full reduce + sync.
        let t0 = Instant::now();
        let mut dm = upload_matrix(&gpu, &mm)?;
        let (perm, r, _piv) = gpu.row_reduce_dev(&mut dm)?;
        let dev_secs = t0.elapsed().as_secs_f64();

        // Materialize device RREF (pivot k = device row perm[k]).
        let dev_limbs = gpu.download(&dm)?;
        let perm_host = gpu.download_u32(&perm)?;
        let mut e_limbs = vec![0u64; n * stride];
        for k in 0..r {
            let src = perm_host[k] as usize * stride;
            e_limbs[k * stride..k * stride + stride].copy_from_slice(&dev_limbs[src..src + stride]);
        }
        let dev_rref = Matrix::from_data(TWO, n, n, e_limbs);

        // CPU baselines (skipped past their thresholds).
        let (m4ri_secs, m4ri_ok) = if exp <= m4ri_max {
            let mut c = mm.clone();
            let t = Instant::now();
            let rank = c.row_reduce();
            (
                Some(t.elapsed().as_secs_f64()),
                Some(rank == r && c == dev_rref),
            )
        } else {
            (None, None)
        };
        let (blas3_secs, blas3_ok) = if exp <= blas3_max {
            let mut c = mm.clone();
            let t = Instant::now();
            let rank = c.row_reduce_blas3();
            (
                Some(t.elapsed().as_secs_f64()),
                Some(rank == r && c == dev_rref),
            )
        } else {
            (None, None)
        };

        // Correctness verdict: prefer a CPU oracle; else known rank + idempotence.
        let check = if let Some(ok) = m4ri_ok {
            if ok {
                "vs-m4ri ok".to_string()
            } else {
                "vs-m4ri BAD".to_string()
            }
        } else if let Some(ok) = blas3_ok {
            if ok {
                "vs-blas3 ok".to_string()
            } else {
                "vs-blas3 BAD".to_string()
            }
        } else {
            // GPU-only: rank must equal n/2 (known by construction), and the RREF
            // must be a fixed point of a second device reduction.
            let mut dm2 = upload_matrix(&gpu, &dev_rref)?;
            let (perm2, r2, _) = gpu.row_reduce_dev(&mut dm2)?;
            let l2 = gpu.download(&dm2)?;
            let p2 = gpu.download_u32(&perm2)?;
            let mut e2 = vec![0u64; n * stride];
            for k in 0..r2 {
                let src = p2[k] as usize * stride;
                e2[k * stride..k * stride + stride].copy_from_slice(&l2[src..src + stride]);
            }
            let idem = r2 == r && Matrix::from_data(TWO, n, n, e2) == dev_rref;
            if r == n / 2 && idem {
                "rank=n/2,idem ok".to_string()
            } else {
                format!("GPU-only BAD (r={r} idem={idem})")
            }
        };

        let f = |o: Option<f64>| {
            o.map(|s| format!("{s:.3}"))
                .unwrap_or_else(|| "  skip".into())
        };
        let ratio = |o: Option<f64>| {
            o.map(|s| format!("{:.2}x", s / dev_secs))
                .unwrap_or_else(|| "    —".into())
        };
        // Effective binary-op throughput: leading-order elimination work
        // 2·m·n·R (each bit-MAC = AND + accumulate = 2 ops), the binary-TOPS
        // convention. Tbop/s = 10^12 binary ops per second.
        let bops = 2.0 * n as f64 * n as f64 * r as f64;
        let tbops = bops / dev_secs / 1e12;
        println!(
            "{:>7}  {:>10.3}  {:>10}  {:>10}   {:>9}  {:>9}  {:>8}  {:>9.1}  {}",
            n,
            dev_secs,
            f(m4ri_secs),
            f(blas3_secs),
            ratio(m4ri_secs),
            ratio(blas3_secs),
            r,
            tbops,
            check,
        );
        if check.contains("BAD") {
            std::process::exit(1);
        }
    }
    Ok(())
}
