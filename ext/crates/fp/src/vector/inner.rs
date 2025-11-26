// This generates better llvm optimization
#![allow(clippy::int_plus_one)]

use std::{
    borrow::Cow,
    ops::{Deref, DerefMut, Range},
};

use crate::{
    field::{Field, element::FieldElement},
    limb::Limb,
    prime::{Prime, ValidPrime},
};

pub trait Repr: Deref<Target = [Limb]> {}

impl<T: Deref<Target = [Limb]>> Repr for T {}

pub trait ReprMut: DerefMut<Target = [Limb]> {}

impl<T: DerefMut<Target = [Limb]>> ReprMut for T {}

/// A vector over a finite field.
///
/// Interally, it packs entries of the vectors into limbs. However, this is an abstraction that must
/// not leave the `fp` library.
///
/// We are generic over a number of types to provide maximal flexibility:
/// - `A` determines whether the vector type is aligned, i.e. if it always starts on a limb
///   boundary. This allows a number of methods to use a fast path.
/// - `R` determines where the limbs are stored. An owned vector will own its limbs in a `Vec`, but
///   a (mutable) slice will hold a (mutable) reference, etc. This allows for more exotic storage
///   options such as `Cow` or `Arc`.
/// - `F` is the underlying field.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct FqVectorBase<const A: bool, R: Repr, F: Field> {
    fq: F,
    limbs: R,
    start: usize,
    end: usize,
}

pub type FqVector<F> = FqVectorBase<true, Vec<Limb>, F>;
pub type FqSlice<'a, F> = FqVectorBase<false, &'a [Limb], F>;
pub type FqSliceMut<'a, F> = FqVectorBase<false, &'a mut [Limb], F>;
pub type FqCow<'a, F> = FqVectorBase<false, Cow<'a, [Limb]>, F>;

// #[derive(Debug, Hash, Eq, PartialEq, Clone)]
// pub struct FqVector<F: Field> {
//     fq: F,
//     len: usize,
//     limbs: Vec<Limb>,
// }

// /// A slice of an `FqVector`.
// ///
// /// This immutably borrows the vector and implements `Copy`.
// #[derive(Debug, Copy, Clone)]
// pub struct FqSlice<'a, F: Field> {
//     fq: F,
//     limbs: &'a [Limb],
//     start: usize,
//     end: usize,
// }

// /// A mutable slice of an `FqVector`.
// ///
// /// This mutably borrows the vector. Since it is a mutable borrow, it cannot implement `Copy`.
// /// However, it has a [`FqSliceMut::copy`] function that imitates the reborrowing, that mutably
// /// borrows `FqSliceMut` and returns a `FqSliceMut` with a shorter lifetime.
// #[derive(Debug)]
// pub struct FqSliceMut<'a, F: Field> {
//     fq: F,
//     limbs: &'a mut [Limb],
//     start: usize,
//     end: usize,
// }

// See impl_* for implementations

// Accessors

impl<const A: bool, R: Repr, F: Field> FqVectorBase<A, R, F> {
    pub(super) fn _new(fq: F, limbs: R, start: usize, end: usize) -> Self {
        assert!(start <= end);
        if A {
            assert!(start.is_multiple_of(fq.entries_per_limb()));
        }

        Self {
            fq,
            limbs,
            start,
            end,
        }
    }

    pub fn fq(&self) -> F {
        self.fq
    }

    pub fn prime(&self) -> ValidPrime {
        self.fq().characteristic().to_dyn()
    }

    pub fn len(&self) -> usize {
        self.end() - self.start()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn slice(&self, start: usize, end: usize) -> FqSlice<'_, F> {
        assert!(start <= end && end <= self.len());

        FqSlice::_new(
            self.fq(),
            self.limbs(),
            self.start() + start,
            self.start() + end,
        )
    }

    pub fn is_zero(&self) -> bool {
        if A {
            return self.limbs().iter().all(|&x| x == 0);
        }

        let limb_range = self.limb_range();
        if limb_range.is_empty() {
            return true;
        }
        let (min_mask, max_mask) = self.limb_masks();
        if self.limbs()[limb_range.start] & min_mask != 0 {
            return false;
        }

        let inner_range = self.limb_range_inner();
        if !inner_range.is_empty() && self.limbs()[inner_range].iter().any(|&x| x != 0) {
            return false;
        }
        if self.limbs()[limb_range.end - 1] & max_mask != 0 {
            return false;
        }
        true
    }

    pub fn entry(&self, index: usize) -> FieldElement<F> {
        debug_assert!(
            index < self.len(),
            "Index {} too large, length of vector is only {}.",
            index,
            self.len()
        );
        let bit_mask = self.fq().bitmask();
        let limb_index = self.fq().limb_bit_index_pair(index + self.start());
        let mut result = self.limbs()[limb_index.limb];
        result >>= limb_index.bit_index;
        result &= bit_mask;
        self.fq().decode(result)
    }

    // Repr accessors

