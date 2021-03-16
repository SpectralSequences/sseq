#![allow(dead_code)]
use crate::prime::ValidPrime;
use crate::prime::NUM_PRIMES;
use crate::prime::PRIME_TO_INDEX_MAP;
use std::cmp::Ordering;
use std::sync::Once;

pub const MAX_DIMENSION: usize = 147500;

// Generated with Mathematica:
//     bitlengths = Prepend[#,1]&@ Ceiling[Log2[# (# - 1) + 1 &[Prime[Range[2, 54]]]]]
// But for 2 it should be 1.
const BIT_LENGHTS: [usize; NUM_PRIMES] = [1, 3, 5, 6, 7, 8, 9, 9];

pub const fn bit_length(p: ValidPrime) -> usize {
    BIT_LENGHTS[PRIME_TO_INDEX_MAP[p.value() as usize]]
}

// This is 2^bitlength - 1.
// Generated with Mathematica:
//     2^bitlengths-1
const BITMASKS: [u32; NUM_PRIMES] = [1, 7, 31, 63, 127, 255, 511, 511];

/// TODO: Would it be simpler to just compute this at "runtime"? It's going to be inlined anyway.
pub const fn bitmask(p: ValidPrime) -> u64 {
    BITMASKS[PRIME_TO_INDEX_MAP[p.value() as usize]] as u64
}

// This is floor(64/bitlength).
// Generated with Mathematica:
//      Floor[64/bitlengths]
const ENTRIES_PER_64_BITS: [usize; NUM_PRIMES] = [64, 21, 12, 10, 9, 8, 7, 7];

pub const fn entries_per_64_bits(p: ValidPrime) -> usize {
    ENTRIES_PER_64_BITS[PRIME_TO_INDEX_MAP[p.value() as usize]]
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
            let entries_per_limb = entries_per_64_bits(p);
            let bit_length = bit_length(p);
            let mut table: Vec<LimbBitIndexPair> = Vec::with_capacity(MAX_DIMENSION);
            for i in 0..MAX_DIMENSION {
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
            limb: idx / 64,
            bit_index: idx % 64,
        },
        _ => {
            let prime_idx = PRIME_TO_INDEX_MAP[*p as usize];
            debug_assert!(idx < MAX_DIMENSION);
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

#[derive(Eq, PartialEq, Clone)]
pub struct FpVectorP<const P: u32> {
    dimension: usize,
    limbs: Vec<u64>,
}

#[derive(Copy, Clone)]
pub struct SliceP<'a, const P: u32> {
    limbs: &'a [u64],
    start: usize,
    end: usize,
}

pub struct SliceMutP<'a, const P: u32> {
    limbs: &'a mut [u64],
    start: usize,
    end: usize,
}

impl<const P: u32> FpVectorP<P> {
    pub fn new(dim: usize) -> Self {
        let number_of_limbs = limb::number::<P>(dim);
        Self {
            dimension: dim,
            limbs: vec![0; number_of_limbs],
        }
    }

    pub(crate) fn from_limbs(dim: usize, limbs: Vec<u64>) -> Self {
        Self {
            dimension: dim,
            limbs,
        }
    }

    pub const fn dimension(&self) -> usize {
        self.dimension
    }

    pub const fn prime(&self) -> ValidPrime {
        ValidPrime::new(P)
    }

