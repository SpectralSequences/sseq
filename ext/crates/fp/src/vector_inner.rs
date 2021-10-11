// This generates better llvm optimization
#![allow(clippy::int_plus_one)]

use crate::limb::{self, Limb};

use crate::constants::BITS_PER_LIMB;
use crate::prime::ValidPrime;
use std::cmp::Ordering;
use std::convert::TryInto;
use std::ops::Range;

use crate::simd;

/// An `FpVectorP` is a vector over $\mathbb{F}_p$ for a fixed prime, implemented using const
/// generics. Due to limitations with const generics, we cannot constrain P to actually be a prime,
/// so we allow it to be any u32. However, most functions will panic if P is not a prime.
///
/// Interally, it packs entries of the vectors into limbs. However, this is an abstraction that
/// must not leave the `fp` library.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct FpVectorP<const P: u32> {
    len: usize,
    limbs: Vec<Limb>,
}

/// A SliceP is a slice of an FpVectorP. This immutably borrows the vector and implements Copy
#[derive(Debug, Copy, Clone)]
pub struct SliceP<'a, const P: u32> {
    limbs: &'a [Limb],
    start: usize,
    end: usize,
}

/// A `SliceMutP` is a mutable slice of an `FpVectorP`. This mutably borrows the vector. Since it
/// is a mutable borrow, it cannot implement `Copy`. However, it has a [`SliceMutP::copy`] function
/// that imitates the reborrowing, that mutably borrows `SliceMutP` and returns a `SliceMutP` with
/// a shorter lifetime.
#[derive(Debug)]
pub struct SliceMutP<'a, const P: u32> {
    limbs: &'a mut [Limb],
    start: usize,
    end: usize,
}

impl<const P: u32> FpVectorP<P> {
    pub fn new_(len: usize) -> Self {
        let number_of_limbs = limb::number::<P>(len);
        Self {
            len,
            limbs: vec![0; number_of_limbs],
        }
    }

    pub fn new_with_capacity_(len: usize, capacity: usize) -> Self {
        let mut limbs = Vec::with_capacity(limb::number::<P>(capacity));
        limbs.resize(limb::number::<P>(len), 0);
        Self { len, limbs }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn prime(&self) -> ValidPrime {
        ValidPrime::new(P)
    }

    pub fn slice(&self, start: usize, end: usize) -> SliceP<'_, P> {
        assert!(start <= end && end <= self.len);
        SliceP {
            limbs: &self.limbs,
            start,
            end,
        }
    }

