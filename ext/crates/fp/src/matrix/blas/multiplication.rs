use super::{
    gemm_block,
    tiling::{MatrixL2BlockSlice, MatrixL2BlockSliceMut},
};
use crate::{matrix::Matrix, prime::TWO};

// Zero-cost loop ordering types
pub struct RCI;
pub struct CRI;
pub struct ICR;
pub struct RIC;
pub struct IRC;
pub struct CIR;

pub mod orders {
    pub use super::{CIR, CRI, ICR, IRC, RCI, RIC};
}

pub trait LoopOrder {
    fn fast_mul_sequential_inner(
        a: MatrixL2BlockSlice,
        b: MatrixL2BlockSlice,
        c: MatrixL2BlockSliceMut,
    );
}

impl LoopOrder for RCI {
    fn fast_mul_sequential_inner(
        a: MatrixL2BlockSlice,
        b: MatrixL2BlockSlice,
        mut c: MatrixL2BlockSliceMut,
    ) {
        for i in 0..a.block_rows() {
            for j in 0..b.block_columns() {
                let mut c_block = c.block_mut_at(i, j);
                for k in 0..b.block_rows() {
                    let a_block = a.block_at(i, k).gather_block();
                    let b_block = b.block_at(k, j).gather_block();
                    gemm_block(true, a_block, b_block, true, c_block.copy());
                }
            }
        }
    }
}

impl LoopOrder for CRI {
    fn fast_mul_sequential_inner(
        a: MatrixL2BlockSlice,
        b: MatrixL2BlockSlice,
        mut c: MatrixL2BlockSliceMut,
    ) {
        for j in 0..b.block_columns() {
            for i in 0..a.block_rows() {
                let mut c_block = c.block_mut_at(i, j);
                for k in 0..b.block_rows() {
                    let a_block = a.block_at(i, k).gather_block();
                    let b_block = b.block_at(k, j).gather_block();
                    gemm_block(true, a_block, b_block, true, c_block.copy());
                }
            }
        }
    }
}

impl LoopOrder for ICR {
    fn fast_mul_sequential_inner(
        a: MatrixL2BlockSlice,
        b: MatrixL2BlockSlice,
        mut c: MatrixL2BlockSliceMut,
    ) {
        for k in 0..b.block_rows() {
            for j in 0..b.block_columns() {
                let b_block = b.block_at(k, j).gather_block();
                for i in 0..a.block_rows() {
                    let a_block = a.block_at(i, k).gather_block();
                    let c_block = c.block_mut_at(i, j);
                    gemm_block(true, a_block, b_block, true, c_block);
                }
            }
        }
    }
}

impl LoopOrder for RIC {
    fn fast_mul_sequential_inner(
        a: MatrixL2BlockSlice,
        b: MatrixL2BlockSlice,
        mut c: MatrixL2BlockSliceMut,
    ) {
        for i in 0..a.block_rows() {
            for k in 0..a.block_columns() {
                let a_block = a.block_at(i, k).gather_block();
                for j in 0..b.block_columns() {
                    let b_block = b.block_at(k, j).gather_block();
                    let c_block = c.block_mut_at(i, j);
                    gemm_block(true, a_block, b_block, true, c_block);
                }
            }
        }
    }
}

impl LoopOrder for IRC {
    fn fast_mul_sequential_inner(
        a: MatrixL2BlockSlice,
        b: MatrixL2BlockSlice,
        mut c: MatrixL2BlockSliceMut,
    ) {
        for k in 0..b.block_rows() {
            for i in 0..a.block_rows() {
                let a_block = a.block_at(i, k).gather_block();
                for j in 0..b.block_columns() {
                    let b_block = b.block_at(k, j).gather_block();
                    let c_block = c.block_mut_at(i, j);
                    gemm_block(true, a_block, b_block, true, c_block);
                }
            }
        }
    }
}

impl LoopOrder for CIR {
    fn fast_mul_sequential_inner(
        a: MatrixL2BlockSlice,
        b: MatrixL2BlockSlice,
        mut c: MatrixL2BlockSliceMut,
    ) {
        for j in 0..b.block_columns() {
            for k in 0..b.block_rows() {
                let b_block = b.block_at(k, j).gather_block();
                for i in 0..a.block_rows() {
                    let a_block = a.block_at(i, k).gather_block();
                    let c_block = c.block_mut_at(i, j);
                    gemm_block(true, a_block, b_block, true, c_block);
                }
            }
        }
    }
}

pub fn fast_mul_sequential<L: LoopOrder>(a: &Matrix, b: &Matrix) -> Matrix {
    assert_eq!(a.prime(), TWO);
    assert_eq!(a.prime(), b.prime());
    assert_eq!(a.columns(), b.rows());

    assert!(a.rows().is_multiple_of(64));
    assert!(a.columns().is_multiple_of(64));
    assert!(b.rows().is_multiple_of(64));
    assert!(b.columns().is_multiple_of(64));

    let mut result = Matrix::new(a.prime(), a.rows(), b.columns());
    let result_l2_block = result.as_l2_block_mut();

    L::fast_mul_sequential_inner(a.as_l2_block(), b.as_l2_block(), result_l2_block);

    result
}

pub fn fast_mul_concurrent<const M: usize, const N: usize, L: LoopOrder>(
    a: &Matrix,
    b: &Matrix,
) -> Matrix {
    assert_eq!(a.prime(), TWO);
    assert_eq!(a.prime(), b.prime());
    assert_eq!(a.columns(), b.rows());

    assert!(a.rows().is_multiple_of(64));
    assert!(a.columns().is_multiple_of(64));
    assert!(b.rows().is_multiple_of(64));
    assert!(b.columns().is_multiple_of(64));

    let mut result = Matrix::new(a.prime(), a.rows(), b.columns());
    let result_l2_block = result.as_l2_block_mut();

    fast_mul_concurrent_inner::<M, N, L>(a.as_l2_block(), b.as_l2_block(), result_l2_block);

    result
}

fn fast_mul_concurrent_inner<const M: usize, const N: usize, L: LoopOrder>(
    a: MatrixL2BlockSlice,
    b: MatrixL2BlockSlice,
    mut c: MatrixL2BlockSliceMut,
) {
    if c.block_rows() > M {
        let (a_first, a_second) = a.split_rows_at(a.block_rows() / 2);
        let (c_first, c_second) = c.split_rows_at_mut(c.block_rows() / 2);
        maybe_rayon::join(
            move || fast_mul_concurrent_inner::<M, N, L>(a_first, b, c_first),
            move || fast_mul_concurrent_inner::<M, N, L>(a_second, b, c_second),
        );
    } else if c.block_columns() > N {
        let (b_first, b_second) = b.split_columns_at(b.block_columns() / 2);
        let (c_first, c_second) = c.split_columns_at_mut(c.block_columns() / 2);
        maybe_rayon::join(
            move || fast_mul_concurrent_inner::<M, N, L>(a, b_first, c_first),
            move || fast_mul_concurrent_inner::<M, N, L>(a, b_second, c_second),
        );
    } else {
        L::fast_mul_sequential_inner(a, b, c);
    }
}
