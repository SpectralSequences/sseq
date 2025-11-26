// This generates better llvm optimization
#![allow(clippy::int_plus_one)]

use crate::{field::Field, limb::Limb};

/// A vector over a finite field.
///
/// Interally, it packs entries of the vectors into limbs. However, this is an abstraction that must
/// not leave the `fp` library.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct FqVector<F: Field> {
    fq: F,
    len: usize,
    limbs: Vec<Limb>,
}

/// A slice of an `FqVector`.
///
/// This immutably borrows the vector and implements `Copy`.
#[derive(Debug, Copy, Clone)]
pub struct FqSlice<'a, F: Field> {
    fq: F,
    limbs: &'a [Limb],
    start: usize,
    end: usize,
}

/// A mutable slice of an `FqVector`.
///
/// This mutably borrows the vector. Since it is a mutable borrow, it cannot implement `Copy`.
/// However, it has a [`FqSliceMut::copy`] function that imitates the reborrowing, that mutably
/// borrows `FqSliceMut` and returns a `FqSliceMut` with a shorter lifetime.
#[derive(Debug)]
pub struct FqSliceMut<'a, F: Field> {
    fq: F,
    limbs: &'a mut [Limb],
    start: usize,
    end: usize,
}

// See impl_* for implementations

// Accessors

impl<F: Field> FqVector<F> {
    pub fn from_raw_parts(fq: F, len: usize, limbs: Vec<Limb>) -> Self {
        debug_assert_eq!(limbs.len(), fq.number(len));
        Self { fq, len, limbs }
    }

    pub fn fq(&self) -> F {
        self.fq
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub(super) fn limbs(&self) -> &[Limb] {
        &self.limbs
    }

    pub(super) fn limbs_mut(&mut self) -> &mut [Limb] {
        &mut self.limbs
    }

    pub(super) fn vec_mut(&mut self) -> &mut Vec<Limb> {
        &mut self.limbs
    }

    pub(super) fn len_mut(&mut self) -> &mut usize {
        &mut self.len
    }
}

impl<'a, F: Field> FqSlice<'a, F> {
    pub(super) fn new(fq: F, limbs: &'a [Limb], start: usize, end: usize) -> Self {
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

    pub(super) const fn start(&self) -> usize {
        self.start
    }

    pub(super) const fn end(&self) -> usize {
        self.end
    }

    pub(super) fn limbs(&self) -> &'a [Limb] {
        self.limbs
    }
}

impl<'a, F: Field> FqSliceMut<'a, F> {
    pub(super) fn new(fq: F, limbs: &'a mut [Limb], start: usize, end: usize) -> Self {
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

    pub(super) fn start(&self) -> usize {
        self.start
    }

    pub(super) fn end(&self) -> usize {
        self.end
    }

    pub(super) fn limbs(&self) -> &[Limb] {
        self.limbs
    }

    pub(super) fn limbs_mut(&mut self) -> &mut [Limb] {
        self.limbs
    }
}
