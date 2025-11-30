// This generates better llvm optimization
#![allow(clippy::int_plus_one)]

use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

use crate::{field::Field, limb::Limb};

pub trait Repr: Deref<Target = [Limb]> {}

impl<T: Deref<Target = [Limb]>> Repr for T {}

pub trait ReprMut: DerefMut<Target = [Limb]> {}

impl<T: DerefMut<Target = [Limb]>> ReprMut for T {}

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

// /// A vector over a finite field.
// ///
// /// Interally, it packs entries of the vectors into limbs. However, this is an abstraction that must
// /// not leave the `fp` library.
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

    pub(super) fn limbs(&self) -> &[Limb] {
        &self.limbs
    }
}

impl<const A: bool, R: ReprMut, F: Field> FqVectorBase<A, R, F> {
    pub(super) fn limbs_mut(&mut self) -> &mut [Limb] {
        &mut *self.limbs
    }
}

impl<F: Field> FqVector<F> {
    pub fn from_raw_parts(fq: F, len: usize, limbs: Vec<Limb>) -> Self {
        debug_assert_eq!(limbs.len(), fq.number(len));

        Self::_new(fq, limbs, 0, len)
    }

    pub const fn len(&self) -> usize {
        self.end
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

    pub(super) const fn start(&self) -> usize {
        self.start
    }

    pub(super) const fn end(&self) -> usize {
        self.end
    }
}

impl<'a, F: Field> FqSliceMut<'a, F> {
    pub(super) fn start(&self) -> usize {
        self.start
    }

    pub(super) fn end(&self) -> usize {
        self.end
    }

    pub(super) fn end_mut(&mut self) -> &mut usize {
        &mut self.end
    }
}
