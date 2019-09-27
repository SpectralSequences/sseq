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

use std::sync::Once;
use std::fmt;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use enum_dispatch::enum_dispatch;

use crate::combinatorics::is_valid_prime;
use crate::combinatorics::PRIME_TO_INDEX_MAP;
use crate::combinatorics::MAX_PRIME_INDEX;

pub const MAX_DIMENSION : usize = 147500;

// Generated with Mathematica:
//     bitlengths = Prepend[#,1]&@ Ceiling[Log2[# (# - 1) + 1 &[Prime[Range[2, 54]]]]]
// But for 2 it should be 1.
static BIT_LENGHTS : [usize; MAX_PRIME_INDEX] = [
     1, 3, 5, 6, 7, 8, 9, 9, 9, 10, 10, 11, 11, 11, 12, 12, 12, 12, 13,
     13, 13, 13, 13, 13, 14, 14, 14, 14, 14, 14, 14, 15, 15, 15, 15, 15,
     15, 15, 15, 15, 15, 15, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16
];

fn bit_length(p : u32) -> usize {
    BIT_LENGHTS[PRIME_TO_INDEX_MAP[p as usize]]
}

// This is 2^bitlength - 1.
// Generated with Mathematica:
//     2^bitlengths-1
static BITMASKS : [u32; MAX_PRIME_INDEX] = [
    1, 7, 31, 63, 127, 255, 511, 511, 511, 1023, 1023, 2047, 2047, 2047,
    4095, 4095, 4095, 4095, 8191, 8191, 8191, 8191, 8191, 8191, 16383,
    16383, 16383, 16383, 16383, 16383, 16383, 32767, 32767, 32767, 32767,
    32767, 32767, 32767, 32767, 32767, 32767, 32767, 65535, 65535, 65535,
    65535, 65535, 65535, 65535, 65535, 65535, 65535, 65535, 65535
];

fn bitmask(p : u32) -> u64{
    BITMASKS[PRIME_TO_INDEX_MAP[p as usize]] as u64
}

// This is floor(64/bitlength).
// Generated with Mathematica:
//      Floor[64/bitlengths]
static ENTRIES_PER_64_BITS : [usize;MAX_PRIME_INDEX] = [
    64, 21, 12, 10, 9, 8, 7, 7, 7, 6, 6, 5, 5, 5, 5, 5, 5, 5, 4, 4, 4,
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4
];

fn entries_per_64_bits(p : u32) -> usize {
    return ENTRIES_PER_64_BITS[PRIME_TO_INDEX_MAP[p as usize]];
}

#[derive(Clone)]
struct LimbBitIndexPair {
    limb : usize,
    bit_index : usize
}

/// This table tells us which limb and which bitfield of that limb to look for a given index of
/// the vector in.
static mut LIMB_BIT_INDEX_TABLE : [Option<Vec<LimbBitIndexPair>>; MAX_PRIME_INDEX] = [
    None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,None,None,None,None
];

static mut LIMB_BIT_INDEX_ONCE_TABLE : [Once; MAX_PRIME_INDEX] = [
    Once::new(),Once::new(), Once::new(), Once::new(), Once::new(),
    Once::new(),Once::new(), Once::new(), Once::new(), Once::new(),
    Once::new(),Once::new(), Once::new(), Once::new(), Once::new(),
    Once::new(),Once::new(), Once::new(), Once::new(), Once::new(),
    Once::new(),Once::new(), Once::new(), Once::new(), Once::new(),
    Once::new(),Once::new(), Once::new(), Once::new(), Once::new(),
    Once::new(),Once::new(), Once::new(), Once::new(), Once::new(),
    Once::new(),Once::new(), Once::new(), Once::new(), Once::new(),
    Once::new(),Once::new(), Once::new(), Once::new(), Once::new(),
    Once::new(),Once::new(), Once::new(), Once::new(), Once::new(),
    Once::new(),Once::new(), Once::new(), Once::new()
];

pub fn initialize_limb_bit_index_table(p : u32){
    if p == 2 {
        return;
    }
    unsafe{
        LIMB_BIT_INDEX_ONCE_TABLE[PRIME_TO_INDEX_MAP[p as usize]].call_once(||{
            let entries_per_limb = entries_per_64_bits(p);
            let bit_length = bit_length(p);
            let mut table : Vec<LimbBitIndexPair> = Vec::with_capacity(MAX_DIMENSION);
            for i in 0 .. MAX_DIMENSION {
                table.push(LimbBitIndexPair{
                    limb : i/entries_per_limb,
                    bit_index : (i % entries_per_limb) * bit_length,
                })
            }
            LIMB_BIT_INDEX_TABLE[PRIME_TO_INDEX_MAP[p as usize]] = Some(table);
        });
    }
}

fn limb_bit_index_pair(p : u32, idx : usize) -> LimbBitIndexPair {
    match p {
        2 => { LimbBitIndexPair
            {
                limb : idx/64,
                bit_index : idx % 64,
            }
        },
        _ => {
            let prime_idx = PRIME_TO_INDEX_MAP[p as usize];
            debug_assert!(is_valid_prime(p));
            debug_assert!(idx < MAX_DIMENSION);
            unsafe {
                let table = &LIMB_BIT_INDEX_TABLE[prime_idx];
                (*table.as_ref().unwrap().get_unchecked(idx)).clone()
            }
        }
    }
}

#[enum_dispatch]
#[derive(Debug, Clone)]
pub enum FpVector {
    FpVector2,
    FpVector3,
    FpVector5,
    FpVectorGeneric
}

#[enum_dispatch(FpVector)]
pub trait FpVectorT {
    fn reduce_limbs(&mut self, start_limb : usize, end_limb : usize );
    fn vector_container(&self) -> &VectorContainer;
    fn vector_container_mut(&mut self) -> &mut VectorContainer;
    fn prime(&self) -> u32;

    fn dimension(&self) -> usize {
        let container = self.vector_container();
        return container.slice_end - container.slice_start;
    }

    fn offset(&self) -> usize {
        let container = self.vector_container();
        let bit_length = bit_length(self.prime());
        let entries_per_64_bits = entries_per_64_bits(self.prime());
        return (container.slice_start * bit_length) % (bit_length * entries_per_64_bits);
    }

    fn min_index(&self) -> usize {
        let container = self.vector_container();
        return container.slice_start;
    }

    fn slice(&self) -> (usize, usize) {
        let container = self.vector_container();
        return (container.slice_start, container.slice_end);
    }

    fn set_slice(&mut self, slice_start : usize, slice_end : usize) {
        let container = self.vector_container_mut();
        container.slice_end = container.slice_start + slice_end;
        container.slice_start += slice_start;
        debug_assert!(container.slice_start <= container.slice_end);
        debug_assert!(container.slice_end <= container.dimension);        
    }

