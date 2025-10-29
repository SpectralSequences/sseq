pub use tiles::{MatrixTileSlice, MatrixTileSliceMut};

use super::block;
use crate::matrix::Matrix;

pub mod tiles;

impl Matrix {
    pub fn as_tile(&self) -> MatrixTileSlice<'_> {
        MatrixTileSlice {
            limbs: self.data().as_ptr(),
            dimensions: [self.physical_rows() / 64, self.columns().div_ceil(64)],
            stride: self.stride(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn as_tile_mut(&mut self) -> MatrixTileSliceMut<'_> {
        MatrixTileSliceMut {
            limbs: self.data_mut().as_mut_ptr(),
            dimensions: [self.physical_rows() / 64, self.columns().div_ceil(64)],
            stride: self.stride(),
            _marker: std::marker::PhantomData,
        }
    }
}

// Zero-cost loop ordering marker types
/// Row-Column-Inner loop order: `for i { for j { for k { ... } } }`
pub struct RCI;
/// Column-Row-Inner loop order: `for j { for i { for k { ... } } }`
pub struct CRI;
/// Inner-Column-Row loop order: `for k { for j { for i { ... } } }`
pub struct ICR;
/// Row-Inner-Column loop order: `for i { for k { for j { ... } } }`
pub struct RIC;
/// Inner-Row-Column loop order: `for k { for i { for j { ... } } }`
pub struct IRC;
/// Column-Inner-Row loop order: `for j { for k { for i { ... } } }`
pub struct CIR;

/// Re-exports of loop ordering types for convenience.
pub mod orders {
    pub use super::{CIR, CRI, ICR, IRC, RCI, RIC};
}

/// Performs tile-level GEMM with a specified loop ordering.
///
/// This is the sequential (non-parallel) version. For large matrices, use [`gemm_concurrent`]
/// instead.
///
/// # Loop Ordering
///
/// The choice of loop order affects cache locality and performance. Benchmarking suggests RIC is
/// optimal for most cases, but this depends on matrix dimensions.
#[inline]
pub fn gemm<L: LoopOrder>(
    alpha: bool,
    a: MatrixTileSlice,
    b: MatrixTileSlice,
    beta: bool,
    c: MatrixTileSliceMut,
) {
    L::gemm(alpha, a, b, beta, c);
}

/// Performs tile-level GEMM with recursive parallelization.
///
/// The matrix is recursively split along rows (if rows > M blocks) or columns (if cols > N blocks)
/// until tiles are small enough, then all tiles are processed in parallel using rayon.
///
/// # Type Parameters
///
/// * `M` - Minimum block rows before parallelization stops
/// * `N` - Minimum block columns before parallelization stops
/// * `L` - Loop ordering strategy (see [`LoopOrder`])
///
/// # Performance
///
/// For best performance, choose M and N based on your matrix sizes. The defaults used in the
/// codebase are M=1, N=16, which work well for many workloads.
#[inline]
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

/// Trait for zero-cost loop ordering strategies.
///
/// Different loop orders have different cache access patterns, which can significantly impact
/// performance. All six permutations are provided: RCI, CRI, ICR, RIC, IRC, CIR (where R=row,
/// C=column, I=inner).
pub trait LoopOrder {
    /// Performs GEMM with this loop ordering strategy.
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
                let mut c_block = c.block_mut_at(i, j).as_slice().gather();
                for k in 0..b.block_rows() {
                    let a_block = a.block_at(i, k).gather();
                    let b_block = b.block_at(k, j).gather();
                    c_block = block::gemm_block(alpha, a_block, b_block, beta, c_block);
                }
                c.block_mut_at(i, j).assign(c_block);
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
                let mut c_block = c.block_mut_at(i, j).as_slice().gather();
                for k in 0..b.block_rows() {
                    let a_block = a.block_at(i, k).gather();
                    let b_block = b.block_at(k, j).gather();
                    c_block = block::gemm_block(alpha, a_block, b_block, beta, c_block);
                }
                c.block_mut_at(i, j).assign(c_block);
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
                    let c_block = c.block_mut_at(i, j).as_slice().gather();
                    let new_c_block = block::gemm_block(alpha, a_block, b_block, beta, c_block);
                    c.block_mut_at(i, j).assign(new_c_block);
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
                    let c_block = c.block_mut_at(i, j).as_slice().gather();
                    let new_c_block = block::gemm_block(alpha, a_block, b_block, beta, c_block);
                    c.block_mut_at(i, j).assign(new_c_block);
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
                    let c_block = c.block_mut_at(i, j).as_slice().gather();
                    let new_c_block = block::gemm_block(alpha, a_block, b_block, beta, c_block);
                    c.block_mut_at(i, j).assign(new_c_block);
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
                    let c_block = c.block_mut_at(i, j).as_slice().gather();
                    let new_c_block = block::gemm_block(alpha, a_block, b_block, beta, c_block);
                    c.block_mut_at(i, j).assign(new_c_block);
                }
            }
        }
    }
}
