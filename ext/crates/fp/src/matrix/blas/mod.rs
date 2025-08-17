use crate::matrix::Matrix;

pub mod avx;
pub mod avx512;
pub mod block;
pub mod multiplication;
pub mod naive;
pub mod scalar;
pub mod tiling;

pub use block::{
    AutoStrategy, Avx512Strategy, AvxStrategy, Block, BlockView, GatherStrategy, Immutable,
    MatrixBlockSlice, MatrixBlockSliceMut, Mutability, Mutable, ScalarStrategy,
};
pub use multiplication::{
    fast_mul_concurrent, fast_mul_sequential, LoopOrder, CIR, CRI, ICR, IRC, RCI, RIC,
};
pub use tiling::{MatrixL2BlockSlice, MatrixL2BlockSliceMut, TiledView};

impl std::ops::Mul for &Matrix {
    type Output = Matrix;

    fn mul(self, rhs: Self) -> Matrix {
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
}

impl Matrix {
    pub fn block_at(&self, row: usize, col: usize) -> BlockView<'_, Immutable> {
        assert!(row.is_multiple_of(64));
        assert!(col.is_multiple_of(64));
        let col_limb_offset = col / 64;
        let start_limb = row * self.stride() + col_limb_offset;
        let stride = self.stride();

        BlockView {
            limbs: unsafe { self.data().as_ptr().add(start_limb) },
            coords: [row, col],
            stride,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn block_mut_at(&mut self, row: usize, col: usize) -> BlockView<'_, Mutable> {
        assert!(row.is_multiple_of(64));
        assert!(col.is_multiple_of(64));
        let col_limb_offset = col / 64;
        let start_limb = row * self.stride() + col_limb_offset;
        let stride = self.stride();

        BlockView {
            limbs: unsafe { self.data_mut().as_mut_ptr().add(start_limb) },
            coords: [row, col],
            stride,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn as_l2_block(&self) -> TiledView<'_, Immutable> {
        assert!(self.rows().is_multiple_of(64));
        assert!(self.columns().is_multiple_of(64));

        TiledView {
            limbs: self.data().as_ptr(),
            dimensions: [self.rows() / 64, self.columns() / 64],
            stride: self.stride(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn as_l2_block_mut(&mut self) -> TiledView<'_, Mutable> {
        assert!(self.rows().is_multiple_of(64));
        assert!(self.columns().is_multiple_of(64));

        TiledView {
            limbs: self.data_mut().as_mut_ptr(),
            dimensions: [self.rows() / 64, self.columns() / 64],
            stride: self.stride(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn fast_mul_sequential_order<L: LoopOrder>(&self, other: &Self) -> Matrix {
        fast_mul_sequential::<L>(self, other)
    }

    pub fn fast_mul_sequential(&self, other: &Self) -> Matrix {
        fast_mul_sequential::<RIC>(self, other)
    }

    pub fn fast_mul_concurrent_blocksize_order<const M: usize, const N: usize, L: LoopOrder>(
        &self,
        other: &Self,
    ) -> Matrix {
        fast_mul_concurrent::<M, N, L>(self, other)
    }

    pub fn fast_mul_concurrent_blocksize<const M: usize, const N: usize>(
        &self,
        other: &Self,
    ) -> Matrix {
        fast_mul_concurrent::<M, N, RIC>(self, other)
    }

    pub fn fast_mul_concurrent(&self, other: &Self) -> Matrix {
        self.fast_mul_concurrent_blocksize::<1, 16>(other)
    }
}

pub fn gemm_block(alpha: bool, a: Block, b: Block, beta: bool, c: MatrixBlockSliceMut) {
    // Call the appropriate BLAS implementation based on the target architecture.
    #[cfg(not(target_arch = "x86_64"))]
    {
        // Fallback to scalar implementation if not on x86_64.
        // TODO: Implement NEON gemm for ARM.
        scalar::gemm_block_scalar(alpha, a, b, beta, c);
        return;
    }

    if is_x86_feature_detected!("avx512f") {
        avx512::gemm_block_avx512_unrolled(alpha, a, b, beta, c);
    } else if is_x86_feature_detected!("avx") {
        avx::gemm_block_avx(alpha, a, b, beta, c);
    } else {
        // Fallback to scalar implementation if no SIMD support is detected.
        scalar::gemm_block_scalar(alpha, a, b, beta, c);
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

    proptest! {
        // #![proptest_config(ProptestConfig {
        //     cases: 10000,
        //     max_shrink_time: 3600_000,
        //     max_shrink_iters: 1_000_000_000,
        //     .. ProptestConfig::default()
        // })]

        #[test]
        fn test_scalar_is_gemm(
            a in Matrix::arbitrary_with(MatrixArbParams {
                p: Some(TWO),
                rows: Just(64).boxed(),
                columns: Just(64).boxed(),
            }),
            b in Matrix::arbitrary_with(MatrixArbParams {
                p: Some(TWO),
                rows: Just(64).boxed(),
                columns: Just(64).boxed(),
            }),
            mut c in Matrix::arbitrary_with(MatrixArbParams {
                p: Some(TWO),
                rows: Just(64).boxed(),
                columns: Just(64).boxed(),
            }),
            alpha: bool,
            beta: bool,
        ) {
            let mut c2 = c.clone();
            naive::gemm_block_naive(
                alpha,
                a.block_at(0, 0).gather_block(),
                b.block_at(0, 0).gather_block(),
                beta,
                c.block_mut_at(0, 0)
            );
            scalar::gemm_block_scalar(
                alpha,
                a.block_at(0, 0).gather_block(),
                b.block_at(0, 0).gather_block(),
                beta,
                c2.block_mut_at(0, 0)
            );
            prop_assert_eq!(c, c2);
        }

        #[test]
        fn test_avx512_unrolled_is_gemm(
            a in Matrix::arbitrary_with(MatrixArbParams {
                p: Some(TWO),
                rows: Just(64).boxed(),
                columns: Just(64).boxed(),
            }),
            b in Matrix::arbitrary_with(MatrixArbParams {
                p: Some(TWO),
                rows: Just(64).boxed(),
                columns: Just(64).boxed(),
            }),
            mut c in Matrix::arbitrary_with(MatrixArbParams {
                p: Some(TWO),
                rows: Just(64).boxed(),
                columns: Just(64).boxed(),
            }),
            alpha: bool,
            beta: bool,
        ) {
            let mut c2 = c.clone();
            scalar::gemm_block_scalar(
                alpha,
                a.block_at(0, 0).gather_block(),
                b.block_at(0, 0).gather_block(),
                beta,
                c.block_mut_at(0, 0)
            );
            avx512::gemm_block_avx512_unrolled(
                alpha,
                a.block_at(0, 0).gather_block(),
                b.block_at(0, 0).gather_block(),
                beta,
                c2.block_mut_at(0, 0)
            );
            prop_assert_eq!(c, c2);
        }

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
            let prod1 = (&m) * (&n);
            let prod2 = m.fast_mul_sequential(&n);
            prop_assert_eq!(prod1, prod2);
        }
    }
}
