// This generates better llvm optimization
#![allow(clippy::int_plus_one)]

use crate::const_for;
use crate::prime::ValidPrime;
use crate::prime::{NUM_PRIMES, PRIMES, PRIME_TO_INDEX_MAP};
use std::cmp::Ordering;
use std::convert::TryInto;
use std::sync::Once;

use crate::simd;

pub(crate) type Limb = u64;
pub(crate) const BYTES_PER_LIMB: usize = std::mem::size_of::<Limb>();
pub(crate) const BITS_PER_LIMB: usize = 8 * BYTES_PER_LIMB;

pub const MAX_LEN: usize = 147500;

const BIT_LENGTHS: [usize; NUM_PRIMES] = {
    let mut result = [0; NUM_PRIMES];
    result[0] = 1;
    const_for! { i in 1 .. NUM_PRIMES {
        let p = PRIMES[i];
        result[i] = (32 - (p * (p - 1)).leading_zeros()) as usize;
    }};
    result
};

pub(crate) const fn bit_length(p: ValidPrime) -> usize {
    BIT_LENGTHS[PRIME_TO_INDEX_MAP[p.value() as usize]]
}

const BITMASKS: [Limb; NUM_PRIMES] = {
    let mut result = [0; NUM_PRIMES];
    const_for! { i in 0 .. NUM_PRIMES {
        result[i] = (1 << BIT_LENGTHS[i]) - 1;
    }};
    result
};

/// TODO: Would it be simpler to just compute this at "runtime"? It's going to be inlined anyway.
pub(crate) const fn bitmask(p: ValidPrime) -> Limb {
    BITMASKS[PRIME_TO_INDEX_MAP[p.value() as usize]]
}

const ENTRIES_PER_LIMB: [usize; NUM_PRIMES] = {
    let mut result = [0; NUM_PRIMES];
    const_for! { i in 0 .. NUM_PRIMES {
        result[i] = BITS_PER_LIMB / BIT_LENGTHS[i];
    }};
    result
};

pub(crate) const fn entries_per_limb(p: ValidPrime) -> usize {
    ENTRIES_PER_LIMB[PRIME_TO_INDEX_MAP[p.value() as usize]]
}

#[derive(Copy, Clone)]
struct LimbBitIndexPair {
    limb: usize,
    bit_index: usize,
}

/// This table tells us which limb and which bitfield of that limb to look for a given index of
/// the vector in.
static mut LIMB_BIT_INDEX_TABLE: [Option<Vec<LimbBitIndexPair>>; NUM_PRIMES] =
    [None, None, None, None, None, None, None, None];

static mut LIMB_BIT_INDEX_ONCE_TABLE: [Once; NUM_PRIMES] = [
    Once::new(),
    Once::new(),
    Once::new(),
    Once::new(),
    Once::new(),
    Once::new(),
    Once::new(),
    Once::new(),
];

pub fn initialize_limb_bit_index_table(p: ValidPrime) {
    if *p == 2 {
        return;
    }
    unsafe {
        LIMB_BIT_INDEX_ONCE_TABLE[PRIME_TO_INDEX_MAP[*p as usize]].call_once(|| {
            let entries_per_limb = entries_per_limb(p);
            let bit_length = bit_length(p);
            let mut table: Vec<LimbBitIndexPair> = Vec::with_capacity(MAX_LEN);
            for i in 0..MAX_LEN {
                table.push(LimbBitIndexPair {
                    limb: i / entries_per_limb,
                    bit_index: (i % entries_per_limb) * bit_length,
                })
            }
            LIMB_BIT_INDEX_TABLE[PRIME_TO_INDEX_MAP[*p as usize]] = Some(table);
        });
    }
}

