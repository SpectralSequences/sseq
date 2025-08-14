use crate::{limb::Limb, matrix::Matrix, prime::TWO};

pub mod avx;
pub mod avx512;
pub mod naive;
pub mod scalar;

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
    pub fn block_at(&self, row: usize, col: usize) -> MatrixBlock<'_> {
        // We make several simplifying assumptions for now. TODO: make this more
        // flexible.
        assert_eq!(row, 0);
        assert_eq!(col, 0);

        MatrixBlock {
            limbs: &self.data(),
            stride: self.stride(),
        }
    }

    pub fn block_mut_at(&mut self, row: usize, col: usize) -> MatrixBlockMut<'_> {
        // We make several simplifying assumptions for now. TODO: make this more
        // flexible.
        assert_eq!(row, 0);
        assert_eq!(col, 0);

        MatrixBlockMut {
            stride: self.stride(),
            limbs: self.data_mut(),
        }
    }

    pub fn fast_mul<const SIMD: bool, const UNROLL: bool>(&self, other: &Self) -> Matrix {
        assert_eq!(self.prime(), other.prime());
        assert_eq!(self.prime(), TWO);
        assert_eq!(self.columns(), other.rows());

        let mut result = Matrix::new(self.prime(), self.rows(), other.columns());

        let a = self.block_at(0, 0);
        let b = other.block_at(0, 0);
        let mut c = result.block_mut_at(0, 0);

        match (SIMD, is_x86_feature_detected!("avx512f"), UNROLL) {
            (true, true, true) => avx512::gemm_block_avx512_unrolled(true, a, b, false, &mut c),
            (true, true, false) => avx512::gemm_block_avx512(true, a, b, false, &mut c),
            (true, false, _) => scalar::gemm_block_scalar(true, a, b, false, &mut c),
            (false, _, _) => scalar::gemm_block_scalar(true, a, b, false, &mut c),
        }
        result
    }
}

pub struct MatrixBlock<'a> {
    limbs: &'a [Limb],
    stride: usize,
}

pub struct MatrixBlockMut<'a> {
    limbs: &'a mut [Limb],
    stride: usize,
}

impl<'a> MatrixBlock<'a> {
    fn get(&self, row: usize) -> Limb {
        self.limbs[row * self.stride]
    }

    fn iter(&self) -> impl Iterator<Item = &Limb> {
        self.limbs.iter().step_by(self.stride)
    }

    fn ptr_at(&self, row: usize) -> *const Limb {
        &raw const self.limbs[row * self.stride]
    }
}

