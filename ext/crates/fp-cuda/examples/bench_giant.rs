use std::time::Instant;

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::{GpuContext, matmul_b1};
use rand::Rng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gpu = GpuContext::new(0)?;
    let (major, minor) = gpu.compute_capability()?;
    println!("GPU: sm_{major}{minor} (H100)");
    println!();

    let mut rng = rand::rng();

    // Several GB per matrix: for an NxN binary matrix, storage = N*N/8 bytes
    // N=131072: 131072^2/8 = 2 GB per matrix
    // N=65536:  65536^2/8  = 512 MB per matrix
    // N=32768:  32768^2/8  = 128 MB per matrix
    //
    // For F2 matmul: 2*N^3 bit-operations (N^3 ANDs + N^3 XORs)
    // Binary TOPS = 2*N^3 / time_seconds / 1e12

    // N*N/8 bytes per matrix:
    //   N=32768,  K=32768:  128 MB each
    //   N=65536,  K=65536:  512 MB each (1 GB pair)
    //   N=131072, K=32768:  512 MB each
    //   N=131072, K=65536:  1 GB each (2 GB pair)
    //   N=131072, K=131072: 2 GB each (4 GB pair)
    //   N=262144, K=65536:  2 GB each (4 GB pair)
    //   N=262144, K=131072: 4 GB each (8 GB pair)
    for &(m, k_actual, n) in &[
        (32768usize, 32768, 32768), // 128 MB each, warmup
        (65536, 65536, 65536),      // 512 MB each
        (131072, 65536, 131072),    // 1 GB each
        (131072, 131072, 131072),   // 2 GB each
        (262144, 65536, 262144),    // 2 GB + 2 GB
        (262144, 131072, 262144),   // 4 GB each
    ] {
        let stride_a = (k_actual + 63) / 64;
        let stride_c = (n + 63) / 64;
        let a_elems = m * stride_a;
        let b_elems = k_actual * stride_c;
        let a_bytes = a_elems * 8;
        let b_bytes = b_elems * 8;

        println!("=== {m} x {k_actual} x {n} ===");
        println!("  A: {m}x{k_actual} = {:.1} MB", a_bytes as f64 / 1e6);
        println!("  B: {k_actual}x{n} = {:.1} MB", b_bytes as f64 / 1e6);

        let a_data: Vec<u64> = (0..a_elems).map(|_| rng.random()).collect();
        let b_data: Vec<u64> = (0..b_elems).map(|_| rng.random()).collect();
        let a = Matrix::from_data(TWO, m, k_actual, a_data);
        let b = Matrix::from_data(TWO, k_actual, n, b_data);

        // Warmup
        let _ = matmul_b1(&gpu, &a, &b)?;

        // Timed runs
        let trials = 3;
        let mut best_secs = f64::MAX;
        for _ in 0..trials {
            let t0 = Instant::now();
            let _ = matmul_b1(&gpu, &a, &b)?;
            let elapsed = t0.elapsed().as_secs_f64();
            best_secs = best_secs.min(elapsed);
        }

        // 2*M*N*K bit-ops (AND + XOR per element)
        let bit_ops = 2.0 * (m as f64) * (n as f64) * (k_actual as f64);
        let tops = bit_ops / best_secs / 1e12;
        println!("  Time: {:.3} ms", best_secs * 1e3);
        println!("  Binary TOPS: {:.2}", tops);
        println!();
    }

    Ok(())
}
