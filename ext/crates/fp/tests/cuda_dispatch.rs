//! GPU-dispatch correctness for `<&Matrix as Mul>::mul` under the `gpu` feature.
#![cfg(feature = "gpu")]

use fp::{
    matrix::{Matrix, arbitrary::MatrixArbParams},
    prime::TWO,
};
use proptest::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

/// A `rows × cols` matrix over F₂ built from `data`, one `u64` limb per 64 entries of a row.
fn from_limbs(rows: usize, cols: usize, mut data: Vec<u64>) -> Matrix {
    let limbs = cols.div_ceil(64);
    // `Matrix` keeps the bits past the last column zero.
    if !cols.is_multiple_of(64) {
        let mask = (1u64 << (cols % 64)) - 1;
        for row in data.chunks_exact_mut(limbs) {
            row[limbs - 1] &= mask;
        }
    }
    Matrix::from_data(TWO, rows, cols, data)
}

/// An arbitrary `rows × cols` matrix over F₂.
fn arb_matrix(rows: usize, cols: usize) -> BoxedStrategy<Matrix> {
    Matrix::arbitrary_with(MatrixArbParams {
        p: Some(TWO),
        rows: Just(rows).boxed(),
        columns: Just(cols).boxed(),
    })
}

/// A pseudorandom `rows × cols` matrix over F₂, for the concurrency test.
fn random_matrix(rows: usize, cols: usize, seed: u64) -> Matrix {
    let mut rng = StdRng::seed_from_u64(seed);
    let data = (0..rows * cols.div_ceil(64))
        .map(|_| rng.random())
        .collect();
    from_limbs(rows, cols, data)
}

/// Mirrors the private `blas::cuda::threshold`, which an integration test cannot reach.
fn threshold() -> usize {
    std::env::var("FP_CUDA_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2048)
}

/// Multipliable operands, every dimension at or above the dispatch threshold and reaching past one
/// K tile of it.
fn arb_operands() -> impl Strategy<Value = (Matrix, Matrix)> {
    let t = threshold();
    let dim = t..=t + 1100;
    (dim.clone(), dim.clone(), dim).prop_flat_map(|(m, k, n)| (arb_matrix(m, k), arb_matrix(k, n)))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    /// The dispatched product must be bit-identical to the CPU BLAS kernel.
    #[test]
    fn gpu_dispatch_matches_cpu((a, b) in arb_operands()) {
        prop_assert_eq!(&a * &b, a.fast_mul_concurrent(&b));
    }
}

/// Many threads matmul-ing on the GPU at once must each stay bit-identical to the CPU.
///
/// Each thread submits on its own stream with its own buffers (see `GpuContext::stream`); shared
/// device state would corrupt results or fail the launch.
#[test]
fn gpu_matmul_concurrent() {
    const THREADS: usize = 16;
    const ITERS: usize = 6;
    std::thread::scope(|s| {
        for t in 0..THREADS {
            s.spawn(move || {
                for i in 0..ITERS {
                    let m = 2048 + 256 * (t % 6);
                    let k = 2048 + 256 * (i % 5);
                    let n = 2048 + 128 * ((t + i) % 6);
                    let seed = (t * ITERS + i) as u64;
                    let a = random_matrix(m, k, 2 * seed);
                    let b = random_matrix(k, n, 2 * seed + 1);
                    let dispatched = &a * &b;
                    let reference = a.fast_mul_concurrent(&b);
                    assert_eq!(
                        dispatched, reference,
                        "concurrent matmul mismatch {m}x{k}*{k}x{n} (t{t} i{i})"
                    );
                }
            });
        }
    });
}