    fn restore_slice(&mut self, slice : (usize, usize)) {
        let container = self.vector_container_mut();
        container.slice_start = slice.0;
        container.slice_end = slice.1;
    }

    fn clear_slice(&mut self) {
        let container = self.vector_container_mut();
        container.slice_start = 0;
        container.slice_end = container.dimension;
    }

    /// Drops every element in the fp_vector that is not in the current slice.
    fn into_slice(&mut self) {
        let p = self.prime();
        let container = self.vector_container_mut();
        let entries_per_64_bits = entries_per_64_bits(p);
        assert_eq!(container.slice_start % entries_per_64_bits, 0);
        let n = container.slice_start / entries_per_64_bits;
        container.limbs.drain(0..n);

        container.slice_end -= container.slice_start;
        container.dimension = container.slice_end;
        container.slice_start = 0;
        container.limbs.truncate((container.slice_end - 1) / entries_per_64_bits + 1);
    }

    fn min_limb(&self) -> usize {
        let p = self.prime();
        let container = self.vector_container();
        limb_bit_index_pair(p,container.slice_start).limb
    }

    fn max_limb(&self) -> usize {
        let p = self.prime();
        let container = self.vector_container();
        if container.slice_end > 0{
            limb_bit_index_pair(p, container.slice_end - 1).limb + 1
        } else {
            0
        }
    }

    fn limbs(&self) -> &Vec<u64> {
        &self.vector_container().limbs
    }

    fn limbs_mut(&mut self) -> &mut Vec<u64> {
        &mut self.vector_container_mut().limbs
    }

    fn limb_mask(&self, limb_idx : usize) -> u64 {
        let offset = self.offset();
        let min_limb = self.min_limb();
        let max_limb = self.max_limb();
        let number_of_limbs = max_limb - min_limb;
        let mut mask = !0;
        if limb_idx == 0 {
            mask <<= offset;
        }
        if limb_idx + 1 == number_of_limbs {
            let p = self.prime();
            let dimension = self.dimension();
            let bit_length = bit_length(p);
            let entries_per_64_bits = entries_per_64_bits(p);
            let bits_needed_for_entire_vector = offset + dimension * bit_length;
            let usable_bits_per_limb = bit_length * entries_per_64_bits;
            let bit_max = 1 + ((bits_needed_for_entire_vector - 1)%(usable_bits_per_limb));
            mask &= (!0) >> (64 - bit_max);
        }
        return mask;
    }
    fn set_to_zero(&mut self){
        let min_limb = self.min_limb();
        let max_limb = self.max_limb();
        let number_of_limbs = max_limb - min_limb;
        if number_of_limbs == 0 {
            return;
        }
        for i in 1 .. number_of_limbs - 1 {
            let limbs = self.limbs_mut();
            limbs[min_limb + i] = 0;
        }
        let mut i = 0; {
            let mask = self.limb_mask(i);
            let limbs = self.limbs_mut();
            limbs[min_limb + i] &= !mask;
        }
        i = number_of_limbs - 1;
        if i > 0 {
            let mask = self.limb_mask(i);
            let limbs = self.limbs_mut();
            limbs[min_limb + i] &= !mask;
        }
    }

    // TODO: implement this directly?
    fn shift_assign(&mut self, other : &FpVector){
        if self.offset() == other.offset() {
            self.assign(other);
            return;
        }
        self.set_to_zero();
        self.shift_add(other, 1);
    }

    fn assign(&mut self, other : &FpVector){
        let min_target_limb = self.min_limb();
        let max_target_limb = self.max_limb();
        let min_source_limb = other.min_limb();
        let number_of_limbs = max_target_limb - min_target_limb;
        if number_of_limbs == 0 {
            return;
        }
        debug_assert!(self.offset() == other.offset());
        debug_assert_eq!(number_of_limbs, other.max_limb() - other.min_limb());
        let target_limbs = self.limbs_mut();
        let source_limbs = other.limbs();
        for i in 1 .. number_of_limbs.saturating_sub(1) {
            target_limbs[min_target_limb + i] = source_limbs[min_source_limb + i];
        }
        let mut i=0; {
            let mask = other.limb_mask(i);
            let result = source_limbs[min_source_limb + i] & mask;
            target_limbs[min_target_limb + i] &= !mask;
            target_limbs[min_target_limb + i] |= result;
        }
        i = number_of_limbs - 1;
        if i > 0 {
            let mask = other.limb_mask(i);
            let result = source_limbs[min_source_limb + i] & mask;
            target_limbs[min_target_limb + i] &= !mask;
            target_limbs[min_target_limb + i] |= result;
        }
    }

    fn is_zero_pure(&self) -> bool {
        for limb in self.limbs().iter() {
            if *limb != 0 {
                return false;
            }
        }
        true
    }

    fn is_zero(&self) -> bool{
        let min_limb = self.min_limb();
        let max_limb = self.max_limb();
        let number_of_limbs = max_limb - min_limb;
        if number_of_limbs == 0 {
            return true;
        }
        let limbs = self.limbs();
        for i in 1 .. number_of_limbs-1 {
            if limbs[min_limb + i] != 0 {
                return false;
            }
        }
        let mut i = 0; {
            let mask = self.limb_mask(i);
            if limbs[min_limb + i] & mask != 0 {
                return false;
            }
        }
        i = number_of_limbs - 1;
        if i > 0 {
            let mask = self.limb_mask(i);
            if limbs[min_limb + i] & mask != 0 {
                return false;
            }
        }
        return true
    }

    fn entry(&self, index : usize) -> u32 {
        debug_assert!(index < self.dimension());
        let p = self.prime();
        let bit_mask = bitmask(p);
        let limb_index = limb_bit_index_pair(p, index + self.min_index());
        let mut result = self.limbs()[limb_index.limb];
        result >>= limb_index.bit_index;
        result &= bit_mask;
        return result as u32;
    }

    fn set_entry(&mut self, index : usize, value : u32){
        debug_assert!(index < self.dimension());
        let p = self.prime();
        let bit_mask = bitmask(p);
        let limb_index = limb_bit_index_pair(p, index + self.min_index());
        let limbs = self.limbs_mut();
        let mut result = limbs[limb_index.limb];
        result &= !(bit_mask << limb_index.bit_index);
        result |= (value as u64) << limb_index.bit_index;
        limbs[limb_index.limb] = result;
    }

    fn add_basis_element(&mut self, index : usize, value : u32){
        let mut x = self.entry(index);
        x += value;
        x = x % self.prime();
        self.set_entry(index, x);
    }