    pub fn slice_mut(&mut self, start: usize, end: usize) -> SliceMutP<'_, P> {
        assert!(start <= end && end <= self.len);
        SliceMutP {
            limbs: &mut self.limbs,
            start,
            end,
        }
    }

    #[inline]
    pub fn as_slice(&self) -> SliceP<'_, P> {
        self.into()
    }

    #[inline]
    pub fn as_slice_mut(&mut self) -> SliceMutP<'_, P> {
        self.into()
    }

    pub fn add_basis_element(&mut self, index: usize, value: u32) {
        self.as_slice_mut().add_basis_element(index, value);
    }

    pub fn entry(&self, index: usize) -> u32 {
        self.as_slice().entry(index)
    }

    pub fn set_entry(&mut self, index: usize, value: u32) {
        self.as_slice_mut().set_entry(index, value);
    }

    pub fn iter(&self) -> FpVectorIterator {
        self.as_slice().iter()
    }

    pub fn iter_nonzero(&self) -> FpVectorNonZeroIteratorP<'_, P> {
        self.as_slice().iter_nonzero()
    }

    pub fn set_to_zero(&mut self) {
        for limb in &mut self.limbs {
            *limb = 0;
        }
    }

    pub fn scale(&mut self, c: u32) {
        match P {
            2 => {
                if c == 0 {
                    self.set_to_zero()
                }
            }
            3 | 5 => {
                for limb in &mut self.limbs {
                    *limb = limb::reduce::<P>(*limb * c as Limb);
                }
            }
            _ => {
                for limb in &mut self.limbs {
                    *limb = limb::pack::<_, P>(limb::unpack::<P>(*limb).map(|x| (x * c) % P));
                }
            }
        }
    }

    /// Add `other` to `self` on the assumption that the first `offset` entries of `other` are
    /// empty.
    pub fn add_offset(&mut self, other: &FpVectorP<P>, c: u32, offset: usize) {
        assert_eq!(self.len(), other.len());
        let min_limb = offset / limb::entries_per_limb_const::<P>();
        if P == 2 {
            if c != 0 {
                simd::add_simd(&mut self.limbs, &other.limbs, min_limb);
            }
        } else {
            for (left, right) in self.limbs.iter_mut().zip(&other.limbs).skip(min_limb) {
                *left = limb::add::<P>(*left, *right, c);
            }
            for limb in &mut self.limbs[min_limb..] {
                *limb = limb::reduce::<P>(*limb);
            }
        }
    }

    /// Add `other` to `self` on the assumption that the first `offset` entries of `other` are
    /// empty.
    pub fn add_offset_nosimd(&mut self, other: &FpVectorP<P>, c: u32, offset: usize) {
        assert_eq!(self.len(), other.len());
        let min_limb = offset / limb::entries_per_limb_const::<P>();
        if P == 2 {
            if c != 0 {
                for i in 0..self.limbs.len() {
                    self.limbs[i] ^= other.limbs[i];
                }
            }
        } else {
            for (left, right) in self.limbs.iter_mut().zip(&other.limbs).skip(min_limb) {
                *left = limb::add::<P>(*left, *right, c);
            }
            for limb in &mut self.limbs[min_limb..] {
                *limb = limb::reduce::<P>(*limb);
            }
        }
    }

    pub fn add(&mut self, other: &FpVectorP<P>, c: u32) {
        self.add_offset(other, c, 0);
    }

    pub fn add_nosimd(&mut self, other: &FpVectorP<P>, c: u32) {
        self.add_offset_nosimd(other, c, 0);
    }

    pub fn assign(&mut self, other: &Self) {
        debug_assert_eq!(self.len(), other.len());
        self.limbs.copy_from_slice(&other.limbs)
    }

    /// A version of [`FpVectorP::assign`] that allows `other` to be shorter than `self`.
    pub fn assign_partial(&mut self, other: &Self) {
        debug_assert!(other.len() <= self.len());
        self.limbs[0..other.limbs.len()].copy_from_slice(&other.limbs);
        for limb in self.limbs[other.limbs.len()..].iter_mut() {
            *limb = 0;
        }
    }

    pub fn is_zero(&self) -> bool {
        self.limbs.iter().all(|&x| x == 0)
    }

    pub(crate) fn limbs(&self) -> &[Limb] {
        &self.limbs
    }

    pub(crate) fn limbs_mut(&mut self) -> &mut [Limb] {
        &mut self.limbs
    }

    /// This function ensures the length of the vector is at least `len`. See also
    /// `set_scratch_vector_size`.
    pub fn extend_len(&mut self, len: usize) {
        if self.len >= len {
            return;
        }
        self.len = len;
        self.limbs.resize(limb::number::<P>(len), 0);
    }

    /// This clears the vector and sets the length to `len`. This is useful for reusing
    /// allocations of temporary vectors.
    pub fn set_scratch_vector_size(&mut self, len: usize) {
        self.limbs.clear();
        self.limbs.resize(limb::number::<P>(len), 0);
        self.len = len;
    }

    /// This replaces the contents of the vector with the contents of the slice. The two must have
    /// the same length.
    pub fn copy_from_slice(&mut self, slice: &[u32]) {
        assert_eq!(self.len, slice.len());

        self.limbs.clear();
        self.limbs.extend(
            slice
                .chunks(limb::entries_per_limb_const::<P>())
                .map(|x| limb::pack::<_, P>(x.iter().copied())),
        );
    }

    /// Permanently remove the first `n` elements in the vector. `n` must be a multiple of
    /// the number of entries per limb
    pub(crate) fn trim_start(&mut self, n: usize) {
        assert!(n <= self.len);
        let entries_per = limb::entries_per_limb_const::<P>();
        assert_eq!(n % entries_per, 0);
        let num_limbs = n / entries_per;
        self.limbs.drain(0..num_limbs);
        self.len -= n;
    }

    pub fn sign_rule(&self, other: &Self) -> bool {
        assert_eq!(P, 2);
        let mut result = 0;
        for target_limb_idx in 0..self.limbs.len() {
            let target_limb = other.limbs[target_limb_idx];
            let source_limb = self.limbs[target_limb_idx];
            result ^= limb::sign_rule(target_limb, source_limb);
            if target_limb.count_ones() % 2 == 0 {
                continue;
            }
            for _ in 0..target_limb_idx {
                result ^= source_limb.count_ones() % 2;
            }
        }
        result == 1
    }

    pub fn add_truncate(&mut self, other: &Self, c: u32) -> Option<()> {
        for (left, right) in self.limbs.iter_mut().zip(&other.limbs) {
            *left = limb::add::<P>(*left, *right, c);
            *left = limb::truncate::<P>(*left)?;
        }
        Some(())
    }

    fn add_carry_limb<T>(&mut self, idx: usize, source: Limb, c: u32, rest: &mut [T]) -> bool
    where
        for<'a> &'a mut T: TryInto<&'a mut Self>,
    {
        if P == 2 {
            if c == 0 {
                return false;
            }
            let mut cur_vec = self;
            let mut carry = source;
            for carry_vec in rest.iter_mut() {
                let carry_vec = carry_vec
                    .try_into()
                    .ok()
                    .expect("rest vectors in add_carry must be of the same prime");
                let rem = cur_vec.limbs[idx] ^ carry;
                let quot = cur_vec.limbs[idx] & carry;
                cur_vec.limbs[idx] = rem;
                carry = quot;
                cur_vec = carry_vec;
                if quot == 0 {
                    return false;
                }
            }
            cur_vec.limbs[idx] ^= carry;
            true
        } else {
            unimplemented!()
        }
    }

    pub fn add_carry<T>(&mut self, other: &Self, c: u32, rest: &mut [T]) -> bool
    where
        for<'a> &'a mut T: TryInto<&'a mut Self>,
    {
        let mut result = false;
        for i in 0..self.limbs.len() {
            result |= self.add_carry_limb(i, other.limbs[i], c, rest);
        }
        result
    }

    /// Find the index and value of the first non-zero entry of the vector. `None` if the vector is zero.
    pub fn first_nonzero(&self) -> Option<(usize, u32)> {
        let entries_per_limb = limb::entries_per_limb_const::<P>();
        let bit_length = limb::bit_length_const::<P>();
        let bitmask = limb::bitmask::<P>();
        for (i, &limb) in self.limbs.iter().enumerate() {
            if limb == 0 {
                continue;
            }
            let index = limb.trailing_zeros() as usize / bit_length;
            return Some((
                i * entries_per_limb + index,
                ((limb >> (index * bit_length)) & bitmask) as u32,
            ));
        }
        None
    }
}