    pub fn slice(&self, start: usize, end: usize) -> SliceP<'_, P> {
        assert!(start <= end && end <= self.dimension);
        SliceP {
            limbs: &self.limbs,
            start,
            end,
        }
    }

    pub fn slice_mut(&mut self, start: usize, end: usize) -> SliceMutP<'_, P> {
        assert!(start <= end && end <= self.dimension);
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

    pub fn iter_nonzero(&self) -> FpVectorNonZeroIterator {
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
                    *limb = limb::reduce::<P>(*limb * c as u64);
                }
            }
            _ => {
                let entries = entries_per_64_bits(ValidPrime::new(P));
                for limb in &mut self.limbs {
                    *limb =
                        limb::pack::<_, P>(limb::unpack::<P>(entries, *limb).map(|x| (x * c) % P));
                }
            }
        }
    }

    pub fn add(&mut self, other: &FpVectorP<P>, c: u32) {
        debug_assert_eq!(self.dimension(), other.dimension());
        for (left, right) in self.limbs.iter_mut().zip(other.limbs.iter()) {
            *left = limb::add::<P>(*left, *right, c);
        }
        self.reduce_limbs();
    }

    pub fn assign(&mut self, other: &Self) {
        debug_assert_eq!(self.dimension(), other.dimension());
        for (left, right) in self.limbs.iter_mut().zip(other.limbs.iter()) {
            *left = *right;
        }
    }

    pub fn is_zero(&self) -> bool {
        self.limbs.iter().all(|&x| x == 0)
    }

    fn reduce_limbs(&mut self) {
        if P != 2 {
            for limb in &mut self.limbs {
                *limb = limb::reduce::<P>(*limb);
            }
        }
    }

    pub(crate) fn limbs(&self) -> &[u64] {
        &self.limbs
    }

    /// This function ensures the dimension of the vector is at least `dim`. See also
    /// `set_scratch_vector_size`.
    pub fn extend_dimension(&mut self, dim: usize) {
        if self.dimension >= dim {
            return;
        }
        self.dimension = dim;
        self.limbs.resize(limb::number::<P>(dim), 0);
    }

    /// This clears the vector and sets the dimension to dim. This is useful for reusing
    /// allocations of temporary vectors.
    pub fn set_scratch_vector_size(&mut self, dim: usize) {
        self.limbs.clear();
        self.limbs.resize(limb::number::<P>(dim), 0);
    }

    /// This replaces the contents of the vector with the contents of the slice. The two must have
    /// the same length.
    pub fn copy_from_slice(&mut self, slice: &[u32]) {
        assert_eq!(self.dimension, slice.len());

        self.limbs.clear();
        self.limbs.extend(slice
                .chunks(entries_per_64_bits(ValidPrime::new(P)))
                .map(|x| limb::pack::<_, P>(x.iter().copied()))
                );
    }
}

impl<'a, const P: u32> From<&'a FpVectorP<P>> for SliceP<'a, P> {
    fn from(v: &'a FpVectorP<P>) -> Self {
        v.slice(0, v.dimension)
    }
}

impl<'a, const P: u32> From<&'a mut FpVectorP<P>> for SliceMutP<'a, P> {
    fn from(v: &'a mut FpVectorP<P>) -> Self {
        v.slice_mut(0, v.dimension)
    }
}

impl<'a, const P: u32> SliceMutP<'a, P> {
    pub fn slice_mut(&mut self, start: usize, end: usize) -> SliceMutP<'_, P> {
        assert!(start <= end && end <= self.as_slice().dimension());

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
}

impl<'a, const P: u32> SliceP<'a, P> {
    pub fn slice(&self, start: usize, end: usize) -> SliceP<'_, P> {
        assert!(start <= end && end <= self.dimension());

        SliceP {
            limbs: &*self.limbs,
            start: self.start + start,
            end: self.start + end,
        }
    }
}

mod limb {
    use super::*;

    pub const fn add<const P: u32>(limb_a: u64, limb_b: u64, coeff: u32) -> u64 {
        if P == 2 {
            limb_a ^ (coeff as u64 * limb_b)
        } else {
            limb_a + (coeff as u64) * limb_b
        }
    }