    /// Unpacks an FpVector onto an array slice. note that the array slice has to be long
    /// enough to hold all the elements in the FpVector.
    fn unpack(&self, target : &mut [u32]){
        debug_assert!(self.dimension() <= target.len());
        let p = self.prime();
        let dimension = self.dimension();
        let offset = self.offset();
        let limbs = self.limbs();
        let mut target_idx = 0;
        for i in 0..limbs.len() {
            target_idx += FpVector::unpack_limb(p, dimension, offset, &mut target[target_idx ..], limbs, i);
        }
    }

    fn to_vector(&self) -> Vec<u32> {
        let mut vec = vec![0; self.dimension()];
        self.unpack(&mut vec);
        vec
    }

    fn pack(&mut self, source : &[u32]){
        debug_assert!(self.dimension() <= source.len());
        let p = self.prime();
        let dimension = self.dimension();
        let offset = self.offset();
        let limbs = self.limbs_mut();
        let mut source_idx = 0;
        for i in 0..limbs.len() {
            source_idx += FpVector::pack_limb(p, dimension, offset, &source[source_idx ..], limbs, i);
        }
    }

    /// `coeff` need not be reduced mod p.
    fn add_tensor(&mut self, offset : usize, coeff : u32, left : &FpVector, right : &FpVector) {
        let right_dim = right.dimension();

        let old_slice = self.slice();
        for i in 0 .. left.dimension() {
            let entry = (left.entry(i) * coeff) % self.prime();
            if entry == 0 {
                continue;
            }
            self.set_slice(offset + i * right_dim, offset + (i + 1) * right_dim);
            self.shift_add(right, entry);
            self.restore_slice(old_slice);
        }
    }

    /// Adds `c` * `other` to `self`. `other` must have the same length, offset, and prime as self, and `c` must be between `0` and `p - 1`.
    fn add(&mut self, other : &FpVector, c : u32){
        debug_assert!(self.prime() == other.prime());
        debug_assert!(self.offset() == other.offset());
        debug_assert!(self.dimension() == other.dimension(),
            format!("self.dim {} not equal to other.dim {}", self.dimension(), other.dimension()));
        if self.dimension() == 0 {
            return;
        }
        let p = self.prime();
        debug_assert!(c < p);
        let min_target_limb = self.min_limb();
        let max_target_limb = self.max_limb();
        let min_source_limb = other.min_limb();
        let max_source_limb = other.max_limb();
        debug_assert!(max_source_limb - min_source_limb == max_target_limb - min_target_limb);
        let number_of_limbs = max_source_limb - min_source_limb;
        let target_limbs = self.limbs_mut();
        let source_limbs = other.limbs();
        for i in 1..number_of_limbs-1 {
            target_limbs[i + min_target_limb] = FpVector::add_limb(p, target_limbs[i + min_target_limb], source_limbs[i + min_source_limb], c);
        }
        let mut i = 0; {
            let mask = other.limb_mask(i);
            let source_limb_masked = source_limbs[min_source_limb + i] & mask;
            target_limbs[i + min_target_limb] = FpVector::add_limb(p, target_limbs[i + min_target_limb], source_limb_masked, c);
        }
        i = number_of_limbs - 1;
        if i > 0 {
            let mask = other.limb_mask(i);
            let source_limb_masked = source_limbs[min_source_limb + i] & mask;
            target_limbs[i + min_target_limb] = FpVector::add_limb(p, target_limbs[i + min_target_limb], source_limb_masked, c);
        }
        self.reduce_limbs(min_target_limb, max_target_limb);
    }

    fn shift_add(&mut self, other : &FpVector, c : u32){
        if self.dimension() == 0 {
            return;
        }

        if self.offset() > other.offset() {
            self.shift_right_add(other, c);
        } else if self.offset() < other.offset() {
            self.shift_left_add(other, c);
        } else {
            self.add(other, c);
        }
    }

    fn shift_right_add(&mut self, other : &FpVector, c : u32){
        debug_assert!(self.prime() == other.prime());
        debug_assert!(self.offset() >= other.offset());
        debug_assert!(self.dimension() == other.dimension(),
            format!("self.dim {} not equal to other.dim {}", self.dimension(), other.dimension()));
        let p = self.prime();
        debug_assert!(c < p);
        let offset_shift = self.offset() - other.offset();
        let bit_length = bit_length(p);
        let entries_per_64_bits = entries_per_64_bits(p);
        let usable_bits_per_limb = bit_length * entries_per_64_bits;
        let tail_shift = usable_bits_per_limb - offset_shift;
        let min_target_limb = self.min_limb();
        let max_target_limb = self.max_limb();
        let min_source_limb = other.min_limb();
        let max_source_limb = other.max_limb();
        let number_of_source_limbs = max_source_limb - min_source_limb;
        let number_of_target_limbs = max_target_limb - min_target_limb;
        let target_limbs = self.limbs_mut();
        let source_limbs = other.limbs();
        for i in 1..number_of_source_limbs-1 {
            target_limbs[i + min_target_limb] = FpVector::add_limb(p, target_limbs[i + min_target_limb], source_limbs[i + min_source_limb] << offset_shift, c);
            target_limbs[i + min_target_limb + 1] = FpVector::add_limb(p, target_limbs[i + min_target_limb + 1], source_limbs[i + min_source_limb] >> tail_shift, c);
        }
        let mut i = 0; {
            let mask = other.limb_mask(i);
            let source_limb_masked = source_limbs[min_source_limb + i] & mask;
            target_limbs[i + min_target_limb] = FpVector::add_limb(p, target_limbs[i + min_target_limb], source_limb_masked << offset_shift, c);
            if number_of_target_limbs > 1 {
                target_limbs[i + min_target_limb + 1] = FpVector::add_limb(p, target_limbs[i + min_target_limb + 1], source_limb_masked >> tail_shift, c);
            }
        }
        i = number_of_source_limbs - 1;
        if i > 0 {
            let mask = other.limb_mask(i);
            let source_limb_masked = source_limbs[min_source_limb + i] & mask;
            target_limbs[i + min_target_limb] = FpVector::add_limb(p, target_limbs[i + min_target_limb], source_limb_masked << offset_shift, c);
            if number_of_target_limbs > number_of_source_limbs {
                target_limbs[i + min_target_limb + 1] = FpVector::add_limb(p, target_limbs[i + min_target_limb + 1], source_limb_masked >> tail_shift, c);
            }            
        }
        self.reduce_limbs(min_target_limb, max_target_limb);
    }

