//
// Created by Hood on 5/22/2019.
//
use crate::combinatorics::valid_prime_q;
use crate::combinatorics::PRIME_TO_INDEX_MAP;
use crate::combinatorics::MAX_PRIME_INDEX;
use crate::memory;
use crate::memory::MemoryAllocator;
use crate::memory::CVec;

use std::fmt;

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

/**
 * Called by initializePrime
 * This table tells us which limb and which bitfield of that limb to look for a given index of
 * the vector in.
 */
pub fn initialize_limb_bit_index_table(p : u32){
    let entries_per_limb = get_entries_per_64_bits(p);
    let bit_length = get_bit_length(p);
    let mut table : Vec<LimbBitIndexPair> = Vec::with_capacity(MAX_DIMENSION);
    for i in 0 .. MAX_DIMENSION {
        table.push(LimbBitIndexPair{
            limb : i/entries_per_limb,
            bit_index : (i % entries_per_limb) * bit_length,
        })
    }
    unsafe {
        LIMB_BIT_INDEX_TABLE[PRIME_TO_INDEX_MAP[p as usize]] = Some(table);
    }
}

fn get_limb_bit_index_pair(p : u32, idx : usize) -> &'static LimbBitIndexPair {
    let prime_idx = PRIME_TO_INDEX_MAP[p as usize];
    assert!(valid_prime_q(p));
    assert!(idx < MAX_DIMENSION);
    unsafe{
        if let Some(table) = &LIMB_BIT_INDEX_TABLE[prime_idx] {
            &table[idx]
        } else {
            assert!(false);
            &LimbBitIndexPair {limb:0,bit_index:0}
        }
    }
}

mod fp_vector_minimal {
    use crate::fp_vector::*;
    pub trait FpVectorMinimal {
        fn get_prime_minimal(&self) -> u32;
        fn get_dimension_minimal(&self) -> usize;
        fn get_offset_minimal(&self) -> usize;
        
        fn add_minimal(&mut self, other: &Self, coeff : u32){
            // println!("add_minimal");
            assert!(self.get_prime_minimal() == other.get_prime_minimal());
            assert!(self.get_offset_minimal() == other.get_offset_minimal());          
            assert!(self.get_dimension_minimal() == other.get_dimension_minimal());
            let p = self.get_prime_minimal();
            let offset = self.get_offset_minimal();
            let dimension = self.get_dimension_minimal();
            let bit_length = get_bit_length(p);
            let number_of_limbs = self.get_limbs_cvec_mut().len();
            // println!("number_of_limbs: {}", number_of_limbs);
            for (i, (d, s)) in self.get_limbs_cvec_mut().iter_mut().zip(other.get_limbs_cvec().iter()).enumerate(){
                let mut source_limb = *s;
                if i==0 {
                    source_limb &= !((1<<offset) - 1);
                }                
                if i == number_of_limbs - 1 {
                    let mut bit_mask = !0;
                    let bit_max = ((offset + dimension*bit_length) % 64) as u64;
                    if bit_max > 0 {
                        bit_mask = (1<<bit_max) - 1;
                    }
                    // println!("{:b}", bit_mask);
                    source_limb &= bit_mask;
                }
                *d += (coeff as u64) * source_limb;
            }
            self.reduce();
        }
        
        fn scale_minimal(&mut self, coeff : u32){
            for d in self.get_limbs_cvec_mut().iter_mut(){
                *d *= coeff as u64;
            }
            self.reduce();
        }

        // Private
        fn get_limbs_cvec(&self) -> &CVec<u64>;
        fn get_limbs_cvec_mut(&mut self) -> &mut CVec<u64>;
        fn reduce(&mut self);

