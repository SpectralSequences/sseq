use super::{
    generic::{FpVectorIterator, FpVectorNonZeroIteratorP, FpVectorP, SliceMutP, SliceP},
    internal::{InternalBaseVectorMutP, InternalBaseVectorP},
};
use crate::{limb::LimbLength, prime::ValidPrime};

pub trait BaseVectorP<const P: u32>: InternalBaseVectorP<P> {
    /// The characteristic of the underlying field.
    fn prime(&self) -> ValidPrime;
    /// The length of the vector.
    fn len(&self) -> usize;
    /// Whether the vector is empty.
    fn is_empty(&self) -> bool;
    /// The entry at index `index`.
    fn entry(&self, index: usize) -> u32;
    /// A slice of the vector starting at entry `start` (inclusive) and ending at entry `end`
    /// (exclusive). This means that `v.slice(start, end).len() == end - start`.
    fn slice<'a>(&self, start: usize, end: usize) -> SliceP<'a, P>
    where
        Self: 'a;
    /// The vector itself as a slice.
    fn as_slice(&self) -> SliceP<P>;
    /// Whether the vector is zero.
    fn is_zero(&self) -> bool;
    /// An iterator over the entries in the vector.
    fn iter(&self) -> FpVectorIterator;
    /// An iterator over the *nonzero* entries in the vector.
    fn iter_nonzero(&self) -> FpVectorNonZeroIteratorP<P>;
    /// The position of the first nonzero entry in the vector, together with its value. This returns
    /// `None` if and only if the vector is zero.
    fn first_nonzero(&self) -> Option<(usize, u32)>;
    /// ???
    fn sign_rule<T: BaseVectorP<P>>(&self, other: T) -> bool;
    /// Copy the contents of the vector into an owned `FpVector`. If the vector is already an
    /// `FpVector`, this is a no-op.
    fn into_owned(self) -> FpVectorP<P>;
    /// The proportion of non-zero entries.
    fn density(&self) -> f32;
}

pub trait BaseVectorMutP<const P: u32>: InternalBaseVectorMutP<P> + BaseVectorP<P> {
    /// Scale all entries by a factor of `c`. It is assumed that `c < P`.
    fn scale(&mut self, c: u32);
    /// Set the vector to the zero vector.
    fn set_to_zero(&mut self);
    /// Set the entry at index `index` to the value `value`. It is assumed that `value < P`.
    fn set_entry(&mut self, index: usize, value: u32);
    /// Copy the contents of `other` into `self`. Both vectors must have the same length.
    fn assign<T: BaseVectorP<P>>(&mut self, other: T);
    /// Add `c` times `other` to `self`. Both `other` and `self` must have the same length, and we
    /// must have `c < P`.
    fn add<T: BaseVectorP<P>>(&mut self, other: T, c: u32);
    /// Add `c` times `other` to `self`, assuming that all entries of `self` with index less than
    /// `offset` are zero. Violating this assumption does not lead to a panic or UB, but is very
    /// likely to produce nonsensical results.
    fn add_offset<T: BaseVectorP<P>>(&mut self, other: T, c: u32, offset: usize);
    /// A mutable slice of the vector starting at entry `start` (inclusive) and ending at entry
    /// `end` (exclusive). This means that `v.slice_mut(start, end).len() == end - start`.
    fn slice_mut(&mut self, start: usize, end: usize) -> SliceMutP<P>;
    /// The vector itself as a mutable slice.
    fn as_slice_mut(&mut self) -> SliceMutP<P>;
    /// Add `value` to the entry at index `index`.
    fn add_basis_element(&mut self, index: usize, value: u32);
    /// Replace the contents of the vector with the contents of the slice. The two must have the
    /// same length.
    fn copy_from_slice(&mut self, slice: &[u32]);
    /// Given a mask v, add the `v[i]`th entry of `other` to the `i`th entry of `self`.
    fn add_masked<T: BaseVectorP<P>>(&mut self, other: T, c: u32, mask: &[usize]);
    /// Given a mask v, add the `i`th entry of `other` to the `v[i]`th entry of `self`.
    fn add_unmasked<T: BaseVectorP<P>>(&mut self, other: T, c: u32, mask: &[usize]);
    /// ???
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