    fn shift_left_add(&mut self, other : &FpVector, c : u32){
        debug_assert!(self.prime() == other.prime());
        debug_assert!(self.offset() <= other.offset());
        debug_assert!(self.dimension() == other.dimension(),
            format!("self.dim {} not equal to other.dim {}", self.dimension(), other.dimension()));
        let p = self.prime();
        debug_assert!(c < p);
        let offset_shift = other.offset() - self.offset();
        let bit_length = bit_length(p);
        let entries_per_64_bits = entries_per_64_bits(p);
        let usable_bits_per_limb = bit_length * entries_per_64_bits;
        let tail_shift = usable_bits_per_limb - offset_shift;
        let min_target_limb = self.min_limb();
        let max_target_limb = self.max_limb();
        let min_source_limb = other.min_limb();
        let max_source_limb = other.max_limb();
        let number_of_source_limbs = max_source_limb - min_source_limb;
        let number_of_target_limbs = max_target_limb - min_target_limb;
        let target_limbs = self.limbs_mut();
        let source_limbs = other.limbs();
        for i in 1..number_of_source_limbs-1 {
            target_limbs[i + min_target_limb] = FpVector::add_limb(p, target_limbs[i + min_target_limb], source_limbs[i + min_source_limb] >> offset_shift, c);
            target_limbs[i + min_target_limb - 1] = FpVector::add_limb(p, target_limbs[i + min_target_limb - 1], source_limbs[i + min_source_limb] << tail_shift, c);
        }
        let mut i = 0; {
            let mask = other.limb_mask(i);
            let source_limb_masked = source_limbs[min_source_limb + i] & mask;
            target_limbs[i + min_target_limb] = FpVector::add_limb(p, target_limbs[i + min_target_limb], source_limb_masked >> offset_shift, c);
        }
        i = number_of_source_limbs - 1;
        if i > 0 {
            let mask = other.limb_mask(i);
            let source_limb_masked = source_limbs[min_source_limb + i] & mask;
            target_limbs[i + min_target_limb - 1] = FpVector::add_limb(p, target_limbs[i + min_target_limb - 1], source_limb_masked << tail_shift, c);
            if number_of_source_limbs == number_of_target_limbs {
                target_limbs[i + min_target_limb] = FpVector::add_limb(p, target_limbs[i + min_target_limb], source_limb_masked >> offset_shift, c);
            }
        }
        self.reduce_limbs(min_target_limb, max_target_limb);
    }

    fn scale(&mut self, c : u32){
        let c = c as u64;
        let min_limb = self.min_limb();
        let max_limb = self.max_limb();
        let number_of_limbs = max_limb - min_limb;
        if number_of_limbs == 0 {
            return;
        }
        for i in 1..number_of_limbs-1 {
            let limbs = self.limbs_mut();
            limbs[i + min_limb] *= c;
        }
        let mut i = 0; {
            let mask = self.limb_mask(i);
            let limbs = self.limbs_mut();
            let full_limb = limbs[min_limb + i];
            let masked_limb = full_limb & mask;
            let rest_limb = full_limb & !mask;
            limbs[i + min_limb] = (masked_limb * c) | rest_limb;
        }
        i = number_of_limbs - 1;
        if i > 0 {
            let mask = self.limb_mask(i);
            let limbs = self.limbs_mut();
            let full_limb = limbs[min_limb + i];
            let masked_limb = full_limb & mask;
            let rest_limb = full_limb & !mask;
            limbs[i + min_limb] = (masked_limb * c) | rest_limb;
        }
        self.reduce_limbs(min_limb, max_limb);
    }
}

impl PartialEq for FpVector {
    fn eq(&self,other : &Self) -> bool {
        let self_min_limb = self.min_limb();
        let self_max_limb = self.max_limb();
        let other_min_limb = other.min_limb();
        let other_max_limb = other.max_limb();
        let number_of_limbs = self_max_limb - self_min_limb;

        if other_max_limb - other_min_limb != number_of_limbs {
            return false;
        }

        let self_limbs = self.limbs();
        let other_limbs = other.limbs();
        for i in 1 .. number_of_limbs-1 {
            if self_limbs[self_min_limb + i] != other_limbs[other_min_limb + i] {
                return false;
            }
        }
        let mut i = 0; {
            let mask = self.limb_mask(i);
            let self_limb_masked = self_limbs[self_min_limb + i] & mask;
            let other_limb_masked = other_limbs[other_min_limb + i] & mask;
            if self_limb_masked != other_limb_masked {
                return false;
            }
        }
        i = number_of_limbs - 1;
        if i > 0 {
            let mask = self.limb_mask(i);
            let self_limb_masked = self_limbs[self_min_limb + i] & mask;
            let other_limb_masked = other_limbs[other_min_limb + i] & mask;
            if self_limb_masked != other_limb_masked {
                return false;
            }
        }
        return true;
    }
}

impl Eq for FpVector {}

#[derive(Debug, Clone)]
pub struct VectorContainer {
    dimension : usize,
    slice_start : usize,
    slice_end : usize,
    limbs : Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct FpVector2 {
    vector_container : VectorContainer
}

#[derive(Debug, Clone)]
pub struct FpVector3 {
    vector_container : VectorContainer
}

#[derive(Debug, Clone)]
pub struct FpVector5 {
    vector_container : VectorContainer
}

#[derive(Debug, Clone)]
pub struct FpVectorGeneric {
    p : u32,
    vector_container : VectorContainer
}

impl FpVectorT for FpVector2 {
    fn reduce_limbs(&mut self, _start_limb : usize, _end_limb : usize){}

    fn prime(&self) -> u32 { 2 }
    fn vector_container (&self) -> &VectorContainer { &self.vector_container }
    fn vector_container_mut (&mut self) -> &mut VectorContainer { &mut self.vector_container }
}

impl FpVectorT for FpVector3 {
    fn reduce_limbs(&mut self, start_limb : usize, end_limb : usize ){
        let limbs = &mut self.vector_container.limbs;
        for i in start_limb..end_limb {
            let top_bit_set_in_each_field = 0x4924924924924924u64;
            let mut limb = limbs[i];
            limb = ((limb & top_bit_set_in_each_field) >> 2) + (limb & (!top_bit_set_in_each_field));
            let mut limb_3s = limb & (limb >> 1);
            limb_3s |= limb_3s << 1;
            limb ^= limb_3s;
            limbs[i] = limb;
        }
    }

    fn prime (&self) -> u32 { 3 }
    fn vector_container (&self) -> &VectorContainer { &self.vector_container }
    fn vector_container_mut (&mut self) -> &mut VectorContainer { &mut self.vector_container }
}


impl FpVectorT for FpVector5 {
    fn reduce_limbs(&mut self, start_limb : usize, end_limb : usize ){
        let limbs = &mut self.vector_container.limbs;
        for i in start_limb..end_limb {
            let bottom_bit = 0x84210842108421u64;
            let bottom_two_bits = bottom_bit | (bottom_bit << 1);
            let bottom_three_bits = bottom_bit | (bottom_two_bits << 1);
            let a = (limbs[i] >> 2) & bottom_three_bits;
            let b = limbs[i] & bottom_two_bits;
            let m = (bottom_bit << 3) - a + b;
            let mut c = (m >> 3) & bottom_bit;
            c |= c << 1;
            let d = m & bottom_three_bits;
            limbs[i] = d + c - bottom_two_bits;
        }
    }