impl<'a, const P: u32> From<&'a FpVectorP<P>> for SliceP<'a, P> {
    fn from(v: &'a FpVectorP<P>) -> Self {
        v.slice(0, v.len)
    }
}

impl<'a, const P: u32> From<&'a mut FpVectorP<P>> for SliceMutP<'a, P> {
    fn from(v: &'a mut FpVectorP<P>) -> Self {
        v.slice_mut(0, v.len)
    }
}

impl<'a, const P: u32> SliceMutP<'a, P> {
    pub fn slice_mut(&mut self, start: usize, end: usize) -> SliceMutP<'_, P> {
        assert!(start <= end && end <= self.as_slice().len());

        SliceMutP {
            limbs: &mut *self.limbs,
            start: self.start + start,
            end: self.start + end,
        }
    }

    #[inline]
    pub fn as_slice(&self) -> SliceP<'_, P> {
        SliceP {
            limbs: &*self.limbs,
            start: self.start,
            end: self.end,
        }
    }

    /// Generates a version of itself with a shorter lifetime
    #[inline]
    pub fn copy(&mut self) -> SliceMutP<'_, P> {
        SliceMutP {
            limbs: self.limbs,
            start: self.start,
            end: self.end,
        }
    }
}

impl<'a, const P: u32> SliceP<'a, P> {
    pub fn slice(&self, start: usize, end: usize) -> SliceP<'_, P> {
        assert!(start <= end && end <= self.len());

        SliceP {
            limbs: self.limbs,
            start: self.start + start,
            end: self.start + end,
        }
    }

    /// Converts a slice to an owned FpVectorP. This is vastly more efficient if the start of the vector is aligned.
    pub fn to_owned(self) -> FpVectorP<P> {
        let mut new = FpVectorP::<P>::new_(self.len());
        if self.start % limb::entries_per_limb_const::<P>() == 0 {
            let limb_range = self.limb_range();
            new.limbs[0..limb_range.len()].copy_from_slice(&self.limbs[limb_range]);
            if !new.limbs.is_empty() {
                let len = new.limbs.len();
                new.limbs[len - 1] &= self.limb_masks().1;
            }
        } else {
            new.as_slice_mut().assign(self);
        }
        new
    }
}

// Public methods
impl<'a, const P: u32> SliceP<'a, P> {
    pub fn prime(&self) -> ValidPrime {
        ValidPrime::new(P)
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn entry(&self, index: usize) -> u32 {
        debug_assert!(
            index < self.len(),
            "Index {} too large, length of vector is only {}.",
            index,
            self.len()
        );
        let bit_mask = limb::bitmask::<P>();
        let limb_index = limb::limb_bit_index_pair::<P>(index + self.start);
        let mut result = self.limbs[limb_index.limb];
        result >>= limb_index.bit_index;
        result &= bit_mask;
        result as u32
    }

    /// TODO: implement prime 2 version
    pub fn iter(self) -> FpVectorIterator<'a> {
        FpVectorIterator::new(self)
    }

    pub fn iter_nonzero(self) -> FpVectorNonZeroIteratorP<'a, P> {
        FpVectorNonZeroIteratorP::new(self)
    }

    pub fn is_zero(&self) -> bool {
        let limb_range = self.limb_range();
        if limb_range.is_empty() {
            return true;
        }
        let (min_mask, max_mask) = self.limb_masks();
        if self.limbs[limb_range.start] & min_mask != 0 {
            return false;
        }

        let inner_range = self.limb_range_inner();
        if !inner_range.is_empty() && self.limbs[inner_range].iter().any(|&x| x != 0) {
            return false;
        }
        if self.limbs[limb_range.end - 1] & max_mask != 0 {
            return false;
        }
        true
    }
}

