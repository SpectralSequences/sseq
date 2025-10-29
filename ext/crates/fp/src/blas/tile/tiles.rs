use super::block::{MatrixBlockSlice, MatrixBlockSliceMut};
use crate::limb::Limb;

/// An immutable view of a tile within a matrix.
///
/// A tile is a rectangular region composed of multiple 64 x 64 blocks. Tiles enable hierarchical
/// parallelization: large matrices are divided into tiles which are processed in parallel, and each
/// tile is further divided into blocks for vectorization.
///
/// # Safety
///
/// The `limbs` pointer must remain valid for the lifetime `'a`, and must point to a region large
/// enough for `dimensions[0] * 64` rows and `dimensions[1]` blocks with the given stride.
#[derive(Debug, Clone, Copy)]
pub struct MatrixTileSlice<'a> {
    pub limbs: *const Limb,
    /// Dimensions of the tile in units of 64 x 64 blocks: [block_rows, block_cols]
    pub dimensions: [usize; 2],
    /// Number of limbs between consecutive rows in the parent matrix
    pub stride: usize,
    pub _marker: std::marker::PhantomData<&'a ()>,
}

/// A mutable view of a tile within a matrix.
///
/// # Safety
///
/// The `limbs` pointer must remain valid and exclusively accessible for the lifetime `'a`, and must
/// point to a region large enough for `dimensions[0] * 64` rows and `dimensions[1]` blocks with the
/// given stride.
#[derive(Debug, Clone, Copy)]
pub struct MatrixTileSliceMut<'a> {
    pub limbs: *mut Limb,
    /// Dimensions of the tile in units of 64 x 64 blocks: [block_rows, block_cols]
    pub dimensions: [usize; 2],
    /// Number of limbs between consecutive rows in the parent matrix
    pub stride: usize,
    pub _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> MatrixTileSlice<'a> {
    /// Returns the number of 64 x 64 block rows in this tile.
    #[inline]
    pub fn block_rows(&self) -> usize {
        self.dimensions[0]
    }

    /// Returns the number of 64 x 64 block columns in this tile.
    #[inline]
    pub fn block_columns(&self) -> usize {
        self.dimensions[1]
    }

    /// Returns a view of the block at the given block coordinates.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the block coordinates are out of bounds.
    #[inline]
    pub fn block_at(&self, block_row: usize, block_col: usize) -> MatrixBlockSlice<'_> {
        debug_assert!(
            block_row < self.dimensions[0],
            "block_row {block_row} out of bounds (max {})",
            self.dimensions[0]
        );
        debug_assert!(
            block_col < self.dimensions[1],
            "block_col {block_col} out of bounds (max {})",
            self.dimensions[1]
        );

        let start_limb = 64 * block_row * self.stride + block_col;
        let stride = self.stride;

        MatrixBlockSlice {
            limbs: unsafe {
                // SAFETY: block coordinates are in bounds (checked above in debug mode), and the
                // parent tile guarantees sufficient memory is allocated
                self.limbs.add(start_limb)
            },
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
        debug_assert!(
            block_row < self.dimensions[0],
            "block_row {block_row} out of bounds (max {})",
            self.dimensions[0]
        );
        debug_assert!(
            block_col < self.dimensions[1],
            "block_col {block_col} out of bounds (max {})",
            self.dimensions[1]
        );

        let start_limb = 64 * block_row * self.stride + block_col;
        let stride = self.stride;

        MatrixBlockSliceMut {
            limbs: unsafe {
                // SAFETY: block coordinates are in bounds (checked above in debug mode), and the
                // parent tile guarantees sufficient memory is allocated
                self.limbs.add(start_limb)
            },
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