    /// Contbuted by Robert Burklund
    pub fn reduce<const P: u32>(limb: u64) -> u64 {
        match P {
            2 => limb,
            3 => {
                let top_bit = 0x4924924924924924u64;
                let mut limb_2 = ((limb & top_bit) >> 2) + (limb & (!top_bit));
                let mut limb_3s = limb_2 & (limb_2 >> 1);
                limb_3s |= limb_3s << 1;
                limb_2 ^= limb_3s;
                limb_2
            }
            5 => {
                let bottom_bit = 0x84210842108421u64;
                let bottom_two_bits = bottom_bit | (bottom_bit << 1);
                let bottom_three_bits = bottom_bit | (bottom_two_bits << 1);
                let a = (limb >> 2) & bottom_three_bits;
                let b = limb & bottom_two_bits;
                let m = (bottom_bit << 3) - a + b;
                let mut c = (m >> 3) & bottom_bit;
                c |= c << 1;
                let d = m & bottom_three_bits;
                d + c - bottom_two_bits
            }
            _ => {
                let entries = entries_per_64_bits(ValidPrime::new(P));
                limb::pack::<_, P>(limb::unpack::<P>(entries, limb).map(|x| x % P))
            }
        }
    }

    pub fn is_reduced<const P: u32>(limb: u64) -> bool {
        limb == reduce::<P>(limb)
    }

    /// Given an interator of u32's, pack all of them into a single limb in order.
    /// It is assumed that
    ///  - The values of the iterator are less than P
    ///  - The values of the iterator fit into a single limb
    ///
    /// If these assumptions are violated, the result will be nonsense.
    pub fn pack<T: Iterator<Item = u32>, const P: u32>(entries: T) -> u64 {
        let p = ValidPrime::new(P);
        let bit_length = bit_length(p);
        let mut result: u64 = 0;
        let mut shift = 0;
        for entry in entries {
            result += (entry as u64) << shift;
            shift += bit_length;
        }
        result
    }

    /// Given a limb, return the first `dim` entries. It is assumed that
    /// `dim` is not greater than the number of entries in a limb.
    pub fn unpack<const P: u32>(dim: usize, mut limb: u64) -> impl Iterator<Item = u32> {
        let p = ValidPrime::new(P);
        let bit_length = bit_length(p);
        let bit_mask = bitmask(p);

        (0..dim).map(move |_| {
            let result = (limb & bit_mask) as u32;
            limb >>= bit_length;
            result
        })
    }

    pub fn number<const P: u32>(dim: usize) -> usize {
        debug_assert!(dim < MAX_DIMENSION);
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
}

// Public methods
impl<'a, const P: u32> SliceP<'a, P> {
    pub fn prime(&self) -> ValidPrime {
        ValidPrime::new(P)
    }

    pub fn dimension(&self) -> usize {
        self.end - self.start
    }