// Limb methods
impl<'a, const P: u32> SliceP<'a, P> {
    #[inline]
    fn offset(&self) -> usize {
        let bit_length = limb::bit_length_const::<P>();
        let entries_per_limb = limb::entries_per_limb_const::<P>();
        (self.start % entries_per_limb) * bit_length
    }

    #[inline]
    fn limb_range(&self) -> Range<usize> {
        limb::range::<P>(self.start, self.end)
    }

    /// This function underflows if `self.end == 0`, which happens if and only if we are taking a
    /// slice of width 0 at the start of an `FpVector`. This should be a very rare edge case.
    /// Dealing with the underflow properly would probably require using `saturating_sub` or
    /// something of that nature, and that has a nontrivial (10%) performance hit.
    #[inline]
    fn limb_range_inner(&self) -> Range<usize> {
        let range = self.limb_range();
        (range.start + 1)..(usize::max(range.start + 1, range.end - 1))
    }

    #[inline(always)]
    fn min_limb_mask(&self) -> Limb {
        !0 << self.offset()
    }

    #[inline(always)]
    fn max_limb_mask(&self) -> Limb {
        let num_entries = 1 + (self.end - 1) % limb::entries_per_limb_const::<P>();
        let bit_max = num_entries * limb::bit_length_const::<P>();

        (!0) >> (BITS_PER_LIMB - bit_max)
    }

