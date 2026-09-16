//! Shared `fp::Matrix` glue for the examples and benches.
//!
//! The `fp-cuda` library itself is fp-agnostic (it takes raw row-major limb
//! slices) so that the `fp` crate can depend on it without a dependency cycle.
//! These thin wrappers restore the `Matrix`-typed convenience the examples want,
//! using `fp` — which `fp-cuda` pulls in only as a dev-dependency.
//!
//! `#![allow(dead_code)]` because each example/bench uses a different subset.
#![allow(dead_code)]

use fp::{matrix::Matrix, prime::TWO};
use fp_cuda::{DeviceMatrix, GpuContext};
use rand::Rng;

/// Random `rows × cols` over F₂, built straight into limbs.
///
/// Going through `Vec<Vec<u32>>` costs one `u32` per BIT, which at the widths the reduce examples
/// use is tens of GB for an operand whose packed form is a few hundred MB.
pub fn random_matrix(rows: usize, cols: usize) -> Matrix {
    let stride = cols.div_ceil(64);
    let mut rng = rand::rng();
    let mut limbs = vec![0u64; rows * stride];
    for l in limbs.iter_mut() {
        *l = rng.random();
    }
    // Bits past `cols` in the final limb of each row must be zero or the matrix is malformed.
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
pub fn half_rank(rows: usize, cols: usize) -> Matrix {
    let rank = (rows / 2).max(1);
    &random_matrix(rows, rank) * &random_matrix(rank, cols)
}

/// Row-major, K-major `u64` limbs, exactly as [`matmul_b1_raw`] expects:
/// `rows × columns.div_ceil(64)` limbs, one bit per entry, no inter-row padding.
///
/// [`matmul_b1_raw`]: fp_cuda::matmul_b1_raw
pub fn to_limbs(m: &Matrix) -> Vec<u64> {
    let stride = m.columns().div_ceil(64);
    let mut bytes = Vec::with_capacity(m.rows() * stride * 8);
    m.to_bytes(&mut bytes).expect("Vec writes never fail");
    let (chunks, _) = bytes.as_chunks::<8>();
    chunks.iter().map(|&c| u64::from_le_bytes(c)).collect()
}

/// `Matrix`-typed wrapper over [`fp_cuda::matmul_b1_raw`].
pub fn matmul_b1(gpu: &GpuContext, a: &Matrix, b: &Matrix) -> anyhow::Result<Matrix> {
    assert_eq!(a.prime(), TWO);
    assert_eq!(b.prime(), TWO);
    assert_eq!(a.columns(), b.rows());
    let (m, k, n) = (a.rows(), a.columns(), b.columns());
    let c = fp_cuda::matmul_b1_raw(gpu, &to_limbs(a), m, k, &to_limbs(b), n)?;
    Ok(Matrix::from_data(TWO, m, n, c))
}

/// `Matrix`-typed wrapper over the device-resident path
/// [`fp_cuda::GpuContext::matmul_b1_dev_roundtrip`] (on-device packing + GEMM).
pub fn matmul_b1_dev(gpu: &GpuContext, a: &Matrix, b: &Matrix) -> anyhow::Result<Matrix> {
    assert_eq!(a.prime(), TWO);
    assert_eq!(b.prime(), TWO);
    assert_eq!(a.columns(), b.rows());
    let (m, k, n) = (a.rows(), a.columns(), b.columns());
    let c = gpu.matmul_b1_dev_roundtrip(&to_limbs(a), m, k, &to_limbs(b), n)?;
    Ok(Matrix::from_data(TWO, m, n, c))
}

/// Upload an `fp::Matrix` to a device-resident [`DeviceMatrix`].
pub fn upload_matrix(gpu: &GpuContext, m: &Matrix) -> anyhow::Result<DeviceMatrix> {
    assert_eq!(m.prime(), TWO);
    gpu.upload(&to_limbs(m), m.rows(), m.columns())
}

/// Download a [`DeviceMatrix`] back into an `fp::Matrix`.
pub fn download_matrix(gpu: &GpuContext, dm: &DeviceMatrix) -> anyhow::Result<Matrix> {
    let limbs = gpu.download(dm)?;
    Ok(Matrix::from_data(TWO, dm.rows, dm.cols, limbs))
}

/// `Matrix`-typed wrapper over [`fp_cuda::matmul_b1_raw_timed`].
pub fn matmul_b1_timed(
    gpu: &GpuContext,
    a: &Matrix,
    b: &Matrix,
    time_iters: usize,
) -> anyhow::Result<(Matrix, f64)> {
    assert_eq!(a.prime(), TWO);
    assert_eq!(b.prime(), TWO);
    assert_eq!(a.columns(), b.rows());
    let (m, k, n) = (a.rows(), a.columns(), b.columns());
    let (c, secs) =
        fp_cuda::matmul_b1_raw_timed(gpu, &to_limbs(a), m, k, &to_limbs(b), n, time_iters)?;
    Ok((Matrix::from_data(TWO, m, n, c), secs))
}