    pub fn entry(&self, index: usize) -> u32 {
        debug_assert!(
            index < self.dimension(),
            "Index {} too large, dimension of vector is only {}.",
            index,
            self.dimension()
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

    /// TODO: We need a custom implementation for dispatch
    pub fn iter_nonzero(self) -> FpVectorNonZeroIterator<'a> {
        FpVectorNonZeroIterator::new(self)
    }

    pub fn is_zero(&self) -> bool {
        let (min_limb, max_limb) = self.limb_range();
        if min_limb == max_limb {
            return true;
        }
        if self.limbs[min_limb] & self.limb_mask(min_limb) != 0 {
            return false;
        }

        if max_limb > min_limb + 1 {
            if self.limbs[min_limb + 1..max_limb - 1]
                .iter()
                .any(|&x| x != 0)
            {
                return false;
            }
            if self.limbs[max_limb - 1] & self.limb_mask(max_limb - 1) != 0 {
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
        let entries_per_64_bits = entries_per_64_bits(self.prime());
        (self.start % entries_per_64_bits) * bit_length
    }

    #[inline]
    fn limb_range(&self) -> (usize, usize) {
        limb::range::<P>(self.start, self.end)
    }

    #[inline(always)]
    fn limb_mask(&self, limb_idx: usize) -> u64 {
        let offset = self.offset();
        let (min_limb, max_limb) = self.limb_range();
        let mut mask = !0;
        if limb_idx == min_limb {
            mask <<= offset;
        }
        if limb_idx + 1 == max_limb {
            let p = self.prime();
            let num_entries = 1 + (self.end - 1) % entries_per_64_bits(p);
            let bit_max = num_entries * bit_length(p);

            mask &= (!0) >> (64 - bit_max);
        }
        mask
    }
}

impl<'a, const P: u32> SliceMutP<'a, P> {
    pub fn prime(&self) -> ValidPrime {
        ValidPrime::new(P)
    }

    pub fn add_basis_element(&mut self, index: usize, value: u32) {
        let mut x = self.as_slice().entry(index);
        x += value;
        x %= P;
        self.set_entry(index, x);
    }

    pub fn set_entry(&mut self, index: usize, value: u32) {
        debug_assert!(index < self.as_slice().dimension());
        let p = self.prime();
        let bit_mask = bitmask(p);
        let limb_index = limb_bit_index_pair(p, index + self.start);
        let mut result = self.limbs[limb_index.limb];
        result &= !(bit_mask << limb_index.bit_index);
        result |= (value as u64) << limb_index.bit_index;
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
        let c = c as u64;
        let (min_limb, max_limb) = self.as_slice().limb_range();
        if min_limb == max_limb {
            return;
        }
        // min_limb
        {
            let mask = self.as_slice().limb_mask(min_limb);
            let limb = self.limbs[min_limb];
            let masked_limb = limb & mask;
            let rest_limb = limb & !mask;
            self.limbs[min_limb] = (masked_limb * c) | rest_limb;
        }
        // remaining limbs
        if max_limb > min_limb + 1 {
            for limb in &mut self.limbs[min_limb + 1..max_limb - 1] {
                *limb *= c;
            }

            let mask = self.as_slice().limb_mask(max_limb - 1);
            let full_limb = self.limbs[max_limb - 1];
            let masked_limb = full_limb & mask;
            let rest_limb = full_limb & !mask;
            self.limbs[max_limb - 1] = (masked_limb * c) | rest_limb;
        }
        self.reduce_limbs();
    }

    pub fn set_to_zero(&mut self) {
        let (min_limb, max_limb) = self.as_slice().limb_range();
        if min_limb == max_limb {
            return;
        }
        self.limbs[min_limb] &= !self.as_slice().limb_mask(min_limb);
        if max_limb > min_limb + 1 {
            for limb in &mut self.limbs[min_limb + 1..max_limb - 1] {
                *limb = 0;
            }
            self.limbs[max_limb - 1] &= !self.as_slice().limb_mask(max_limb - 1);
        }
    }

    pub fn add(&mut self, other: SliceP<'_, P>, c: u32) {
        debug_assert!(c < P);
        if self.as_slice().dimension() == 0 {
            return;
        }

        match self.as_slice().offset().cmp(&other.offset()) {
            Ordering::Equal => self.add_shift_none(other, c),
            Ordering::Less => self.add_shift_left(other, c),
            Ordering::Greater => self.add_shift_right(other, c),
        };
    }

    /// TODO: improve efficiency
    pub fn assign(&mut self, other: SliceP<'_, P>) {
        self.set_to_zero();
        self.add(other, 1);
    }

    /// Adds `c` * `other` to `self`. `other` must have the same length, offset, and prime as self, and `c` must be between `0` and `p - 1`.
    pub fn add_shift_none(&mut self, other: SliceP<'_, P>, c: u32) {
        let dat = AddShiftNoneData::new(self.as_slice(), other);
        let mut i = 0;
        {
            self.limbs[i + dat.min_target_limb] = limb::add::<P>(
                self.limbs[i + dat.min_target_limb],
                dat.mask_first_limb(other, i + dat.min_source_limb),
                c,
            );
            self.limbs[i + dat.min_target_limb] =
                limb::reduce::<P>(self.limbs[i + dat.min_target_limb]);
        }
        for i in 1..dat.number_of_limbs - 1 {
            self.limbs[i + dat.min_target_limb] = limb::add::<P>(
                self.limbs[i + dat.min_target_limb],
                dat.mask_middle_limb(other, i + dat.min_source_limb),
                c,
            );
            self.limbs[i + dat.min_target_limb] =
                limb::reduce::<P>(self.limbs[i + dat.min_target_limb]);
        }
        i = dat.number_of_limbs - 1;
        if i > 0 {
            self.limbs[i + dat.min_target_limb] = limb::add::<P>(
                self.limbs[i + dat.min_target_limb],
                dat.mask_last_limb(other, i + dat.min_source_limb),
                c,
            );
            self.limbs[i + dat.min_target_limb] =
                limb::reduce::<P>(self.limbs[i + dat.min_target_limb]);
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
    }
}

struct AddShiftNoneData {
    min_source_limb: usize,
    min_target_limb: usize,
    number_of_limbs: usize,
}

impl AddShiftNoneData {
    fn new<const P: u32>(target: SliceP<'_, P>, source: SliceP<'_, P>) -> Self {
        debug_assert_eq!(target.prime(), source.prime());
        debug_assert_eq!(target.offset(), source.offset());
        debug_assert_eq!(
            target.dimension(),
            source.dimension(),
            "Adding vectors of different dimensions"
        );
        let (min_target_limb, max_target_limb) = target.limb_range();
        let (min_source_limb, max_source_limb) = source.limb_range();
        debug_assert!(max_source_limb - min_source_limb == max_target_limb - min_target_limb);
        let number_of_limbs = max_source_limb - min_source_limb;
        Self {
            min_target_limb,
            min_source_limb,
            number_of_limbs,
        }
    }

    fn mask_first_limb<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> u64 {
        other.limbs[i] & other.limb_mask(i)
    }

    fn mask_middle_limb<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> u64 {
        other.limbs[i]
    }

    fn mask_last_limb<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> u64 {
        other.limbs[i] & other.limb_mask(i)
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
}

impl AddShiftLeftData {
    fn new<const P: u32>(target: SliceP<'_, P>, source: SliceP<'_, P>) -> Self {
        debug_assert!(target.prime() == source.prime());
        debug_assert!(target.offset() <= source.offset());
        debug_assert!(
            target.dimension() == source.dimension(),
            "self.dim {} not equal to other.dim {}",
            target.dimension(),
            source.dimension()
        );
        let p = target.prime();
        let offset_shift = source.offset() - target.offset();
        let bit_length = bit_length(p);
        let entries_per_64_bits = entries_per_64_bits(p);
        let usable_bits_per_limb = bit_length * entries_per_64_bits;
        let tail_shift = usable_bits_per_limb - offset_shift;
        let zero_bits = 64 - usable_bits_per_limb;
        let (min_target_limb, max_target_limb) = target.limb_range();
        let (min_source_limb, max_source_limb) = source.limb_range();
        let number_of_source_limbs = max_source_limb - min_source_limb;
        let number_of_target_limbs = max_target_limb - min_target_limb;

        Self {
            offset_shift,
            tail_shift,
            zero_bits,
            min_source_limb,
            min_target_limb,
            number_of_source_limbs,
            number_of_target_limbs,
        }
    }

    fn mask_first_limb<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> u64 {
        (other.limbs[self.min_source_limb + i] & other.limb_mask(i)) >> self.offset_shift
    }

    fn mask_middle_limb_a<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> u64 {
        other.limbs[i + self.min_source_limb] >> self.offset_shift
    }

    fn mask_middle_limb_b<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> u64 {
        (other.limbs[i + self.min_source_limb] << (self.tail_shift + self.zero_bits))
            >> self.zero_bits
    }

    fn mask_last_limb_a<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> u64 {
        let mask = other.limb_mask(i);
        let source_limb_masked = other.limbs[self.min_source_limb + i] & mask;
        source_limb_masked << self.tail_shift
    }

    fn mask_last_limb_b<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> u64 {
        let mask = other.limb_mask(i);
        let source_limb_masked = other.limbs[self.min_source_limb + i] & mask;
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
}

impl AddShiftRightData {
    fn new<const P: u32>(target: SliceP<'_, P>, source: SliceP<'_, P>) -> Self {
        debug_assert!(target.prime() == source.prime());
        debug_assert!(target.offset() >= source.offset());
        debug_assert!(
            target.dimension() == source.dimension(),
            "self.dim {} not equal to other.dim {}",
            target.dimension(),
            source.dimension()
        );
        let p = target.prime();
        let offset_shift = target.offset() - source.offset();
        let bit_length = bit_length(p);
        let entries_per_64_bits = entries_per_64_bits(p);
        let usable_bits_per_limb = bit_length * entries_per_64_bits;
        let tail_shift = usable_bits_per_limb - offset_shift;
        let zero_bits = 64 - usable_bits_per_limb;
        let (min_target_limb, max_target_limb) = target.limb_range();
        let (min_source_limb, max_source_limb) = source.limb_range();
        let number_of_source_limbs = max_source_limb - min_source_limb;
        let number_of_target_limbs = max_target_limb - min_target_limb;
        Self {
            offset_shift,
            tail_shift,
            zero_bits,
            min_source_limb,
            min_target_limb,
            number_of_source_limbs,
            number_of_target_limbs,
        }
    }

    fn mask_first_limb_a<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> u64 {
        let mask = other.limb_mask(i);
        let source_limb_masked = other.limbs[i] & mask;
        (source_limb_masked << (self.offset_shift + self.zero_bits)) >> self.zero_bits
    }

    fn mask_first_limb_b<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> u64 {
        let mask = other.limb_mask(i);
        let source_limb_masked = other.limbs[i] & mask;
        source_limb_masked >> self.tail_shift
    }

    fn mask_middle_limb_a<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> u64 {
        (other.limbs[i] << (self.offset_shift + self.zero_bits)) >> self.zero_bits
    }

    fn mask_middle_limb_b<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> u64 {
        other.limbs[i] >> self.tail_shift
    }

    fn mask_last_limb_a<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> u64 {
        let mask = other.limb_mask(i);
        let source_limb_masked = other.limbs[i] & mask;
        source_limb_masked << self.offset_shift
    }

    fn mask_last_limb_b<const P: u32>(&self, other: SliceP<'_, P>, i: usize) -> u64 {
        let mask = other.limb_mask(i);
        let source_limb_masked = other.limbs[i] & mask;
        source_limb_masked >> self.tail_shift
    }
}

impl<T: AsRef<[u32]>, const P: u32> From<&T> for FpVectorP<P> {
    fn from(slice: &T) -> Self {
        Self {
            dimension: slice.as_ref().len(),
            limbs: slice
                .as_ref()
                .chunks(entries_per_64_bits(ValidPrime::new(P)))
                .map(|x| limb::pack::<_, P>(x.iter().copied()))
                .collect(),
        }
    }
}

impl<const P: u32> From<&FpVectorP<P>> for Vec<u32> {
    fn from(vec: &FpVectorP<P>) -> Vec<u32> {
        vec.iter().collect()
    }
}

pub struct FpVectorIterator<'a> {
    limbs: &'a [u64],
    bit_length: usize,
    bit_mask: u64,
    entries_per_64_bits_m_1: usize,
    limb_index: usize,
    entries_left: usize,
    cur_limb: u64,
    counter: usize,
}

impl<'a> FpVectorIterator<'a> {
    fn new<const P: u32>(vec: SliceP<'a, P>) -> Self {
        let counter = vec.dimension();
        let limbs = &vec.limbs;

        if counter == 0 {
            return Self {
                limbs,
                bit_length: 0,
                entries_per_64_bits_m_1: 0,
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

        let entries_per_64_bits = entries_per_64_bits(p);
        Self {
            limbs,
            bit_length,
            entries_per_64_bits_m_1: entries_per_64_bits - 1,
            bit_mask: bitmask(p),
            limb_index: pair.limb,
            entries_left: entries_per_64_bits - (vec.start % entries_per_64_bits),
            cur_limb,
            counter,
        }
    }

    pub fn skip_n(&mut self, mut n: usize) {
        if n >= self.counter {
            self.counter = 0;
            return;
        }
        let entries_per_64_bits = self.entries_per_64_bits_m_1 + 1;
        if n < self.entries_left {
            self.entries_left -= n;
            self.counter -= n;
            self.cur_limb >>= self.bit_length * n;
            return;
        }

        n -= self.entries_left;
        self.counter -= self.entries_left;
        self.entries_left = 0;

        let skip_limbs = n / entries_per_64_bits;
        self.limb_index += skip_limbs;
        self.counter -= skip_limbs * entries_per_64_bits;
        n -= skip_limbs * entries_per_64_bits;

        if n > 0 {
            self.entries_left = entries_per_64_bits - n;
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
            self.entries_left = self.entries_per_64_bits_m_1;
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

pub struct FpVectorNonZeroIterator<'a> {
    limbs: &'a [u64],
    bit_length: usize,
    bit_mask: u64,
    entries_per_64_bits_m_1: usize,
    limb_index: usize,
    entries_left: usize,
    cur_limb: u64,
    counter: usize,
    enumeration: usize,
}

impl<'a> FpVectorNonZeroIterator<'a> {
    fn new<const P: u32>(vec: SliceP<'a, P>) -> Self {
        let counter = vec.dimension();
        let limbs = &vec.limbs;

        if counter == 0 {
            return Self {
                limbs,
                bit_length: 0,
                entries_per_64_bits_m_1: 0,
                bit_mask: 0,
                limb_index: 0,
                entries_left: 0,
                cur_limb: 0,
                enumeration: 0,
                counter,
            };
        }
        let p = vec.prime();

        let pair = limb_bit_index_pair(p, vec.start);

        let bit_length = bit_length(p);
        let cur_limb = limbs[pair.limb] >> pair.bit_index;

        let entries_per_64_bits = entries_per_64_bits(p);
        Self {
            limbs,
            bit_length,
            entries_per_64_bits_m_1: entries_per_64_bits - 1,
            bit_mask: bitmask(p),
            limb_index: pair.limb,
            entries_left: entries_per_64_bits - (vec.start % entries_per_64_bits),
            cur_limb,
            enumeration: 0,
            counter,
        }
    }

    pub fn skip_n(&mut self, mut n: usize) {
        if n >= self.counter {
            self.counter = 0;
            return;
        }
        let entries_per_64_bits = self.entries_per_64_bits_m_1 + 1;
        if n < self.entries_left {
            self.entries_left -= n;
            self.counter -= n;
            self.cur_limb >>= self.bit_length * n;
            return;
        }
        self.enumeration += n;

        n -= self.entries_left;
        self.counter -= self.entries_left;
        self.entries_left = 0;

        let skip_limbs = n / entries_per_64_bits;
        self.limb_index += skip_limbs;
        self.counter -= skip_limbs * entries_per_64_bits;
        n -= skip_limbs * entries_per_64_bits;

        if n > 0 {
            self.entries_left = entries_per_64_bits - n;
            self.limb_index += 1;
            self.cur_limb = self.limbs[self.limb_index] >> (n * self.bit_length);
            self.counter -= n;
        }
    }
}

impl<'a> Iterator for FpVectorNonZeroIterator<'a> {
    type Item = (usize, u32);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.counter == 0 {
                return None;
            } else if self.entries_left == 0 {
                self.limb_index += 1;
                self.cur_limb = self.limbs[self.limb_index];
                self.entries_left = self.entries_per_64_bits_m_1;
            } else {
                self.entries_left -= 1;
            }

            let result = (self.cur_limb & self.bit_mask) as u32;
            let enumeration = self.enumeration;
            self.counter -= 1;
            self.enumeration += 1;
            self.cur_limb >>= self.bit_length;

            if result != 0 {
                return Some((enumeration, result));
            }
        }
    }
}