    #[inline(always)]
    fn limb_masks(&self) -> (Limb, Limb) {
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

impl<'a, const P: u32> SliceMutP<'a, P> {
    pub fn prime(&self) -> ValidPrime {
        ValidPrime::new(P)
    }

    pub fn add_basis_element(&mut self, index: usize, value: u32) {
        if P == 2 {
            // Checking for value % 2 == 0 appears to be less performant
            let pair = limb::limb_bit_index_pair::<2>(index + self.start);
            self.limbs[pair.limb] ^= (value as Limb % 2) << pair.bit_index;
        } else {
            let mut x = self.as_slice().entry(index);
            x += value;
            x %= P;
            self.set_entry(index, x);
        }
    }

    pub fn set_entry(&mut self, index: usize, value: u32) {
        debug_assert!(index < self.as_slice().len());
        let bit_mask = limb::bitmask::<P>();
        let limb_index = limb::limb_bit_index_pair::<P>(index + self.start);
        let mut result = self.limbs[limb_index.limb];
        result &= !(bit_mask << limb_index.bit_index);
        result |= (value as Limb) << limb_index.bit_index;
        self.limbs[limb_index.limb] = result;
    }

    fn reduce_limbs(&mut self) {
        if P != 2 {
            let limb_range = self.as_slice().limb_range();

            for limb in &mut self.limbs[limb_range] {
                *limb = limb::reduce::<P>(*limb);
            }
        }
    }

    pub fn scale(&mut self, c: u32) {
        if P == 2 {
            if c == 0 {
                self.set_to_zero();
            }
            return;
        }

        let c = c as Limb;
        let limb_range = self.as_slice().limb_range();
        if limb_range.is_empty() {
            return;
        }
        let (min_mask, max_mask) = self.as_slice().limb_masks();

        let limb = self.limbs[limb_range.start];
        let masked_limb = limb & min_mask;
        let rest_limb = limb & !min_mask;
        self.limbs[limb_range.start] = (masked_limb * c) | rest_limb;

        let inner_range = self.as_slice().limb_range_inner();
        for limb in &mut self.limbs[inner_range] {
            *limb *= c;
        }
        if limb_range.len() > 1 {
            let full_limb = self.limbs[limb_range.end - 1];
            let masked_limb = full_limb & max_mask;
            let rest_limb = full_limb & !max_mask;
            self.limbs[limb_range.end - 1] = (masked_limb * c) | rest_limb;
        }
        self.reduce_limbs();
    }

    pub fn set_to_zero(&mut self) {
        let limb_range = self.as_slice().limb_range();
        if limb_range.is_empty() {
            return;
        }
        let (min_mask, max_mask) = self.as_slice().limb_masks();
        self.limbs[limb_range.start] &= !min_mask;

        let inner_range = self.as_slice().limb_range_inner();
        for limb in &mut self.limbs[inner_range] {
            *limb = 0;
        }
        self.limbs[limb_range.end - 1] &= !max_mask;
    }

    pub fn add(&mut self, other: SliceP<'_, P>, c: u32) {
        debug_assert!(c < P);
        if self.as_slice().is_empty() {
            return;
        }

        if P == 2 {
            if c != 0 {
                match self.as_slice().offset().cmp(&other.offset()) {
                    Ordering::Equal => self.add_shift_none(other, 1),
                    Ordering::Less => self.add_shift_left(other, 1),
                    Ordering::Greater => self.add_shift_right(other, 1),
                };
            }
        } else {
            match self.as_slice().offset().cmp(&other.offset()) {
                Ordering::Equal => self.add_shift_none(other, c),
                Ordering::Less => self.add_shift_left(other, c),
                Ordering::Greater => self.add_shift_right(other, c),
            };
        }
    }

    /// `coeff` need not be reduced mod p.
    /// Adds v otimes w to self.
    pub fn add_tensor(&mut self, offset: usize, coeff: u32, left: SliceP<P>, right: SliceP<P>) {
        let right_dim = right.len();

        for (i, v) in left.iter_nonzero() {
            let entry = (v * coeff) % *self.prime();
            self.slice_mut(offset + i * right_dim, offset + (i + 1) * right_dim)
                .add(right, entry);
        }
    }

    /// TODO: improve efficiency
    pub fn assign(&mut self, other: SliceP<'_, P>) {
        if self.as_slice().offset() != other.offset() {
            self.set_to_zero();
            self.add(other, 1);
            return;
        }
        let target_range = self.as_slice().limb_range();
        let source_range = other.limb_range();

        if target_range.is_empty() {
            return;
        }

        let (min_mask, max_mask) = other.limb_masks();

        let result = other.limbs[source_range.start] & min_mask;
        self.limbs[target_range.start] &= !min_mask;
        self.limbs[target_range.start] |= result;

        let target_inner_range = self.as_slice().limb_range_inner();
        let source_inner_range = other.limb_range_inner();
        if !target_inner_range.is_empty() && !source_inner_range.is_empty() {
            self.limbs[target_inner_range].clone_from_slice(&other.limbs[source_inner_range]);
        }

        let result = other.limbs[source_range.end - 1] & max_mask;
        self.limbs[target_range.end - 1] &= !max_mask;
        self.limbs[target_range.end - 1] |= result;
    }

    /// Adds `c` * `other` to `self`. `other` must have the same length, offset, and prime as self, and `c` must be between `0` and `p - 1`.
    pub fn add_shift_none(&mut self, other: SliceP<'_, P>, c: u32) {
        let target_range = self.as_slice().limb_range();
        let source_range = other.limb_range();

        let (min_mask, max_mask) = other.limb_masks();

        self.limbs[target_range.start] = limb::add::<P>(
            self.limbs[target_range.start],
            other.limbs[source_range.start] & min_mask,
            c,
        );
        self.limbs[target_range.start] = limb::reduce::<P>(self.limbs[target_range.start]);

        let target_inner_range = self.as_slice().limb_range_inner();
        let source_inner_range = other.limb_range_inner();
        if !source_inner_range.is_empty() {
            for (left, right) in self.limbs[target_inner_range]
                .iter_mut()
                .zip(&other.limbs[source_inner_range])
            {
                *left = limb::add::<P>(*left, *right, c);
                *left = limb::reduce::<P>(*left);
            }
        }
        if source_range.len() > 1 {
            // The first and last limbs are distinct, so we process the last.
            self.limbs[target_range.end - 1] = limb::add::<P>(
                self.limbs[target_range.end - 1],
                other.limbs[source_range.end - 1] & max_mask,
                c,
            );
            self.limbs[target_range.end - 1] = limb::reduce::<P>(self.limbs[target_range.end - 1]);
        }
    }

    fn add_shift_left(&mut self, other: SliceP<'_, P>, c: u32) {
        let dat = AddShiftLeftData::new(self.as_slice(), other);
        let mut i = 0;
        {
            self.limbs[i + dat.min_target_limb] = limb::add::<P>(
                self.limbs[i + dat.min_target_limb],
                dat.mask_first_limb(other, i + dat.min_source_limb),
                c,
            );
        }
        for i in 1..dat.number_of_source_limbs - 1 {
            self.limbs[i + dat.min_target_limb] = limb::add::<P>(
                self.limbs[i + dat.min_target_limb],
                dat.mask_middle_limb_a(other, i + dat.min_source_limb),
                c,
            );
            self.limbs[i + dat.min_target_limb - 1] = limb::add::<P>(
                self.limbs[i + dat.min_target_limb - 1],
                dat.mask_middle_limb_b(other, i + dat.min_source_limb),
                c,
            );
            self.limbs[i + dat.min_target_limb - 1] =
                limb::reduce::<P>(self.limbs[i + dat.min_target_limb - 1]);
        }
        i = dat.number_of_source_limbs - 1;
        if i > 0 {
            self.limbs[i + dat.min_target_limb - 1] = limb::add::<P>(
                self.limbs[i + dat.min_target_limb - 1],
                dat.mask_last_limb_a(other, i + dat.min_source_limb),
                c,
            );
            self.limbs[i + dat.min_target_limb - 1] =
                limb::reduce::<P>(self.limbs[i + dat.min_target_limb - 1]);
            if dat.number_of_source_limbs == dat.number_of_target_limbs {
                self.limbs[i + dat.min_target_limb] = limb::add::<P>(
                    self.limbs[i + dat.min_target_limb],
                    dat.mask_last_limb_b(other, i + dat.min_source_limb),
                    c,
                );
                self.limbs[i + dat.min_target_limb] =
                    limb::reduce::<P>(self.limbs[i + dat.min_target_limb]);
            }
        } else {
            self.limbs[i + dat.min_target_limb] =
                limb::reduce::<P>(self.limbs[i + dat.min_target_limb]);
        }
    }

    fn add_shift_right(&mut self, other: SliceP<'_, P>, c: u32) {
        let dat = AddShiftRightData::new(self.as_slice(), other);
        let mut i = 0;
        {
            self.limbs[i + dat.min_target_limb] = limb::add::<P>(
                self.limbs[i + dat.min_target_limb],
                dat.mask_first_limb_a(other, i + dat.min_source_limb),
                c,
            );
            self.limbs[i + dat.min_target_limb] =
                limb::reduce::<P>(self.limbs[i + dat.min_target_limb]);
            if dat.number_of_target_limbs > 1 {
                self.limbs[i + dat.min_target_limb + 1] = limb::add::<P>(
                    self.limbs[i + dat.min_target_limb + 1],
                    dat.mask_first_limb_b(other, i + dat.min_source_limb),
                    c,
                );
            }
        }
        for i in 1..dat.number_of_source_limbs - 1 {
            self.limbs[i + dat.min_target_limb] = limb::add::<P>(
                self.limbs[i + dat.min_target_limb],
                dat.mask_middle_limb_a(other, i + dat.min_source_limb),
                c,
            );
            self.limbs[i + dat.min_target_limb] =
                limb::reduce::<P>(self.limbs[i + dat.min_target_limb]);
            self.limbs[i + dat.min_target_limb + 1] = limb::add::<P>(
                self.limbs[i + dat.min_target_limb + 1],
                dat.mask_middle_limb_b(other, i + dat.min_source_limb),
                c,
            );
        }
        i = dat.number_of_source_limbs - 1;
        if i > 0 {
            self.limbs[i + dat.min_target_limb] = limb::add::<P>(
                self.limbs[i + dat.min_target_limb],
                dat.mask_last_limb_a(other, i + dat.min_source_limb),
                c,
            );
            self.limbs[i + dat.min_target_limb] =
                limb::reduce::<P>(self.limbs[i + dat.min_target_limb]);
            if dat.number_of_target_limbs > dat.number_of_source_limbs {
                self.limbs[i + dat.min_target_limb + 1] = limb::add::<P>(
                    self.limbs[i + dat.min_target_limb + 1],
                    dat.mask_last_limb_b(other, i + dat.min_source_limb),
                    c,
                );
            }
        }
        if dat.number_of_target_limbs > dat.number_of_source_limbs {
            self.limbs[i + dat.min_target_limb + 1] =
                limb::reduce::<P>(self.limbs[i + dat.min_target_limb + 1]);
        }
    }
}

struct AddShiftLeftData {
    offset_shift: usize,
    tail_shift: usize,
    zero_bits: usize,
    min_source_limb: usize,
    min_target_limb: usize,
    number_of_source_limbs: usize,
    number_of_target_limbs: usize,
    min_mask: Limb,
    max_mask: Limb,
}

impl AddShiftLeftData {
    fn new<const P: u32>(target: SliceP<'_, P>, source: SliceP<'_, P>) -> Self {
        debug_assert!(target.prime() == source.prime());
        debug_assert!(target.offset() <= source.offset());
        debug_assert!(
            target.len() == source.len(),
            "self.dim {} not equal to other.dim {}",
            target.len(),
            source.len()
        );
        let offset_shift = source.offset() - target.offset();
        let bit_length = limb::bit_length_const::<P>();
        let entries_per_limb = limb::entries_per_limb_const::<P>();
        let usable_bits_per_limb = bit_length * entries_per_limb;
        let tail_shift = usable_bits_per_limb - offset_shift;
        let zero_bits = BITS_PER_LIMB - usable_bits_per_limb;
        let source_range = source.limb_range();
        let target_range = target.limb_range();
        let min_source_limb = source_range.start;
        let min_target_limb = target_range.start;
        let number_of_source_limbs = source_range.len();
        let number_of_target_limbs = target_range.len();
        let (min_mask, max_mask) = source.limb_masks();

        Self {
            offset_shift,
            tail_shift,
            zero_bits,
            min_source_limb,
            min_target_limb,
            number_of_source_limbs,
            number_of_target_limbs,
            min_mask,
            max_mask,
        }
    }

    fn mask_first_limb<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
        (other.limbs[i] & self.min_mask) >> self.offset_shift
    }

    fn mask_middle_limb_a<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
        other.limbs[i] >> self.offset_shift
    }

