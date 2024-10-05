//! This module defines methods that assist in plumbing together methods that operate on `u32`s,
//! such as the ones on `FpVector`, with methods that operate on `FieldElement<Fp<P>>`s, such as the
//! ones on `FqVector`.
//!
//! The difficulty is that the field is necessary to make the conversion, through the
//! `FieldInternal::el` method, but depending on the field, the return type will be different. For
//! example, depending on whether an `FpVector` is over F2 or F3, the field element will be either
//! `FieldElement<Fp<P2>>` or `FieldElement<Fp<P3>>`. Therefore, it is the struct itself that needs
//! to make the conversion using its own field attribute.
//!
//! It would in theory be possible to just hide the conversion in a more complicated macro, but
//! because the `u32` arguments have various names and appear in different positions in the
//! signature of the methods, I suspect it would be a major hassle.

use itertools::Itertools;

use super::{FqSlice, FqSliceMut, FqVector, FqVectorIterator, FqVectorNonZeroIterator};
use crate::field::Field;

impl<F: Field> FqVector<F> {
    pub(super) fn scale_helper(&mut self, c: F::ElementContainer) {
        self.scale(self.fq.el(c))
    }

    pub(super) fn entry_helper(&self, index: usize) -> F::ElementContainer {
        self.entry(index).val()
    }

    pub(super) fn set_entry_helper(&mut self, index: usize, value: F::ElementContainer) {
        self.set_entry(index, self.fq.el(value))
    }

    pub(super) fn add_helper(&mut self, other: &Self, c: F::ElementContainer) {
        self.add(other, self.fq.el(c))
    }

    pub(super) fn add_offset_helper(
        &mut self,
        other: &Self,
        c: F::ElementContainer,
        offset: usize,
    ) {
        self.add_offset(other, self.fq.el(c), offset)
    }

    pub(super) fn add_basis_element_helper(&mut self, index: usize, value: F::ElementContainer) {
        self.add_basis_element(index, self.fq.el(value))
    }

    pub(super) fn copy_from_slice_helper(&mut self, other: &[F::ElementContainer]) {
        self.copy_from_slice(&other.iter().map(|x| self.fq.el(x.clone())).collect_vec())
    }

    pub(super) fn add_truncate_helper(
        &mut self,
        other: &Self,
        c: F::ElementContainer,
    ) -> Option<()> {
        self.add_truncate(other, self.fq.el(c))
    }

    pub(super) fn add_carry_helper<T>(
        &mut self,
        other: &Self,
        c: F::ElementContainer,
        rest: &mut [T],
    ) -> bool
    where
        for<'a> &'a mut T: TryInto<&'a mut Self>,
    {
        self.add_carry(other, self.fq.el(c), rest)
    }

    pub(super) fn first_nonzero_helper(&self) -> Option<(usize, F::ElementContainer)> {
        self.first_nonzero().map(|(idx, c)| (idx, c.val()))
    }
}

impl<F: Field> FqSlice<'_, F> {
    pub(super) fn entry_helper(&self, index: usize) -> F::ElementContainer {
        self.entry(index).val()
    }
}

impl<F: Field> FqSliceMut<'_, F> {
    pub(super) fn scale_helper(&mut self, c: F::ElementContainer) {
        self.scale(self.fq.el(c))
    }

    pub(super) fn add_helper(&mut self, other: FqSlice<F>, c: F::ElementContainer) {
        self.add(other, self.fq.el(c))
    }

    pub(super) fn set_entry_helper(&mut self, index: usize, value: F::ElementContainer) {
        self.set_entry(index, self.fq.el(value))
    }

    pub(super) fn add_basis_element_helper(&mut self, index: usize, value: F::ElementContainer) {
        self.add_basis_element(index, self.fq.el(value))
    }

    pub(super) fn add_masked_helper(
        &mut self,
        other: FqSlice<F>,
        c: F::ElementContainer,
        mask: &[usize],
    ) {
        self.add_masked(other, self.fq.el(c), mask)
    }

    pub(super) fn add_unmasked_helper(
        &mut self,
        other: FqSlice<F>,
        c: F::ElementContainer,
        mask: &[usize],
    ) {
        self.add_unmasked(other, self.fq.el(c), mask)
    }

    pub(super) fn add_tensor_helper(
        &mut self,
        offset: usize,
        coeff: F::ElementContainer,
        left: FqSlice<F>,
        right: FqSlice<F>,
    ) {
        self.add_tensor(offset, self.fq.el(coeff), left, right)
    }
}

impl<F: Field> FqVectorIterator<'_, F> {
    pub(super) fn next_helper(&mut self) -> Option<F::ElementContainer> {
        self.next().map(|x| x.val())
    }
}

impl<F: Field> FqVectorNonZeroIterator<'_, F> {
    pub(super) fn next_helper(&mut self) -> Option<(usize, F::ElementContainer)> {
        self.next().map(|x| (x.0, x.1.val()))
    }
}
