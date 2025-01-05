//! BLAS-like operations for F_2 matrices.
//!
//! This module provides highly optimized matrix multiplication kernels using a hierarchical tiling
//! approach:
//!
//! # Architecture
//!
//! - **Tiles**: Matrices are divided into tiles, where each tile contains multiple 64 x 64 bit
//!   blocks
//! - **Blocks**: The fundamental unit of computation (64 x 64 bits = 64 rows  x  64 columns of
//!   bits)
//! - **SIMD kernels**: Block-level operations use AVX-512 or scalar fallbacks
//!
//! # Performance Strategy
//!
//! 1. **Loop ordering**: Six different loop orderings (CIR, CRI, ICR, IRC, RCI, RIC) to optimize
//!    cache locality depending on matrix dimensions
//! 2. **Parallelization**: Recursive divide-and-conquer using rayon for large matrices
//! 3. **Vectorization**: AVX-512 intrinsics for significant speedup on supported CPUs
//!
//! # Implementation Notes
//!
//! - Only `prime = 2` is optimized; other primes fall back to naive multiplication
//! - The optimal loop order and tile size depend on matrix dimensions (see benchmarks)
//! - Default configuration uses RIC ordering with 1 x 16 tiles for best average performance

use tile::{orders::*, LoopOrder};

use crate::matrix::Matrix;

pub mod block;
pub mod tile;

impl std::ops::Mul for &Matrix {
    type Output = Matrix;

    fn mul(self, rhs: Self) -> Matrix {
        assert_eq!(self.prime(), rhs.prime());
        assert_eq!(self.columns(), rhs.rows());

        if self.prime() == 2 && self.physical_rows() % 64 == 0 && rhs.physical_rows() % 64 == 0 {
            // Can use optimized BLAS operations (matrix rows are padded to multiple of 64)
            // TODO: Use different block sizes and loop orders based on the size of the matrices
            self.fast_mul_concurrent(rhs)
        } else {
            // Use naive multiplication for:
            // - Matrices over fields other than F_2
            // - Thin matrices (< 32 rows) that aren't padded
            self.naive_mul(rhs)
        }
    }
}

impl Matrix {
    pub fn naive_mul(&self, rhs: &Self) -> Self {
        assert_eq!(self.prime(), rhs.prime());
        assert_eq!(self.columns(), rhs.rows());

        let mut result = Self::new(self.prime(), self.rows(), rhs.columns());
        for i in 0..self.rows() {
            for j in 0..rhs.columns() {
                for k in 0..self.columns() {
                    result
                        .row_mut(i)
                        .add_basis_element(j, self.row(i).entry(k) * rhs.row(k).entry(j));
                }
            }
        }
        result
    }

    pub fn fast_mul_sequential(&self, other: &Self) -> Self {
        // Benchmarking shows that `RIC` is the best loop order in general
        self.fast_mul_sequential_order::<RIC>(other)
    }

    pub fn fast_mul_sequential_order<L: LoopOrder>(&self, other: &Self) -> Self {
        assert_eq!(self.prime(), 2);
        assert_eq!(self.prime(), other.prime());
        assert_eq!(self.columns(), other.rows());

        let mut result = Self::new(self.prime(), self.rows(), other.columns());
        tile::gemm::<L>(
            true,
            self.as_tile(),
            other.as_tile(),
            true,
            result.as_tile_mut(),
        );

        result
    }

    pub fn fast_mul_concurrent(&self, other: &Self) -> Self {
        // Benchmarking shows that, surprisingly enough, `1x16` is the best block size for many
        // large matrices
        self.fast_mul_concurrent_blocksize::<1, 16>(other)
    }

    pub fn fast_mul_concurrent_blocksize<const M: usize, const N: usize>(
        &self,
        other: &Self,
    ) -> Self {
        // Benchmarking shows that `RIC` is the best loop order in general
        self.fast_mul_concurrent_blocksize_order::<M, N, RIC>(other)
    }

    pub fn fast_mul_concurrent_blocksize_order<const M: usize, const N: usize, L: LoopOrder>(
        &self,
        other: &Self,
    ) -> Self {
        assert_eq!(self.prime(), 2);
        assert_eq!(self.prime(), other.prime());
        assert_eq!(self.columns(), other.rows());

        let mut result = Self::new(self.prime(), self.rows(), other.columns());
        tile::gemm_concurrent::<M, N, L>(
            true,
            self.as_tile(),
            other.as_tile(),
            true,
            result.as_tile_mut(),
        );

        result
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::{matrix::arbitrary::MatrixArbParams, prime::TWO};

    // We need at least 32 rows, otherwise the matrices are not padded
    const DIMS: [usize; 11] = [32, 63, 64, 65, 128, 129, 192, 193, 256, 320, 449];

    fn arb_multipliable_matrices(max: Option<usize>) -> impl Strategy<Value = (Matrix, Matrix)> {
        let max_idx = max
            .map(|max| DIMS.iter().position(|&size| size > max))
            .flatten()
            .unwrap_or(DIMS.len());
        let arb_dim = proptest::sample::select(&DIMS[0..max_idx]);
        arb_dim.clone().prop_flat_map(move |size| {
            (
                Matrix::arbitrary_with(MatrixArbParams {
                    p: Some(TWO),
                    rows: arb_dim.clone().boxed(),
                    columns: Just(size).boxed(),
                }),
                Matrix::arbitrary_with(MatrixArbParams {
                    p: Some(TWO),
                    rows: Just(size).boxed(),
                    columns: arb_dim.clone().boxed(),
                }),
            )
        })
    }

    macro_rules! test_fast_mul {
        () => {
            test_fast_mul!(1);
            test_fast_mul!(2);
            test_fast_mul!(4);
        };
        ($m:literal) => {
            test_fast_mul!($m, 1);
            test_fast_mul!($m, 2);
            test_fast_mul!($m, 4);
        };
        ($m:literal, $n:literal) => {
            test_fast_mul!($m, $n, CIR);
            test_fast_mul!($m, $n, CRI);
            test_fast_mul!($m, $n, ICR);
            test_fast_mul!($m, $n, IRC);
            test_fast_mul!($m, $n, RCI);
            test_fast_mul!($m, $n, RIC);
        };
        ($m:literal, $n:literal, $loop_order:ty) => {
            paste::paste! {
                proptest! {
                    #[test]
                    fn [<test_fast_mul_concurrent_ $m _ $n _ $loop_order:lower _ is_mul>]((
                        m, n
                    ) in arb_multipliable_matrices(None)) {
                        let prod1 = m.fast_mul_sequential(&n);
                        let prod2 = m.fast_mul_concurrent_blocksize_order::<$m, $n, $loop_order>(&n);
                        prop_assert_eq!(prod1, prod2);
                    }
                }
            }
        };
    }

    test_fast_mul!();

    proptest! {
        // We limit to small-ish matrices because `naive_mul` is SLOW
        #[test]
        fn test_fast_mul_sequential_is_mul((m, n) in arb_multipliable_matrices(Some(64))) {
            let prod1 = m.naive_mul(&n);
            let prod2 = m.fast_mul_sequential(&n);
            prop_assert_eq!(prod1, prod2);
        }
    }
}
