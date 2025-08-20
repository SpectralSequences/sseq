use super::block::{MatrixBlockSlice, MatrixBlockSliceMut};
use crate::limb::Limb;

#[derive(Debug, Clone, Copy)]
pub struct MatrixTileSlice<'a> {
    pub limbs: *const Limb,
    /// Dimensions of the tile in units of blocks of 64 x 64 bits.
    pub dimensions: [usize; 2],
    pub stride: usize,
    pub _marker: std::marker::PhantomData<&'a ()>,
}

#[derive(Debug, Clone, Copy)]
pub struct MatrixTileSliceMut<'a> {
    pub limbs: *mut Limb,
    /// Dimensions of the tile in units of blocks of 64 x 64 bits.
    pub dimensions: [usize; 2],
    pub stride: usize,
    pub _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> MatrixTileSlice<'a> {
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

    pub fn split_rows_at(&self, block_rows: usize) -> (MatrixTileSlice<'_>, MatrixTileSlice<'_>) {
        let (first_rows, second_rows) = (block_rows, self.block_rows() - block_rows);

        let first = MatrixTileSlice {
            limbs: self.limbs,
            dimensions: [first_rows, self.dimensions[1]],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        let second = MatrixTileSlice {
            limbs: unsafe { self.limbs.add(64 * first_rows * self.stride) },
            dimensions: [second_rows, self.dimensions[1]],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        (first, second)
    }

    pub fn split_columns_at(
        &self,
        block_columns: usize,
    ) -> (MatrixTileSlice<'_>, MatrixTileSlice<'_>) {
        let (first_cols, second_cols) = (block_columns, self.block_columns() - block_columns);

        let first = MatrixTileSlice {
            limbs: self.limbs,
            dimensions: [self.dimensions[0], first_cols],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        let second = MatrixTileSlice {
            limbs: unsafe { self.limbs.add(first_cols) },
            dimensions: [self.dimensions[0], second_cols],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        (first, second)
    }
}

impl<'a> MatrixTileSliceMut<'a> {
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

    pub fn split_rows_at_mut(
        &mut self,
        block_rows: usize,
    ) -> (MatrixTileSliceMut<'_>, MatrixTileSliceMut<'_>) {
        let (first_rows, second_rows) = (block_rows, self.block_rows() - block_rows);

        let first = MatrixTileSliceMut {
            limbs: self.limbs,
            dimensions: [first_rows, self.dimensions[1]],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        let second = MatrixTileSliceMut {
            limbs: unsafe { self.limbs.add(64 * first_rows * self.stride) },
            dimensions: [second_rows, self.dimensions[1]],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        (first, second)
    }

    pub fn split_columns_at_mut(
        &mut self,
        block_columns: usize,
    ) -> (MatrixTileSliceMut<'_>, MatrixTileSliceMut<'_>) {
        let (first_cols, second_cols) = (block_columns, self.block_columns() - block_columns);

        let first = MatrixTileSliceMut {
            limbs: self.limbs,
            dimensions: [self.dimensions[0], first_cols],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        let second = MatrixTileSliceMut {
            limbs: unsafe { self.limbs.add(first_cols) },
            dimensions: [self.dimensions[0], second_cols],
            stride: self.stride,
            _marker: std::marker::PhantomData,
        };
        (first, second)
    }
}

unsafe impl Send for MatrixTileSlice<'_> {}
unsafe impl Sync for MatrixTileSlice<'_> {}

unsafe impl Send for MatrixTileSliceMut<'_> {}