        fn unpack_limb(&self, limb_array : &mut [u32], limb_idx : usize) -> usize {
            let p = self.get_prime_minimal();
            let bit_length = get_bit_length(p);
            let entries_per_64_bits = get_entries_per_64_bits(p);
            let bit_mask = get_bitmask(p);    
            let mut bit_min = 0usize;
            let mut bit_max = bit_length * entries_per_64_bits;    
            if limb_idx == 0 {
                bit_min = self.get_offset_minimal();
            }
            let limbs = self.get_limbs_cvec();
            if limb_idx == limbs.len() - 1 {
                // Calculates how many bits of the last field we need to use. But if it divides
                // perfectly, we want bit max equal to bit_length * entries_per_64_bits, so subtract and add 1.
                // to make the output in the range 1 -- bit_length * entries_per_64_bits.
                let bits_needed_for_entire_vector = self.get_offset_minimal() + self.get_dimension_minimal() * bit_length;
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

        fn pack_limb(&mut self, limb_array : &[u32], limb_idx : usize) -> usize {
            let p = self.get_prime_minimal();
            let bit_length = get_bit_length(p);
            let entries_per_64_bits = get_entries_per_64_bits(p);
            let bit_mask = get_bitmask(p);    
            let offset = self.get_offset_minimal();
            let dimension = self.get_dimension_minimal();
            let mut bit_min = 0usize;
            let mut bit_max = bit_length * entries_per_64_bits;    
            if limb_idx == 0 {
                bit_min = self.get_offset_minimal();
            }
            let limbs = self.get_limbs_cvec_mut();
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
    }
}
use fp_vector_minimal::FpVectorMinimal;

pub trait FpVectorTrait {
    fn get_prime(&self) -> u32;
    fn get_dimension(&self) -> usize;
    fn get_offset(&self) -> usize;

    fn set_to_zero(&mut self);
    fn get_entry(&self, index : usize ) -> u32;
    fn set_entry(&mut self, index : usize, value : u32);
    fn add_basis_element(&mut self, index : usize, value : u32);

    fn zeroq(&self) -> bool;
    fn equalq(&self, other : &Self) -> bool;

    fn assign(&mut self, other : &Self);
    fn slice(&mut self, min_idx : usize, max_idx : usize) -> FpVector;

    fn unpack(&self, target : &mut [u32]);
    fn pack(&mut self, source : &[u32]);    

    fn add(&mut self, other: &Self, coeff : u32);
    fn scale(&mut self, coeff : u32);

}

impl<T> FpVectorTrait for T where T: FpVectorMinimal {
    fn get_prime(&self) -> u32 {
        self.get_prime_minimal()
    }

    fn get_dimension(&self) -> usize {
        self.get_dimension_minimal()
    }

    fn get_offset(&self) -> usize {
        self.get_offset_minimal()
    }

    fn set_to_zero(&mut self){ 
        for limb in self.get_limbs_cvec_mut().iter_mut() {
            *limb = 0;
        }
    }

    fn assign(&mut self, other : &Self){
        for (d, s) in self.get_limbs_cvec_mut().iter_mut().zip(other.get_limbs_cvec().iter()){
            *d = *s
        }
    }    

    fn zeroq(&self) -> bool{
        for limb in self.get_limbs_cvec().iter(){
            if *limb != 0u64 {
                return false
            }
        }
        return true
    }

    fn equalq(&self, other : &Self) -> bool{
        self.get_limbs_cvec().to_slice() == other.get_limbs_cvec().to_slice()
    }

    fn get_entry(&self, index : usize) -> u32 {
        let p = self.get_prime();   
        let bit_length = get_bit_length(p);
        let bit_mask = get_bitmask(p);
        let limb_index = get_limb_bit_index_pair(p, index + self.get_offset()/bit_length);
        let mut result = self.get_limbs_cvec()[limb_index.limb];
        result >>= limb_index.bit_index;
        result &= bit_mask;
        return result as u32;
    }

    fn set_entry(&mut self, index : usize, value : u32){
        let p = self.get_prime();   
        let bit_length = get_bit_length(p);
        let bit_mask = get_bitmask(p);
        let limb_index = get_limb_bit_index_pair(p, index + self.get_offset()/bit_length);
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
        let mut target_idx = 0;
        for i in 0..self.get_limbs_cvec().len() {
            target_idx += self.unpack_limb(&mut target[target_idx ..], i);
        }
    }

    fn pack(&mut self, source : &[u32]){
        assert!(self.get_dimension() <= source.len());
        let mut source_idx = 0;
        for i in 0..self.get_limbs_cvec().len() {
            source_idx += self.pack_limb(&source[source_idx ..], i);
        }
    }    

    // TODO: Do we need to wrap our data in UnsafeCell in order to prevent aliasing assumptions 
    // from making slice lead to undefined behavior?
    // How does aliasing work in Rust? Why does it have to be so hard?
    fn slice(&mut self, start : usize, end : usize) -> FpVector {
        assert!(start <= end);
        assert!(end <= self.get_dimension());
        let p = self.get_prime();
        let dimension = end - start;
        let mut ptr : *mut u64 = std::ptr::null::<u64>() as *mut u64;
        if dimension == 0 {
            let vector_container = VectorContainer {
                dimension,
                offset : 0,
                limbs : CVec::from_parts(ptr, 0, None)
            };
            return wrap_container(p, vector_container);
        }
        let bit_length = get_bit_length(p);
        let limb_index = get_limb_bit_index_pair(p, start + self.get_offset_minimal()/bit_length);
        let number_of_limbs = get_limb_bit_index_pair(p, dimension + self.get_offset_minimal()/bit_length - 1).limb + 1;
        let offset = limb_index.bit_index;
        unsafe{
            ptr = self.get_limbs_cvec_mut().get_ptr().offset(limb_index.limb as isize);
        }
        let vector_container = VectorContainer {
            dimension,
            offset,
            limbs : CVec::from_parts(ptr, number_of_limbs, None)
        };
        return wrap_container(p, vector_container);
    }

    fn add(&mut self, other: &Self, coeff : u32){
        self.add_minimal(other, coeff);
    }
    
    fn scale(&mut self, coeff : u32){
        self.scale_minimal(coeff);
    }
}

struct VectorContainer {
    dimension : usize, // These have to match the definition of Vector in FpVector.h
    offset : usize,
// Private fields:
    limbs : memory::CVec<u64>
}

pub struct VectorContainerGeneric {
    p : u32,
    vector_container : VectorContainer
}

pub struct VectorContainer2 {
    vector_container : VectorContainer
}

pub struct VectorContainer3 {
    vector_container : VectorContainer
}

pub struct VectorContainer5 {
    vector_container : VectorContainer
}

fn wrap_container(p : u32, vector_container : VectorContainer) -> FpVector {
    match p {
        2 => FpVector::Vector2(VectorContainer2 { vector_container }),
        3 => FpVector::Vector3(VectorContainer3 { vector_container }),
        5 => FpVector::Vector5(VectorContainer5 { vector_container }),
        _ => FpVector::VectorGeneric(VectorContainerGeneric { p, vector_container })
    }
}

// The only function that isn't boilerplate is reduce
impl FpVectorMinimal for VectorContainer2 {
    fn get_prime_minimal(&self) -> u32 {
        2
    }

    fn reduce(&mut self){}

    fn add_minimal(&mut self, other: &Self, coeff : u32){
        assert!(self.get_prime_minimal() == other.get_prime_minimal());
        assert!(self.get_offset_minimal() == other.get_offset_minimal());          
        assert!(self.get_dimension_minimal() == other.get_dimension_minimal());
        let p = self.get_prime_minimal();
        let offset = self.get_offset_minimal();
        let dimension = self.get_dimension_minimal();
        let bit_length = get_bit_length(p);
        let number_of_limbs = self.get_limbs_cvec_mut().len();
        // println!("number_of_limbs: {}", number_of_limbs);
        for (i, (d, s)) in self.get_limbs_cvec_mut().iter_mut().zip(other.get_limbs_cvec().iter()).enumerate(){
            let mut source_limb = *s;
            if i==0 {
                source_limb &= !((1<<offset) - 1);
            }                
            if i == number_of_limbs - 1 {
                let mut bit_mask = !0;
                let bit_max = ((offset + dimension*bit_length) % 64) as u64;
                if bit_max > 0 {
                    bit_mask = (1<<bit_max) - 1;
                }
                // println!("{:b}", bit_mask);
                source_limb &= bit_mask;
            }
            *d ^= source_limb;
        }
    }

    fn scale_minimal(&mut self, coeff : u32){}

    // The rest is boilerplate
    fn get_dimension_minimal(&self) -> usize {
        self.vector_container.dimension
    }


    fn get_offset_minimal(&self) -> usize {
        self.vector_container.offset
    }

    fn get_limbs_cvec(&self) -> &CVec<u64> {
        &self.vector_container.limbs
    }

    fn get_limbs_cvec_mut(&mut self) -> &mut CVec<u64> {
        &mut self.vector_container.limbs
    }
}

impl FpVectorMinimal for VectorContainer3 {
    fn get_prime_minimal(&self) -> u32 {
        3
    }

    fn reduce(&mut self){
        let top_bit_set_in_each_field = 0x4924924924924924u64;        
        for limb in self.vector_container.limbs.iter_mut() {
            *limb = ((*limb & top_bit_set_in_each_field) >> 2) + (*limb & (!top_bit_set_in_each_field));
            let mut limb_3s = *limb & (*limb >> 1);
            limb_3s |= limb_3s << 1;
            *limb ^= limb_3s;
        }
    }

    // The rest is boilerplate
    fn get_dimension_minimal(&self) -> usize {
        self.vector_container.dimension
    }

    fn get_offset_minimal(&self) -> usize {
        self.vector_container.offset
    }
    
    fn get_limbs_cvec(&self) -> &CVec<u64> {
        &self.vector_container.limbs
    }

    fn get_limbs_cvec_mut(&mut self) -> &mut CVec<u64> {
        &mut self.vector_container.limbs
    }
}


impl FpVectorMinimal for VectorContainer5 {
    fn get_prime_minimal(&self) -> u32 {
        5
    }
    
    fn reduce(&mut self){
        let top_bit_set_in_each_field = 0x4924924924924924u64;        
        for limb in self.vector_container.limbs.iter_mut() {
            let bottom_bit = 0x84210842108421u64;
            let bottom_two_bits = bottom_bit | (bottom_bit << 1);
            let bottom_three_bits = bottom_bit | (bottom_two_bits << 1);
            let a = (*limb >> 2) & bottom_three_bits;
            let b = *limb & bottom_two_bits;
            let m = (bottom_bit << 3) - a + b;
            let mut c = (m >> 3) & bottom_bit;
            c |= c << 1;
            let d = m & bottom_three_bits;
            *limb = d + c - bottom_two_bits;
        }
    }

    // The rest is boilerplate
    fn get_dimension_minimal(&self) -> usize {
        self.vector_container.dimension
    }

    fn get_offset_minimal(&self) -> usize {
        self.vector_container.offset
    }
    
    // Private
    fn get_limbs_cvec(&self) -> &CVec<u64> {
        &self.vector_container.limbs
    }

    fn get_limbs_cvec_mut(&mut self) -> &mut CVec<u64> {
        &mut self.vector_container.limbs
    }
}


impl FpVectorMinimal for VectorContainerGeneric {
    fn get_prime_minimal(&self) -> u32 {
        self.p
    }

    fn reduce(&mut self){
        let entries_per_64_bits = get_entries_per_64_bits(self.p);       
        let mut unpacked_limb = Vec::with_capacity(entries_per_64_bits);
        for i in 0..entries_per_64_bits {
            unpacked_limb.push(0);
        }
        let limbs = self.get_limbs_cvec();
        for i in 0..limbs.len() {
            self.unpack_limb(&mut unpacked_limb, i);
            for j in 0..entries_per_64_bits {
                unpacked_limb[i] = unpacked_limb[i] % self.p;
            }
            self.pack_limb(&unpacked_limb, i);
        }
    }

    // The rest is boilerplate
    fn get_dimension_minimal(&self) -> usize {
        self.vector_container.dimension
    }

    fn get_offset_minimal(&self) -> usize {
        self.vector_container.offset
    }
    
    // Private
    fn get_limbs_cvec(&self) -> &CVec<u64> {
        &self.vector_container.limbs
    }

    fn get_limbs_cvec_mut(&mut self) -> &mut CVec<u64> {
        &mut self.vector_container.limbs
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

pub enum FpVector {
    Vector2(VectorContainer2),
    Vector3(VectorContainer3),
    Vector5(VectorContainer5),
    VectorGeneric(VectorContainerGeneric),
}

impl FpVectorMinimal for FpVector {
    fn get_prime_minimal(&self) -> u32 {
        match self {
            FpVector::Vector2(v) => v.get_prime(),
            FpVector::Vector3(v) => v.get_prime(),
            FpVector::Vector5(v) => v.get_prime(),
            FpVector::VectorGeneric(v) => v.get_prime(),
        }
    }

    fn reduce(&mut self){
        match self {
            FpVector::Vector2(v) => v.reduce(),
            FpVector::Vector3(v) => v.reduce(),
            FpVector::Vector5(v) => v.reduce(),
            FpVector::VectorGeneric(v) => v.reduce(),
        }
    }

    fn add_minimal(&mut self, other : &Self, c : u32){
        match self {
            FpVector::Vector2(v) => {
                if let FpVector::Vector2(o) = other {
                    v.add_minimal(o, c);
                } else {
                    assert!(false);
                }
            }
            FpVector::Vector3(v) => {
                if let FpVector::Vector3(o) = other {
                    v.add_minimal(o, c);
                } else {
                    assert!(false);
                }
            }
            FpVector::Vector5(v) => {
                if let FpVector::Vector5(o) = other {
                    v.add_minimal(o, c);
                } else {
                    assert!(false);
                }
            }
            FpVector::VectorGeneric(v) => {
                if let FpVector::VectorGeneric(o) = other {
                    v.add_minimal(o, c);
                } else {
                    assert!(false);
                }
            }
        }
    }

    fn scale_minimal(&mut self, c : u32){
        match self {
            FpVector::Vector2(v) => v.scale_minimal(c),
            FpVector::Vector3(v) => v.scale_minimal(c),
            FpVector::Vector5(v) => v.scale_minimal(c),
            FpVector::VectorGeneric(v) => v.scale_minimal(c),
        }
    }

    fn get_dimension_minimal(&self) -> usize {
        match self {
            FpVector::Vector2(v) => v.get_dimension(),
            FpVector::Vector3(v) => v.get_dimension(),
            FpVector::Vector5(v) => v.get_dimension(),
            FpVector::VectorGeneric(v) => v.get_dimension(),
        }
    }

    fn get_offset_minimal(&self) -> usize {
        match self {
            FpVector::Vector2(v) => v.get_offset(),
            FpVector::Vector3(v) => v.get_offset(),
            FpVector::Vector5(v) => v.get_offset(),
            FpVector::VectorGeneric(v) => v.get_offset(),
        }
    }
    
    // Private

    fn get_limbs_cvec(&self) -> &CVec<u64> {
        match self {
            FpVector::Vector2(v) => v.get_limbs_cvec(),
            FpVector::Vector3(v) => v.get_limbs_cvec(),
            FpVector::Vector5(v) => v.get_limbs_cvec(),
            FpVector::VectorGeneric(v) => v.get_limbs_cvec(),
        }
    }

    fn get_limbs_cvec_mut(&mut self) -> &mut CVec<u64> {
        match self {
            FpVector::Vector2(v) => v.get_limbs_cvec_mut(),
            FpVector::Vector3(v) => v.get_limbs_cvec_mut(),
            FpVector::Vector5(v) => v.get_limbs_cvec_mut(),
            FpVector::VectorGeneric(v) => v.get_limbs_cvec_mut(),
        }
    }

}

impl FpVector {
    pub fn new(p : u32, dimension : usize, offset : usize) -> FpVector {
        assert!(offset < 64);
        let number_of_limbs = get_number_of_limbs(p, dimension, offset);
        let mut limbs_inner : Vec<u64> = Vec::with_capacity(number_of_limbs);
        for i in 0..number_of_limbs {
            limbs_inner.push(0);
        }
        let limbs = memory::CVec::from_vec(limbs_inner);
        let vector_container = VectorContainer {dimension, offset, limbs };
        wrap_container(p, vector_container)
    }

    pub fn new_from_allocator<T : MemoryAllocator>(allocator : &T, p : u32, dimension : usize, offset : usize) -> FpVector {
        assert!(offset < 64);
        let number_of_limbs = get_number_of_limbs(p, dimension, offset);
        let limbs = allocator.alloc_vec(number_of_limbs);
        let vector_container = VectorContainer {dimension, offset, limbs };        
        wrap_container(p, vector_container)
    }

    pub fn get_padded_dimension(p : u32, dimension : usize, offset : usize) -> usize {
        let entries_per_limb = get_entries_per_64_bits(p);
        let bit_length = get_bit_length(p);
        return ((dimension + offset/bit_length + entries_per_limb - 1)/entries_per_limb)*entries_per_limb;
    }
}

impl FpVector {
    pub fn iter(&self) -> FpVectorIterator{
        FpVectorIterator {
            vect : &self,
            index : 0
        }
    }
}
pub struct FpVectorIterator<'a> {
    vect : &'a FpVector,
    index : usize
}


impl<'a> Iterator for FpVectorIterator<'a> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item>{
        if self.index < self.vect.get_dimension() {
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