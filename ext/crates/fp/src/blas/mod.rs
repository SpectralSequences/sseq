//! BLAS-like operations for matrices.
//!
//! We mostly just focus on general matrix multiplication (`gemm`) for now. We use a block-based
//! approach, where matrices are divided into tiles, and each tile is further divided into blocks of
//! 64x64 bits.
//!
//! - `gemm_block_avx512` => Highly optimized microkernel
//! - `gemm_block` => Selects the appropriate implementation based on current architecture
//! - ...
//!
//! This module is laid out as follows:
//! - `block.rs`: Defines the `MatrixBlock` and `MatrixBlockView` types, which represent a block of
//!   a matrix.
//! - `tiling.rs`: Defines the `MatrixTile` and `MatrixTileView` types, which represent a tile of a
//!   matrix.
//! - `naive.rs`: Implements a naive matrix multiplication algorithm.
//! - `scalar.rs`: Implements a scalar matrix multiplication algorithm.
//! - `avx.rs`: Implements an AVX-based matrix multiplication algorithm.
//! - `avx512.rs`: Implements an AVX-512-based matrix multiplication algorithm.
//! - `multiplication.rs`: Implements the main matrix multiplication functions, including both
//!   sequential and concurrent versions.

use tile::{orders::*, LoopOrder};

use crate::matrix::Matrix;

pub mod block;
pub mod tile;

impl std::ops::Mul for &Matrix {
    type Output = Matrix;

    fn mul(self, rhs: Self) -> Matrix {
        assert_eq!(self.prime(), rhs.prime());
        assert_eq!(self.columns(), rhs.rows());

        // TODO: Use different block sizes and loop orders based on the size of the matrices
        self.fast_mul_concurrent(rhs)
    }
}

impl Matrix {
    pub fn naive_mul(&self, rhs: &Self) -> Matrix {
        assert_eq!(self.prime(), rhs.prime());
        assert_eq!(self.columns(), rhs.rows());

        let mut result = Matrix::new(self.prime(), self.rows(), rhs.columns());
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

    pub fn fast_mul_sequential(&self, other: &Self) -> Matrix {
        // Benchmarking shows that `RIC` is the best loop order in general
        self.fast_mul_sequential_order::<RIC>(other)
    }

    pub fn fast_mul_sequential_order<L: LoopOrder>(&self, other: &Self) -> Matrix {
        assert_eq!(self.prime(), crate::prime::TWO);
        assert_eq!(self.prime(), other.prime());
        assert_eq!(self.columns(), other.rows());

        assert!(self.rows().is_multiple_of(64));
        assert!(self.columns().is_multiple_of(64));
        assert!(other.rows().is_multiple_of(64));
        assert!(other.columns().is_multiple_of(64));

        let mut result = Matrix::new(self.prime(), self.rows(), other.columns());
        tile::gemm::<L>(
            true,
            self.as_tile(),
            other.as_tile(),
            true,
            result.as_tile_mut(),
        );

        result
    }

    pub fn fast_mul_concurrent(&self, other: &Self) -> Matrix {
        // Benchmarking shows that, surprisingly enough, `1x16` is the best block size for many
        // large matrices
        self.fast_mul_concurrent_blocksize::<1, 16>(other)
    }

    pub fn fast_mul_concurrent_blocksize<const M: usize, const N: usize>(
        &self,
        other: &Self,
    ) -> Matrix {
        // Benchmarking shows that `RIC` is the best loop order in general
        self.fast_mul_concurrent_blocksize_order::<M, N, RIC>(other)
    }

    pub fn fast_mul_concurrent_blocksize_order<const M: usize, const N: usize, L: LoopOrder>(
        &self,
        other: &Self,
    ) -> Matrix {
        assert_eq!(self.prime(), crate::prime::TWO);
        assert_eq!(self.prime(), other.prime());
        assert_eq!(self.columns(), other.rows());

        assert!(self.rows().is_multiple_of(64));
        assert!(self.columns().is_multiple_of(64));
        assert!(other.rows().is_multiple_of(64));
        assert!(other.columns().is_multiple_of(64));

        let mut result = Matrix::new(self.prime(), self.rows(), other.columns());
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

    fn arb_multipliable_matrices() -> impl Strategy<Value = (Matrix, Matrix)> {
        prop_oneof![Just(64), Just(128), Just(256)].prop_flat_map(|size| {
            (
                Matrix::arbitrary_with(MatrixArbParams {
                    p: Some(TWO),
                    rows: prop_oneof![Just(64), Just(128), Just(256)].boxed(),
                    columns: Just(size).boxed(),
                }),
                Matrix::arbitrary_with(MatrixArbParams {
                    p: Some(TWO),
                    rows: Just(size).boxed(),
                    columns: prop_oneof![Just(64), Just(128), Just(256)].boxed(),
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
                    ) in arb_multipliable_matrices()) {
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
        #[test]
        fn test_fast_mul_sequential_is_mul((m, n) in arb_multipliable_matrices()) {
            let prod1 = m.naive_mul(&n);
            let prod2 = m.fast_mul_sequential(&n);
            prop_assert_eq!(prod1, prod2);
        }
    }
}
