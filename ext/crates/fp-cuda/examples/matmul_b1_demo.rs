//! Smoke test for the `fp-cuda` matmul kernel.
//!
//! Multiplies one pair of small F_2 matrices on the GPU and verifies the result against
//! `fp::blas`. Run with `cargo oxide run -p fp-cuda --example matmul_b1_demo`.

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::{GpuContext, matmul_b1};
use rand::Rng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    let (major, minor) = gpu.compute_capability()?;
    println!("=== fp-cuda matmul_b1 demo ===");
    println!("GPU compute capability: sm_{major}{minor}");

    let mut rng = rand::rng();
    let mut make = |rows: usize, cols: usize| {
        let data: Vec<u64> = (0..rows * cols.div_ceil(64))
            .map(|_| rng.random())
            .collect();
        Matrix::from_data(TWO, rows, cols, data)
    };

    for &(m, k, n) in &[
        (64, 256, 64),
        (128, 256, 128),
        (256, 256, 256),
        (512, 512, 512),
        (1024, 1024, 1024),
        (2048, 512, 2048),
        (4096, 256, 4096),
        (8192, 256, 8192),
    ] {
        let a = make(m, k);
        let b = make(k, n);
        let cpu = &a * &b;
        let gpu_out = matmul_b1(&gpu, &a, &b)?;
        let ok = cpu == gpu_out;
        println!(
            "  {m}x{k} * {k}x{n}: {}",
            if ok { "OK" } else { "MISMATCH" }
        );
        if !ok {
            let mut cb = Vec::new();
            cpu.to_bytes(&mut cb).unwrap();
            let mut gb = Vec::new();
            gpu_out.to_bytes(&mut gb).unwrap();
            let cv = u64::from_le_bytes(cb[..8].try_into().unwrap());
            let gv = u64::from_le_bytes(gb[..8].try_into().unwrap());
            println!("    row 0: cpu={cv:016x} gpu={gv:016x}");
            println!("    GPU all zeros: {}", gb.iter().all(|&b| b == 0));
            std::process::exit(1);
        }
    }

    println!("All shapes matched.");
    Ok(())
}
