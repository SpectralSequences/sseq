#![allow(dead_code)]
//
// Created by Hood on 5/22/2019.
//
//! An `FpVector` is a vector with entries in F<sub>p</sub>. We use this instead of `Vec<u32>`
//! because we can pack a lot of entries into a single `u64`, especially for p small. This not only
//! saves memory, but also leads to faster addition, for example (e.g. a single ^ can add 64
//! elements of F<sub>2</sub> at the same time).
//!
//! The organization of this file is a bit funny. There are in fact 4 different implementations of
//! `FpVector` &mdash; the cases p = 2, 3, 5 are handled separately with some extra documentation.
//! Hence there are structs `FpVector2`, `FpVector3`, `FpVector5` and `FpVectorGeneric`. `FpVector`
//! itself is an enum that can be either of these. All of these implement the trait `FpVectorT`,
//! which is where most functions lie. The implementations for `FpVector` of course just calls the
//! implementations of the specific instances, and this is automated via `enum_dispatch`.
//!
//! To understand the methods of `FpVector`, one should mostly look at the documentation for
//! `FpVectorT`. However, the static functions for `FpVector` are implemented in `FpVector` itself,
//! and hence is documented there as well. The documentation of `FpVector2`, `FpVector3`,
//! `FpVector5`, `FpVectorGeneric` are basically useless (and empty).
//!
//! In practice, one only ever needs to work with the enum `FpVector` and the associated functions.
//! However, the way this structured means one always has to import both `FpVector` and
//! `FpVectorT`, since you cannot use the functions of a trait unless you have imported the trait.

use crate::prime::ValidPrime;
use crate::prime::NUM_PRIMES;
use crate::prime::PRIME_TO_INDEX_MAP;
use std::ops::{Deref, DerefMut};
use std::sync::Once;

pub const MAX_DIMENSION: usize = 147500;

// Generated with Mathematica:
//     bitlengths = Prepend[#,1]&@ Ceiling[Log2[# (# - 1) + 1 &[Prime[Range[2, 54]]]]]
// But for 2 it should be 1.
static BIT_LENGHTS: [usize; NUM_PRIMES] = [1, 3, 5, 6, 7, 8, 9, 9];

pub fn bit_length(p: ValidPrime) -> usize {
    BIT_LENGHTS[PRIME_TO_INDEX_MAP[*p as usize]]
}

// This is 2^bitlength - 1.
// Generated with Mathematica:
//     2^bitlengths-1
static BITMASKS: [u32; NUM_PRIMES] = [1, 7, 31, 63, 127, 255, 511, 511];

pub fn bitmask(p: ValidPrime) -> u64 {
    BITMASKS[PRIME_TO_INDEX_MAP[*p as usize]] as u64
}

// This is floor(64/bitlength).
// Generated with Mathematica:
//      Floor[64/bitlengths]
static ENTRIES_PER_64_BITS: [usize; NUM_PRIMES] = [64, 21, 12, 10, 9, 8, 7, 7];

pub fn entries_per_64_bits(p: ValidPrime) -> usize {
    ENTRIES_PER_64_BITS[PRIME_TO_INDEX_MAP[*p as usize]]
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

pub struct FpVectorP<const P: u32> {
    dimension: usize,
    limbs: Vec<u64>,
}

pub struct SliceP<T: Deref<Target = [u64]>, const P: u32> {
    limbs: T,
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

    pub fn slice(&self, start: usize, end: usize) -> SliceP<&[u64], P> {
        assert!(start <= end && end <= self.dimension);
        SliceP {
            limbs: &self.limbs,
            start,
            end,
        }
    }

    pub fn slice_mut(&mut self, start: usize, end: usize) -> SliceP<&mut [u64], P> {
        assert!(start <= end && end <= self.dimension);
        SliceP {
            limbs: &mut self.limbs,
            start,
            end,
        }
    }

    fn as_slice(&self) -> SliceP<&'_ [u64], P> {
        self.into()
    }

    fn as_slice_mut(&mut self) -> SliceP<&'_ mut [u64], P> {
        self.into()
    }

    fn add_basis_element(&mut self, index: usize, value: u32) {
        self.as_slice_mut().add_basis_element(index, value);
    }
}