impl<'a> MatrixBlockMut<'a> {
    fn get_mut(&mut self, row: usize) -> &mut Limb {
        &mut self.limbs[row * self.stride]
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Limb> {
        self.limbs.iter_mut().step_by(self.stride)
    }

    fn ptr_at(&mut self, row: usize) -> *const Limb {
        &raw const self.limbs[row * self.stride]
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::{matrix::arbitrary::MatrixArbParams, prime::TWO};

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 10000,
            max_shrink_time: 3600_000,
            max_shrink_iters: 1_000_000_000,
            .. ProptestConfig::default()
        })]

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
            })
        ) {
            let mut c2 = c.clone();
            naive::gemm_block_naive(
                true,
                a.block_at(0, 0),
                b.block_at(0, 0),
                true,
                &mut c.block_mut_at(0, 0)
            );
            scalar::gemm_block_scalar(
                true,
                a.block_at(0, 0),
                b.block_at(0, 0),
                true,
                &mut c2.block_mut_at(0, 0)
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
            })
        ) {
            let mut c2 = c.clone();
            scalar::gemm_block_scalar(
                true,
                a.block_at(0, 0),
                b.block_at(0, 0),
                true,
                &mut c.block_mut_at(0, 0)
            );
            avx512::gemm_block_avx512_unrolled(
                true,
                a.block_at(0, 0),
                b.block_at(0, 0),
                true,
                &mut c2.block_mut_at(0, 0)
            );
            prop_assert_eq!(c, c2);
        }

        #[test]
        fn test_avx512_looped_is_gemm(
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
            })
        ) {
            let mut c2 = c.clone();
            scalar::gemm_block_scalar(
                true,
                a.block_at(0, 0),
                b.block_at(0, 0),
                true,
                &mut c.block_mut_at(0, 0)
            );
            avx512::gemm_block_avx512(
                true,
                a.block_at(0, 0),
                b.block_at(0, 0),
                true,
                &mut c2.block_mut_at(0, 0)
            );
            prop_assert_eq!(c, c2);
        }

        // #[test]
        // fn test_avx_is_gemm(
        //     a in Matrix::arbitrary_with(MatrixArbParams {
        //         p: Some(TWO),
        //         rows: Just(64).boxed(),
        //         columns: Just(64).boxed(),
        //     }),
        //     b in Matrix::arbitrary_with(MatrixArbParams {
        //         p: Some(TWO),
        //         rows: Just(64).boxed(),
        //         columns: Just(64).boxed(),
        //     }),
        //     mut c in Matrix::arbitrary_with(MatrixArbParams {
        //         p: Some(TWO),
        //         rows: Just(64).boxed(),
        //         columns: Just(64).boxed(),
        //     })
        // ) {
        //     let mut c2 = c.clone();
        //     scalar::gemm_block_scalar(
        //         true,
        //         a.block_at(0, 0),
        //         b.block_at(0, 0),
        //         true,
        //         &mut c.block_mut_at(0, 0)
        //     );
        //     avx::gemm_block_avx(
        //         true,
        //         a.block_at(0, 0),
        //         b.block_at(0, 0),
        //         true,
        //         &mut c2.block_mut_at(0, 0)
        //     );
        //     prop_assert_eq!(c, c2);
        // }
    }

    // proptest! {
    //     #[test]
    //     fn test_fast_mul_is_mul(m in Matrix::arbitrary_with(MatrixArbParams {
    //         p: Some(TWO),
    //         rows: Just(64).boxed(),
    //         columns: Just(64).boxed(),
    //     }), n in Matrix::arbitrary_with(MatrixArbParams {
    //         p: Some(TWO),
    //         rows: Just(64).boxed(),
    //         columns: Just(64).boxed(),
    //     })) {
    //         let prod1 = (&m) * (&n);
    //         let prod2 = m.fast_mul::<true, false>(&n);
    //         prop_assert_eq!(prod1, prod2);
    //     }

    //     #[test]
    //     fn test_fast_mul_simd_scalar_agree(m in Matrix::arbitrary_with(MatrixArbParams {
    //         p: Some(TWO),
    //         rows: Just(64).boxed(),
    //         columns: Just(64).boxed(),
    //     }), n in Matrix::arbitrary_with(MatrixArbParams {
    //         p: Some(TWO),
    //         rows: Just(64).boxed(),
    //         columns: Just(64).boxed(),
    //     })) {
    //         let prod1 = m.fast_mul::<true, true>(&n);
    //         let prod2 = m.fast_mul::<false, true>(&n);
    //         prop_assert_eq!(prod1, prod2);
    //     }

    //     #[test]
    //     fn test_fast_muls_agree(m in Matrix::arbitrary_with(MatrixArbParams {
    //         p: Some(TWO),
    //         rows: Just(64).boxed(),
    //         columns: Just(64).boxed(),
    //     }), n in Matrix::arbitrary_with(MatrixArbParams {
    //         p: Some(TWO),
    //         rows: Just(64).boxed(),
    //         columns: Just(64).boxed(),
    //     })) {
    //         let prod1 = m.fast_mul::<true, false>(&n);
    //         let prod2 = m.fast_mul::<true, true>(&n);
    //         prop_assert_eq!(prod1, prod2);
    //     }
    // }
}