    fn mask_middle_limb_b<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
        (other.limbs[i] << (self.tail_shift + self.zero_bits)) >> self.zero_bits
    }

    fn mask_last_limb_a<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
        let source_limb_masked = other.limbs[i] & self.max_mask;
        source_limb_masked << self.tail_shift
    }

    fn mask_last_limb_b<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
        let source_limb_masked = other.limbs[i] & self.max_mask;
        source_limb_masked >> self.offset_shift
    }
}

struct AddShiftRightData {
    offset_shift: usize,
    tail_shift: usize,
    zero_bits: usize,
    min_source_limb: usize,
    min_target_limb: usize,
    number_of_source_limbs: usize,
    number_of_target_limbs: usize,
    min_mask: Limb,
    max_mask: Limb,
}

impl AddShiftRightData {
    fn new<const P: u32>(target: SliceP<'_, P>, source: SliceP<'_, P>) -> Self {
        debug_assert!(target.prime() == source.prime());
        debug_assert!(target.offset() >= source.offset());
        debug_assert!(
            target.len() == source.len(),
            "self.dim {} not equal to other.dim {}",
            target.len(),
            source.len()
        );
        let offset_shift = target.offset() - source.offset();
        let bit_length = limb::bit_length_const::<P>();
        let entries_per_limb = limb::entries_per_limb_const::<P>();
        let usable_bits_per_limb = bit_length * entries_per_limb;
        let tail_shift = usable_bits_per_limb - offset_shift;
        let zero_bits = BITS_PER_LIMB - usable_bits_per_limb;
        let source_range = source.limb_range();
        let target_range = target.limb_range();
        let min_source_limb = source_range.start;
        let min_target_limb = target_range.start;
        let number_of_source_limbs = source_range.len();
        let number_of_target_limbs = target_range.len();
        let (min_mask, max_mask) = source.limb_masks();
        Self {
            offset_shift,
            tail_shift,
            zero_bits,
            min_source_limb,
            min_target_limb,
            number_of_source_limbs,
            number_of_target_limbs,
            min_mask,
            max_mask,
        }
    }