    fn prime(&self) -> u32 { 5 }
    fn vector_container (&self) -> &VectorContainer { &self.vector_container }
    fn vector_container_mut (&mut self) -> &mut VectorContainer { &mut self.vector_container }
}


impl FpVectorT for FpVectorGeneric {
    fn reduce_limbs(&mut self, start_limb : usize, end_limb : usize){
        let entries_per_64_bits = entries_per_64_bits(self.p);
        let mut unpacked_limb = Vec::with_capacity(entries_per_64_bits);
        for _ in 0..entries_per_64_bits {
            unpacked_limb.push(0);
        }
        let p = self.p;
        let dimension = self.vector_container.dimension;
        let limbs = &mut self.vector_container.limbs;
        for i in start_limb..end_limb {
            FpVector::unpack_limb(p, dimension, 0, &mut unpacked_limb, limbs, i);
            for j in 0..unpacked_limb.len() {
                unpacked_limb[j] = unpacked_limb[j] % self.p;
            }
            FpVector::pack_limb(p, dimension, 0, &unpacked_limb, limbs, i);
        }
    }

    fn prime (&self) -> u32 { self.p }
    fn vector_container (&self) -> &VectorContainer { &self.vector_container }
    fn vector_container_mut (&mut self) -> &mut VectorContainer { &mut self.vector_container }
}

impl FpVector {
    pub fn new(p : u32, dimension : usize) -> FpVector {
        let slice_start = 0;
        let slice_end = dimension;
        let number_of_limbs = Self::number_of_limbs(p, dimension);
        let limbs = vec![0; number_of_limbs];
        let vector_container = VectorContainer {dimension, limbs, slice_start, slice_end };
        match p  {
            2 => FpVector::from(FpVector2 { vector_container }),
            3 => FpVector::from(FpVector3 { vector_container }),
            5 => FpVector::from(FpVector5 { vector_container }),
            _ => FpVector::from(FpVectorGeneric { p, vector_container })
        }
    }

    pub fn from_vec(p : u32, vec : &[u32]) -> FpVector {
        let mut result = FpVector::new(p, vec.len());
        result.pack(&vec);
        result
    }

    fn add_limb(p : u32, limb_a : u64, limb_b : u64, coeff : u32) -> u64 {
        match p {
           2 => limb_a ^ (coeff as u64 * limb_b),
           _ => limb_a + (coeff as u64) * limb_b
        }
    }

    pub fn number_of_limbs(p : u32, dimension : usize) -> usize {
        debug_assert!(dimension < MAX_DIMENSION);
        if dimension == 0 {
            return 0;
        } else {
            return limb_bit_index_pair(p, dimension - 1).limb + 1;
        }
    }

    pub fn padded_dimension(p : u32, dimension : usize) -> usize {
        let entries_per_limb = entries_per_64_bits(p);
        return ((dimension + entries_per_limb - 1)/entries_per_limb)*entries_per_limb;
    }

    pub fn scratch_vector(p : u32, dimension : usize) -> Self {
        let mut result = FpVector::new(p, FpVector::padded_dimension(p, dimension));
        result.set_slice(0, dimension);
        return result;
    }

    pub fn set_scratch_vector_size(mut self, dimension : usize) -> Self {
        let p = self.prime();
        self.clear_slice();
        let mut result;
        if dimension <= self.dimension() {
            result = self;
        } else {
            result = FpVector::scratch_vector(p, dimension);
        }
        result.set_slice(0, dimension);
        return result;
    }

    pub fn iter(&self) -> FpVectorIterator {
        FpVectorIterator::new(self)
    }

    fn pack_limb(p : u32, dimension : usize, offset : usize, limb_array : &[u32], limbs : &mut Vec<u64>, limb_idx : usize) -> usize {
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
            let bits_needed_for_entire_vector = offset + dimension * bit_length;
            let usable_bits_per_limb = bit_length * entries_per_64_bits;
            bit_max = 1 + ((bits_needed_for_entire_vector - 1)%(usable_bits_per_limb));
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
        for j in (bit_min .. bit_max).step_by(bit_length) {
            limb_value |= (limb_array[idx] as u64) << j;
            idx += 1;
        }
        limbs[limb_idx] = limb_value;
        return idx;
    }

    fn unpack_limb(p : u32, dimension : usize, offset : usize, limb_array : &mut [u32], limbs : &Vec<u64>, limb_idx : usize) -> usize {
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
            let bits_needed_for_entire_vector = offset + dimension * bit_length;
            let usable_bits_per_limb = bit_length * entries_per_64_bits;
            bit_max = 1 + ((bits_needed_for_entire_vector - 1)%(usable_bits_per_limb));
        }

        let limb_value = limbs[limb_idx];
        let mut idx = 0;
        for j in (bit_min .. bit_max).step_by(bit_length) {
            limb_array[idx] = ((limb_value >> j) & bit_mask) as u32;
            idx += 1;
        }
        return idx;
    }
}

pub struct FpVectorIterator<'a> {
    limbs : &'a Vec<u64>,
    bit_length : usize,
    bit_mask : u64,
    entries_per_64_bits_m_1 : usize,
    limb_index : usize,
    entries_left : usize,
    cur_limb : u64,
    counter : usize,
}

impl<'a> FpVectorIterator<'a> {
    fn new(vec : &'a FpVector) -> FpVectorIterator {
        let counter = vec.dimension();
        let limbs = vec.limbs();

        if counter == 0 {
            return FpVectorIterator {
                limbs,
                bit_length : 0,
                entries_per_64_bits_m_1 : 0,
                bit_mask : 0,
                limb_index : 0,
                entries_left : 0,
                cur_limb: 0,
                counter
            }
        }
        let p = vec.prime();

        let min_index = vec.min_index();
        let pair = limb_bit_index_pair(p,min_index);

        let bit_length = bit_length(p);
        let cur_limb = limbs[pair.limb] >> pair.bit_index;

        let offset = vec.offset();

        let entries_per_64_bits = entries_per_64_bits(p);
        FpVectorIterator {
            limbs,
            bit_length,
            entries_per_64_bits_m_1 : entries_per_64_bits - 1,
            bit_mask : bitmask(p),
            limb_index : pair.limb,
            entries_left : entries_per_64_bits - (min_index % entries_per_64_bits),
            cur_limb,
            counter
        }
    }

