use std::num::NonZeroUsize;

use crate::limb::Limb;

/// A contiguous 64 x 64 block of bits stored in row-major order.
///
/// Each limb represents one row of 64 bits. The 128-byte alignment ensures efficient SIMD
/// operations and cache line alignment.
#[repr(align(128))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatrixBlock([Limb; 64]);

impl MatrixBlock {
    #[inline]
    pub fn new(limbs: [Limb; 64]) -> Self {
        Self(limbs)
    }

    /// Creates a zero-initialized block.
    #[inline]
    pub fn zero() -> Self {
        Self([0; 64])
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Limb> {
        self.0.iter()
    }

    /// Returns a mutable iterator over the limbs (rows) of this block.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Limb> {
        self.0.iter_mut()
    }

    #[cfg_attr(not(target_feature = "avx512f"), allow(dead_code))]
    pub(crate) fn limbs_ptr(&self) -> *const Limb {
        self.0.as_ptr()
    }

    #[cfg_attr(not(target_feature = "avx512f"), allow(dead_code))]
    pub(crate) fn limbs_mut_ptr(&mut self) -> *mut Limb {
        self.0.as_mut_ptr()
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
    limbs: *const Limb,
    /// Number of limbs between consecutive rows
    stride: NonZeroUsize,
    _marker: std::marker::PhantomData<&'a ()>,
}

/// A mutable non-contiguous view of a 64 x 64 block within a larger matrix.
///
/// # Safety
///
/// The `limbs` pointer must remain valid and exclusively accessible for the lifetime `'a`, and must
/// point to at least 64 valid rows spaced `stride` limbs apart.
pub struct MatrixBlockSliceMut<'a> {
    limbs: *mut Limb,
    /// Number of limbs between consecutive rows
    stride: NonZeroUsize,
    _marker: std::marker::PhantomData<&'a mut ()>,
}

impl<'a> MatrixBlockSlice<'a> {
    pub(super) fn new(limbs: *const Limb, stride: NonZeroUsize) -> Self {
        Self {
            limbs,
            stride,
            _marker: std::marker::PhantomData,
        }
    }

    pub(crate) fn limbs(&self) -> *const Limb {
        self.limbs
    }

    pub(crate) fn stride(&self) -> NonZeroUsize {
        self.stride
    }

    /// Returns an iterator over the 64 rows of this block.
    ///
    /// # Safety
    ///
    /// Each element is obtained via `self.limbs.add(i * self.stride)`, which is safe because the
    /// constructor guarantees 64 valid rows at the given stride.
    pub fn iter(self) -> impl Iterator<Item = &'a Limb> {
        (0..64).map(move |i| unsafe {
            // SAFETY: Constructor guarantees 64 rows at stride intervals
            &*self.limbs.add(i * self.stride.get())
        })
    }

    /// Gathers the non-contiguous block into a contiguous `MatrixBlock`.
    ///
    /// This operation is necessary before performing block-level GEMM since the AVX-512 kernel
    /// expects contiguous data.
    #[inline]
    pub fn gather(self) -> MatrixBlock {
        // Delegate to SIMD specializations
        crate::simd::gather_block_simd(self)
    }
}

impl<'a> MatrixBlockSliceMut<'a> {
    pub(super) fn new(limbs: *mut Limb, stride: NonZeroUsize) -> Self {
        Self {
            limbs,
            stride,
            _marker: std::marker::PhantomData,
        }
    }

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
            &mut *self.limbs.add(row * self.stride.get())
        }
    }

    /// Returns a mutable iterator over the 64 rows of this block.
    #[inline]
    pub fn iter_mut<'b>(&'b mut self) -> impl Iterator<Item = &'b mut Limb> + use<'a, 'b> {
        (0..64).map(move |i| unsafe {
            // SAFETY: Constructor guarantees 64 rows at stride intervals
            &mut *self.limbs.add(i * self.stride.get())
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
            .zip(block.iter())
            .for_each(|(dst, &src)| *dst = src);
    }
}

unsafe impl<'a> Send for MatrixBlockSlice<'a> {}
unsafe impl<'a> Send for MatrixBlockSliceMut<'a> {}

unsafe impl<'a> Sync for MatrixBlockSlice<'a> {}

/// Performs block-level GEMM: `C = alpha * A * B + beta * C` for 64 x 64 bit blocks.
///
/// # Arguments
///
/// * `alpha` - If `false`, the `A * B` term is skipped (for F_2, this is the only scaling)
/// * `a` - Left input block (64 x 64 bits)
/// * `b` - Right input block (64 x 64 bits)
/// * `beta` - If `false`, C is zeroed before accumulation
/// * `c` - Accumulator block (64 x 64 bits)
///
/// For efficiency reasons, we mutate `C` in-place.
///
/// # Implementation Selection
///
/// - **x86_64 with AVX-512**: Uses optimized assembly kernel
/// - **Other platforms**: Falls back to scalar implementation
#[inline]
pub fn gemm_block(alpha: bool, a: MatrixBlock, b: MatrixBlock, beta: bool, c: &mut MatrixBlock) {
    // Delegate to SIMD specializations
    crate::simd::gemm_block_simd(alpha, a, b, beta, c)
}

#[cfg(feature = "proptest")]
mod arbitrary {

    use proptest::prelude::*;

    use super::*;

    impl Arbitrary for MatrixBlock {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            proptest::array::uniform(any::<Limb>())
                .prop_map(Self)
                .boxed()
        }
    }
}