    pub(super) fn start(&self) -> usize {
        self.start
    }

    pub(super) fn end(&self) -> usize {
        self.end
    }

    pub(super) fn limbs(&self) -> &[Limb] {
        &self.limbs
    }

    // Limb methods

    #[inline]
    pub(super) fn offset(&self) -> usize {
        let bit_length = self.fq().bit_length();
        let entries_per_limb = self.fq().entries_per_limb();
        (self.start() % entries_per_limb) * bit_length
    }

    #[inline]
    pub(super) fn limb_range(&self) -> Range<usize> {
        self.fq().range(self.start(), self.end())
    }

    /// This function underflows if `self.end() == 0`, which happens if and only if we are taking a
    /// slice of width 0 at the start of an `FpVector`. This should be a very rare edge case.
    /// Dealing with the underflow properly would probably require using `saturating_sub` or
    /// something of that nature, and that has a nontrivial (10%) performance hit.
    #[inline]
    pub(super) fn limb_range_inner(&self) -> Range<usize> {
        let range = self.limb_range();
        (range.start + 1)..(usize::max(range.start + 1, range.end - 1))
    }

    #[inline(always)]
    pub(super) fn min_limb_mask(&self) -> Limb {
        !0 << self.offset()
    }

    #[inline(always)]
    pub(super) fn max_limb_mask(&self) -> Limb {
        let num_entries = 1 + (self.end() - 1) % self.fq().entries_per_limb();
        let bit_max = num_entries * self.fq().bit_length();

        (!0) >> (crate::constants::BITS_PER_LIMB - bit_max)
    }

    #[inline(always)]
    pub(super) fn limb_masks(&self) -> (Limb, Limb) {
        if self.limb_range().len() == 1 {
            (
                self.min_limb_mask() & self.max_limb_mask(),
                self.min_limb_mask() & self.max_limb_mask(),
            )
        } else {
            (self.min_limb_mask(), self.max_limb_mask())
        }
    }
}

impl<const A: bool, R: ReprMut, F: Field> FqVectorBase<A, R, F> {
    pub fn set_to_zero(&mut self) {
        if A {
            // This is sound because `fq.encode(fq.zero())` is always zero.
            for limb in self.limbs_mut() {
                *limb = 0;
            }
            return;
        }

        let limb_range = self.limb_range();
        if limb_range.is_empty() {
            return;
        }
        let (min_mask, max_mask) = self.limb_masks();
        self.limbs_mut()[limb_range.start] &= !min_mask;

        let inner_range = self.limb_range_inner();
        for limb in self.limbs_mut()[inner_range].iter_mut() {
            *limb = 0;
        }
        self.limbs_mut()[limb_range.end - 1] &= !max_mask;
    }

    pub fn set_entry(&mut self, index: usize, value: FieldElement<F>) {
        assert_eq!(self.fq(), value.field());
        assert!(index < self.len());

        let bit_mask = self.fq().bitmask();
        let limb_index = self.fq().limb_bit_index_pair(index + self.start());

        let mut result = self.limbs()[limb_index.limb];
        result &= !(bit_mask << limb_index.bit_index);
        result |= self.fq().encode(value) << limb_index.bit_index;
        self.limbs_mut()[limb_index.limb] = result;
    }

    pub fn add_basis_element(&mut self, index: usize, value: FieldElement<F>) {
        assert_eq!(self.fq(), value.field());
        if self.fq().q() == 2 {
            let pair = self.fq().limb_bit_index_pair(index + self.start());
            self.limbs_mut()[pair.limb] ^= self.fq().encode(value) << pair.bit_index;
        } else {
            let mut x = self.entry(index);
            x += value;
            self.set_entry(index, x);
        }
    }

    #[must_use]
    pub fn slice_mut(&mut self, start: usize, end: usize) -> FqSliceMut<'_, F> {
        assert!(start <= end && end <= self.len());
        let orig_start = self.start();

        FqSliceMut::_new(
            self.fq(),
            self.limbs_mut(),
            orig_start + start,
            orig_start + end,
        )
    }

    pub(super) fn limbs_mut(&mut self) -> &mut [Limb] {
        &mut *self.limbs
    }
}

impl<F: Field> FqVector<F> {
    pub fn from_raw_parts(fq: F, len: usize, limbs: Vec<Limb>) -> Self {
        debug_assert_eq!(limbs.len(), fq.number(len));

        Self::_new(fq, limbs, 0, len)
    }

    pub(super) fn vec_mut(&mut self) -> &mut Vec<Limb> {
        &mut self.limbs
    }

    pub(super) fn len_mut(&mut self) -> &mut usize {
        &mut self.end
    }
}

impl<'a, F: Field> FqSlice<'a, F> {
    pub(super) fn into_limbs(self) -> &'a [Limb] {
        self.limbs
    }
}

impl<'a, F: Field> FqSliceMut<'a, F> {
    pub(super) fn end_mut(&mut self) -> &mut usize {
        &mut self.end
    }
}