    pub fn skip_n(&mut self, mut n : usize) {
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
            self.cur_limb = self.limbs[self.limb_index] >> n * self.bit_length;
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
            self.entries_left = self.entries_per_64_bits_m_1; // Set to entries_per_64_bits, then immediately decrement 1
        } else {
            self.entries_left -= 1;
        }

        let result = (self.cur_limb & self.bit_mask) as u32;
        self.counter -= 1;
        self.cur_limb >>= self.bit_length;

        Some(result)
    }
}

impl fmt::Display for FpVector {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let mut it = self.iter();
        if let Some(x) = it.next(){
            write!(f,"[{}", x)?;
        } else {
            write!(f, "[]")?;
            return Ok(());
        }
        for x in it {
            write!(f, ", {}", x)?;
        }
        write!(f,"]")?;
        Ok(())
    }
}

impl Serialize for FpVector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S : Serializer,
    {
        self.to_vector().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FpVector {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
        where D : Deserializer<'de>
    {
        Ok(FpVector::new(2, 0)) // Implement this? This would require proper deserializing
    }
}

/// An FpVectorMask encodes a subset of the basis elements of an Fp vector space. This is used to
/// project onto the subspace spanned by the selected basis elements.
#[derive(Debug)]
pub struct FpVectorMask {
    p : u32,
    dimension : usize,
    masks : Vec<u64>
}

impl FpVectorMask {
    pub fn new(p : u32, dimension : usize) -> Self {
        let number_of_limbs = FpVector::number_of_limbs(p, dimension);
        Self {
            p,
            dimension,
            masks : vec![!0; number_of_limbs]
        }
    }

    pub fn set_zero(&mut self) {
        for limb in self.masks.iter_mut() {
            *limb = 0;
        }
    }

    /// If `on` is true, we add the `i`th basis element to the subset. Otherwise, we remove it.
    pub fn set_mask(&mut self, i : usize, on : bool) {
        let pair = limb_bit_index_pair(self.p, i);
        let limb = &mut self.masks[pair.limb];

        if on {
            *limb |= bitmask(self.p) << pair.bit_index;
        } else  {
            *limb &= !(bitmask(self.p) << pair.bit_index);
        }
    }

    /// This projects `target` onto the subspace spanned by the designated subset of basis
    /// elements.
    pub fn apply(&self, target : &mut FpVector) {
        debug_assert_eq!(self.dimension, target.dimension());
        debug_assert_eq!(target.vector_container().slice_start, 0);
        debug_assert_eq!(target.vector_container().slice_end, target.dimension());

        let target = &mut target.vector_container_mut().limbs;
        for i in 0 .. self.masks.len() {
            target[i] &= self.masks[i];
        }
    }
}

use std::io;
use std::io::{Read, Write};
use saveload::{Save, Load};

impl Save for FpVector {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        self.dimension().save(buffer)?;
        for limb in self.limbs().iter() {
            limb.save(buffer)?;
        }
        Ok(())
    }
}

impl Load for FpVector {
    type AuxData = u32;

