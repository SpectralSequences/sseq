pub use tiles::{MatrixTileSlice, MatrixTileSliceMut};

use super::block;
use crate::matrix::Matrix;

pub mod tiles;

impl Matrix {
    pub(crate) fn as_tile(&self) -> MatrixTileSlice<'_> {
        assert!(self.rows().is_multiple_of(64));
        assert!(self.columns().is_multiple_of(64));

        MatrixTileSlice {
            limbs: self.data().as_ptr(),
            dimensions: [self.rows() / 64, self.columns() / 64],
            stride: self.stride(),
            _marker: std::marker::PhantomData,
        }
    }

    pub(crate) fn as_tile_mut(&mut self) -> MatrixTileSliceMut<'_> {
        assert!(self.rows().is_multiple_of(64));
        assert!(self.columns().is_multiple_of(64));

        MatrixTileSliceMut {
            limbs: self.data_mut().as_mut_ptr(),
            dimensions: [self.rows() / 64, self.columns() / 64],
            stride: self.stride(),
            _marker: std::marker::PhantomData,
        }
    }
}

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

pub fn gemm<L: LoopOrder>(
    alpha: bool,
    a: MatrixTileSlice,
    b: MatrixTileSlice,
    beta: bool,
    c: MatrixTileSliceMut,
) {
    L::gemm(alpha, a, b, beta, c);
}

pub fn gemm_concurrent<const M: usize, const N: usize, L: LoopOrder>(
    alpha: bool,
    a: MatrixTileSlice,
    b: MatrixTileSlice,
    beta: bool,
    mut c: MatrixTileSliceMut,
) {
    if c.block_rows() > M {
        let (a_first, a_second) = a.split_rows_at(a.block_rows() / 2);
        let (c_first, c_second) = c.split_rows_at_mut(c.block_rows() / 2);
        maybe_rayon::join(
            move || gemm_concurrent::<M, N, L>(alpha, a_first, b, beta, c_first),
            move || gemm_concurrent::<M, N, L>(alpha, a_second, b, beta, c_second),
        );
    } else if c.block_columns() > N {
        let (b_first, b_second) = b.split_columns_at(b.block_columns() / 2);
        let (c_first, c_second) = c.split_columns_at_mut(c.block_columns() / 2);
        maybe_rayon::join(
            move || gemm_concurrent::<M, N, L>(alpha, a, b_first, beta, c_first),
            move || gemm_concurrent::<M, N, L>(alpha, a, b_second, beta, c_second),
        );
    } else {
        gemm::<L>(alpha, a, b, beta, c);
    }
}

pub trait LoopOrder {
    fn gemm(alpha: bool, a: MatrixTileSlice, b: MatrixTileSlice, beta: bool, c: MatrixTileSliceMut);
}

impl LoopOrder for RCI {
    fn gemm(
        alpha: bool,
        a: MatrixTileSlice,
        b: MatrixTileSlice,
        beta: bool,
        mut c: MatrixTileSliceMut,
    ) {
        for i in 0..a.block_rows() {
            for j in 0..b.block_columns() {
                let mut c_block = c.block_mut_at(i, j);
                for k in 0..b.block_rows() {
                    let a_block = a.block_at(i, k).gather();
                    let b_block = b.block_at(k, j).gather();
                    block::gemm_block(alpha, a_block, b_block, beta, c_block.copy());
                }
            }
        }
    }
}

impl LoopOrder for CRI {
    fn gemm(
        alpha: bool,
        a: MatrixTileSlice,
        b: MatrixTileSlice,
        beta: bool,
        mut c: MatrixTileSliceMut,
    ) {
        for j in 0..b.block_columns() {
            for i in 0..a.block_rows() {
                let mut c_block = c.block_mut_at(i, j);
                for k in 0..b.block_rows() {
                    let a_block = a.block_at(i, k).gather();
                    let b_block = b.block_at(k, j).gather();
                    block::gemm_block(alpha, a_block, b_block, beta, c_block.copy());
                }
            }
        }
    }
}

impl LoopOrder for ICR {
    fn gemm(
        alpha: bool,
        a: MatrixTileSlice,
        b: MatrixTileSlice,
        beta: bool,
        mut c: MatrixTileSliceMut,
    ) {
        for k in 0..b.block_rows() {
            for j in 0..b.block_columns() {
                let b_block = b.block_at(k, j).gather();
                for i in 0..a.block_rows() {
                    let a_block = a.block_at(i, k).gather();
                    let c_block = c.block_mut_at(i, j);
                    block::gemm_block(alpha, a_block, b_block, beta, c_block);
                }
            }
        }
    }
}

impl LoopOrder for RIC {
    fn gemm(
        alpha: bool,
        a: MatrixTileSlice,
        b: MatrixTileSlice,
        beta: bool,
        mut c: MatrixTileSliceMut,
    ) {
        for i in 0..a.block_rows() {
            for k in 0..a.block_columns() {
                let a_block = a.block_at(i, k).gather();
                for j in 0..b.block_columns() {
                    let b_block = b.block_at(k, j).gather();
                    let c_block = c.block_mut_at(i, j);
                    block::gemm_block(alpha, a_block, b_block, beta, c_block);
                }
            }
        }
    }
}

impl LoopOrder for IRC {
    fn gemm(
        alpha: bool,
        a: MatrixTileSlice,
        b: MatrixTileSlice,
        beta: bool,
        mut c: MatrixTileSliceMut,
    ) {
        for k in 0..b.block_rows() {
            for i in 0..a.block_rows() {
                let a_block = a.block_at(i, k).gather();
                for j in 0..b.block_columns() {
                    let b_block = b.block_at(k, j).gather();
                    let c_block = c.block_mut_at(i, j);
                    block::gemm_block(alpha, a_block, b_block, beta, c_block);
                }
            }
        }
    }
}

impl LoopOrder for CIR {
    fn gemm(
        alpha: bool,
        a: MatrixTileSlice,
        b: MatrixTileSlice,
        beta: bool,
        mut c: MatrixTileSliceMut,
    ) {
        for j in 0..b.block_columns() {
            for k in 0..b.block_rows() {
                let b_block = b.block_at(k, j).gather();
                for i in 0..a.block_rows() {
                    let a_block = a.block_at(i, k).gather();
                    let c_block = c.block_mut_at(i, j);
                    block::gemm_block(alpha, a_block, b_block, beta, c_block);
                }
            }
        }
    }
}
