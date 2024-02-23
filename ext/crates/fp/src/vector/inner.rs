// This generates better llvm optimization
#![allow(clippy::int_plus_one)]

use crate::{field::Field, limb::Limb};

/// An `FpVectorP` is a vector over $\mathbb{F}_p$ for a fixed prime, implemented using const
/// generics. Due to limitations with const generics, we cannot constrain P to actually be a prime,
/// so we allow it to be any u32. However, most functions will panic if P is not a prime.
///
/// Interally, it packs entries of the vectors into limbs. However, this is an abstraction that
/// must not leave the `fp` library.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct FqVectorP<F: Field> {
    pub(super) fq: F,
    pub(super) len: usize,
    pub(super) limbs: Vec<Limb>,
}

/// A SliceP is a slice of an FpVectorP. This immutably borrows the vector and implements Copy
#[derive(Debug, Copy, Clone)]
pub struct SliceP<'a, F: Field> {
    pub(super) fq: F,
    pub(super) limbs: &'a [Limb],
    pub(super) start: usize,
    pub(super) end: usize,
}

/// A `SliceMutP` is a mutable slice of an `FpVectorP`. This mutably borrows the vector. Since it
/// is a mutable borrow, it cannot implement `Copy`. However, it has a [`SliceMutP::copy`] function
/// that imitates the reborrowing, that mutably borrows `SliceMutP` and returns a `SliceMutP` with
/// a shorter lifetime.
#[derive(Debug)]
pub struct SliceMutP<'a, F: Field> {
    pub(super) fq: F,
    pub(super) limbs: &'a mut [Limb],
    pub(super) start: usize,
    pub(super) end: usize,
}

// See impl_* for implementations