fn limb_bit_index_pair(p: ValidPrime, idx: usize) -> LimbBitIndexPair {
    match *p {
        2 => LimbBitIndexPair {
            limb: idx / BITS_PER_LIMB,
            bit_index: idx % BITS_PER_LIMB,
        },
        _ => {
            let prime_idx = PRIME_TO_INDEX_MAP[*p as usize];
            debug_assert!(idx < MAX_LEN);
            unsafe {
                let table = &LIMB_BIT_INDEX_TABLE[prime_idx];
                debug_assert!(table.is_some());
                *table
                    .as_ref()
                    .unwrap_or_else(|| std::hint::unreachable_unchecked())
                    .get_unchecked(idx)
            }
        }
    }
}

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
        let min_limb = offset / entries_per_limb(self.prime());
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

    pub fn add(&mut self, other: &FpVectorP<P>, c: u32) {
        self.add_offset(other, c, 0);
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
                .chunks(entries_per_limb(ValidPrime::new(P)))
                .map(|x| limb::pack::<_, P>(x.iter().copied())),
        );
    }

    /// Permanently remove the first `n` elements in the vector. `n` must be a multiple of
    /// the number of entries per limb
    pub(crate) fn trim_start(&mut self, n: usize) {
        assert!(n <= self.len);
        let entries_per = entries_per_limb(ValidPrime::new(P));
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
        let entries_per_limb = entries_per_limb(self.prime());
        let bit_length = bit_length(self.prime());
        let bitmask = bitmask(self.prime());
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
            limbs: &mut self.limbs,
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
        if self.start % entries_per_limb(self.prime()) == 0 {
            let (min, max) = self.limb_range();
            new.limbs[0..(max - min)].copy_from_slice(&self.limbs[min..max]);
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

pub(crate) mod limb {
    use super::*;

    pub const fn add<const P: u32>(limb_a: Limb, limb_b: Limb, coeff: u32) -> Limb {
        if P == 2 {
            limb_a ^ (coeff as Limb * limb_b)
        } else {
            limb_a + (coeff as Limb) * limb_b
        }
    }

    /// Contbuted by Robert Burklund
    pub fn reduce<const P: u32>(limb: Limb) -> Limb {
        match P {
            2 => limb,
            3 => {
                // Set top bit to 1 in every limb
                const TOP_BIT: Limb = (!0 / 7) << (2 - BITS_PER_LIMB % 3);
                let mut limb_2 = ((limb & TOP_BIT) >> 2) + (limb & (!TOP_BIT));
                let mut limb_3s = limb_2 & (limb_2 >> 1);
                limb_3s |= limb_3s << 1;
                limb_2 ^= limb_3s;
                limb_2
            }
            5 => {
                // Set bottom bit to 1 in every limb
                const BOTTOM_BIT: Limb = (!0 / 31) >> (BITS_PER_LIMB % 5);
                const BOTTOM_TWO_BITS: Limb = BOTTOM_BIT | (BOTTOM_BIT << 1);
                const BOTTOM_THREE_BITS: Limb = BOTTOM_BIT | (BOTTOM_TWO_BITS << 1);
                let a = (limb >> 2) & BOTTOM_THREE_BITS;
                let b = limb & BOTTOM_TWO_BITS;
                let m = (BOTTOM_BIT << 3) - a + b;
                let mut c = (m >> 3) & BOTTOM_BIT;
                c |= c << 1;
                let d = m & BOTTOM_THREE_BITS;
                d + c - BOTTOM_TWO_BITS
            }
            _ => limb::pack::<_, P>(limb::unpack::<P>(limb).map(|x| x % P)),
        }
    }

    pub fn is_reduced<const P: u32>(limb: Limb) -> bool {
        limb == reduce::<P>(limb)
    }

    /// Given an interator of u32's, pack all of them into a single limb in order.
    /// It is assumed that
    ///  - The values of the iterator are less than P
    ///  - The values of the iterator fit into a single limb
    ///
    /// If these assumptions are violated, the result will be nonsense.
    pub fn pack<T: Iterator<Item = u32>, const P: u32>(entries: T) -> Limb {
        let p = ValidPrime::new(P);
        let bit_length = bit_length(p);
        let mut result: Limb = 0;
        let mut shift = 0;
        for entry in entries {
            result += (entry as Limb) << shift;
            shift += bit_length;
        }
        result
    }

    /// Give an iterator over the entries of a limb.
    pub fn unpack<const P: u32>(mut limb: Limb) -> impl Iterator<Item = u32> {
        let p = ValidPrime::new(P);
        let entries = entries_per_limb(ValidPrime::new(P));
        let bit_length = bit_length(p);
        let bit_mask = bitmask(p);

        (0..entries).map(move |_| {
            let result = (limb & bit_mask) as u32;
            limb >>= bit_length;
            result
        })
    }

    pub fn number<const P: u32>(dim: usize) -> usize {
        debug_assert!(dim < MAX_LEN);
        if dim == 0 {
            0
        } else {
            limb_bit_index_pair(ValidPrime::new(P), dim - 1).limb + 1
        }
    }

    pub fn range<const P: u32>(start: usize, end: usize) -> (usize, usize) {
        let p = ValidPrime::new(P);
        let min = limb_bit_index_pair(p, start).limb;
        let max = if end > 0 {
            limb_bit_index_pair(p, end - 1).limb + 1
        } else {
            0
        };
        (min, max)
    }

    pub fn sign_rule(mut target: Limb, mut source: Limb) -> u32 {
        let mut result = 0;
        let mut n = 1;
        // Empirically, the compiler unrolls this loop because BITS_PER_LIMB is a constant.
        while 2 * n < BITS_PER_LIMB {
            // This is 1 every 2n bits.
            let mask: Limb = !0 / ((1 << (2 * n)) - 1);
            result ^= (mask & (source >> n) & target).count_ones() % 2;
            source = source ^ (source >> n);
            target = target ^ (target >> n);
            n *= 2;
        }
        result ^= (1 & (source >> (BITS_PER_LIMB / 2)) & target) as u32;
        result
    }

    /// Returns: either Some(sum) if no carries happen in the limb or None if some carry does
    /// happen.
    pub fn truncate<const P: u32>(sum: Limb) -> Option<Limb> {
        if is_reduced::<P>(sum) {
            Some(sum)
        } else {
            None
        }
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
        let p = self.prime();
        let bit_mask = bitmask(p);
        let limb_index = limb_bit_index_pair(p, index + self.start);
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
        let (min_limb, max_limb) = self.limb_range();
        if min_limb == max_limb {
            return true;
        }
        let (min_mask, max_mask) = self.limb_masks();
        if self.limbs[min_limb] & min_mask != 0 {
            return false;
        }

        if max_limb - 1 >= min_limb + 1 {
            if self.limbs[min_limb + 1..max_limb - 1]
                .iter()
                .any(|&x| x != 0)
            {
                return false;
            }
            if self.limbs[max_limb - 1] & max_mask != 0 {
                return false;
            }
        }
        true
    }
}

// Limb methods
impl<'a, const P: u32> SliceP<'a, P> {
    #[inline]
    fn offset(&self) -> usize {
        let bit_length = bit_length(self.prime());
        let entries_per_limb = entries_per_limb(self.prime());
        (self.start % entries_per_limb) * bit_length
    }

    #[inline]
    fn limb_range(&self) -> (usize, usize) {
        limb::range::<P>(self.start, self.end)
    }

    #[inline(always)]
    fn min_limb_mask(&self) -> Limb {
        !0 << self.offset()
    }

    #[inline(always)]
    fn max_limb_mask(&self) -> Limb {
        let p = self.prime();
        let num_entries = 1 + (self.end - 1) % entries_per_limb(p);
        let bit_max = num_entries * bit_length(p);

        (!0) >> (64 - bit_max)
    }

    #[inline(always)]
    fn limb_masks(&self) -> (Limb, Limb) {
        let (min_limb, max_limb) = self.limb_range();
        if min_limb + 1 == max_limb {
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
            let pair = limb_bit_index_pair(ValidPrime::new(2), index + self.start);
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
        let p = self.prime();
        let bit_mask = bitmask(p);
        let limb_index = limb_bit_index_pair(p, index + self.start);
        let mut result = self.limbs[limb_index.limb];
        result &= !(bit_mask << limb_index.bit_index);
        result |= (value as Limb) << limb_index.bit_index;
        self.limbs[limb_index.limb] = result;
    }

    fn reduce_limbs(&mut self) {
        if P != 2 {
            let (min_limb, max_limb) = self.as_slice().limb_range();

            for limb in &mut self.limbs[min_limb..max_limb] {
                *limb = limb::reduce::<P>(*limb);
            }
        }
    }

    pub fn scale(&mut self, c: u32) {
        let c = c as Limb;
        let (min_limb, max_limb) = self.as_slice().limb_range();
        if min_limb == max_limb {
            return;
        }
        let (min_mask, max_mask) = self.as_slice().limb_masks();

        let limb = self.limbs[min_limb];
        let masked_limb = limb & min_mask;
        let rest_limb = limb & !min_mask;
        self.limbs[min_limb] = (masked_limb * c) | rest_limb;

        if max_limb - 1 >= min_limb + 1 {
            for limb in &mut self.limbs[min_limb + 1..max_limb - 1] {
                *limb *= c;
            }

            let full_limb = self.limbs[max_limb - 1];
            let masked_limb = full_limb & max_mask;
            let rest_limb = full_limb & !max_mask;
            self.limbs[max_limb - 1] = (masked_limb * c) | rest_limb;
        }
        self.reduce_limbs();
    }

    pub fn set_to_zero(&mut self) {
        let (min_limb, max_limb) = self.as_slice().limb_range();
        if min_limb == max_limb {
            return;
        }
        let (min_mask, max_mask) = self.as_slice().limb_masks();
        self.limbs[min_limb] &= !min_mask;
        if max_limb - 1 >= min_limb + 1 {
            for limb in &mut self.limbs[min_limb + 1..max_limb - 1] {
                *limb = 0;
            }
            self.limbs[max_limb - 1] &= !max_mask;
        }
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
        let (min_target_limb, max_target_limb) = self.as_slice().limb_range();
        let (min_source_limb, max_source_limb) = other.limb_range();

        if min_target_limb == max_target_limb {
            return;
        }

        let (min_mask, max_mask) = other.limb_masks();

        let result = other.limbs[min_source_limb] & min_mask;
        self.limbs[min_target_limb] &= !min_mask;
        self.limbs[min_target_limb] |= result;

        if max_source_limb - 1 >= min_source_limb + 1 {
            self.limbs[min_target_limb + 1..max_target_limb - 1]
                .clone_from_slice(&other.limbs[min_source_limb + 1..max_source_limb - 1]);

            let result = other.limbs[max_source_limb - 1] & max_mask;
            self.limbs[max_target_limb - 1] &= !max_mask;
            self.limbs[max_target_limb - 1] |= result;
        }
    }

    /// Adds `c` * `other` to `self`. `other` must have the same length, offset, and prime as self, and `c` must be between `0` and `p - 1`.
    pub fn add_shift_none(&mut self, other: SliceP<'_, P>, c: u32) {
        let (min_target_limb, max_target_limb) = self.as_slice().limb_range();
        let (min_source_limb, max_source_limb) = other.limb_range();

        let (min_mask, max_mask) = other.limb_masks();

        self.limbs[min_target_limb] = limb::add::<P>(
            self.limbs[min_target_limb],
            other.limbs[min_source_limb] & min_mask,
            c,
        );
        self.limbs[min_target_limb] = limb::reduce::<P>(self.limbs[min_target_limb]);

        if max_source_limb - 1 >= min_source_limb + 1 {
            for (left, right) in self.limbs[min_target_limb + 1..max_target_limb - 1]
                .iter_mut()
                .zip(&other.limbs[min_source_limb + 1..max_source_limb - 1])
            {
                *left = limb::add::<P>(*left, *right, c);
                *left = limb::reduce::<P>(*left);
            }

            self.limbs[max_target_limb - 1] = limb::add::<P>(
                self.limbs[max_target_limb - 1],
                other.limbs[max_source_limb - 1] & max_mask,
                c,
            );
            self.limbs[max_target_limb - 1] = limb::reduce::<P>(self.limbs[max_target_limb - 1]);
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
        let p = target.prime();
        let offset_shift = source.offset() - target.offset();
        let bit_length = bit_length(p);
        let entries_per_limb = entries_per_limb(p);
        let usable_bits_per_limb = bit_length * entries_per_limb;
        let tail_shift = usable_bits_per_limb - offset_shift;
        let zero_bits = BITS_PER_LIMB - usable_bits_per_limb;
        let (min_target_limb, max_target_limb) = target.limb_range();
        let (min_source_limb, max_source_limb) = source.limb_range();
        let number_of_source_limbs = max_source_limb - min_source_limb;
        let number_of_target_limbs = max_target_limb - min_target_limb;
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
        let p = target.prime();
        let offset_shift = target.offset() - source.offset();
        let bit_length = bit_length(p);
        let entries_per_limb = entries_per_limb(p);
        let usable_bits_per_limb = bit_length * entries_per_limb;
        let tail_shift = usable_bits_per_limb - offset_shift;
        let zero_bits = BITS_PER_LIMB - usable_bits_per_limb;
        let (min_target_limb, max_target_limb) = target.limb_range();
        let (min_source_limb, max_source_limb) = source.limb_range();
        let number_of_source_limbs = max_source_limb - min_source_limb;
        let number_of_target_limbs = max_target_limb - min_target_limb;
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
                .chunks(entries_per_limb(ValidPrime::new(P)))
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
        let p = vec.prime();

        let pair = limb_bit_index_pair(p, vec.start);

        let bit_length = bit_length(p);
        let cur_limb = limbs[pair.limb] >> pair.bit_index;

        let entries_per_limb = entries_per_limb(p);
        Self {
            limbs,
            bit_length,
            entries_per_limb_m_1: entries_per_limb - 1,
            bit_mask: bitmask(p),
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
        let entries_per_limb = entries_per_limb(ValidPrime::new(P));

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
        let pair = limb_bit_index_pair(vec.prime(), min_index);
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
        let bit_length: usize = bit_length(ValidPrime::new(P));
        let bitmask: Limb = bitmask(ValidPrime::new(P));
        let entries_per_limb: usize = entries_per_limb(ValidPrime::new(P));
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