    fn mask_first_limb_a<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
        let source_limb_masked = other.limbs[i] & self.min_mask;
        (source_limb_masked << (self.offset_shift + self.zero_bits)) >> self.zero_bits
    }

    fn mask_first_limb_b<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
        let source_limb_masked = other.limbs[i] & self.min_mask;
        source_limb_masked >> self.tail_shift
    }

    fn mask_middle_limb_a<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
        (other.limbs[i] << (self.offset_shift + self.zero_bits)) >> self.zero_bits
    }

    fn mask_middle_limb_b<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
        other.limbs[i] >> self.tail_shift
    }

    fn mask_last_limb_a<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
        let source_limb_masked = other.limbs[i] & self.max_mask;
        source_limb_masked << self.offset_shift
    }

    fn mask_last_limb_b<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
        let source_limb_masked = other.limbs[i] & self.max_mask;
        source_limb_masked >> self.tail_shift
    }
}

impl<T: AsRef<[u32]>, const P: u32> From<&T> for FpVectorP<P> {
    fn from(slice: &T) -> Self {
        let mut v = Self::new_(slice.as_ref().len());
        v.limbs.clear();
        v.limbs.extend(
            slice
                .as_ref()
                .chunks(limb::entries_per_limb_const::<P>())
                .map(|x| limb::pack::<_, P>(x.iter().copied())),
        );
        v
    }
}

impl<const P: u32> From<&FpVectorP<P>> for Vec<u32> {
    fn from(vec: &FpVectorP<P>) -> Vec<u32> {
        vec.iter().collect()
    }
}

pub struct FpVectorIterator<'a> {
    limbs: &'a [Limb],
    bit_length: usize,
    bit_mask: Limb,
    entries_per_limb_m_1: usize,
    limb_index: usize,
    entries_left: usize,
    cur_limb: Limb,
    counter: usize,
}

impl<'a> FpVectorIterator<'a> {
    fn new<const P: u32>(vec: SliceP<'a, P>) -> Self {
        let counter = vec.len();
        let limbs = &vec.limbs;

        if counter == 0 {
            return Self {
                limbs,
                bit_length: 0,
                entries_per_limb_m_1: 0,
                bit_mask: 0,
                limb_index: 0,
                entries_left: 0,
                cur_limb: 0,
                counter,
            };
        }
        let pair = limb::limb_bit_index_pair::<P>(vec.start);

        let bit_length = limb::bit_length_const::<P>();
        let cur_limb = limbs[pair.limb] >> pair.bit_index;

        let entries_per_limb = limb::entries_per_limb_const::<P>();
        Self {
            limbs,
            bit_length,
            entries_per_limb_m_1: entries_per_limb - 1,
            bit_mask: limb::bitmask::<P>(),
            limb_index: pair.limb,
            entries_left: entries_per_limb - (vec.start % entries_per_limb),
            cur_limb,
            counter,
        }
    }

