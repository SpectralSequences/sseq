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

    #[cfg_attr(not(target_arch = "x86_64"), allow(dead_code))]
    pub(crate) fn limbs(&self) -> *const Limb {
        self.limbs
    }

    #[cfg_attr(not(target_arch = "x86_64"), allow(dead_code))]
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

    pub fn zero_out(&mut self) {
        for limb in self.iter_mut() {
            *limb = 0;
        }
    }
}

// SAFETY: The slices have &Limb / &mut Limb semantics, so inherit the same Send / Sync behavior.

unsafe impl Send for MatrixBlockSlice<'_> {}
unsafe impl Send for MatrixBlockSliceMut<'_> {}

unsafe impl Sync for MatrixBlockSlice<'_> {}

/// Performs block-level GEMM: `C = A * B + C` for 64 x 64 bit blocks.
///
/// # Arguments
///
/// * `a` - Left input block (64 x 64 bits)
/// * `b` - Right input block (64 x 64 bits)
/// * `c` - Accumulator block (64 x 64 bits)
///
/// For efficiency reasons, we mutate `C` in-place.
///
/// # Implementation Selection
///
/// - **x86_64 with AVX-512**: Uses optimized assembly kernel
/// - **Other platforms**: Falls back to scalar implementation
#[inline]
pub fn gemm_block(a: MatrixBlock, b: MatrixBlock, c: &mut MatrixBlock) {
    // Delegate to SIMD specializations
    crate::simd::gemm_block_simd(a, b, c)
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

/// Transpose the 64-square bit block held in each lane, all lanes at once.
///
/// `block[i][p]` is row `i` of lane `p`'s block; on return row `j` of lane `p` holds what was bit
/// `j` of each of that lane's rows.
///
/// The recursive delta swap of Hacker's Delight 7-3: each round exchanges the off-diagonal
/// quadrants of every sub-block at the current scale, so a full 64-square transpose costs six
/// masked passes rather than the 4096 single-bit extractions an entry-at-a-time transpose performs.
/// Every lane runs the same swap on the same row indices, so nothing moves between lanes and the
/// inner loop is a straight elementwise operation over `LANES` limbs — the shape a vector unit
/// wants, and the reason to transpose a panel of blocks together rather than one at a time.
#[inline]
pub fn transpose_lanes<const LANES: usize>(block: &mut [[Limb; LANES]; 64]) {
    let mut s = 32;
    // Selects the columns whose index has bit `s` clear: the left half of each 2s-wide group.
    let mut m: Limb = !0 >> 32;
    while s != 0 {
        let mut k = 0;
        while k < 64 {
            // `k` never has bit `s` set, so `k` and `k | s` are the upper and lower row halves of
            // one 2s-square block; this exchanges its upper-right and lower-left quadrants.
            let (upper, lower) = {
                let (a, b) = block.split_at_mut(k | s);
                (&mut a[k], &mut b[0])
            };
            for p in 0..LANES {
                let t = ((upper[p] >> s) ^ lower[p]) & m;
                lower[p] ^= t;
                upper[p] ^= t << s;
            }
            k = (k + s + 1) & !s;
        }
        s >>= 1;
        m ^= m << s;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The entry-at-a-time transpose, as an oracle for [`transpose_lanes`].
    fn naive_transpose(block: &[[Limb; 1]; 64]) -> [[Limb; 1]; 64] {
        let mut out = [[0]; 64];
        for (i, &[row]) in block.iter().enumerate() {
            for (j, [slot]) in out.iter_mut().enumerate() {
                *slot |= ((row >> j) & 1) << i;
            }
        }
        out
    }

    /// A xorshift keeps these deterministic without pulling `rand` into a unit test.
    fn xorshift(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }

    #[test]
    fn transpose_matches_naive() {
        let mut state: u64 = 0x243f_6a88_85a3_08d3;
        for _ in 0..64 {
            let mut limbs = [0; 64];
            for entry in &mut limbs {
                *entry = xorshift(&mut state);
            }
            let mut block = limbs.map(|limb| [limb]);
            let expected = naive_transpose(&block);
            transpose_lanes(&mut block);
            assert_eq!(block, expected);
        }
    }

    #[test]
    fn transpose_is_an_involution() {
        let mut state: u64 = 0x1319_8a2e_0370_7344;
        let mut limbs = [0; 64];
        for entry in &mut limbs {
            *entry = xorshift(&mut state);
        }
        let mut block = limbs.map(|limb| [limb]);
        let original = block;
        transpose_lanes(&mut block);
        transpose_lanes(&mut block);
        assert_eq!(block, original);
    }

    #[test]
    fn transpose_sends_single_bit_to_its_mirror() {
        for i in [0, 1, 17, 62, 63] {
            for j in [0, 5, 31, 63] {
                let mut limbs = [0; 64];
                limbs[i] = 1 << j;
                let mut block = limbs.map(|limb| [limb]);
                transpose_lanes(&mut block);
                let mut expected = [[0]; 64];
                expected[j] = [1 << i];
                assert_eq!(block, expected, "bit ({i}, {j})");
            }
        }
    }
}
