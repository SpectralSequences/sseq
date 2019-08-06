//
// Created by Hood on 5/22/2019.
//

use std::sync::Once;
use std::fmt;
use enum_dispatch::enum_dispatch;

use crate::combinatorics::valid_prime_q;
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

fn get_bit_length(p : u32) -> usize {
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

fn get_bitmask(p : u32) -> u64{
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

fn get_entries_per_64_bits(p : u32) -> usize {
    return ENTRIES_PER_64_BITS[PRIME_TO_INDEX_MAP[p as usize]];
}

struct LimbBitIndexPair {
    limb : usize,
    bit_index : usize
}

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

/**
 * Called by initializePrime
 * This table tells us which limb and which bitfield of that limb to look for a given index of
 * the vector in.
 */
pub fn initialize_limb_bit_index_table(p : u32){
    unsafe{
        LIMB_BIT_INDEX_ONCE_TABLE[PRIME_TO_INDEX_MAP[p as usize]].call_once(||{
            let entries_per_limb = get_entries_per_64_bits(p);
            let bit_length = get_bit_length(p);
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

fn get_limb_bit_index_pair(p : u32, idx : usize) -> &'static LimbBitIndexPair {
    let prime_idx = PRIME_TO_INDEX_MAP[p as usize];
    debug_assert!(valid_prime_q(p));
    debug_assert!(idx < MAX_DIMENSION);
    unsafe {
        let table = &LIMB_BIT_INDEX_TABLE[prime_idx];
        return table.as_ref().unwrap().get_unchecked(idx);
    }
}

#[enum_dispatch]
pub enum FpVector {
    FpVector2,
    FpVector3,
    FpVector5,
    FpVectorGeneric
}

#[enum_dispatch(FpVector)]
pub trait FpVectorT {
    fn reduce_limbs(&mut self, start_limb : usize, end_limb : usize );
    fn get_vector_container(&self) -> &VectorContainer;
    fn get_vector_container_mut(&mut self) -> &mut VectorContainer;
    fn get_prime(&self) -> u32;

    fn get_dimension(&self) -> usize {
        let container = self.get_vector_container();
        return container.slice_end - container.slice_start;
    }

    fn get_offset(&self) -> usize {
        let container = self.get_vector_container();
        let bit_length = get_bit_length(self.get_prime());
        let entries_per_64_bits = get_entries_per_64_bits(self.get_prime());
        return (container.offset + container.slice_start * bit_length) % (bit_length * entries_per_64_bits);
    }

    fn get_min_index(&self) -> usize {
        let container = self.get_vector_container();
        let bit_length = get_bit_length(self.get_prime());
        return container.offset/bit_length + container.slice_start;
    }

    fn get_slice(&self) -> (usize, usize) {
        let container = self.get_vector_container();
        return (container.slice_start, container.slice_end);
    }

    fn set_slice(&mut self, slice_start : usize, slice_end : usize) {
        let container = self.get_vector_container_mut();
        container.slice_end = container.slice_start + slice_end;
        container.slice_start += slice_start;
    }

    fn restore_slice(&mut self, slice : (usize, usize)) {
        let container = self.get_vector_container_mut();
        container.slice_start = slice.0;
        container.slice_end = slice.1;
    }

    fn clear_slice(&mut self) {
        let container = self.get_vector_container_mut();
        container.slice_start = 0;
        container.slice_end = container.dimension;
    }

    fn get_min_limb(&self) -> usize {
        let p = self.get_prime();
        let bit_length = get_bit_length(p);
        let container = self.get_vector_container();
        get_limb_bit_index_pair(p,container.offset/bit_length + container.slice_start).limb
    }

    fn get_max_limb(&self) -> usize {
        let p = self.get_prime();
        let bit_length = get_bit_length(p);
        let container = self.get_vector_container();
        if container.offset/bit_length + container.slice_end > 0{
            get_limb_bit_index_pair(p, container.offset/bit_length + container.slice_end - 1).limb + 1
        } else {
            0
        }
    }

    // Private

    fn get_limbs_cvec(&self) -> &Vec<u64> {
        &self.get_vector_container().limbs
    }

    fn get_limbs_cvec_mut(&mut self) -> &mut Vec<u64> {
        &mut self.get_vector_container_mut().limbs
    }

    fn get_limb_mask(&self, limb_idx : usize) -> u64 {
        let offset = self.get_offset();
        let min_limb = self.get_min_limb();
        let max_limb = self.get_max_limb();
        let number_of_limbs = max_limb - min_limb;
        let mut mask = !0;
        if limb_idx == 0 {
            mask <<= offset;
        }
        if limb_idx + 1 == number_of_limbs {
            let p = self.get_prime();
            let dimension = self.get_dimension();
            let bit_length = get_bit_length(p);
            let entries_per_64_bits = get_entries_per_64_bits(p);
            let bits_needed_for_entire_vector = offset + dimension * bit_length;
            let usable_bits_per_limb = bit_length * entries_per_64_bits;
            let bit_max = 1 + ((bits_needed_for_entire_vector - 1)%(usable_bits_per_limb));
            mask &= (!0) >> (64 - bit_max);
        }
        return mask;
    }
    fn set_to_zero(&mut self){
        let min_limb = self.get_min_limb();
        let max_limb = self.get_max_limb();
        let number_of_limbs = max_limb - min_limb;
        for i in 1..number_of_limbs-1 {
            let limbs = self.get_limbs_cvec_mut();
            limbs[min_limb + i] = 0;
        }
        let mut i = 0; {
            let mask = self.get_limb_mask(i);
            let limbs = self.get_limbs_cvec_mut();
            limbs[min_limb + i] &= !mask;
        }
        i = number_of_limbs - 1;
        if i > 0 {
            let mask = self.get_limb_mask(i);
            let limbs = self.get_limbs_cvec_mut();
            limbs[min_limb + i] &= !mask;
        }
    }

    fn assign(&mut self, other : &FpVector){
        let min_target_limb = self.get_min_limb();
        let max_target_limb = self.get_max_limb();
        let min_source_limb = other.get_min_limb();
        let number_of_limbs = max_target_limb - min_target_limb;
        assert_eq!(number_of_limbs, other.get_max_limb() - other.get_min_limb());
        assert!(self.get_offset() == other.get_offset());
        let target_limbs = self.get_limbs_cvec_mut();
        let source_limbs = other.get_limbs_cvec();
        for i in 1 .. number_of_limbs.saturating_sub(1) {
            target_limbs[min_target_limb + i] = source_limbs[min_source_limb + i];
        }
        let mut i=0; {
            let mask = other.get_limb_mask(i);
            let result = source_limbs[min_source_limb + i] & mask;
            target_limbs[min_target_limb + i] &= !mask;
            target_limbs[min_target_limb + i] |= result;
        }
        i = number_of_limbs - 1;
        if i > 0 {
            let mask = other.get_limb_mask(i);
            let result = source_limbs[min_source_limb + i] & mask;
            target_limbs[min_target_limb + i] &= !mask;
            target_limbs[min_target_limb + i] |= result;
        }
    }

    fn is_zero(&self) -> bool{
        let min_limb = self.get_min_limb();
        let max_limb = self.get_max_limb();
        let number_of_limbs = max_limb - min_limb;
        let limbs = self.get_limbs_cvec();
        for i in 1 .. number_of_limbs-1 {
            if limbs[min_limb + i] != 0 {
                return false;
            }
        }
        let mut i = 0; {
            let mask = self.get_limb_mask(i);
            if limbs[min_limb + i] & mask != 0 {
                return false;
            }
        }
        i = number_of_limbs - 1;
        if i > 0 {
            let mask = self.get_limb_mask(i);
            if limbs[min_limb + i] & mask != 0 {
                return false;
            }
        }
        return true
    }

    fn is_equal_to(&self, other : &FpVector) -> bool{
        let self_min_limb = self.get_min_limb();
        let self_max_limb = self.get_max_limb();
        let other_min_limb = other.get_min_limb();
        let other_max_limb = other.get_max_limb();
        let number_of_limbs = self_max_limb - self_min_limb;
        assert_eq!(other_max_limb - other_min_limb, number_of_limbs);
        let self_limbs = self.get_limbs_cvec();
        let other_limbs = other.get_limbs_cvec();
        for i in 1 .. number_of_limbs-1 {
            if self_limbs[self_min_limb + i] != other_limbs[other_min_limb + i] {
                return false;
            }
        }
        let mut i = 0; {
            let mask = self.get_limb_mask(i);
            let self_limb_masked = self_limbs[self_min_limb + i] & mask;
            let other_limb_masked = other_limbs[other_min_limb + i] & mask;
            if self_limb_masked != other_limb_masked {
                return false;
            }
        }
        i = number_of_limbs - 1;
        if i > 0 {
            let mask = self.get_limb_mask(i);
            let self_limb_masked = self_limbs[self_min_limb + i] & mask;
            let other_limb_masked = other_limbs[other_min_limb + i] & mask;
            if self_limb_masked != other_limb_masked {
                return false;
            }
        }
        return true;
    }

    fn get_entry(&self, index : usize) -> u32 {
        let p = self.get_prime();
        let bit_mask = get_bitmask(p);
        let limb_index = get_limb_bit_index_pair(p, index + self.get_min_index());
        let mut result = self.get_limbs_cvec()[limb_index.limb];
        result >>= limb_index.bit_index;
        result &= bit_mask;
        return result as u32;
    }

    fn set_entry(&mut self, index : usize, value : u32){
        let p = self.get_prime();
        let bit_mask = get_bitmask(p);
        let limb_index = get_limb_bit_index_pair(p, index + self.get_min_index());
        let limbs = self.get_limbs_cvec_mut();
        let mut result = limbs[limb_index.limb];
        result &= !(bit_mask << limb_index.bit_index);
        result |= (value as u64) << limb_index.bit_index;
        limbs[limb_index.limb] = result;
    }

    fn add_basis_element(&mut self, index : usize, value : u32){
        let mut x = self.get_entry(index);
        x += value;
        x = x % self.get_prime();
        self.set_entry(index, x);
    }

    fn unpack(&self, target : &mut [u32]){
        assert!(self.get_dimension() <= target.len());
        let p = self.get_prime();
        let dimension = self.get_dimension();
        let offset = self.get_offset();
        let limbs = self.get_limbs_cvec();
        let mut target_idx = 0;
        for i in 0..limbs.len() {
            target_idx += FpVector::unpack_limb(p, dimension, offset, &mut target[target_idx ..], limbs, i);
        }
    }

    fn pack(&mut self, source : &[u32]){
        assert!(self.get_dimension() <= source.len());
        let p = self.get_prime();
        let dimension = self.get_dimension();
        let offset = self.get_offset();
        let limbs = self.get_limbs_cvec_mut();
        let mut source_idx = 0;
        for i in 0..limbs.len() {
            source_idx += FpVector::pack_limb(p, dimension, offset, &source[source_idx ..], limbs, i);
        }
    }

    fn add(&mut self, other : &FpVector, c : u32){
        debug_assert!(self.get_prime() == other.get_prime());
        debug_assert!(self.get_offset() == other.get_offset());
        debug_assert!(self.get_dimension() == other.get_dimension());
        let p = self.get_prime();
        let min_target_limb = self.get_min_limb();
        let max_target_limb = self.get_max_limb();
        let min_source_limb = other.get_min_limb();
        let max_source_limb = other.get_max_limb();
        debug_assert!(max_source_limb - min_source_limb == max_target_limb - min_target_limb);
        let number_of_limbs = max_source_limb - min_source_limb;
        let target_limbs = self.get_limbs_cvec_mut();
        let source_limbs = other.get_limbs_cvec();
        for i in 1..number_of_limbs-1 {
            target_limbs[i + min_target_limb] = FpVector::add_limb(p, target_limbs[i + min_target_limb], source_limbs[i + min_source_limb], c);
        }
        let mut i = 0; {
            let mask = other.get_limb_mask(i);
            let source_limb_masked = source_limbs[min_source_limb + i] & mask;
            target_limbs[i + min_target_limb] = FpVector::add_limb(p, target_limbs[i + min_target_limb], source_limb_masked, c);
        }
        i = number_of_limbs - 1;
        if i > 0 {
            let mask = other.get_limb_mask(i);
            let source_limb_masked = source_limbs[min_source_limb + i] & mask;
            target_limbs[i + min_target_limb] = FpVector::add_limb(p, target_limbs[i + min_target_limb], source_limb_masked, c);
        }
        self.reduce_limbs(min_target_limb, max_target_limb);
    }

    fn scale(&mut self, c : u32){
        let c = c as u64;
        let min_limb = self.get_min_limb();
        let max_limb = self.get_max_limb();
        let number_of_limbs = max_limb - min_limb;
        for i in 1..number_of_limbs-1 {
            let limbs = self.get_limbs_cvec_mut();
            limbs[i + min_limb] *= c;
        }
        let mut i = 0; {
            let mask = self.get_limb_mask(i);
            let limbs = self.get_limbs_cvec_mut();
            let full_limb = limbs[min_limb + i];
            let masked_limb = full_limb & mask;
            let rest_limb = full_limb & !mask;
            limbs[i + min_limb] = (masked_limb * c) | rest_limb;
        }
        i = number_of_limbs - 1;
        if i > 0 {
            let mask = self.get_limb_mask(i);
            let limbs = self.get_limbs_cvec_mut();
            let full_limb = limbs[min_limb + i];
            let masked_limb = full_limb & mask;
            let rest_limb = full_limb & !mask;
            limbs[i + min_limb] = (masked_limb * c) | rest_limb;
        }
        self.reduce_limbs(min_limb, max_limb);
    }
}

pub struct VectorContainer {
    dimension : usize,
    offset : usize,
    slice_start : usize,
    slice_end : usize,
    limbs : Vec<u64>,
}

pub struct FpVector2 {
    vector_container : VectorContainer
}

pub struct FpVector3 {
    vector_container : VectorContainer
}

pub struct FpVector5 {
    vector_container : VectorContainer
}

pub struct FpVectorGeneric {
    p : u32,
    vector_container : VectorContainer
}

impl FpVectorT for FpVector2 {
    fn reduce_limbs(&mut self, _start_limb : usize, _end_limb : usize){}

    fn get_prime(&self) -> u32 { 2 }
    fn get_vector_container (&self) -> &VectorContainer { &self.vector_container }
    fn get_vector_container_mut (&mut self) -> &mut VectorContainer { &mut self.vector_container }
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

    fn get_prime (&self) -> u32 { 3 }
    fn get_vector_container (&self) -> &VectorContainer { &self.vector_container }
    fn get_vector_container_mut (&mut self) -> &mut VectorContainer { &mut self.vector_container }
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

    fn get_prime(&self) -> u32 { 5 }
    fn get_vector_container (&self) -> &VectorContainer { &self.vector_container }
    fn get_vector_container_mut (&mut self) -> &mut VectorContainer { &mut self.vector_container }
}


impl FpVectorT for FpVectorGeneric {
    fn reduce_limbs(&mut self, start_limb : usize, end_limb : usize){
        let entries_per_64_bits = get_entries_per_64_bits(self.p);
        let mut unpacked_limb = Vec::with_capacity(entries_per_64_bits);
        for _ in 0..entries_per_64_bits {
            unpacked_limb.push(0);
        }
        let p = self.p;
        let dimension = self.vector_container.dimension;
        let offset = self.vector_container.offset;
        let limbs = &mut self.vector_container.limbs;
        for i in start_limb..end_limb {
            FpVector::unpack_limb(p, dimension, offset, &mut unpacked_limb, limbs, i);
            for j in 0..unpacked_limb.len() {
                unpacked_limb[j] = unpacked_limb[j] % self.p;
            }
            FpVector::pack_limb(p, dimension, offset, &unpacked_limb, limbs, i);
        }
    }

    fn get_prime (&self) -> u32 { self.p }
    fn get_vector_container (&self) -> &VectorContainer { &self.vector_container }
    fn get_vector_container_mut (&mut self) -> &mut VectorContainer { &mut self.vector_container }
}

impl FpVector {
    pub fn new(p : u32, dimension : usize, offset : usize) -> FpVector {
        assert!(offset < 64);
        assert_eq!(offset % get_bit_length(p), 0);
        let slice_start = 0;
        let slice_end = dimension;
        let number_of_limbs = Self::get_number_of_limbs(p, dimension, offset);
        let limbs = vec![0; number_of_limbs];
        let vector_container = VectorContainer {dimension, offset, limbs, slice_start, slice_end };
        match p  {
            2 => FpVector::from(FpVector2 { vector_container }),
            3 => FpVector::from(FpVector3 { vector_container }),
            5 => FpVector::from(FpVector5 { vector_container }),
            _ => FpVector::from(FpVectorGeneric { p, vector_container })
        }
    }
    fn add_limb(p : u32, limb_a : u64, limb_b : u64, coeff : u32) -> u64 {
        match p {
           2 => limb_a ^ limb_b,
           _ => limb_a + (coeff as u64) * limb_b
        }
    }

    pub fn get_number_of_limbs(p : u32, dimension : usize, offset : usize) -> usize {
        assert!(dimension < MAX_DIMENSION);
        assert!(offset < 64);
        let bit_length = get_bit_length(p);
        if dimension == 0 {
            return 0;
        } else {
            return get_limb_bit_index_pair(p, dimension + offset/bit_length - 1).limb + 1;
        }
    }

    pub fn get_padded_dimension(p : u32, dimension : usize, offset : usize) -> usize {
        let entries_per_limb = get_entries_per_64_bits(p);
        let bit_length = get_bit_length(p);
        return ((dimension + offset/bit_length + entries_per_limb - 1)/entries_per_limb)*entries_per_limb;
    }

    pub fn iter(&self) -> FpVectorIterator{
        FpVectorIterator {
            vect : &self,
            dim : self.get_dimension(),
            index : 0
        }
    }

    fn pack_limb(p : u32, dimension : usize, offset : usize, limb_array : &[u32], limbs : &mut Vec<u64>, limb_idx : usize) -> usize {
        let bit_length = get_bit_length(p);
        assert_eq!(offset % bit_length, 0);
        let entries_per_64_bits = get_entries_per_64_bits(p);
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
        let bit_length = get_bit_length(p);
        let entries_per_64_bits = get_entries_per_64_bits(p);
        let bit_mask = get_bitmask(p);
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
    vect : &'a FpVector,
    dim : usize,
    index : usize
}


impl<'a> Iterator for FpVectorIterator<'a> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item>{
        if self.index < self.dim {
            let result = Some(self.vect.get_entry(self.index));
            self.index += 1;
            result
        } else {
            None
        }
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

#[cfg(test)]
extern crate rstest;

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
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
            let mut v = FpVector::new(p, *dim, 0);
            let v_arr = random_vector(p*(p-1), *dim);
            v.pack(&v_arr);
            v.reduce_limbs(v.get_min_limb(), v.get_max_limb());
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
            let mut v = FpVector::new(p, *dim, 0);
            let mut w = FpVector::new(p, *dim, 0);
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
            let mut v = FpVector::new(p, *dim, 0);
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
    fn test_get_entry(p : u32) {
        initialize_limb_bit_index_table(p);
        let dim_list = [10, 20, 70, 100, 1000];
        for dim in dim_list.iter() {
            let dim = *dim;
            let mut v = FpVector::new(p, dim, 0);
            let v_arr = random_vector(p, dim);
            v.pack(&v_arr);
            let mut diffs = Vec::new();
            for i in 0..dim {
                if v.get_entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.get_entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    #[rstest_parametrize(p,  case(2), case(3), case(5), case(7))]//
    fn test_get_entry_slice(p : u32) {
        initialize_limb_bit_index_table(p);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let mut v = FpVector::new(p, dim, 0);
            let v_arr = random_vector(p, dim);
            v.pack(&v_arr);
            println!("v: {}", v);
            v.set_slice(slice_start, slice_end);
            println!("slice_start: {}, slice_end: {}, slice: {}", slice_start, slice_end, v);
            let mut diffs = Vec::new();
            for i in 0 .. v.get_dimension() {
                if v.get_entry(i) != v_arr[i + slice_start] {
                    diffs.push((i, v_arr[i+slice_start], v.get_entry(i)));
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
            let mut v = FpVector::new(p, dim, 0);
            let v_arr = random_vector(p, dim);
            for i in 0..dim {
                v.set_entry(i, v_arr[i]);
            }
            let mut diffs = Vec::new();
            for i in 0..dim {
                if v.get_entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.get_entry(i)));
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
            let mut v = FpVector::new(p, dim, 0);
            v.set_slice(slice_start, slice_end);
            let slice_dim  = v.get_dimension();
            let v_arr = random_vector(p, slice_dim);
            for i in 0 .. slice_dim {
                v.set_entry(i, v_arr[i]);
            }
            // println!("slice_start: {}, slice_end: {}, slice: {}", slice_start, slice_end, v);
            let mut diffs = Vec::new();
            for i in 0 .. slice_dim {
                if v.get_entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.get_entry(i)));
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
            let mut v = FpVector::new(p, dim, 0);
            v.pack(&v_arr);
            v.set_slice(slice_start, slice_end);
            v.set_to_zero();
            assert!(v.is_zero());
            v.clear_slice();
            assert!(!v.is_zero()); // The first entry is 1, so it's not zero.
            let mut diffs = Vec::new();
            for i in 0..slice_start {
                if v.get_entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.get_entry(i)));
                }
            }
            for i in slice_start .. slice_end {
                if v.get_entry(i) != 0 {
                    diffs.push((i, 0, v.get_entry(i)));
                }
            }
            for i in slice_end..dim {
                if v.get_entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.get_entry(i)));
                }
            }
            assert_eq!(diffs, []);
            println!("{}", v);
        }
    }

    #[rstest_parametrize(p, case(2), case(3), case(5), case(7))]//
    fn test_add_to_slice(p : u32) {
        println!("p : {}", p);
        initialize_limb_bit_index_table(p);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let v_arr = random_vector(p, dim);
            let mut v = FpVector::new(p, dim, 0);
            v.pack(&v_arr);
            v.set_slice(slice_start, slice_end);
            let w_arr = random_vector(p, v.get_dimension());
            let mut w = FpVector::new(p, v.get_dimension(), v.get_offset());
            w.pack(&w_arr);
            v.add(&w, 1);
            v.clear_slice();
            let mut diffs = Vec::new();
            for i in 0..slice_start {
                if v.get_entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.get_entry(i)));
                }
            }
            for i in slice_start .. slice_end {
                if v.get_entry(i) != (v_arr[i] + w_arr[i - slice_start]) % p {
                    diffs.push((i, (v_arr[i] + w_arr[i - slice_start]) % p, v.get_entry(i)));
                }
            }
            for i in slice_end..dim {
                if v.get_entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.get_entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    #[rstest_parametrize(p, case(2), case(3), case(5), case(7))]//
    fn test_add_from_slice(p : u32) {
        println!("p : {}", p);
        initialize_limb_bit_index_table(p);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let v_arr = random_vector(p, dim);
            let mut v = FpVector::new(p, dim, 0);
            v.pack(&v_arr);
            v.set_slice(slice_start, slice_end);
            println!("slice_start : {}, slice_end : {}, v.get_dimension() : {}",slice_start, slice_end, v.get_dimension());
            let w_arr = random_vector(p, v.get_dimension());
            let mut w = FpVector::new(p, v.get_dimension(), v.get_offset());
            w.pack(&w_arr);
            w.add(&v, 1);
            v.clear_slice();
            let mut diffs = Vec::new();
            for i in 0..w.get_dimension() {
                let goal_value = (w_arr[i] + v_arr[i + slice_start]) % p;
                if w.get_entry(i) != goal_value {
                    diffs.push((i, goal_value, w.get_entry(i)));
                }
            }
            assert_eq!(diffs, []);
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
            let mut v = FpVector::new(p, dim, 0);
            v.pack(&v_arr);
            let w_arr = random_vector(p, dim);
            let mut w = FpVector::new(p, dim, 0);
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
                if v.get_entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.get_entry(i)));
                }
            }
            for i in slice_start .. slice_end {
                if v.get_entry(i) != (v_arr[i] + w_arr[i]) % p {
                    diffs.push((i, (v_arr[i] + w_arr[i]) % p, v.get_entry(i)));
                }
            }
            for i in slice_end..dim {
                if v.get_entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.get_entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    // Tests assign and is_equal_to
    #[rstest_parametrize(p, case(2), case(3), case(5), case(7))]//
    fn test_assign(p : u32) {
        initialize_limb_bit_index_table(p);
        for dim in [10, 20, 70, 100, 1000].iter() {
            println!("p: {}, dim: {}", p, dim);
            let mut v = FpVector::new(p, *dim, 0);
            let mut w = FpVector::new(p, *dim, 0);
            let v_arr = random_vector(p, *dim);
            let w_arr = random_vector(p, *dim);
            let mut result = Vec::with_capacity(*dim);
            for i in 0..*dim {
                result.push(0);
            }
            v.pack(&v_arr);
            w.pack(&w_arr);
            v.assign(&w);
            assert!(v.is_equal_to(&w));
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
    fn test_assign_from_slice(p : u32) {
        println!("p : {}", p);
        initialize_limb_bit_index_table(p);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let v_arr = random_vector(p, dim);
            let mut v = FpVector::new(p, dim, 0);
            v.pack(&v_arr);
            v.set_slice(slice_start, slice_end);
            println!("slice_start : {}, slice_end : {}, v.get_dimension() : {}",slice_start, slice_end, v.get_dimension());
            let w_arr = random_vector(p, v.get_dimension());
            let mut w = FpVector::new(p, v.get_dimension(), v.get_offset());
            w.pack(&w_arr);
            w.assign(&v);
            v.clear_slice();
            let mut diffs = Vec::new();
            for i in 0..w.get_dimension() {
                let goal_value = v_arr[i + slice_start];
                if w.get_entry(i) != goal_value {
                    diffs.push((i, goal_value, w.get_entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    #[rstest_parametrize(p, case(2), case(3), case(5), case(7))]//
    fn test_assign_to_slice(p : u32) {
        println!("p : {}", p);
        initialize_limb_bit_index_table(p);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let v_arr = random_vector(p, dim);
            let mut v = FpVector::new(p, dim, 0);
            v.pack(&v_arr);
            v.set_slice(slice_start, slice_end);
            let w_arr = random_vector(p, v.get_dimension());
            let mut w = FpVector::new(p, v.get_dimension(), v.get_offset());
            w.pack(&w_arr);
            v.assign(&w);
            v.clear_slice();
            let mut diffs = Vec::new();
            for i in 0..slice_start {
                if v.get_entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.get_entry(i)));
                }
            }
            for i in slice_start .. slice_end {
                if v.get_entry(i) != w_arr[i - slice_start] {
                    diffs.push((i, w_arr[i - slice_start], v.get_entry(i)));
                }
            }
            for i in slice_end..dim {
                if v.get_entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.get_entry(i)));
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
            let mut v = FpVector::new(p, dim, 0);
            v.pack(&v_arr);
            let mut w_arr = random_vector(p, dim);
            w_arr[0] = 0; // Ensure v != w.
            let mut w = FpVector::new(p, dim, 0);
            w.pack(&w_arr);
            v.set_slice(slice_start, slice_end);
            w.set_slice(slice_start, slice_end);
            v.assign(&w);
            assert!(v.is_equal_to(&w));
            v.clear_slice();
            w.clear_slice();
            assert!(!v.is_equal_to(&w));
            let mut diffs = Vec::new();
            for i in 0..slice_start {
                if v.get_entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.get_entry(i)));
                }
            }
            for i in slice_start .. slice_end {
                if v.get_entry(i) != w_arr[i] {
                    diffs.push((i, w_arr[i], v.get_entry(i)));
                }
            }
            for i in slice_end..dim {
                if v.get_entry(i) != v_arr[i] {
                    diffs.push((i, v_arr[i], v.get_entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }
}