    pub fn skip_n(&mut self, mut n: usize) {
        if n >= self.counter {
            self.counter = 0;
            return;
        }
        let entries_per_limb = self.entries_per_limb_m_1 + 1;
        if n < self.entries_left {
            self.entries_left -= n;
            self.counter -= n;
            self.cur_limb >>= self.bit_length * n;
            return;
        }

        n -= self.entries_left;
        self.counter -= self.entries_left;
        self.entries_left = 0;

        let skip_limbs = n / entries_per_limb;
        self.limb_index += skip_limbs;
        self.counter -= skip_limbs * entries_per_limb;
        n -= skip_limbs * entries_per_limb;

        if n > 0 {
            self.entries_left = entries_per_limb - n;
            self.limb_index += 1;
            self.cur_limb = self.limbs[self.limb_index] >> (n * self.bit_length);
            self.counter -= n;
        }
    }
}

impl<'a> Iterator for FpVectorIterator<'a> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.counter == 0 {
            return None;
        } else if self.entries_left == 0 {
            self.limb_index += 1;
            self.cur_limb = self.limbs[self.limb_index];
            self.entries_left = self.entries_per_limb_m_1;
        } else {
            self.entries_left -= 1;
        }

        let result = (self.cur_limb & self.bit_mask) as u32;
        self.counter -= 1;
        self.cur_limb >>= self.bit_length;

        Some(result)
    }
}

impl<'a> ExactSizeIterator for FpVectorIterator<'a> {
    fn len(&self) -> usize {
        self.counter
    }
}

/// Iterator over non-zero entries of an FpVector. This is monomorphized over P for significant
/// performance gains.
pub struct FpVectorNonZeroIteratorP<'a, const P: u32> {
    limbs: &'a [Limb],
    limb_index: usize,
    cur_limb_entries_left: usize,
    cur_limb: Limb,
    idx: usize,
    dim: usize,
}

impl<'a, const P: u32> FpVectorNonZeroIteratorP<'a, P> {
    fn new(vec: SliceP<'a, P>) -> Self {
        let entries_per_limb = limb::entries_per_limb_const::<P>();

        let dim = vec.len();
        let limbs = vec.limbs;

        if dim == 0 {
            return Self {
                limbs,
                limb_index: 0,
                cur_limb_entries_left: 0,
                cur_limb: 0,
                idx: 0,
                dim: 0,
            };
        }
        let min_index = vec.start;
        let pair = limb::limb_bit_index_pair::<P>(min_index);
        let cur_limb = limbs[pair.limb] >> pair.bit_index;
        let cur_limb_entries_left = entries_per_limb - (min_index % entries_per_limb);
        Self {
            limbs,
            limb_index: pair.limb,
            cur_limb_entries_left,
            cur_limb,
            idx: 0,
            dim,
        }
    }
}

impl<'a, const P: u32> Iterator for FpVectorNonZeroIteratorP<'a, P> {
    type Item = (usize, u32);
    fn next(&mut self) -> Option<Self::Item> {
        let bit_length: usize = limb::bit_length_const::<P>();
        let bitmask: Limb = limb::bitmask::<P>();
        let entries_per_limb: usize = limb::entries_per_limb_const::<P>();
        loop {
            let bits_left = (self.cur_limb_entries_left * bit_length) as u32;
            #[allow(clippy::unnecessary_cast)]
            let tz_real = (self.cur_limb | (1 as Limb).checked_shl(bits_left as u32).unwrap_or(0))
                .trailing_zeros();
            let tz_rem = ((tz_real as u8) % (bit_length as u8)) as u32;
            let tz_div = ((tz_real as u8) / (bit_length as u8)) as u32;
            let tz = tz_real - tz_rem;
            self.idx += tz_div as usize;
            if self.idx >= self.dim {
                return None;
            }
            self.cur_limb_entries_left -= tz_div as usize;
            if self.cur_limb_entries_left == 0 {
                self.limb_index += 1;
                self.cur_limb_entries_left = entries_per_limb;
                self.cur_limb = self.limbs[self.limb_index];
                continue;
            }
            self.cur_limb >>= tz;
            if tz == 0 {
                break;
            }
        }
        let result = (self.idx, (self.cur_limb & bitmask) as u32);
        self.idx += 1;
        self.cur_limb_entries_left -= 1;
        self.cur_limb >>= bit_length;
        Some(result)
    }
}