    fn load(buffer : &mut impl Read, p : &u32) -> io::Result<Self> {
        let p = *p;

        let dimension = usize::load(buffer, &())?;

        if dimension == 0 {
            return Ok(FpVector::new(p, 0));
        }

        let entries_per_64_bits = entries_per_64_bits(p);

        let num_limbs = (dimension - 1) / entries_per_64_bits + 1;

        let mut limbs : Vec<u64> = Vec::with_capacity(num_limbs);

        for _ in 0 .. num_limbs {
            limbs.push(u64::load(buffer, &())?);
        }

        let vector_container = VectorContainer {
            dimension,
            slice_start : 0,
            slice_end : dimension,
            limbs
        };

        let result = match p  {
            2 => FpVector::from(FpVector2 { vector_container }),
            3 => FpVector::from(FpVector3 { vector_container }),
            5 => FpVector::from(FpVector5 { vector_container }),
            _ => FpVector::from(FpVectorGeneric { p, vector_container })
        };

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    fn random_vector(p : u32, dimension : usize) -> Vec<u32> {
        let mut result = Vec::with_capacity(dimension);
        let mut rng = rand::thread_rng();
        for i in 0..dimension {
            result.push(rng.gen::<u32>() % p);
        }
        return result;
    }
    use rstest::rstest_parametrize;

    #[rstest_parametrize(p, case(3), case(5), case(7))]
    fn test_reduce_limb(p : u32){
        initialize_limb_bit_index_table(p);
        for dim in [10, 20, 70, 100, 1000].iter() {
            println!("p: {}, dim: {}", p, dim);
            let mut v = FpVector::new(p, *dim);
            let v_arr = random_vector(p*(p-1), *dim);
            v.pack(&v_arr);
            v.reduce_limbs(v.min_limb(), v.max_limb());
            let mut result = Vec::with_capacity(*dim);
            for i in 0..*dim {
                result.push(0);
            }
            v.unpack(&mut result);
            let mut diffs = Vec::new();
            for i in 0..*dim {
                if result[i] != v_arr[i] % p {
                    diffs.push((i, result[i],v_arr[i]));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    #[rstest_parametrize(p,  case(2), case(3), case(5), case(7))]
    fn test_add(p : u32){
        initialize_limb_bit_index_table(p);
        for dim in [10, 20, 70, 100, 1000].iter() {
            println!("p: {}, dim: {}", p, dim);
            let mut v = FpVector::new(p, *dim);
            let mut w = FpVector::new(p, *dim);
            let mut v_arr = random_vector(p, *dim);
            let w_arr = random_vector(p, *dim);
            let mut result = Vec::with_capacity(*dim);
            for i in 0..*dim {
                result.push(0);
            }
            v.pack(&v_arr);
            w.pack(&w_arr);
            v.add(&w, 1);
            v.unpack(&mut result);
            for i in 0..*dim {
                v_arr[i] = (v_arr[i] + w_arr[i]) % p;
            }
            let mut diffs = Vec::new();
            for i in 0..*dim {
                if result[i] != v_arr[i] {
                    diffs.push((i, result[i],v_arr[i]));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    #[rstest_parametrize(p,  case(2), case(3), case(5), case(7))]
    fn test_scale(p : u32){
        initialize_limb_bit_index_table(p);
        for dim in [10, 20, 70, 100, 1000].iter() {
            println!("p: {}, dim: {}", p, dim);
            let mut v = FpVector::new(p, *dim);
            let mut v_arr = random_vector(p, *dim);
            let mut result = Vec::with_capacity(*dim);
            let mut rng = rand::thread_rng();
            let c = rng.gen::<u32>() % p;
            for i in 0..*dim {
                result.push(0);
            }
            v.pack(&v_arr);
            v.scale(c);
            for i in 0..*dim {
                v_arr[i] = (v_arr[i] * c) % p;
            }
            v.unpack(&mut result);
            let mut diffs = Vec::new();
            for i in 0..*dim {
                if result[i] != v_arr[i] {
                    diffs.push((i, result[i],v_arr[i]));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    #[rstest_parametrize(p,  case(2), case(3), case(5), case(7))]
    fn test_entry(p : u32) {
        initialize_limb_bit_index_table(p);
        let dim_list = [10, 20, 70, 100, 1000];
        for dim in dim_list.iter() {
            let dim = *dim;
            let mut v = FpVector::new(p, dim);
            let v_arr = random_vector(p, dim);
            v.pack(&v_arr);
            let mut diffs = Vec::new();
            for i in 0..dim {
                if v.entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    #[rstest_parametrize(p,  case(2), case(3), case(5), case(7))]//
    fn test_entry_slice(p : u32) {
        initialize_limb_bit_index_table(p);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let mut v = FpVector::new(p, dim);
            let v_arr = random_vector(p, dim);
            v.pack(&v_arr);
            println!("v: {}", v);
            v.set_slice(slice_start, slice_end);
            println!("slice_start: {}, slice_end: {}, slice: {}", slice_start, slice_end, v);
            let mut diffs = Vec::new();
            for i in 0 .. v.dimension() {
                if v.entry(i) != v_arr[i + slice_start] {
                    diffs.push((i, v_arr[i+slice_start], v.entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    #[rstest_parametrize(p,  case(2), case(3), case(5), case(7))]
    fn test_set_entry(p : u32) {
        initialize_limb_bit_index_table(p);
        let dim_list = [10, 20, 70, 100, 1000];
        for dim in dim_list.iter() {
            let dim = *dim;
            let mut v = FpVector::new(p, dim);
            let v_arr = random_vector(p, dim);
            for i in 0..dim {
                v.set_entry(i, v_arr[i]);
            }
            let mut diffs = Vec::new();
            for i in 0..dim {
                if v.entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    #[rstest_parametrize(p,  case(2), case(3), case(5), case(7))]//
    fn test_set_entry_slice(p : u32) {
        initialize_limb_bit_index_table(p);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let mut v = FpVector::new(p, dim);
            v.set_slice(slice_start, slice_end);
            let slice_dim  = v.dimension();
            let v_arr = random_vector(p, slice_dim);
            for i in 0 .. slice_dim {
                v.set_entry(i, v_arr[i]);
            }
            // println!("slice_start: {}, slice_end: {}, slice: {}", slice_start, slice_end, v);
            let mut diffs = Vec::new();
            for i in 0 .. slice_dim {
                if v.entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    // Tests set_to_zero for a slice and also is_zero.
    #[rstest_parametrize(p,  case(2), case(3), case(5), case(7))]
    fn test_set_to_zero_slice(p : u32) {
        initialize_limb_bit_index_table(p);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            println!("slice_start : {}, slice_end : {}", slice_start, slice_end);
            let mut v_arr = random_vector(p, dim);
            v_arr[0] = 1; // make sure that v isn't zero
            let mut v = FpVector::new(p, dim);
            v.pack(&v_arr);
            v.set_slice(slice_start, slice_end);
            v.set_to_zero();
            assert!(v.is_zero());
            v.clear_slice();
            assert!(!v.is_zero()); // The first entry is 1, so it's not zero.
            let mut diffs = Vec::new();
            for i in 0..slice_start {
                if v.entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.entry(i)));
                }
            }
            for i in slice_start .. slice_end {
                if v.entry(i) != 0 {
                    diffs.push((i, 0, v.entry(i)));
                }
            }
            for i in slice_end..dim {
                if v.entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.entry(i)));
                }
            }
            assert_eq!(diffs, []);
            println!("{}", v);
        }
    }

    #[rstest_parametrize(p, case(2), case(3), case(5), case(7))]//
    fn test_add_slice_to_slice(p : u32) {
        println!("p : {}", p);
        initialize_limb_bit_index_table(p);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let v_arr = random_vector(p, dim);
            let mut v = FpVector::new(p, dim);
            v.pack(&v_arr);
            let w_arr = random_vector(p, dim);
            let mut w = FpVector::new(p, dim);
            w.pack(&w_arr);
            println!("slice_start : {}, slice_end : {}", slice_start, slice_end);
            println!("v : {}", v);
            println!("w : {}", w);
            v.set_slice(slice_start, slice_end);
            w.set_slice(slice_start, slice_end);
            println!("v : {}", v);
            println!("w : {}", w);
            v.add(&w, 1);
            v.clear_slice();
            println!("v : {}", v);
            let mut diffs = Vec::new();
            for i in 0..slice_start {
                if v.entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.entry(i)));
                }
            }
            for i in slice_start .. slice_end {
                if v.entry(i) != (v_arr[i] + w_arr[i]) % p {
                    diffs.push((i, (v_arr[i] + w_arr[i]) % p, v.entry(i)));
                }
            }
            for i in slice_end..dim {
                if v.entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    // Tests assign and Eq
    #[rstest_parametrize(p, case(2), case(3), case(5), case(7))]//
    fn test_assign(p : u32) {
        initialize_limb_bit_index_table(p);
        for dim in [10, 20, 70, 100, 1000].iter() {
            println!("p: {}, dim: {}", p, dim);
            let mut v = FpVector::new(p, *dim);
            let mut w = FpVector::new(p, *dim);
            let v_arr = random_vector(p, *dim);
            let w_arr = random_vector(p, *dim);
            let mut result = Vec::with_capacity(*dim);
            for i in 0..*dim {
                result.push(0);
            }
            v.pack(&v_arr);
            w.pack(&w_arr);
            v.assign(&w);
            assert_eq!(v, w);
            v.unpack(&mut result);
            let mut diffs = Vec::new();
            for i in 0..*dim {
                if result[i] != w_arr[i] {
                    diffs.push((i, w_arr[i], result[i]));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    #[rstest_parametrize(p, case(2), case(3), case(5), case(7))]//
    fn test_assign_slice_to_slice(p : u32) {
        println!("p : {}", p);
        initialize_limb_bit_index_table(p);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let mut v_arr = random_vector(p, dim);
            v_arr[0] = 1; // Ensure v != w.
            let mut v = FpVector::new(p, dim);
            v.pack(&v_arr);
            let mut w_arr = random_vector(p, dim);
            w_arr[0] = 0; // Ensure v != w.
            let mut w = FpVector::new(p, dim);
            w.pack(&w_arr);
            v.set_slice(slice_start, slice_end);
            w.set_slice(slice_start, slice_end);
            v.assign(&w);
            assert_eq!(v, w);
            v.clear_slice();
            w.clear_slice();
            assert!(v != w);
            let mut diffs = Vec::new();
            for i in 0..slice_start {
                if v.entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.entry(i)));
                }
            }
            for i in slice_start .. slice_end {
                if v.entry(i) != w_arr[i] {
                    diffs.push((i, w_arr[i], v.entry(i)));
                }
            }
            for i in slice_end..dim {
                if v.entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    #[rstest_parametrize(p, case(2), case(3), case(5), case(7))]
    fn test_add_shift_right(p : u32) {
        println!("p : {}", p);
        initialize_limb_bit_index_table(p);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let v_arr = random_vector(p, dim);
            let mut v = FpVector::new(p, dim);
            v.pack(&v_arr);
            let w_arr = random_vector(p, dim);
            let mut w = FpVector::new(p, dim);
            w.pack(&w_arr);
            println!("\n\n\n");
            println!("dim : {}, slice_start : {}, slice_end : {}", dim, slice_start, slice_end);
            println!("v : {}", v);
            println!("w : {}", w);
            v.set_slice(slice_start + 2, slice_end + 2);
            w.set_slice(slice_start, slice_end);
            println!("v : {}", v);
            println!("w : {}", w);
            v.shift_add(&w, 1);
            v.clear_slice();
            println!("v : {}", v);
            let mut diffs = Vec::new();
            for i in 0..slice_start + 2 {
                if v.entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.entry(i)));
                }
            }
            for i in slice_start + 2 .. slice_end + 2 {
                if v.entry(i) != (v_arr[i] + w_arr[i - 2]) % p {
                    diffs.push((i, (v_arr[i] + w_arr[i - 2]) % p, v.entry(i)));
                }
            }
            for i in slice_end  + 2 .. dim {
                if v.entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    #[rstest_parametrize(p, case(2), case(3), case(5), case(7))]
    fn test_add_shift_left(p : u32) {
        println!("p : {}", p);
        initialize_limb_bit_index_table(p);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let v_arr = random_vector(p, dim);
            let mut v = FpVector::new(p, dim);
            v.pack(&v_arr);
            let w_arr = random_vector(p, dim);
            let mut w = FpVector::new(p, dim);
            w.pack(&w_arr);
            println!("\n\n\n");
            println!("p : {}, dim : {}, slice_start : {}, slice_end : {}", p, dim, slice_start, slice_end);
            println!("entries_per_64 : {}, bits_per_entry : {}", entries_per_64_bits(p), bit_length(p));
            println!("v full: {}", v);
            println!("w full: {}", w);
            v.set_slice(slice_start - 2, slice_end - 2);
            w.set_slice(slice_start, slice_end);
            println!("v slice: {}", v);
            println!("w slice: {}", w);
            v.shift_add(&w, 1);
            println!("v resu: {}", v);
            v.clear_slice();
            println!("v resu: {}", v);
            let mut diffs = Vec::new();
            for i in 0..slice_start - 2 {
                if v.entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.entry(i)));
                }
            }
            for i in slice_start - 2 .. slice_end - 2 {
                if v.entry(i) != (v_arr[i] + w_arr[i + 2]) % p {
                    diffs.push((i, (v_arr[i] + w_arr[i + 2]) % p, v.entry(i)));
                }
            }
            for i in slice_end - 2 .. dim {
                if v.entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    #[rstest_parametrize(p, case(2), case(3), case(5), case(7))]
    fn test_iterator_slice(p : u32) {
        initialize_limb_bit_index_table(p);
        let ep = entries_per_64_bits(p);
        for dim in &[5, 10, ep, ep - 1, ep + 1, 3 * ep, 3 * ep - 1, 3 * ep + 1] {
            let mut v = FpVector::new(p, *dim);
            let v_arr = random_vector(p, *dim);
            v.pack(&v_arr);
            v.set_slice(3, dim - 1);

            let w = v.iter();
            let mut counter = 0;
            for (i, x) in w.enumerate() {
                println!("i: {}, dim : {}", i, dim);
                assert_eq!(v.entry(i), x);
                counter += 1;
            }
            assert_eq!(counter, v.dimension());
        }
    }

    #[rstest_parametrize(p, case(2), case(3), case(5), case(7))]
    fn test_iterator_skip(p : u32) {
        initialize_limb_bit_index_table(p);
        let ep = entries_per_64_bits(p);
        let dim = 5 * ep;
        for num_skip in &[ep, ep - 1, ep + 1, 3 * ep, 3 * ep - 1, 3 * ep + 1, 6 * ep] {
            let mut v = FpVector::new(p, dim);
            let v_arr = random_vector(p, dim);
            v.pack(&v_arr);

            let mut w = v.iter();
            w.skip_n(*num_skip);
            let mut counter = 0;
            for (i, x) in w.enumerate() {
                assert_eq!(v.entry(i + *num_skip), x);
                counter += 1;
            }
            if *num_skip != 6 * ep {
                assert_eq!(counter, v.dimension() - *num_skip);
            } else {
                assert_eq!(counter, 0);
            }
        }
    }

    #[rstest_parametrize(p, case(2), case(3), case(5), case(7))]
    fn test_iterator(p : u32) {
        initialize_limb_bit_index_table(p);
        let ep = entries_per_64_bits(p);
        for dim in &[0, 5, 10, ep, ep - 1, ep + 1, 3 * ep, 3 * ep - 1, 3 * ep + 1] {
            let mut v = FpVector::new(p, *dim);
            let v_arr = random_vector(p, *dim);
            v.pack(&v_arr);

            let w = v.iter();
            let mut counter = 0;
            for (i, x) in w.enumerate() {
                assert_eq!(v.entry(i), x);
                counter += 1;
            }
            assert_eq!(counter, v.dimension());
        }
    }

    #[test]
    fn test_masks() {
        test_mask(2, &[1, 0, 1, 1, 0], &[true, true, false, true, false]);
        test_mask(7, &[3, 2, 6, 4, 0, 6, 0], &[true, false, false, true, false, true, true]);
    }

    fn test_mask(p : u32, vec : &[u32], mask : &[bool]) {
        initialize_limb_bit_index_table(p);
        assert_eq!(vec.len(), mask.len());
        let mut v = FpVector::from_vec(p, vec);
        let mut m = FpVectorMask::new(p, vec.len());
        for (i, item) in mask.iter().enumerate() {
            m.set_mask(i, *item);
        }
        m.apply(&mut v);
        for (i, item) in v.iter().enumerate() {
            if mask[i] {
                assert_eq!(item, vec[i]);
            } else {
                assert_eq!(item, 0);
            }
        }
    }
}
