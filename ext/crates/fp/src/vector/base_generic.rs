use super::{
    generic::{FpVectorIterator, FpVectorNonZeroIteratorP, FpVectorP, SliceMutP, SliceP},
    internal::{InternalBaseVectorMutP, InternalBaseVectorP},
};
use crate::{limb::LimbLength, prime::ValidPrime};

pub trait BaseVectorP<const P: u32>: InternalBaseVectorP<P> {
    fn prime(&self) -> ValidPrime;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn entry(&self, index: usize) -> u32;
    fn slice<'a>(&self, start: usize, end: usize) -> SliceP<'a, P>
    where
        Self: 'a;
    fn as_slice(&self) -> SliceP<P>;
    fn is_zero(&self) -> bool;
    fn iter(&self) -> FpVectorIterator;
    fn iter_nonzero(&self) -> FpVectorNonZeroIteratorP<P>;
    fn first_nonzero(&self) -> Option<(usize, u32)>;
    fn sign_rule<T: BaseVectorP<P>>(&self, other: T) -> bool;
    fn into_owned(self) -> FpVectorP<P>;
    fn density(&self) -> f32;
}

pub trait BaseVectorMutP<const P: u32>: InternalBaseVectorMutP<P> + BaseVectorP<P> {
    fn scale(&mut self, c: u32);
    fn set_to_zero(&mut self);
    fn set_entry(&mut self, index: usize, value: u32);
    fn assign<T: BaseVectorP<P>>(&mut self, other: T);
    fn add<T: BaseVectorP<P>>(&mut self, other: T, c: u32);
    fn add_offset<T: BaseVectorP<P>>(&mut self, other: T, c: u32, offset: usize);
    fn slice_mut(&mut self, start: usize, end: usize) -> SliceMutP<P>;
    fn as_slice_mut(&mut self) -> SliceMutP<P>;
    fn add_basis_element(&mut self, index: usize, value: u32);
    fn copy_from_slice(&mut self, slice: &[u32]);
    fn add_masked<T: BaseVectorP<P>>(&mut self, other: T, c: u32, mask: &[usize]);
    fn add_unmasked<T: BaseVectorP<P>>(&mut self, other: T, c: u32, mask: &[usize]);
    fn add_truncate<T: BaseVectorP<P>>(&mut self, other: T, c: u32) -> Option<()>;
}

impl<T: InternalBaseVectorP<P>, const P: u32> BaseVectorP<P> for T {
    fn prime(&self) -> ValidPrime {
        self._prime()
    }

    fn len(&self) -> usize {
        self._len().logical()
    }

    fn is_empty(&self) -> bool {
        self._is_empty()
    }

    fn entry(&self, index: usize) -> u32 {
        self._entry(index)
    }

    fn slice<'a>(&self, start: usize, end: usize) -> SliceP<'a, P>
    where
        Self: 'a,
    {
        self._slice(LimbLength::from_start_end(start, end))
    }

    fn as_slice(&self) -> SliceP<P> {
        self._as_slice()
    }

    fn is_zero(&self) -> bool {
        self._is_zero()
    }

    fn iter(&self) -> FpVectorIterator {
        self._iter()
    }

    fn iter_nonzero(&self) -> FpVectorNonZeroIteratorP<P> {
        self._iter_nonzero()
    }

    fn first_nonzero(&self) -> Option<(usize, u32)> {
        self._first_nonzero()
    }

    fn sign_rule<S: BaseVectorP<P>>(&self, other: S) -> bool {
        self._sign_rule(other)
    }

    fn into_owned(self) -> FpVectorP<P> {
        self._into_owned()
    }

    fn density(&self) -> f32 {
        self._density()
    }
}

impl<T: InternalBaseVectorMutP<P>, const P: u32> BaseVectorMutP<P> for T {
    fn scale(&mut self, c: u32) {
        self._scale(c)
    }

    fn set_to_zero(&mut self) {
        self._set_to_zero()
    }

    fn set_entry(&mut self, index: usize, value: u32) {
        self._set_entry(index, value)
    }

    fn assign<S: BaseVectorP<P>>(&mut self, other: S) {
        self._assign(other)
    }

    fn add<S: BaseVectorP<P>>(&mut self, other: S, c: u32) {
        self._add(other, c)
    }

    fn add_offset<S: BaseVectorP<P>>(&mut self, other: S, c: u32, offset: usize) {
        self._add_offset(other, c, offset)
    }

    fn slice_mut(&mut self, start: usize, end: usize) -> SliceMutP<P> {
        self._slice_mut(LimbLength::from_start_end(start, end))
    }

    fn as_slice_mut(&mut self) -> SliceMutP<P> {
        self._as_slice_mut()
    }

    fn add_basis_element(&mut self, index: usize, value: u32) {
        self._add_basis_element(index, value)
    }

    fn copy_from_slice(&mut self, slice: &[u32]) {
        self._copy_from_slice(slice)
    }

    fn add_masked<S: BaseVectorP<P>>(&mut self, other: S, c: u32, mask: &[usize]) {
        self._add_masked(other, c, mask)
    }

    fn add_unmasked<S: BaseVectorP<P>>(&mut self, other: S, c: u32, mask: &[usize]) {
        self._add_unmasked(other, c, mask)
    }

    fn add_truncate<S: BaseVectorP<P>>(&mut self, other: S, c: u32) -> Option<()> {
        self._add_truncate(other, c)
    }
}
