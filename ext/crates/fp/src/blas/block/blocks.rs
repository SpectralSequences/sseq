use crate::limb::Limb;

/// A contiguous 64 x 64 block of bits stored in row-major order.
///
/// Each limb represents one row of 64 bits. The 128-byte alignment ensures efficient SIMD
/// operations and cache line alignment.
#[repr(align(128))]
#[derive(Debug, Clone, Copy)]
pub struct MatrixBlock {
    pub limbs: [Limb; 64],
}

impl MatrixBlock {
    /// Creates a zero-initialized block.
    #[inline]
    pub fn zero() -> Self {
        Self { limbs: [0; 64] }
    }

    /// Returns a mutable iterator over the limbs (rows) of this block.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Limb> {
        self.limbs.iter_mut()
    }
}

/// A non-contiguous view of a 64 x 64 block within a larger matrix.
///
/// The block is stored in row-major order with a configurable stride between rows. This allows
/// efficient access to sub-blocks within a matrix without copying data.
///
/// # Safety
///
/// The `limbs` pointer must remain valid for the lifetime `'a`, and must point to at least 64 valid
/// rows spaced `stride` limbs apart.
pub struct MatrixBlockSlice<'a> {
    pub limbs: *const Limb,
    /// Number of limbs between consecutive rows
    pub stride: usize,
    pub _marker: std::marker::PhantomData<&'a ()>,
}

/// A mutable non-contiguous view of a 64 x 64 block within a larger matrix.
///
/// # Safety
///
/// The `limbs` pointer must remain valid and exclusively accessible for the lifetime `'a`, and must
/// point to at least 64 valid rows spaced `stride` limbs apart.
pub struct MatrixBlockSliceMut<'a> {
    pub limbs: *mut Limb,
    /// Number of limbs between consecutive rows
    pub stride: usize,
    pub _marker: std::marker::PhantomData<&'a mut ()>,
}

impl<'a> MatrixBlockSlice<'a> {
    /// Returns an iterator over the 64 rows of this block.
    ///
    /// # Safety
    ///
    /// Each element is obtained via `self.limbs.add(i * self.stride)`, which is safe because the
    /// constructor guarantees 64 valid rows at the given stride.
    pub fn iter(self) -> impl Iterator<Item = &'a Limb> {
        (0..64).map(move |i| unsafe {
            // SAFETY: Constructor guarantees 64 rows at stride intervals
            &*self.limbs.add(i * self.stride)
        })
    }

    /// Gathers the non-contiguous block into a contiguous `MatrixBlock`.
    ///
    /// This operation is necessary before performing block-level GEMM since the AVX-512 kernel
    /// expects contiguous data.
    #[inline]
    pub fn gather(self) -> MatrixBlock {
        if is_x86_feature_detected!("avx512f") {
            super::avx512::gather_block_avx512(self).as_matrix_block()
        } else {
            super::scalar::gather_block_scalar(self)
        }
    }
}

impl<'a> MatrixBlockSliceMut<'a> {
    /// Returns a mutable reference to the limb at the given row.
    ///
    /// # Safety
    ///
    /// The pointer arithmetic `self.limbs.add(row * self.stride)` is safe because the constructor
    /// guarantees 64 valid rows, and this method will panic in debug mode if `row >= 64` (via debug
    /// assertions in the caller).
    #[inline]
    pub fn get_mut(&mut self, row: usize) -> &mut Limb {
        debug_assert!(row < 64, "row index {row} out of bounds for 64 x 64 block");
        unsafe {
            // SAFETY: Constructor guarantees 64 rows at stride intervals
            &mut *self.limbs.add(row * self.stride)
        }
    }

    /// Returns a mutable iterator over the 64 rows of this block.
    #[inline]
    pub fn iter_mut<'b>(&'b mut self) -> impl Iterator<Item = &'b mut Limb> + use<'a, 'b> {
        (0..64).map(move |i| unsafe {
            // SAFETY: Constructor guarantees 64 rows at stride intervals
            &mut *self.limbs.add(i * self.stride)
        })
    }

    /// Creates a copy of this mutable slice with a shorter lifetime.
    ///
    /// This is useful for splitting the lifetime when you need to pass the slice to a function that
    /// doesn't need to hold it for the full `'a` lifetime.
    #[inline]
    pub fn copy(&mut self) -> MatrixBlockSliceMut<'_> {
        MatrixBlockSliceMut {
            limbs: self.limbs,
            stride: self.stride,
            _marker: std::marker::PhantomData,
        }
    }

    /// Converts this mutable slice into an immutable slice.
    #[inline]
    pub fn as_slice(&self) -> MatrixBlockSlice<'_> {
        MatrixBlockSlice {
            limbs: self.limbs,
            stride: self.stride,
            _marker: std::marker::PhantomData,
        }
    }

    /// Scatters a contiguous block into this non-contiguous slice.
    ///
    /// This is the inverse of `gather` and is used to write GEMM results back into the parent
    /// matrix.
    #[inline]
    pub fn assign(&mut self, block: MatrixBlock) {
        self.iter_mut()
            .zip(block.limbs.iter())
            .for_each(|(dst, &src)| *dst = src);
    }
}

unsafe impl<'a> Send for MatrixBlockSlice<'a> {}
unsafe impl<'a> Send for MatrixBlockSliceMut<'a> {}

unsafe impl<'a> Sync for MatrixBlockSlice<'a> {}
