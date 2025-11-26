// This generates better llvm optimization
#![allow(clippy::int_plus_one)]

use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
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

    pub(super) fn start(&self) -> usize {
        self.start
    }

    pub(super) fn end(&self) -> usize {
        self.end
    }

    pub(super) fn limbs(&self) -> &[Limb] {
        &self.limbs
    }
}

impl<const A: bool, R: ReprMut, F: Field> FqVectorBase<A, R, F> {
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
