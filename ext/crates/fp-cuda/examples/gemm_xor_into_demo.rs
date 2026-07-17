//! Phase 3 gate (BLAS3-GPU-HANDOFF §5): the fused trailing-update epilogue
//! `gemm_xor_into` must reproduce the CPU blas3 Step B — `M[:, off:] ^= L·U`,
//! in place over a persistent device buffer — bit-for-bit.
//!
//! For each shape we build well-formed random `M` (m×N), `L` (m×k), `U` (k×t)
//! with `off + t == N` and `off` a limb boundary, compute the reference the way
//! `blas3.rs` does (whole-limb XOR of the CPU product `L·U` into M's trailing
//! limbs), run the device epilogue, and compare the downloaded buffer.
//!
//! Run with `cargo run -p fp-cuda --example gemm_xor_into_demo`.

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::GpuContext;

mod common;
use common::{download_matrix, to_limbs, upload_matrix};
use rand::Rng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    println!("=== fp-cuda gemm_xor_into (Step B) demo ===");

    let mut rng = rand::rng();
    let mut make = |rows: usize, cols: usize| {
        let v: Vec<Vec<u32>> = (0..rows)
            .map(|_| (0..cols).map(|_| rng.random::<bool>() as u32).collect())
            .collect();
        Matrix::from_vec(TWO, &v)
    };

    let mut any_fail = false;
    // (m, k, off, t): N = off + t. `off` a multiple of 64; k = inner (pivots);
    // t = trailing width. Includes off=0 (whole-matrix update), tiny k, large m.
    let shapes: &[(usize, usize, usize, usize)] = &[
        (2048, 4, 0, 4),
        (2048, 16, 64, 200),
        (4096, 32, 128, 300),
        (512, 100, 512, 700),
        (4096, 3, 256, 257),
        (100000, 8, 256, 500),
    ];
    for &(m, k, off, t) in shapes {
        let big_n = off + t;
        let stride = big_n.div_ceil(64);
        let gw = t.div_ceil(64);
        let goff = off / 64;

        let mm = make(m, big_n);
        let l = make(m, k);
        let u = make(k, t);

        // CPU reference: whole-limb XOR of the product into M's trailing limbs.
        let g = &l * &u; // m × t
        let m_limbs = to_limbs(&mm);
        let g_limbs = to_limbs(&g);
        let mut reference = m_limbs.clone();
        for j in 0..m {
            for c in 0..gw {
                reference[j * stride + goff + c] ^= g_limbs[j * gw + c];
            }
        }

        // Device epilogue over a persistent buffer.
        let mut dm = upload_matrix(&gpu, &mm)?;
        let dl = upload_matrix(&gpu, &l)?;
        let du = upload_matrix(&gpu, &u)?;
        gpu.gemm_xor_into(&mut dm, &dl, &du, off)?;
        let out = to_limbs(&download_matrix(&gpu, &dm)?);

        let ok = out == reference;
        println!(
            "  m={m} k={k} off={off} t={t} (N={big_n}): {}",
            if ok { "OK" } else { "MISMATCH" }
        );
        if !ok {
            any_fail = true;
        }
    }

    if any_fail {
        println!("Some shapes mismatched.");
        std::process::exit(1);
    }
    println!("All shapes matched (device trailing-update == CPU Step B).");
    Ok(())
}