impl<'a, const P: u32> From<&'a FpVectorP<P>> for SliceP<&'a [u64], P> {
    fn from(v: &'a FpVectorP<P>) -> Self {
        v.slice(0, v.dimension)
    }
}

impl<'a, const P: u32> From<&'a mut FpVectorP<P>> for SliceP<&'a mut [u64], P> {
    fn from(v: &'a mut FpVectorP<P>) -> Self {
        v.slice_mut(0, v.dimension)
    }
}

impl<T: DerefMut<Target = [u64]>, const P: u32> SliceP<T, P> {
    fn slice_mut(&mut self, start: usize, end: usize) -> SliceP<&'_ mut [u64], P> {
        assert!(start <= end && end <= self.dimension());

        SliceP {
            limbs: &mut *self.limbs,
            start: self.start + start,
            end: self.start + end,
        }
    }
}

impl<T: Deref<Target = [u64]>, const P: u32> SliceP<T, P> {
    fn slice(&self, start: usize, end: usize) -> SliceP<&'_ [u64], P> {
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

    pub fn add<const P: u32>(limb_a: u64, limb_b: u64, coeff: u32) -> u64 {
        if P == 2 {
            limb_a & (coeff as u64 * limb_b)
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
            _ => unimplemented!(),
        }
    }

    pub fn is_reduced<const P: u32>(limb: u64) -> bool {
        limb == reduce::<P>(limb)
    }

    pub fn pack<const P: u32>(
        dim: usize,
        offset: usize,
        limb_array: &[u32],
        limbs: &mut [u64],
        limb_idx: usize,
    ) -> usize {
        let p = ValidPrime::new(P);
        let bit_length = bit_length(p);
        debug_assert_eq!(offset % bit_length, 0);
        let entries_per_64_bits = entries_per_64_bits(p);
        let mut bit_min = 0usize;
        let mut bit_max = bit_length * entries_per_64_bits;
        if limb_idx == 0 {
            bit_min = offset;
        }
        if limb_idx == limbs.len() - 1 {
            // Calculates how many bits of the last field we need to use. But if it divides
            // perfectly, we want bit max equal to bit_length * entries_per_64_bits, so subtract and add 1.
            // to make the output in the range 1 -- bit_length * entries_per_64_bits.
            let bits_needed_for_entire_vector = offset + dim * bit_length;
            let usable_bits_per_limb = bit_length * entries_per_64_bits;
            bit_max = 1 + ((bits_needed_for_entire_vector - 1) % (usable_bits_per_limb));
        }
        let mut bit_mask = 0;
        if bit_max - bit_min < 64 {
            bit_mask = (1u64 << (bit_max - bit_min)) - 1;
            bit_mask <<= bit_min;
            bit_mask = !bit_mask;
        }
        // copy data in
        let mut idx = 0;
        let mut limb_value = limbs[limb_idx] & bit_mask;
        for j in (bit_min..bit_max).step_by(bit_length) {
            limb_value |= (limb_array[idx] as u64) << j;
            idx += 1;
        }
        limbs[limb_idx] = limb_value;
        idx
    }
    pub fn unpack<const P: u32>(
        dim: usize,
        offset: usize,
        limb_array: &mut [u32],
        limbs: &[u64],
        limb_idx: usize,
    ) -> usize {
        let p = ValidPrime::new(P);

        let bit_length = bit_length(p);
        let entries_per_64_bits = entries_per_64_bits(p);
        let bit_mask = bitmask(p);
        let mut bit_min = 0usize;
        let mut bit_max = bit_length * entries_per_64_bits;
        if limb_idx == 0 {
            bit_min = offset;
        }
        if limb_idx == limbs.len() - 1 {
            // Calculates how many bits of the last field we need to use. But if it divides
            // perfectly, we want bit max equal to bit_length * entries_per_64_bits, so subtract and add 1.
            // to make the output in the range 1 -- bit_length * entries_per_64_bits.
            let bits_needed_for_entire_vector = offset + dim * bit_length;
            let usable_bits_per_limb = bit_length * entries_per_64_bits;
            bit_max = 1 + ((bits_needed_for_entire_vector - 1) % (usable_bits_per_limb));
        }

        let limb_value = limbs[limb_idx];
        let mut idx = 0;
        for j in (bit_min..bit_max).step_by(bit_length) {
            limb_array[idx] = ((limb_value >> j) & bit_mask) as u32;
            idx += 1;
        }
        idx
    }

