use std::marker::PhantomData;

use super::block::{BlockView, Immutable, Mutability, Mutable};
use crate::limb::Limb;

#[derive(Debug, Clone, Copy)]
pub struct TiledView<'a, M: Mutability> {
    pub limbs: M::Pointer<Limb>,
    pub dimensions: [usize; 2], // dimensions in blocks, not elements
    pub stride: usize,
    pub _marker: PhantomData<&'a ()>,
}

// Type aliases for convenience
pub type MatrixL2BlockSlice<'a> = TiledView<'a, Immutable>;
pub type MatrixL2BlockSliceMut<'a> = TiledView<'a, Mutable>;

impl<'a, M: Mutability> TiledView<'a, M> {
    pub fn block_rows(&self) -> usize {
        self.dimensions[0]
    }

    pub fn block_columns(&self) -> usize {
        self.dimensions[1]
    }
}

impl<'a> TiledView<'a, Immutable> {
    pub fn block_at(&self, block_row: usize, block_col: usize) -> BlockView<'_, Immutable> {
        let start_limb = 64 * block_row * self.stride + block_col;
        let stride = self.stride;

        BlockView {
            limbs: unsafe { self.limbs.add(start_limb) },
            coords: [block_row, block_col],
            stride,
            _marker: PhantomData,
        }
    }

    pub fn split_rows_at(
        &self,
        block_rows: usize,
    ) -> (TiledView<'_, Immutable>, TiledView<'_, Immutable>) {
        let (first_rows, second_rows) = (block_rows, self.block_rows() - block_rows);

        let first = TiledView {
            limbs: self.limbs,
            dimensions: [first_rows, self.dimensions[1]],
            stride: self.stride,
            _marker: PhantomData,
        };
        let second = TiledView {
            limbs: unsafe { self.limbs.add(64 * first_rows * self.stride) },
            dimensions: [second_rows, self.dimensions[1]],
            stride: self.stride,
            _marker: PhantomData,
        };
        (first, second)
    }

    pub fn split_columns_at(
        &self,
        block_columns: usize,
    ) -> (TiledView<'_, Immutable>, TiledView<'_, Immutable>) {
        let (first_cols, second_cols) = (block_columns, self.block_columns() - block_columns);

        let first = TiledView {
            limbs: self.limbs,
            dimensions: [self.dimensions[0], first_cols],
            stride: self.stride,
            _marker: PhantomData,
        };
        let second = TiledView {
            limbs: unsafe { self.limbs.add(first_cols) },
            dimensions: [self.dimensions[0], second_cols],
            stride: self.stride,
            _marker: PhantomData,
        };
        (first, second)
    }
}

impl<'a> TiledView<'a, Mutable> {
    pub fn block_mut_at(&mut self, block_row: usize, block_col: usize) -> BlockView<'_, Mutable> {
        let start_limb = 64 * block_row * self.stride + block_col;
        let stride = self.stride;

        BlockView {
            limbs: unsafe { self.limbs.add(start_limb) },
            coords: [block_row, block_col],
            stride,
            _marker: PhantomData,
        }
    }

    pub fn split_rows_at_mut(
        &mut self,
        block_rows: usize,
    ) -> (TiledView<'_, Mutable>, TiledView<'_, Mutable>) {
        let (first_rows, second_rows) = (block_rows, self.block_rows() - block_rows);

        let first = TiledView {
            limbs: self.limbs,
            dimensions: [first_rows, self.dimensions[1]],
            stride: self.stride,
            _marker: PhantomData,
        };
        let second = TiledView {
            limbs: unsafe { self.limbs.add(64 * first_rows * self.stride) },
            dimensions: [second_rows, self.dimensions[1]],
            stride: self.stride,
            _marker: PhantomData,
        };
        (first, second)
    }

    pub fn split_columns_at_mut(
        &mut self,
        block_columns: usize,
    ) -> (TiledView<'_, Mutable>, TiledView<'_, Mutable>) {
        let (first_cols, second_cols) = (block_columns, self.block_columns() - block_columns);

        let first = TiledView {
            limbs: self.limbs,
            dimensions: [self.dimensions[0], first_cols],
            stride: self.stride,
            _marker: PhantomData,
        };
        let second = TiledView {
            limbs: unsafe { self.limbs.add(first_cols) },
            dimensions: [self.dimensions[0], second_cols],
            stride: self.stride,
            _marker: PhantomData,
        };
        (first, second)
    }
}

unsafe impl<M: Mutability> Send for TiledView<'_, M> {}
