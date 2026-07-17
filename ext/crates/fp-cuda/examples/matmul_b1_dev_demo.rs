//! Validation for the **device-resident** GEMM path (`matmul_b1_dev`): on-device
//! packing (interleave A, bit-transpose B) + the wgmma.b1 kernel, no host
//! round-trip of the operands. Checks the product bit-for-bit against both the
//! CPU reference (`fp::blas`) and the host packing path (`matmul_b1_raw`), so the
//! new device packing kernels are pinned to the already-validated oracle.
//!
//! Run with `cargo run -p fp-cuda --example matmul_b1_dev_demo`.

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::GpuContext;

mod common;
use common::{matmul_b1, matmul_b1_dev};
use rand::Rng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    let (major, minor) = gpu.compute_capability()?;
    println!("=== fp-cuda matmul_b1_dev (device-resident) demo ===");
    println!("GPU compute capability: sm_{major}{minor}");

    let mut rng = rand::rng();
    // Build via from_vec with genuine 0/1 entries so bits past the last real
    // column/row are masked to zero — the invariant every real fp::Matrix holds.
    // (from_data with raw random limbs would leave garbage in the unused high
    // bits, which the CPU and GPU multiplies legitimately treat differently.)
    let mut make = |rows: usize, cols: usize| {
        let v: Vec<Vec<u32>> = (0..rows)
            .map(|_| (0..cols).map(|_| rng.random::<bool>() as u32).collect())
            .collect();
        Matrix::from_vec(TWO, &v)
    };

    let mut any_fail = false;
    // A spread of shapes: partial-limb columns (n not a multiple of 64), K below
    // and above the 1024 TILE_K boundary, M straddling the 384-row cluster pad,
    // and the same large shapes the host demo exercises.
    for &(m, k, n) in &[
        (1, 64, 1),
        (7, 3, 5),
        (64, 256, 64),
        (100, 100, 100),
        (128, 200, 130),
        (200, 65, 300),
        (256, 256, 256),
        (383, 1000, 257),
        (512, 512, 512),
        (1000, 100, 1000),
        (1024, 1024, 1024),
        (2048, 512, 2048),
        (4096, 256, 4096),
        // Large-M, tiny-K/N: the shapes the last row-reduction panels produce
        // (L is m×pr, U is pr×t with pr, t small but m large). These must be
        // exact for the port even though the tiny-M shapes above are not.
        (2048, 1, 1),
        (2048, 5, 3),
        (4096, 3, 4097),
        (5000, 2, 7),
        (100000, 4, 4),
    ] {
        let a = make(m, k);
        let b = make(k, n);
        let cpu = &a * &b;
        let host_gpu = matmul_b1(&gpu, &a, &b)?;
        let dev_gpu = matmul_b1_dev(&gpu, &a, &b)?;

        let ok_cpu = cpu == dev_gpu;
        let ok_host = host_gpu == dev_gpu;
        println!(
            "  {m}x{k} * {k}x{n}: vs cpu {} | vs host-gemm {}",
            if ok_cpu { "OK" } else { "MISMATCH" },
            if ok_host { "OK" } else { "MISMATCH" },
        );
        if !ok_cpu || !ok_host {
            any_fail = true;
        }
    }

    if any_fail {
        println!("Some shapes mismatched.");
        std::process::exit(1);
    }
    println!("All shapes matched (device-resident packing is bit-exact).");
    Ok(())
}