    pub fn number<const P: u32>(dim: usize) -> usize {
        debug_assert!(dim < MAX_DIMENSION);
        if dim == 0 {
            0
        } else {
            limb_bit_index_pair(ValidPrime::new(P), dim - 1).limb + 1
        }
    }
}

impl<T: Deref<Target = [u64]>, const P: u32> SliceP<T, P> {
    fn prime(&self) -> ValidPrime {
        ValidPrime::new(P)
    }

    fn dimension(&self) -> usize {
        self.end - self.start
    }

    fn entry(&self, index: usize) -> u32 {
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

    fn offset(&self) -> usize {
        let bit_length = bit_length(self.prime());
        let entries_per_64_bits = entries_per_64_bits(self.prime());
        (self.start * bit_length) % (bit_length * entries_per_64_bits)
    }
}

impl<T: DerefMut<Target = [u64]>, const P: u32> SliceP<T, P> {
    fn add_basis_element(&mut self, index: usize, value: u32) {
        let mut x = self.entry(index);
        x += value;
        x %= P;
        self.set_entry(index, x);
    }

    fn set_entry(&mut self, index: usize, value: u32) {
        debug_assert!(index < self.dimension());
        let p = self.prime();
        let bit_mask = bitmask(p);
        let limb_index = limb_bit_index_pair(p, index + self.start);
        let mut result = self.limbs[limb_index.limb];
        result &= !(bit_mask << limb_index.bit_index);
        result |= (value as u64) << limb_index.bit_index;
        self.limbs[limb_index.limb] = result;
    }

    fn reduce_limbs(&mut self, start_limb: usize, end_limb: usize) {
        match P {
            2 => (),
            3 | 5 => {
                for limb in &mut *self.limbs {
                    *limb = limb::reduce::<P>(*limb);
                }
            }
            _ => {
                let p = self.prime();
                let mut unpacked_limb = vec![0; entries_per_64_bits(p)];
                let dimension = self.dimension();
                let limbs = &mut self.limbs;
                for i in start_limb..end_limb {
                    limb::unpack::<P>(dimension, 0, &mut unpacked_limb, limbs, i);
                    for limb in &mut unpacked_limb {
                        *limb %= *p;
                    }
                    limb::pack::<P>(dimension, 0, &unpacked_limb, limbs, i);
                }
            }
        }
    }
}

impl<const P: u32> FpVectorP<P> {
    pub fn entry(&self, index: usize) -> u32 {
        SliceP::<&[u64], P>::from(self).entry(index)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_slice_add_basis() {
        let mut v = FpVectorP::<2>::new(10);
        let mut slice = v.slice_mut(4, 10);
        slice.add_basis_element(2, 1);

        let mut slice2 = slice.slice_mut(4, 6);
        slice2.add_basis_element(1, 1);

        slice.add_basis_element(1, 1);

        assert_eq!(v.entry(6), 1);
        assert_eq!(v.entry(9), 1);
        assert_eq!(v.entry(5), 1);
    }

    #[test]
    fn test_slice() {
        let mut v = FpVectorP::<2>::new(10);

        v.add_basis_element(2, 1);
        v.add_basis_element(5, 1);

        let v = v;
        let slice1 = v.slice(0, 3);
        let slice2 = v.slice(2, 7);

        assert_eq!(slice1.entry(2), 1);
        assert_eq!(slice1.entry(1), 0);

        assert_eq!(slice2.entry(0), 1);
        assert_eq!(slice2.entry(3), 1);
    }
}
