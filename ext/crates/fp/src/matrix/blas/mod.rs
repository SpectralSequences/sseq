use maybe_rayon::prelude::*;

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
    pub fn block_at(&self, row: usize, col: usize) -> MatrixBlockSlice<'_> {
        assert!(row.is_multiple_of(64));
        assert!(col.is_multiple_of(64));
        let col_limb_offset = col / 64;
        let start_limb = row * self.stride() + col_limb_offset;
        let stride = self.stride();

        MatrixBlockSlice {
            limbs: unsafe { self.data().as_ptr().add(start_limb) },
            coords: [row, col],
            stride,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn block_mut_at(&mut self, row: usize, col: usize) -> MatrixBlockSliceMut<'_> {
        assert!(row.is_multiple_of(64));
        assert!(col.is_multiple_of(64));
        let col_limb_offset = col / 64;
        let start_limb = row * self.stride() + col_limb_offset;
        let stride = self.stride();

        MatrixBlockSliceMut {
            limbs: unsafe { self.data_mut().as_mut_ptr().add(start_limb) },
            coords: [row, col],
            stride,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn as_l2_block(&self) -> MatrixL2BlockSlice<'_> {
        assert!(self.rows().is_multiple_of(64));
        assert!(self.columns().is_multiple_of(64));

        MatrixL2BlockSlice {
            limbs: self.data().as_ptr(),
            dimensions: [self.rows() / 64, self.columns() / 64],
            stride: self.stride(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn as_l2_block_mut(&mut self) -> MatrixL2BlockSliceMut<'_> {
        assert!(self.rows().is_multiple_of(64));
        assert!(self.columns().is_multiple_of(64));

        MatrixL2BlockSliceMut {
            limbs: self.data_mut().as_mut_ptr(),
            dimensions: [self.rows() / 64, self.columns() / 64],
            stride: self.stride(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn fast_mul_sequential_rci(&self, other: &Self) -> Matrix {
        let mut result = Matrix::new(self.prime(), self.rows(), other.columns());
        for i in (0..self.rows()).step_by(64) {
            for j in (0..other.columns()).step_by(64) {
                let mut c_block = result.block_mut_at(i, j);
                for k in (0..other.rows()).step_by(64) {
                    let a_block = self.block_at(i, k).gather_block();
                    let b_block = other.block_at(k, j).gather_block();
                    gemm_block(true, a_block, b_block, true, &mut c_block);
                }
            }
        }
        result
    }

    pub fn fast_mul_sequential_cri(&self, other: &Self) -> Matrix {
        let mut result = Matrix::new(self.prime(), self.rows(), other.columns());
        for j in (0..other.columns()).step_by(64) {
            for i in (0..self.rows()).step_by(64) {
                let mut c_block = result.block_mut_at(i, j);
                for k in (0..other.rows()).step_by(64) {
                    let a_block = self.block_at(i, k).gather_block();
                    let b_block = other.block_at(k, j).gather_block();
                    gemm_block(true, a_block, b_block, true, &mut c_block);
                }
            }
        }
        result
    }

    pub fn fast_mul_sequential_icr(&self, other: &Self) -> Matrix {
        let mut result = Matrix::new(self.prime(), self.rows(), other.columns());
        for k in (0..other.rows()).step_by(64) {
            for j in (0..other.columns()).step_by(64) {
                let b_block = other.block_at(k, j).gather_block();
                for i in (0..self.rows()).step_by(64) {
                    let a_block = self.block_at(i, k).gather_block();
                    let mut c_block = result.block_mut_at(i, j);
                    gemm_block(true, a_block, b_block, true, &mut c_block);
                }
            }
        }
        result
    }

    pub fn fast_mul_sequential_ric(&self, other: &Self) -> Matrix {
        let mut result = Matrix::new(self.prime(), self.rows(), other.columns());
        for i in (0..self.rows()).step_by(64) {
            for k in (0..other.rows()).step_by(64) {
                let a_block = self.block_at(i, k).gather_block();
                for j in (0..other.columns()).step_by(64) {
                    let b_block = other.block_at(k, j).gather_block();
                    let mut c_block = result.block_mut_at(i, j);
                    gemm_block(true, a_block, b_block, true, &mut c_block);
                }
            }
        }
        result
    }

    pub fn fast_mul_sequential_irc(&self, other: &Self) -> Matrix {
        let mut result = Matrix::new(self.prime(), self.rows(), other.columns());
        for k in (0..other.rows()).step_by(64) {
            for i in (0..self.rows()).step_by(64) {
                let a_block = self.block_at(i, k).gather_block();
                for j in (0..other.columns()).step_by(64) {
                    let b_block = other.block_at(k, j).gather_block();
                    let mut c_block = result.block_mut_at(i, j);
                    gemm_block(true, a_block, b_block, true, &mut c_block);
                }
            }
        }
        result
    }

    pub fn fast_mul_sequential_cir(&self, other: &Self) -> Matrix {
        let mut result = Matrix::new(self.prime(), self.rows(), other.columns());
        for j in (0..other.columns()).step_by(64) {
            for k in (0..other.rows()).step_by(64) {
                let b_block = other.block_at(k, j).gather_block();
                for i in (0..self.rows()).step_by(64) {
                    let a_block = self.block_at(i, k).gather_block();
                    let mut c_block = result.block_mut_at(i, j);
                    gemm_block(true, a_block, b_block, true, &mut c_block);
                }
            }
        }
        result
    }

    pub fn fast_mul_sequential(&self, other: &Self) -> Matrix {
        self.fast_mul_sequential_ric(other)
    }

    pub fn fast_mul_concurrent(&self, other: &Self) -> Matrix {
        assert_eq!(self.prime(), TWO);
        assert_eq!(self.prime(), other.prime());
        assert_eq!(self.columns(), other.rows());

        assert!(self.rows().is_multiple_of(64));
        assert!(self.columns().is_multiple_of(64));
        assert!(other.rows().is_multiple_of(64));
        assert!(other.columns().is_multiple_of(64));

        let mut result = Matrix::new(self.prime(), self.rows(), other.columns());

        MatrixTiling::new(&mut result)
            .maybe_par_iter_mut()
            .for_each(|c_block| {
                for k in (0..other.rows()).step_by(64) {
                    let a_block = self.block_at(c_block.i(), k).gather_block();
                    let b_block = other.block_at(k, c_block.j()).gather_block();
                    gemm_block(true, a_block, b_block, true, c_block);
                }
            });

        result
    }

    pub fn fast_mul_concurrent_recursive(&self, other: &Self) -> Matrix {
        assert_eq!(self.prime(), TWO);
        assert_eq!(self.prime(), other.prime());
        assert_eq!(self.columns(), other.rows());

        assert!(self.rows().is_multiple_of(64));
        assert!(self.columns().is_multiple_of(64));
        assert!(other.rows().is_multiple_of(64));
        assert!(other.columns().is_multiple_of(64));

        let mut result = Matrix::new(self.prime(), self.rows(), other.columns());
        let mut result_l2_block = result.as_l2_block_mut();

        fast_mul_l2_block(
            self.as_l2_block(),
            other.as_l2_block(),
            &mut result_l2_block,
        );

        result
    }
}

fn fast_mul_l2_block(a: MatrixL2BlockSlice, b: MatrixL2BlockSlice, c: &mut MatrixL2BlockSliceMut) {
    if c.block_rows() > 1 {
        let (a_first, a_second) = a.split_rows_at(a.block_rows() / 2);
        let (mut c_first, mut c_second) = c.split_rows_at_mut(c.block_rows() / 2);
        maybe_rayon::join(
            || fast_mul_l2_block(a_first, b, &mut c_first),
            || fast_mul_l2_block(a_second, b, &mut c_second),
        );
    } else if c.block_columns() > 1 {
        let (b_first, b_second) = b.split_columns_at(b.block_columns() / 2);
        let (mut c_first, mut c_second) = c.split_columns_at_mut(c.block_columns() / 2);
        maybe_rayon::join(
            || fast_mul_l2_block(a, b_first, &mut c_first),
            || fast_mul_l2_block(a, b_second, &mut c_second),
        );
    } else {
        for i in 0..a.block_rows() {
            for k in 0..a.block_columns() {
                let a_block = a.block_at(i, k).gather_block();
                for j in 0..b.block_columns() {
                    let b_block = b.block_at(k, j).gather_block();
                    let mut c_block = c.block_mut_at(i, j);
                    gemm_block(true, a_block, b_block, true, &mut c_block);
                }
            }
        }
    }
}

pub struct MatrixTiling<'a> {
    blocks: Vec<MatrixBlockSliceMut<'a>>,
}

impl<'a> MatrixTiling<'a> {
    pub fn new(matrix: &'a mut Matrix) -> Self {
        assert!(matrix.rows().is_multiple_of(64));
        assert!(matrix.columns().is_multiple_of(64));
        let rows = matrix.rows();
        let columns = matrix.columns();

        let mut blocks = Vec::new();
        for i in (0..rows).step_by(64) {
            for j in (0..columns).step_by(64) {
                blocks.push(unsafe { matrix.block_at(i, j).make_mut() });
            }
        }

        Self { blocks }
    }

    pub fn maybe_par_iter_mut(
        &mut self,
    ) -> impl MaybeParallelIterator<Item = &mut MatrixBlockSliceMut<'a>> {
        self.blocks.maybe_par_iter_mut()
    }
}

unsafe impl Send for MatrixTiling<'_> {}
unsafe impl Sync for MatrixTiling<'_> {}

#[derive(Debug, Clone, Copy)]
pub struct MatrixL2BlockSlice<'a> {
    limbs: *const Limb,
    dimensions: [usize; 2],
    stride: usize,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> MatrixL2BlockSlice<'a> {
    pub fn block_rows(&self) -> usize {
        self.dimensions[0]
    }

    pub fn block_columns(&self) -> usize {
        self.dimensions[1]
    }

    pub fn block_at(&self, block_row: usize, block_col: usize) -> MatrixBlockSlice<'_> {
        let start_limb = 64 * block_row * self.stride + block_col;
        let stride = self.stride;

        MatrixBlockSlice {
            limbs: unsafe { self.limbs.add(start_limb) },
            coords: [block_row, block_col],
            stride,
            _marker: std::marker::PhantomData,
        }
    }

    fn split_rows_at(&self, block_rows: usize) -> (MatrixL2BlockSlice<'_>, MatrixL2BlockSlice<'_>) {
        let (first_rows, second_rows) = (block_rows, self.block_rows() - block_rows);

        let first = MatrixL2BlockSlice {
            limbs: self.limbs,
            dimensions: [first_rows, self.dimensions[1]],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        let second = MatrixL2BlockSlice {
            limbs: unsafe { self.limbs.add(64 * first_rows * self.stride) },
            dimensions: [second_rows, self.dimensions[1]],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        (first, second)
    }

    fn split_columns_at(
        &self,
        block_columns: usize,
    ) -> (MatrixL2BlockSlice<'_>, MatrixL2BlockSlice<'_>) {
        let (first_cols, second_cols) = (block_columns, self.block_columns() - block_columns);

        let first = MatrixL2BlockSlice {
            limbs: self.limbs,
            dimensions: [self.dimensions[0], first_cols],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        let second = MatrixL2BlockSlice {
            limbs: unsafe { self.limbs.add(first_cols) },
            dimensions: [self.dimensions[0], second_cols],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        (first, second)
    }
}

unsafe impl Send for MatrixL2BlockSlice<'_> {}
unsafe impl Sync for MatrixL2BlockSlice<'_> {}

pub struct MatrixL2BlockSliceMut<'a> {
    limbs: *mut Limb,
    dimensions: [usize; 2],
    stride: usize,
    _marker: std::marker::PhantomData<&'a mut ()>,
}

impl<'a> MatrixL2BlockSliceMut<'a> {
    pub fn block_rows(&self) -> usize {
        self.dimensions[0]
    }

    pub fn block_columns(&self) -> usize {
        self.dimensions[1]
    }

    pub fn block_mut_at(&mut self, block_row: usize, block_col: usize) -> MatrixBlockSliceMut<'_> {
        let start_limb = 64 * block_row * self.stride + block_col;
        let stride = self.stride;

        MatrixBlockSliceMut {
            limbs: unsafe { self.limbs.add(start_limb) },
            coords: [block_row, block_col],
            stride,
            _marker: std::marker::PhantomData,
        }
    }

    fn split_rows_at_mut(
        &mut self,
        block_rows: usize,
    ) -> (MatrixL2BlockSliceMut<'_>, MatrixL2BlockSliceMut<'_>) {
        let (first_rows, second_rows) = (block_rows, self.block_rows() - block_rows);

        let first = MatrixL2BlockSliceMut {
            limbs: self.limbs,
            dimensions: [first_rows, self.dimensions[1]],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        let second = MatrixL2BlockSliceMut {
            limbs: unsafe { self.limbs.add(64 * first_rows * self.stride) },
            dimensions: [second_rows, self.dimensions[1]],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        (first, second)
    }

    fn split_columns_at_mut(
        &mut self,
        block_columns: usize,
    ) -> (MatrixL2BlockSliceMut<'_>, MatrixL2BlockSliceMut<'_>) {
        let (first_cols, second_cols) = (block_columns, self.block_columns() - block_columns);

        let first = MatrixL2BlockSliceMut {
            limbs: self.limbs,
            dimensions: [self.dimensions[0], first_cols],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        let second = MatrixL2BlockSliceMut {
            limbs: unsafe { self.limbs.add(first_cols) },
            dimensions: [self.dimensions[0], second_cols],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        (first, second)
    }
}

unsafe impl Send for MatrixL2BlockSliceMut<'_> {}

#[repr(align(128))]
#[derive(Debug, Clone, Copy)]
pub struct MatrixBlock {
    limbs: [Limb; 64],
}

#[derive(Clone, Copy)]
pub struct MatrixBlockSlice<'a> {
    limbs: *const Limb,
    coords: [usize; 2],
    stride: usize,
    _marker: std::marker::PhantomData<&'a ()>,
}

pub struct MatrixBlockSliceMut<'a> {
    limbs: *mut Limb,
    coords: [usize; 2],
    stride: usize,
    _marker: std::marker::PhantomData<&'a mut ()>,
}

impl<'a> MatrixBlockSlice<'a> {
    fn iter(self) -> impl Iterator<Item = &'a Limb> {
        (0..64).map(move |i| unsafe { &*self.limbs.add(i * self.stride) })
    }

    pub fn gather_block(self) -> MatrixBlock {
        if is_x86_feature_detected!("avx512f") {
            avx512::gather_block_avx512(self).as_matrix_block()
        // } else if is_x86_feature_detected!("avx") {
        //     avx::gather_block_avx(self)
        } else {
            scalar::gather_block_scalar(self)
        }
    }

    unsafe fn make_mut(self) -> MatrixBlockSliceMut<'a> {
        MatrixBlockSliceMut {
            limbs: self.limbs as *mut Limb,
            coords: self.coords,
            stride: self.stride,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'a> MatrixBlockSliceMut<'a> {
    fn i(&self) -> usize {
        self.coords[0]
    }

    fn j(&self) -> usize {
        self.coords[1]
    }

    fn get_mut(&mut self, row: usize) -> &mut Limb {
        unsafe { &mut *self.limbs.add(row * self.stride) }
    }

    fn iter_mut<'b>(&'b mut self) -> impl Iterator<Item = &'b mut Limb> + use<'a, 'b> {
        (0..64).map(move |i| unsafe { &mut *self.limbs.add(i * self.stride) })
    }

    fn as_slice(&self) -> MatrixBlockSlice<'_> {
        MatrixBlockSlice {
            limbs: self.limbs,
            coords: self.coords,
            stride: self.stride,
            _marker: std::marker::PhantomData,
        }
    }
}

unsafe impl Send for MatrixBlockSliceMut<'_> {}

pub fn gemm_block(
    alpha: bool,
    a: MatrixBlock,
    b: MatrixBlock,
    beta: bool,
    c: &mut MatrixBlockSliceMut,
) {
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
                &mut c.block_mut_at(0, 0)
            );
            scalar::gemm_block_scalar(
                alpha,
                a.block_at(0, 0).gather_block(),
                b.block_at(0, 0).gather_block(),
                beta,
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
                &mut c.block_mut_at(0, 0)
            );
            avx512::gemm_block_avx512_unrolled(
                alpha,
                a.block_at(0, 0).gather_block(),
                b.block_at(0, 0).gather_block(),
                beta,
                &mut c2.block_mut_at(0, 0)
            );
            prop_assert_eq!(c, c2);
        }

        #[test]
        fn test_fast_mul_concurrent_is_mul((m, n) in arb_multipliable_matrices()) {
            let prod1 = m.fast_mul_sequential(&n);
            let prod2 = m.fast_mul_concurrent(&n);
            prop_assert_eq!(prod1, prod2);
        }

        #[test]
        fn test_fast_mul_concurrent_cache_agnostic_is_mul((m, n) in arb_multipliable_matrices()) {
            let prod1 = m.fast_mul_concurrent(&n);
            let prod2 = m.fast_mul_concurrent_recursive(&n);
            prop_assert_eq!(prod1, prod2);
        }
    }

    proptest! {
        #[test]
        fn test_fast_mul_sequential_is_mul((m, n) in arb_multipliable_matrices()) {
            let prod1 = (&m) * (&n);
            let prod2 = m.fast_mul_sequential(&n);
            prop_assert_eq!(prod1, prod2);
        }
    }
}
