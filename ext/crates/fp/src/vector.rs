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

use itertools::Itertools;
use std::cmp::Ordering;
use std::sync::Once;
use std::fmt;
use std::hash::{Hash, Hasher};
#[cfg(feature = "json")]
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use enum_dispatch::enum_dispatch;

use crate::prime::ValidPrime;
use crate::prime::PRIME_TO_INDEX_MAP;
use crate::prime::NUM_PRIMES;

pub const MAX_DIMENSION : usize = 147500;

// Generated with Mathematica:
//     bitlengths = Prepend[#,1]&@ Ceiling[Log2[# (# - 1) + 1 &[Prime[Range[2, 54]]]]]
// But for 2 it should be 1.
static BIT_LENGHTS : [usize; NUM_PRIMES] = [
     1, 3, 5, 6, 7, 8, 9, 9
];

pub fn bit_length(p : ValidPrime) -> usize {
    BIT_LENGHTS[PRIME_TO_INDEX_MAP[*p as usize]]
}

// This is 2^bitlength - 1.
// Generated with Mathematica:
//     2^bitlengths-1
static BITMASKS : [u32; NUM_PRIMES] = [
    1, 7, 31, 63, 127, 255, 511, 511
];

pub fn bitmask(p : ValidPrime) -> u64{
    BITMASKS[PRIME_TO_INDEX_MAP[*p as usize]] as u64
}

// This is floor(64/bitlength).
// Generated with Mathematica:
//      Floor[64/bitlengths]
static ENTRIES_PER_64_BITS : [usize;NUM_PRIMES] = [
    64, 21, 12, 10, 9, 8, 7, 7
];

pub fn entries_per_64_bits(p : ValidPrime) -> usize {
    ENTRIES_PER_64_BITS[PRIME_TO_INDEX_MAP[*p as usize]]
}

#[derive(Copy, Clone)]
struct LimbBitIndexPair {
    limb : usize,
    bit_index : usize
}

/// This table tells us which limb and which bitfield of that limb to look for a given index of
/// the vector in.
static mut LIMB_BIT_INDEX_TABLE : [Option<Vec<LimbBitIndexPair>>; NUM_PRIMES] = [
    None,None,None,None,None,None,None,None
];

static mut LIMB_BIT_INDEX_ONCE_TABLE : [Once; NUM_PRIMES] = [
    Once::new(),Once::new(), Once::new(), Once::new(), Once::new(),
    Once::new(),Once::new(), Once::new()
];

pub fn initialize_limb_bit_index_table(p : ValidPrime){
    if *p == 2 {
        return;
    }
    unsafe {
        LIMB_BIT_INDEX_ONCE_TABLE[PRIME_TO_INDEX_MAP[*p as usize]].call_once(||{
            let entries_per_limb = entries_per_64_bits(p);
            let bit_length = bit_length(p);
            let mut table : Vec<LimbBitIndexPair> = Vec::with_capacity(MAX_DIMENSION);
            for i in 0 .. MAX_DIMENSION {
                table.push(LimbBitIndexPair{
                    limb : i/entries_per_limb,
                    bit_index : (i % entries_per_limb) * bit_length,
                })
            }
            LIMB_BIT_INDEX_TABLE[PRIME_TO_INDEX_MAP[*p as usize]] = Some(table);
        });
    }
}

fn limb_bit_index_pair(p : ValidPrime, idx : usize) -> LimbBitIndexPair {
    match *p {
        2 => { LimbBitIndexPair
            {
                limb : idx/64,
                bit_index : idx % 64,
            }
        },
        _ => {
            let prime_idx = PRIME_TO_INDEX_MAP[*p as usize];
            debug_assert!(idx < MAX_DIMENSION);
            unsafe {
                let table = &LIMB_BIT_INDEX_TABLE[prime_idx];
                debug_assert!(table.is_some());
                *table.as_ref().unwrap_or_else(|| std::hint::unreachable_unchecked()).get_unchecked(idx)
            }
        }
    }
}

#[enum_dispatch]
#[derive(Debug, Clone)]
#[cfg(not(feature = "prime-two"))]
pub enum FpVector {
    FpVector2,
    FpVector3,
    FpVector5,
    FpVectorGeneric
}

#[enum_dispatch]
#[derive(Debug, Clone)]
#[cfg(feature = "prime-two")]
pub enum FpVector {
    FpVector2,
}


struct AddShiftNoneData {
    min_source_limb : usize,
    min_target_limb : usize,
    number_of_limbs : usize
}

impl AddShiftNoneData {
    fn new(target : &(impl FpVectorT + ?Sized), source : &(impl FpVectorT + ?Sized)) -> Self {
        debug_assert_eq!(target.prime(), source.prime());
        debug_assert_eq!(target.offset(), source.offset());
        debug_assert_eq!(target.dimension(), source.dimension(), "Adding vectors of different dimensions");
        let min_target_limb = target.min_limb();
        let max_target_limb = target.max_limb();
        let min_source_limb = source.min_limb();
        let max_source_limb = source.max_limb();
        debug_assert!(max_source_limb - min_source_limb == max_target_limb - min_target_limb);
        let number_of_limbs = max_source_limb - min_source_limb;
        Self {
            min_target_limb,
            min_source_limb,
            number_of_limbs
        }
    }

    fn mask_first_limb(&self, other : &(impl FpVectorT + ?Sized), i : usize) -> u64 {
        other.limbs()[self.min_source_limb + i] & other.limb_mask(i)
    }

    fn mask_middle_limb(&self, other : &(impl FpVectorT + ?Sized), i : usize) -> u64 {
        other.limbs()[self.min_source_limb + i]
    }

    fn mask_last_limb(&self, other : &(impl FpVectorT + ?Sized), i : usize) -> u64 {
        other.limbs()[self.min_source_limb + i] & other.limb_mask(i)
    }
}

struct AddShiftLeftData {
    offset_shift : usize,
    tail_shift : usize,
    zero_bits : usize,
    min_source_limb : usize,
    min_target_limb : usize,
    number_of_source_limbs : usize,
    number_of_target_limbs : usize
}

impl AddShiftLeftData {
    fn new(target : &(impl FpVectorT + ?Sized), source : &(impl FpVectorT + ?Sized)) -> Self {
        debug_assert!(target.prime() == source.prime());
        debug_assert!(target.offset() <= source.offset());
        debug_assert!(target.dimension() == source.dimension(),
            "self.dim {} not equal to other.dim {}", target.dimension(), source.dimension());
        let p = target.prime();
        let offset_shift = source.offset() - target.offset();
        let bit_length = bit_length(p);
        let entries_per_64_bits = entries_per_64_bits(p);
        let usable_bits_per_limb = bit_length * entries_per_64_bits;
        let tail_shift = usable_bits_per_limb - offset_shift;
        let zero_bits = 64 - usable_bits_per_limb;
        let min_target_limb = target.min_limb();
        let max_target_limb = target.max_limb();
        let min_source_limb = source.min_limb();
        let max_source_limb = source.max_limb();
        let number_of_source_limbs = max_source_limb - min_source_limb;
        let number_of_target_limbs = max_target_limb - min_target_limb;

        Self {
            offset_shift,
            tail_shift,
            zero_bits,
            min_source_limb,
            min_target_limb,
            number_of_source_limbs,
            number_of_target_limbs
        }
    }

    fn mask_first_limb(&self, other : &(impl FpVectorT + ?Sized), i : usize) -> u64 {
        (other.limbs()[self.min_source_limb + i] & other.limb_mask(i)) >> self.offset_shift
    }

    fn mask_middle_limb_a(&self, other : &(impl FpVectorT + ?Sized), i : usize) -> u64 {
        other.limbs()[i + self.min_source_limb] >> self.offset_shift
    }

    fn mask_middle_limb_b(&self, other : &(impl FpVectorT + ?Sized), i : usize) -> u64 {
        (other.limbs()[i + self.min_source_limb] << (self.tail_shift + self.zero_bits)) >> self.zero_bits
    }

    fn mask_last_limb_a(&self, other : &(impl FpVectorT + ?Sized), i : usize) -> u64 {
        let mask = other.limb_mask(i);
        let source_limb_masked = other.limbs()[self.min_source_limb + i] & mask;
        source_limb_masked << self.tail_shift
    }

    fn mask_last_limb_b(&self, other : &(impl FpVectorT + ?Sized), i : usize) -> u64 {
        let mask = other.limb_mask(i);
        let source_limb_masked = other.limbs()[self.min_source_limb + i] & mask;
        source_limb_masked >> self.offset_shift
    }
}

struct AddShiftRightData {
    offset_shift : usize,
    tail_shift : usize,
    zero_bits : usize,
    min_source_limb : usize,
    min_target_limb : usize,
    number_of_source_limbs : usize,
    number_of_target_limbs : usize
}


impl AddShiftRightData {
    fn new(target : &(impl FpVectorT + ?Sized), source : &(impl FpVectorT + ?Sized)) -> Self {
        debug_assert!(target.prime() == source.prime());
        debug_assert!(target.offset() >= source.offset());
        debug_assert!(target.dimension() == source.dimension(),
            "self.dim {} not equal to other.dim {}", target.dimension(), source.dimension());
        let p = target.prime();
        let offset_shift = target.offset() - source.offset();
        let bit_length = bit_length(p);
        let entries_per_64_bits = entries_per_64_bits(p);
        let usable_bits_per_limb = bit_length * entries_per_64_bits;
        let tail_shift = usable_bits_per_limb - offset_shift;
        let zero_bits = 64 - usable_bits_per_limb;
        let min_target_limb = target.min_limb();
        let max_target_limb = target.max_limb();
        let min_source_limb = source.min_limb();
        let max_source_limb = source.max_limb();
        let number_of_source_limbs = max_source_limb - min_source_limb;
        let number_of_target_limbs = max_target_limb - min_target_limb;
        Self {
            offset_shift,
            tail_shift,
            zero_bits,
            min_source_limb,
            min_target_limb,
            number_of_source_limbs,
            number_of_target_limbs
        }
    }

    fn mask_first_limb_a(&self, other : &(impl FpVectorT + ?Sized), i : usize) -> u64 {
        let mask = other.limb_mask(i);
        let source_limb_masked = other.limbs()[self.min_source_limb + i] & mask;
        (source_limb_masked << (self.offset_shift + self.zero_bits)) >> self.zero_bits
    }

    fn mask_first_limb_b(&self, other : &(impl FpVectorT + ?Sized), i : usize) -> u64 {
        let mask = other.limb_mask(i);
        let source_limb_masked = other.limbs()[self.min_source_limb + i] & mask;
        source_limb_masked >> self.tail_shift
    }

    fn mask_middle_limb_a(&self, other : &(impl FpVectorT + ?Sized), i : usize) -> u64 {
        (other.limbs()[i + self.min_source_limb] << (self.offset_shift + self.zero_bits)) >> self.zero_bits
    }

    fn mask_middle_limb_b(&self, other : &(impl FpVectorT + ?Sized), i : usize) -> u64 {
        other.limbs()[i + self.min_source_limb] >> self.tail_shift
    }

    fn mask_last_limb_a(&self, other : &(impl FpVectorT + ?Sized), i : usize) -> u64 {
        let mask = other.limb_mask(i);
        let source_limb_masked = other.limbs()[self.min_source_limb + i] & mask;
        source_limb_masked << self.offset_shift
    }

    fn mask_last_limb_b(&self, other : &(impl FpVectorT + ?Sized), i : usize) -> u64 {
        let mask = other.limb_mask(i);
        let source_limb_masked = other.limbs()[self.min_source_limb + i] & mask;
        source_limb_masked >> self.tail_shift
    }    
}


#[enum_dispatch(FpVector)]
pub trait FpVectorT {
    fn add_limb(&self, limb_a : u64, limb_b : u64, coeff : u32) -> u64 {
        limb_a + (coeff as u64 * limb_b)
    }

    fn all_leq_limb(&self, limb_a : u64, limb_b : u64) -> bool;

    fn all_leq(&self, other : &FpVector) -> bool {
        match self.offset().cmp(&other.offset()) {
            Ordering::Equal => self.all_leq_shift_none(other),
            Ordering::Less => self.all_leq_shift_left(other),
            Ordering::Greater => self.all_leq_shift_right(other),
        }
    }

    fn all_leq_shift_none(&self, other : &FpVector) -> bool {
        let dat = AddShiftNoneData::new(self, other);
        let mut i = 0; {
            let self_limb = dat.mask_first_limb(self, i); 
            let other_limb = dat.mask_first_limb(other, i);
            if !self.all_leq_limb(self_limb, other_limb) {
                return false;
            }
        }
        for i in 1 .. dat.number_of_limbs - 1 {
            let self_limb = dat.mask_middle_limb(self, i); 
            let other_limb = dat.mask_middle_limb(other, i);
            if !self.all_leq_limb(self_limb, other_limb) {
                return false;
            }
        }
        i = dat.number_of_limbs - 1;
        if i > 0 {
            let self_limb = dat.mask_last_limb(self, i); 
            let other_limb = dat.mask_last_limb(other, i);
            if !self.all_leq_limb(self_limb, other_limb) {
                return false;
            }
        }
        true
    }

    fn all_leq_shift_left(&self, _other : &FpVector) -> bool {
        unimplemented!()
    }

    fn all_leq_shift_right(&self, _other : &FpVector) -> bool {
        unimplemented!()
    }

    fn add(&mut self, other : &FpVector, c : u32){
        debug_assert!(c < *self.prime());
        if self.dimension() == 0 {
            return;
        }

        match self.offset().cmp(&other.offset()) {
            Ordering::Equal => self.add_shift_none(other, c),
            Ordering::Less => self.add_shift_left(other, c),
            Ordering::Greater => self.add_shift_right(other, c),
        };
    }

    /// Ignores any slice and any edge conditions. For use in row_reduce. Included because profiling showed that
    /// handling edge conditions was taking up 2%
    fn add_shift_none_pure(&mut self, other : &FpVector, c : u32){
        debug_assert_eq!(self.prime(), other.prime());
        debug_assert_eq!(self.offset(), other.offset());
        debug_assert_eq!(self.dimension(), other.dimension(), "Adding vectors of different dimensions");
        let min_limb = self.min_limb();
        let max_limb = self.max_limb();
        let mut target_limbs = self.take_limbs();
        for i in min_limb .. max_limb {
            target_limbs[i] = self.reduce_limb(self.add_limb(target_limbs[i], other.limbs()[i], c));
        }
        self.put_limbs(target_limbs);
    }


    /// Adds `c` * `other` to `self`. `other` must have the same length, offset, and prime as self, and `c` must be between `0` and `p - 1`.
    fn add_shift_none(&mut self, other : &FpVector, c : u32){
        let dat = AddShiftNoneData::new(self, other);
        let mut target_limbs = self.take_limbs();
        let mut i = 0; {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_first_limb(other, i), c);
            target_limbs[i + dat.min_target_limb] = self.reduce_limb(target_limbs[i + dat.min_target_limb]);
        }
        for i in 1..dat.number_of_limbs-1 {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_middle_limb(other, i), c);
            target_limbs[i + dat.min_target_limb] = self.reduce_limb(target_limbs[i + dat.min_target_limb]);
        }
        i = dat.number_of_limbs - 1;
        if i > 0 {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_last_limb(other, i), c);
            target_limbs[i + dat.min_target_limb] = self.reduce_limb(target_limbs[i + dat.min_target_limb]);
        }
        self.put_limbs(target_limbs);
    }


    fn add_shift_left(&mut self, other : &FpVector, c : u32){
        let dat = AddShiftLeftData::new(self, other);
        let mut target_limbs = self.take_limbs();
        let mut i = 0; {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_first_limb(other, i) , c);
        }
        for i in 1 .. dat.number_of_source_limbs - 1 {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_middle_limb_a(other, i), c);
            target_limbs[i + dat.min_target_limb - 1] = self.add_limb(target_limbs[i + dat.min_target_limb - 1], dat.mask_middle_limb_b(other, i), c);
            target_limbs[i + dat.min_target_limb - 1] = self.reduce_limb(target_limbs[i + dat.min_target_limb - 1]);
        }
        i = dat.number_of_source_limbs - 1; 
        if i > 0 {
            target_limbs[i + dat.min_target_limb - 1] = self.add_limb(target_limbs[i + dat.min_target_limb - 1], dat.mask_last_limb_a(other, i), c);
            target_limbs[i + dat.min_target_limb - 1] = self.reduce_limb(target_limbs[i + dat.min_target_limb - 1]);
            if dat.number_of_source_limbs == dat.number_of_target_limbs {
                target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_last_limb_b(other, i), c);
                target_limbs[i + dat.min_target_limb] = self.reduce_limb(target_limbs[i + dat.min_target_limb]);
            }
        } else {
            target_limbs[i + dat.min_target_limb] = self.reduce_limb(target_limbs[i + dat.min_target_limb]);
        }
        self.put_limbs(target_limbs);
    }


    fn add_shift_right(&mut self, other : &FpVector, c : u32){
        let dat = AddShiftRightData::new(self, other);
        let mut target_limbs = self.take_limbs();
        let mut i = 0; {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_first_limb_a(other, i), c);
            target_limbs[i + dat.min_target_limb] = self.reduce_limb(target_limbs[i + dat.min_target_limb]);
            if dat.number_of_target_limbs > 1 {
                target_limbs[i + dat.min_target_limb + 1] = self.add_limb(target_limbs[i + dat.min_target_limb + 1], dat.mask_first_limb_b(other, i), c);
            }
        }
        for i in 1 .. dat.number_of_source_limbs-1 {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_middle_limb_a(other, i), c);
            target_limbs[i + dat.min_target_limb] = self.reduce_limb(target_limbs[i + dat.min_target_limb]);
            target_limbs[i + dat.min_target_limb + 1] = self.add_limb(target_limbs[i + dat.min_target_limb + 1], dat.mask_middle_limb_b(other, i), c);
        }
        i = dat.number_of_source_limbs - 1;
        if i > 0 {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_last_limb_a(other, i), c);
            target_limbs[i + dat.min_target_limb] = self.reduce_limb(target_limbs[i + dat.min_target_limb]);
            if dat.number_of_target_limbs > dat.number_of_source_limbs {
                target_limbs[i + dat.min_target_limb + 1] = self.add_limb(target_limbs[i + dat.min_target_limb + 1], dat.mask_last_limb_b(other, i), c);
            }
        }
        // if dat.number_of_target_limbs > 1 {
            // target_limbs[i + dat.min_target_limb] = self.reduce_limb(target_limbs[i + dat.min_target_limb]);
        // }
        self.put_limbs(target_limbs);
    }



    // This one takes &self so we can figure out how to reduce.
    // Returns: either (true, sum) if no carries happen in the limb or (false, ???) if some carry does happen.
    fn truncate_limb(&self, sum : u64) -> Option<u64> {
        if self.is_reduced_limb(sum) {
            Some(sum)
        } else {          
            None
        }
    }


    fn add_truncate(&mut self, other : &FpVector, c : u32) -> Option<()> {
        if self.dimension() == 0 {
            return Some(());
        }

        match self.offset().cmp(&other.offset()) {
            Ordering::Equal => self.add_truncate_shift_none(other, c),
            Ordering::Less => self.add_truncate_shift_left(other, c),
            Ordering::Greater => self.add_truncate_shift_right(other, c),
        }
    }

    /// Adds `c` * `other` to `self`. `other` must have the same length, offset, and prime as self, and `c` must be between `0` and `p - 1`.
    /// If any of the fields exceeds p after doing this, return "false" and quit as soon as this condition is detected.
    /// In this case, "self" will contain undefined nonsense.
    /// Otherwise return "true" and "self" will contain the sum.
    /// You get these "_truncate" variants from the normal variants by: every time "self.add_limb(<args>)" shows up
    /// in the original variant, replace it with "self.add_limb_truncate(<args>)?".
    /// Also have to add some extra Some(())'s.
    /// Adds `c` * `other` to `self`. `other` must have the same length, offset, and prime as self, and `c` must be between `0` and `p - 1`.

    fn add_truncate_shift_none(&mut self, other : &FpVector, c : u32) -> Option<()> {
        let dat = AddShiftNoneData::new(self, other);
        let mut target_limbs = self.take_limbs();
        let mut i = 0; {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_first_limb(other, i), c);
            target_limbs[i + dat.min_target_limb] = self.truncate_limb(target_limbs[i + dat.min_target_limb])?;
        }
        for i in 1..dat.number_of_limbs-1 {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_middle_limb(other, i), c);
            target_limbs[i + dat.min_target_limb] = self.truncate_limb(target_limbs[i + dat.min_target_limb])?;
        }
        i = dat.number_of_limbs - 1;
        if i > 0 {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_last_limb(other, i), c);
            target_limbs[i + dat.min_target_limb] = self.truncate_limb(target_limbs[i + dat.min_target_limb])?;
        }
        self.put_limbs(target_limbs);
        Some(())
    }


    fn add_truncate_shift_left(&mut self, other : &FpVector, c : u32) -> Option<()> {
        let dat = AddShiftLeftData::new(self, other);
        let mut target_limbs = self.take_limbs();
        let mut i = 0; {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_first_limb(other, i) , c);
        }
        for i in 1 .. dat.number_of_source_limbs - 1 {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_middle_limb_a(other, i), c);
            target_limbs[i + dat.min_target_limb - 1] = self.add_limb(target_limbs[i + dat.min_target_limb - 1], dat.mask_middle_limb_b(other, i), c);
            target_limbs[i + dat.min_target_limb - 1] = self.truncate_limb(target_limbs[i + dat.min_target_limb - 1])?;
        }
        i = dat.number_of_source_limbs - 1; 
        if i > 0 {
            target_limbs[i + dat.min_target_limb - 1] = self.add_limb(target_limbs[i + dat.min_target_limb - 1], dat.mask_last_limb_a(other, i), c);
            target_limbs[i + dat.min_target_limb - 1] = self.truncate_limb(target_limbs[i + dat.min_target_limb - 1])?;
            if dat.number_of_source_limbs == dat.number_of_target_limbs {
                target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_last_limb_b(other, i), c);
                target_limbs[i + dat.min_target_limb] = self.truncate_limb(target_limbs[i + dat.min_target_limb])?;
            }
        } else {
            target_limbs[i + dat.min_target_limb] = self.truncate_limb(target_limbs[i + dat.min_target_limb])?;
        }
        self.put_limbs(target_limbs);
        Some(())
    }


    fn add_truncate_shift_right(&mut self, other : &FpVector, c : u32) -> Option<()> {
        let dat = AddShiftRightData::new(self, other);
        let mut target_limbs = self.take_limbs();
        let mut i = 0; {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_first_limb_a(other, i), c);
            target_limbs[i + dat.min_target_limb] = self.truncate_limb(target_limbs[i + dat.min_target_limb])?;
            if dat.number_of_target_limbs > 1 {
                target_limbs[i + dat.min_target_limb + 1] = self.add_limb(target_limbs[i + dat.min_target_limb + 1], dat.mask_first_limb_b(other, i), c);
            }
        }
        for i in 1 .. dat.number_of_source_limbs-1 {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_middle_limb_a(other, i), c);
            target_limbs[i + dat.min_target_limb] = self.truncate_limb(target_limbs[i + dat.min_target_limb])?;
            target_limbs[i + dat.min_target_limb + 1] = self.add_limb(target_limbs[i + dat.min_target_limb + 1], dat.mask_middle_limb_b(other, i), c);
        }
        i = dat.number_of_source_limbs - 1;
        if i > 0 {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_last_limb_a(other, i), c);
            target_limbs[i + dat.min_target_limb] = self.truncate_limb(target_limbs[i + dat.min_target_limb])?;
            if dat.number_of_target_limbs > dat.number_of_source_limbs {
                target_limbs[i + dat.min_target_limb + 1] = self.add_limb(target_limbs[i + dat.min_target_limb + 1], dat.mask_last_limb_b(other, i), c);
            }
        }
        self.put_limbs(target_limbs);
        Some(())
    }

    // These could be static but enum_dispatch needs them to take &self.
    fn is_reduced_limb(&self, limb : u64) -> bool;
    fn reduce_limb(&self, limb : u64) -> u64;
    fn reduce_quotient_limb(&self, limb : u64) -> (u64, u64);
    
    fn reduce_limbs(&mut self, start_limb : usize, end_limb : usize ){
        let mut limbs = std::mem::take(&mut self.vector_container_mut().limbs);
        for limb in &mut limbs[start_limb..end_limb] {
            *limb = self.reduce_limb(*limb);
        }
        self.vector_container_mut().limbs = limbs;
    }

    fn vector_container(&self) -> &VectorContainer;
    fn vector_container_mut(&mut self) -> &mut VectorContainer;
    fn prime(&self) -> ValidPrime;

    fn dimension(&self) -> usize {
        let container = self.vector_container();
        container.slice_end - container.slice_start
    }

    fn offset(&self) -> usize {
        let container = self.vector_container();
        let bit_length = bit_length(self.prime());
        let entries_per_64_bits = entries_per_64_bits(self.prime());
        (container.slice_start * bit_length) % (bit_length * entries_per_64_bits)
    }

    fn min_index(&self) -> usize {
        self.vector_container().slice_start
    }

    fn slice(&self) -> (usize, usize) {
        let container = self.vector_container();
        (container.slice_start, container.slice_end)
    }

    fn is_set_slice_valid(&self, slice_start : usize, slice_end : usize) -> bool {
        let container = self.vector_container();
        let slice_end = container.slice_start + slice_end;
        let slice_start =  container.slice_start + slice_start;
        (slice_start <= slice_end) && (slice_end <= container.dimension)
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
        debug_assert!(slice.1 <= container.dimension);
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

    fn take_limbs(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.vector_container_mut().limbs)
    }

    fn put_limbs(&mut self, limbs : Vec<u64>) {
        self.vector_container_mut().limbs = limbs
    }

    #[inline(always)]
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
        mask
    }

    fn set_to_zero_pure (&mut self){
        for limb in self.limbs_mut().iter_mut() {
            *limb = 0;
        }
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
        self.add(other, 1);
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
        {
            let start = 1;
            let end = number_of_limbs - 1;
            if end > start {
                target_limbs[start + min_target_limb .. end + min_target_limb]
                    .clone_from_slice(&source_limbs[start + min_source_limb .. end + min_source_limb]);
            }
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
        true
    }

    fn entry(&self, index : usize) -> u32 {
        debug_assert!(index < self.dimension(), "Index {} too large, dimension of vector is only {}.", index, self.dimension());
        let p = self.prime();
        let bit_mask = bitmask(p);
        let limb_index = limb_bit_index_pair(p, index + self.min_index());
        let mut result = self.limbs()[limb_index.limb];
        result >>= limb_index.bit_index;
        result &= bit_mask;
        result as u32
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
        x %= *self.prime();
        self.set_entry(index, x);
    }

    /// Unpacks an FpVector onto an array slice. note that the array slice has to be long
    /// enough to hold all the elements in the FpVector.
    fn unpack(&self, target : &mut [u32]){
        debug_assert!(self.dimension() <= target.len());
        let p = self.prime();
        let dimension = self.dimension();
        if dimension == 0 {
            return;
        }
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
    /// Adds v otimes w to self.
    fn add_tensor(&mut self, offset : usize, coeff : u32, left : &FpVector, right : &FpVector) {
        let right_dim = right.dimension();

        let old_slice = self.slice();
        // println!("v : {}, dim(v) : {}, slice: {:?}", left, left.dimension(), left.slice());
        // println!(" debug v : {:?}", left);
        for (i, v) in left.iter_nonzero() {
            let entry = (v * coeff) % *self.prime();
            // println!("   left_dim : {}, right_dim : {}, i : {}, v : {}", left.dimension(), right.dimension(), i, v);
            // println!("   set slice: {} -- {} dimension: {}", offset + i * right_dim, offset + (i + 1) * right_dim, self.dimension());
            self.set_slice(offset + i * right_dim, offset + (i + 1) * right_dim);
            self.add(right, entry);
            self.restore_slice(old_slice);
        }
    }

    // fn add_truncate(&mut self, other : &FpVector, c : u32) -> bool {

    // }

    // fn add_with_carry_truncate(&mut self, other : &FpVector, c : u32) -> bool {

    // }


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

        if number_of_limbs == 0 {
            return true;
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
        true
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
#[repr(transparent)]
pub struct FpVector2 {
    vector_container : VectorContainer
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct FpVector3 {
    vector_container : VectorContainer
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct FpVector5 {
    vector_container : VectorContainer
}

#[derive(Debug, Clone)]
pub struct FpVectorGeneric {
    p : ValidPrime,
    vector_container : VectorContainer
}

impl FpVector2 {
    fn add_truncate_limb(&self, limb_a : u64, limb_b : u64, coeff : u32) -> Option<u64> {
        let scaled_limb_b = coeff as u64 * limb_b;
        if limb_a & scaled_limb_b == 0 {
            Some(limb_a ^ scaled_limb_b)
        } else {
            None
        }
    }
}

impl FpVectorT for FpVector2 {
    // Use special handling at 2. 
    fn is_reduced_limb(&self, _limb : u64) -> bool { panic!() }
    fn reduce_limb(&self, limb : u64) -> u64 { limb }
    fn reduce_quotient_limb(&self, _limb : u64) -> (u64, u64) { panic!() }  
    fn reduce_limbs(&mut self, _start_limb : usize, _end_limb : usize){ }

    fn all_leq_limb(&self, limb_a : u64, limb_b : u64) -> bool {
        limb_a | limb_b == limb_b
    }


    fn add_limb(&self, limb_a : u64, limb_b : u64, coeff : u32) -> u64 {
        limb_a ^ (coeff as u64 * limb_b)
    }
    


    fn add_truncate_shift_none(&mut self, other : &FpVector, c : u32) -> Option<()> {
        let dat = AddShiftNoneData::new(self, other);
        let mut target_limbs = self.take_limbs();
        let mut i = 0; {
            target_limbs[i + dat.min_target_limb] = self.add_truncate_limb(target_limbs[i + dat.min_target_limb], dat.mask_first_limb(other, i), c)?;
        }
        for i in 1..dat.number_of_limbs-1 {
            target_limbs[i + dat.min_target_limb] = self.add_truncate_limb(target_limbs[i + dat.min_target_limb], dat.mask_middle_limb(other, i), c)?;
        }
        i = dat.number_of_limbs - 1;
        if i > 0 {
            target_limbs[i + dat.min_target_limb] = self.add_truncate_limb(target_limbs[i + dat.min_target_limb], dat.mask_last_limb(other, i), c)?;
        }
        self.put_limbs(target_limbs);
        Some(())
    }

    // Have to reduce twice at odd primes b/c of 2.
    // Perhaps should biforcate implementations at p odd and p=2...
    fn add_truncate_shift_left(&mut self, other : &FpVector, c : u32) -> Option<()> {
        let dat = AddShiftLeftData::new(self, other);
        let mut target_limbs = self.take_limbs();
        let mut i = 0; {
            target_limbs[i + dat.min_target_limb] = self.add_truncate_limb(target_limbs[i + dat.min_target_limb], dat.mask_first_limb(other, i) , c)?;
        }
        for i in 1 .. dat.number_of_source_limbs - 1 {
            target_limbs[i + dat.min_target_limb] = self.add_truncate_limb(target_limbs[i + dat.min_target_limb], dat.mask_middle_limb_a(other, i), c)?;
            target_limbs[i + dat.min_target_limb - 1] = self.add_truncate_limb(target_limbs[i + dat.min_target_limb - 1], dat.mask_middle_limb_b(other, i), c)?;
        }
        i = dat.number_of_source_limbs - 1; 
        if i > 0 {
            target_limbs[i + dat.min_target_limb - 1] = self.add_truncate_limb(target_limbs[i + dat.min_target_limb - 1], dat.mask_last_limb_a(other, i), c)?;
            if dat.number_of_source_limbs == dat.number_of_target_limbs {
                target_limbs[i + dat.min_target_limb] = self.add_truncate_limb(target_limbs[i + dat.min_target_limb], dat.mask_last_limb_b(other, i), c)?;
            }
        }
        self.put_limbs(target_limbs);
        Some(())
    }

    fn add_truncate_shift_right(&mut self, other : &FpVector, c : u32) -> Option<()> {
        let dat = AddShiftRightData::new(self, other);
        let mut target_limbs = self.take_limbs();
        let mut i = 0; {
            target_limbs[i + dat.min_target_limb] = self.add_truncate_limb(target_limbs[i + dat.min_target_limb], dat.mask_first_limb_a(other, i), c)?;
            if dat.number_of_target_limbs > 1 {
                target_limbs[i + dat.min_target_limb + 1] = self.add_truncate_limb(target_limbs[i + dat.min_target_limb + 1], dat.mask_first_limb_b(other, i), c)?;
            }
        }
        for i in 1 .. dat.number_of_source_limbs-1 {
            target_limbs[i + dat.min_target_limb] = self.add_truncate_limb(target_limbs[i + dat.min_target_limb], dat.mask_middle_limb_a(other, i), c)?;
            target_limbs[i + dat.min_target_limb + 1] = self.add_truncate_limb(target_limbs[i + dat.min_target_limb + 1], dat.mask_middle_limb_b(other, i), c)?;
        }
        i = dat.number_of_source_limbs - 1;
        if i > 0 {
            target_limbs[i + dat.min_target_limb] = self.add_truncate_limb(target_limbs[i + dat.min_target_limb], dat.mask_last_limb_a(other, i), c)?;
            if dat.number_of_target_limbs > dat.number_of_source_limbs {
                target_limbs[i + dat.min_target_limb + 1] = self.add_truncate_limb(target_limbs[i + dat.min_target_limb + 1], dat.mask_last_limb_b(other, i), c)?;
            }
        }
        self.put_limbs(target_limbs);
        Some(())
    }


    fn prime(&self) -> ValidPrime { ValidPrime::new(2) }
    fn vector_container (&self) -> &VectorContainer { &self.vector_container }
    fn vector_container_mut(&mut self) -> &mut VectorContainer { &mut self.vector_container }

    fn add_basis_element(&mut self, index : usize, value : u32){
        let limb_index = limb_bit_index_pair(self.prime(), index + self.min_index());
        let value = (value % 2) as u64;
        self.vector_container.limbs[limb_index.limb] ^= value << limb_index.bit_index;
    }
}

impl FpVector2 {

    pub fn add_carry2(&mut self, other : &FpVector, c : u32, rest : &mut [FpVector]) -> bool {
        if self.dimension() == 0 {
            return false;
        }
        if c == 0 { 
            return false;
        }
        match self.offset().cmp(&other.offset()) {
            Ordering::Equal => self.add_carry_shift_none2(other, rest),
            Ordering::Less => self.add_carry_shift_left2(other, rest),
            Ordering::Greater => self.add_carry_shift_right2(other, rest),
        }
    }

    pub fn add_carry_limb2(&mut self, idx : usize, source : u64, rest : &mut [FpVector]) -> bool {
        let mut cur_vec = self;
        let mut target_limbs;
        let mut carry = source;
        for carry_vec in rest.iter_mut() {
            let carry_vec = match carry_vec {
                FpVector::FpVector2(v) => v,
                _ => panic!()
            };
            target_limbs = cur_vec.take_limbs();
            let rem = target_limbs[idx] ^ carry;
            let quot = target_limbs[idx] & carry;
            target_limbs[idx] = rem;
            carry = quot;
            cur_vec.put_limbs(target_limbs);
            cur_vec = carry_vec;
            if quot == 0 {
                return false;
            }
        }
        target_limbs = cur_vec.take_limbs();
        target_limbs[idx] = target_limbs[idx] ^ carry;
        cur_vec.put_limbs(target_limbs);
        return true;
    }

    pub fn add_carry_shift_none2(&mut self, other : &FpVector, rest : &mut [FpVector]) -> bool {
        let dat = AddShiftNoneData::new(self, other);
        let mut result = false;
        let mut i = 0; {
            result |= self.add_carry_limb2(i + dat.min_target_limb, dat.mask_first_limb(other, i), rest);
        }
        for i in 1..dat.number_of_limbs-1 {
            result |= self.add_carry_limb2(i + dat.min_target_limb, dat.mask_middle_limb(other, i), rest)
        }
        i = dat.number_of_limbs - 1;
        if i > 0 {
            result |= self.add_carry_limb2(i + dat.min_target_limb, dat.mask_last_limb(other, i), rest);
        }
        result
    }

    
    pub fn add_carry_shift_left2(&mut self, other : &FpVector, rest : &mut [FpVector]) -> bool {
        let dat = AddShiftLeftData::new(self, other);
        let mut result = false;
        let mut i = 0; {
            result |= self.add_carry_limb2(i + dat.min_target_limb, dat.mask_first_limb(other, i), rest);
        }
        for i in 1 .. dat.number_of_source_limbs - 1 {
            result |= self.add_carry_limb2(i + dat.min_target_limb, dat.mask_middle_limb_a(other, i), rest);
            result |= self.add_carry_limb2(i + dat.min_target_limb - 1, dat.mask_middle_limb_b(other, i), rest);
        }
        i = dat.number_of_source_limbs - 1; 
        if i > 0 {
            result |= self.add_carry_limb2(i + dat.min_target_limb - 1, dat.mask_last_limb_a(other, i), rest);
            if dat.number_of_source_limbs == dat.number_of_target_limbs {
                result |= self.add_carry_limb2(i + dat.min_target_limb, dat.mask_last_limb_b(other, i), rest);
            }
        }
        result
    }


    pub fn add_carry_shift_right2(&mut self, other : &FpVector, rest : &mut [FpVector]) -> bool {
        let dat = AddShiftRightData::new(self, other);
        let mut result = false;
        let mut i = 0; {
            result |= self.add_carry_limb2(i + dat.min_target_limb, dat.mask_first_limb_a(other, i), rest);
            if dat.number_of_target_limbs > 1 {
                self.add_carry_limb2(i + dat.min_target_limb + 1, dat.mask_first_limb_b(other, i), rest);
            }
        }
        for i in 1 .. dat.number_of_source_limbs-1 {
            result |= self.add_carry_limb2(i + dat.min_target_limb, dat.mask_middle_limb_a(other, i), rest);
            result |= self.add_carry_limb2(i + dat.min_target_limb + 1, dat.mask_middle_limb_b(other, i), rest);
        }
        i = dat.number_of_source_limbs - 1;
        if i > 0 {
            result |= self.add_carry_limb2(i + dat.min_target_limb, dat.mask_last_limb_a(other, i), rest);
            if dat.number_of_target_limbs > dat.number_of_source_limbs {
                result |= self.add_carry_limb2(i + dat.min_target_limb + 1, dat.mask_last_limb_b(other, i), rest);
            }
        }
        result
    }
}


impl FpVectorT for FpVector3 {
    // This code contributed by Robert Burklund
    fn is_reduced_limb(&self, limb : u64) -> bool {
        let top_bit = 0x4924924924924924u64;
        let bottom_bit = top_bit >> 2;
        (limb + bottom_bit ) & ( top_bit) == 0
    }

    // This code contributed by Robert Burklund
    fn reduce_limb(&self, limb : u64) -> u64 {
        let top_bit = 0x4924924924924924u64;
        let mut limb_2 = ((limb & top_bit) >> 2) + (limb & (!top_bit));
        let mut limb_3s = limb_2 & (limb_2 >> 1);
        limb_3s |= limb_3s << 1;
        limb_2 ^= limb_3s;
        return limb_2;
    }

    fn reduce_quotient_limb(&self, limb : u64) -> (u64, u64) {
        let rem = self.reduce_limb(limb);
        let a = limb - rem;
        let quot = a & (a >> 1);
        (rem, quot)
    }

    fn all_leq_limb(&self, limb_a : u64, limb_b : u64) -> bool {
        let top_bit = 0x4924924924924924u64;
        limb_b.wrapping_sub(limb_a) & top_bit == 0
    }

    fn prime (&self) -> ValidPrime { ValidPrime::new(3) }
    fn vector_container (&self) -> &VectorContainer { &self.vector_container }
    fn vector_container_mut (&mut self) -> &mut VectorContainer { &mut self.vector_container }
}

impl FpVectorT for FpVector5 {
    // This code contributed by Robert Burklund
    fn is_reduced_limb(&self, limb : u64) -> bool {
        let bottom_bit = 0x84210842108421u64;
        let bottom_two_bits = bottom_bit | (bottom_bit << 1);
        let top_two_bits = bottom_two_bits << 3;
        (limb + bottom_two_bits) & (top_two_bits) == 0
    }

    // This code contributed by Robert Burklund
    fn reduce_limb(&self, limb : u64) -> u64 {
        let bottom_bit = 0x84210842108421u64;
        let bottom_two_bits = bottom_bit | (bottom_bit << 1);
        let bottom_three_bits = bottom_bit | (bottom_two_bits << 1);
        let a = (limb >> 2) & bottom_three_bits;
        let b = limb & bottom_two_bits;
        let m = (bottom_bit << 3) - a + b;
        let mut c = (m >> 3) & bottom_bit;
        c |= c << 1;
        let d = m & bottom_three_bits;
        return d + c - bottom_two_bits;
    }

    // This code contributed by Robert Burklund
    fn reduce_quotient_limb(&self, limb : u64) -> (u64, u64) {
        let bottom_bit = 0x84210842108421u64;
        let bottom_two_bits = bottom_bit | (bottom_bit << 1);
        let top_three_bits = !bottom_two_bits;
        let top_bit = bottom_bit << 4;
        let bottom_four_bits = !top_bit;
        let a = (limb & top_three_bits) >> 2;
        let b = ((bottom_four_bits - (limb & bottom_two_bits) + a) & top_bit) >> 4;
        let quot = a - b;
        let rem = limb - 5*quot;
        (rem, quot)
    }

    fn all_leq_limb(&self, limb_a : u64, limb_b : u64) -> bool {
        let bottom_bit = 0x84210842108421u64;
        let top_bit = bottom_bit << 4;
        limb_b.wrapping_sub(limb_a) & top_bit == 0
    }    

    fn prime(&self) -> ValidPrime { ValidPrime::new(5) }
    fn vector_container (&self) -> &VectorContainer { &self.vector_container }
    fn vector_container_mut (&mut self) -> &mut VectorContainer { &mut self.vector_container }
}

// TODO: FIXME!
impl FpVectorT for FpVectorGeneric {
    fn is_reduced_limb(&self, limb : u64) -> bool {
        self.reduce_limb(limb) == limb
    }
    fn reduce_limb(&self, _limb : u64) -> u64 { unimplemented!() }
    fn reduce_quotient_limb(&self, _limb : u64) -> (u64, u64) { unimplemented!() }

    fn reduce_limbs(&mut self, start_limb : usize, end_limb : usize){
        let p = self.p;
        let mut unpacked_limb = vec![0; entries_per_64_bits(p)];
        let dimension = self.vector_container.dimension;
        let limbs = &mut self.vector_container.limbs;
        for i in start_limb..end_limb {
            FpVector::unpack_limb(p, dimension, 0, &mut unpacked_limb, limbs, i);
            for limb in &mut unpacked_limb {
                *limb %= *p;
            }
            FpVector::pack_limb(p, dimension, 0, &unpacked_limb, limbs, i);
        }
    }

    fn all_leq_limb(&self, _limb_a : u64, _limb_b : u64) -> bool {
        unimplemented!()
    }    


    fn prime (&self) -> ValidPrime { self.p }
    fn vector_container (&self) -> &VectorContainer { &self.vector_container }
    fn vector_container_mut (&mut self) -> &mut VectorContainer { &mut self.vector_container }
}

impl FpVector {
    pub fn new(p : ValidPrime, dimension : usize) -> Self {
        let number_of_limbs = Self::number_of_limbs(p, dimension);
        let limbs = vec![0; number_of_limbs];
        let slice_start = 0;
        let slice_end = dimension;
        let vector_container = VectorContainer { dimension, limbs, slice_start, slice_end };

        #[cfg(feature = "prime-two")]
        {
            Self::from(FpVector2 { vector_container })
        }

        #[cfg(not(feature = "prime-two"))]
        {
            match *p  {
                2 => Self::from(FpVector2 { vector_container }),
                3 => Self::from(FpVector3 { vector_container }),
                5 => Self::from(FpVector5 { vector_container }),
                _ => Self::from(FpVectorGeneric { p, vector_container })
            }
        }
    }

    /// This function ensures the length of the vector is at least `len`. This *must* be applied on
    /// an unsliced vector and returns an unsliced vector. See also `set_scratch_vector_size`.
    pub fn extend_dimension(&mut self, len: usize) {
        let p = self.prime();
        self.clear_slice();
        let container = self.vector_container_mut();
        // assert_eq!((container.slice_start, container.slice_end), (0, container.dimension));

        if len <= container.dimension {
            return;
        }
        container.dimension = len;
        container.slice_end = len;
        let num_limbs = Self::number_of_limbs(p, len);
        container.limbs.resize(num_limbs, 0);
    }

    pub fn from_vec(p : ValidPrime, vec : &[u32]) -> FpVector {
        let mut result = FpVector::new(p, vec.len());
        result.pack(&vec);
        result
    }

    pub fn number_of_limbs(p : ValidPrime, dimension : usize) -> usize {
        debug_assert!(dimension < MAX_DIMENSION);
        if dimension == 0 {
            0
        } else {
            limb_bit_index_pair(p, dimension - 1).limb + 1
        }
    }

    pub fn padded_dimension(p : ValidPrime, dimension : usize) -> usize {
        let entries_per_limb = entries_per_64_bits(p);
        ((dimension + entries_per_limb - 1)/entries_per_limb)*entries_per_limb
    }

    pub fn set_scratch_vector_size(&mut self, dimension : usize) {
        self.clear_slice();
        self.extend_dimension(dimension);
        self.set_slice(0, dimension);
        self.set_to_zero_pure();
    }

    pub fn iter(&self) -> FpVectorIterator {
        FpVectorIterator::new(self)
    }

    pub fn iter_nonzero(&self) -> FpVectorIteratorNonzero {
        FpVectorIteratorNonzero::new(self)
    }    

    // pub fn 

    fn pack_limb(p : ValidPrime, dimension : usize, offset : usize, limb_array : &[u32], limbs : &mut Vec<u64>, limb_idx : usize) -> usize {
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
        idx
    }

    pub fn limb_string(p : ValidPrime, limb : u64) -> String {
        let bit_length = bit_length(p);
        let entries_per_64_bits = entries_per_64_bits(p);
        let bit_mask = bitmask(p);
        let bit_min = 0usize;
        let bit_max = bit_length * entries_per_64_bits;
        let mut result = String::new();
        result.push_str("[");
        for j in (bit_min .. bit_max).step_by(bit_length) {
            let s = format!("{}, ", ((limb >> j) & bit_mask) as u32);
            result.push_str(&s);
        }
        result.push_str("]");  
        result
    }

    pub fn limb_string_x(p : ValidPrime, limb : u64) -> String {
        let bit_length = bit_length(p);
        let entries_per_64_bits = entries_per_64_bits(p);
        let bit_mask = bitmask(p);
        let bit_min = 0usize;
        let bit_max = bit_length * entries_per_64_bits;
        let mut result = String::new();
        result.push_str("[");
        for j in (bit_min .. bit_max).step_by(bit_length) {
            let s = format!("{:b}, ",  ((limb >> j) & bit_mask) as u32);
            result.push_str(&s);
        }
        result.push_str("]");  
        result
    }

    // Panics on arithmetic overflow from "bits_needed_for_entire_vector - 1" if dimension == 0.
    fn unpack_limb(p : ValidPrime, dimension : usize, offset : usize, limb_array : &mut [u32], limbs : &[u64], limb_idx : usize) -> usize {
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
        idx
    }

    pub fn borrow_slice(&mut self, start: usize, end: usize) -> FpVectorSlice<'_> {
        let old_slice = self.slice();
        self.set_slice(start, end);
        FpVectorSlice {
            old_slice,
            inner: self
        }
    }


    pub fn add_carry(&mut self, other : &FpVector, c : u32, rest : &mut [FpVector]) -> bool {
        if self.dimension() == 0 {
            return false;
        }
        if let FpVector::FpVector2(v) = self {
            return v.add_carry2(other, c, rest);
        }
        match self.offset().cmp(&other.offset()) {
            Ordering::Equal => self.add_carry_shift_none(other, c, rest),
            Ordering::Less => self.add_carry_shift_left(other, c, rest),
            Ordering::Greater => self.add_carry_shift_right(other, c, rest),
        }
    }

    pub fn add_carry_propagate(&mut self, rest : &mut [FpVector]) -> bool {
        let min_target_limb = self.min_limb();
        let max_target_limb = self.max_limb();
        let number_of_limbs = max_target_limb - min_target_limb;
        let mut cur_vec = self;
        let mut target_limbs;
        for carry_vec in rest.iter_mut() {
            target_limbs = cur_vec.take_limbs();
            let mut carries_occurred = 0;
            for i in 0 .. number_of_limbs {
                let (rem, quot) = cur_vec.reduce_quotient_limb(target_limbs[i + min_target_limb]);
                target_limbs[i + min_target_limb] = rem;
                carry_vec.limbs_mut()[i + min_target_limb] = carry_vec.add_limb(carry_vec.limbs()[i + min_target_limb], quot, 1);
                carries_occurred |= quot;
            }
            cur_vec.put_limbs(target_limbs);
            cur_vec = carry_vec;
            if carries_occurred == 0 {
                return false;
            }
        }
        target_limbs = cur_vec.take_limbs();
        for i in 0 .. number_of_limbs {
            target_limbs[i + min_target_limb] = cur_vec.reduce_limb(target_limbs[i + min_target_limb]);
        }
        cur_vec.put_limbs(target_limbs);
        return true;
    }

    pub fn add_carry_shift_none(&mut self, other : &FpVector, c : u32, rest : &mut [FpVector]) -> bool {
        let dat = AddShiftNoneData::new(self, other);
        let mut target_limbs = self.take_limbs();
        let mut i = 0; {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_first_limb(other, i), c);
        }
        for i in 1..dat.number_of_limbs-1 {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_middle_limb(other, i), c);
        }
        i = dat.number_of_limbs - 1;
        if i > 0 {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_last_limb(other, i), c);
        }
        self.put_limbs(target_limbs);
        self.add_carry_propagate(rest)
    }

    
    pub fn add_carry_shift_left(&mut self, other : &FpVector, c : u32, rest : &mut [FpVector]) -> bool {
        let dat = AddShiftLeftData::new(self, other);
        let mut target_limbs = self.take_limbs();
        let mut i = 0; {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_first_limb(other, i) , c);
        }
        for i in 1 .. dat.number_of_source_limbs - 1 {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_middle_limb_a(other, i), c);
            target_limbs[i + dat.min_target_limb - 1] = self.add_limb(target_limbs[i + dat.min_target_limb - 1], dat.mask_middle_limb_b(other, i), c);
        }
        i = dat.number_of_source_limbs - 1; 
        if i > 0 {
            target_limbs[i + dat.min_target_limb - 1] = self.add_limb(target_limbs[i + dat.min_target_limb - 1], dat.mask_last_limb_a(other, i), c);
            if dat.number_of_source_limbs == dat.number_of_target_limbs {
                target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_last_limb_b(other, i), c);
            }
        }
        self.put_limbs(target_limbs);
        self.add_carry_propagate(rest)
    }


    pub fn add_carry_shift_right(&mut self, other : &FpVector, c : u32, rest : &mut [FpVector]) -> bool {
        let dat = AddShiftRightData::new(self, other);
        let mut target_limbs = self.take_limbs();
        let mut i = 0; {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_first_limb_a(other, i), c);
            if dat.number_of_target_limbs > 1 {
                target_limbs[i + dat.min_target_limb + 1] = self.add_limb(target_limbs[i + dat.min_target_limb + 1], dat.mask_first_limb_b(other, i), c);
            }
        }
        for i in 1 .. dat.number_of_source_limbs - 1 {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_middle_limb_a(other, i), c);
            target_limbs[i + dat.min_target_limb + 1] = self.add_limb(target_limbs[i + dat.min_target_limb + 1], dat.mask_middle_limb_b(other, i), c);
        }
        i = dat.number_of_source_limbs - 1;
        if i > 0 {
            target_limbs[i + dat.min_target_limb] = self.add_limb(target_limbs[i + dat.min_target_limb], dat.mask_last_limb_a(other, i), c);
            if dat.number_of_target_limbs > dat.number_of_source_limbs {
                target_limbs[i + dat.min_target_limb + 1] = self.add_limb(target_limbs[i + dat.min_target_limb + 1], dat.mask_last_limb_b(other, i), c);
            }
        }
        self.put_limbs(target_limbs);
        self.add_carry_propagate(rest)
    }

    pub fn sign_rule(&self, other : &FpVector) -> bool {
        match self {
            FpVector::FpVector2(_) => {},
            _ => panic!()
        };
        match self.offset().cmp(&other.offset()) {
            Ordering::Equal => self.sign_rule_shift_none(other),
            Ordering::Less => unimplemented!(),
            Ordering::Greater => unimplemented!(),
        }
    }

    pub fn sign_rule_shift_none(&self, other : &FpVector) -> bool {
        let mut result = 0;
        let min_limb = self.min_limb();
        let max_limb = self.max_limb();
        for target_limb_idx in min_limb .. max_limb {
            let target_limb = other.limbs()[target_limb_idx] & other.limb_mask(target_limb_idx - min_limb);
            let source_limb = self.limbs()[target_limb_idx] & self.limb_mask(target_limb_idx - min_limb);
            result ^= self.sign_rule_limb(target_limb, source_limb);
            if target_limb.count_ones() % 2 == 0 {
                continue;
            }    
            for source_limb_idx in min_limb .. target_limb_idx {
                let source_limb = self.limbs()[target_limb_idx] & self.limb_mask(source_limb_idx - min_limb);
                result ^= source_limb.count_ones() % 2;
            }
        }
        result == 1
    }

    pub fn sign_rule_limb(&self, mut target : u64, mut source : u64) -> u32 {
        let every_other_bit  = 0x5555555555555555;
        let every_fourth_bit = 0x1111111111111111;
        let every_eight_bit  = 0x0101010101010101;
        let every_16th_bit   = 0x0001000100010001;
        let every_32nd_bit   = 0x0000000100000001;
        let mut result = 0;
        result ^= (every_other_bit & (source >> 1) & target).count_ones() % 2;
        source = (source & every_other_bit) ^ ((source >> 1) & every_other_bit);
        target = (target & every_other_bit) ^ ((target >> 1) & every_other_bit);
        
        result ^= (every_fourth_bit & (source >> 2) & target).count_ones() % 2;
        source = (source & every_fourth_bit) ^ ((source >> 2) & every_fourth_bit);
        target = (target & every_fourth_bit) ^ ((target >> 2) & every_fourth_bit);

        result ^= (every_eight_bit & (source >> 4) & target).count_ones() % 2;
        source = (source & every_eight_bit) ^ ((source >> 4) & every_eight_bit);
        target = (target & every_eight_bit) ^ ((target >> 4) & every_eight_bit);
        
        result ^= (every_16th_bit & (source >> 8) & target).count_ones() % 2;
        source = (source & every_16th_bit) ^ ((source >> 8) & every_16th_bit);
        target = (target & every_16th_bit) ^ ((target >> 8) & every_16th_bit);
        
        result ^= (every_32nd_bit & (source >> 16) & target).count_ones() % 2;
        source = (source & every_32nd_bit) ^ ((source >> 16) & every_32nd_bit);
        target = (target & every_32nd_bit) ^ ((target >> 16) & every_32nd_bit);
        
        result ^= ((source >> 32) & target).count_ones() % 2;
        result
    }

}

impl std::ops::AddAssign<&FpVector> for FpVector {
    fn add_assign(&mut self, other: &FpVector) {
        self.add(other, 1);
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
    fn new(vec : &'a FpVector) -> Self {
        let counter = vec.dimension();
        let limbs = vec.limbs();

        if counter == 0 {
            return Self {
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

        let entries_per_64_bits = entries_per_64_bits(p);
        Self {
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



pub struct FpVector2IteratorNonzero<'a> {
    limbs : &'a Vec<u64>,
    limb_index : usize,
    cur_limb_entries_left : usize,
    cur_limb : u64,
    idx : usize,
    dim : usize
}

impl<'a> FpVector2IteratorNonzero<'a> {
    fn new(vec : &'a FpVector) -> Self {
        const ENTRIES_PER_LIMB : usize = 64;
        let dim = vec.dimension();
        let limbs = vec.limbs();

        if dim == 0 {
            return Self {
                limbs,
                limb_index : 0,
                cur_limb_entries_left : 0,
                cur_limb: 0,
                idx : 0,
                dim
            }
        }
        let min_index = vec.min_index();
        let pair = limb_bit_index_pair(vec.prime(), min_index);
        let cur_limb = limbs[pair.limb] >> pair.bit_index;

        Self {
            limbs,
            limb_index : pair.limb,
            cur_limb_entries_left : ENTRIES_PER_LIMB - (min_index % ENTRIES_PER_LIMB),
            cur_limb,
            idx : 0,
            dim
        }
    }
}

impl<'a> Iterator for FpVector2IteratorNonzero<'a> {
    type Item = (usize, u32);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let tz = (self.cur_limb | 1u64.checked_shl(self.cur_limb_entries_left as u32).unwrap_or(0)).trailing_zeros();
            self.idx += tz as usize;
            self.cur_limb_entries_left -= tz as usize;
            // println!("cur_limb_entries_left : {}", self.cur_limb_entries_left);
            if self.idx >= self.dim {
                return None;
            }
            if self.cur_limb_entries_left == 0 {
                self.limb_index += 1;
                self.cur_limb_entries_left = 64;
                if self.limb_index < self.limbs.len() {
                    self.cur_limb = self.limbs[self.limb_index];
                } else {
                    return None;
                }
                continue;
            } 
            self.cur_limb >>= tz;
            if tz == 0 {
                break;
            }
        }
        let result = (self.idx, 1);
        self.idx += 1;
        self.cur_limb_entries_left -= 1;
        self.cur_limb >>= 1;
        Some(result)
    }
}

pub struct FpVector3IteratorNonzero<'a> {
    limbs : &'a Vec<u64>,
    limb_index : usize,
    cur_limb_entries_left : usize,
    cur_limb_bits_left : usize,
    cur_limb : u64,
    idx : usize
}

impl<'a> FpVector3IteratorNonzero<'a> {
    fn new(vec : &'a FpVector) -> Self {
        const BITS_PER_ENTRY : usize = 3;
        const ENTRIES_PER_LIMB : usize = 21;
        // const USABLE_BITS_PER_LIMB = ENTRIES_PER_LIMB * BITS_PER_ENTRY;
        let dim = vec.dimension() as isize;
        let limbs = vec.limbs();

        if dim == 0 {
            return Self {
                limbs,
                limb_index : 0,
                cur_limb_entries_left : 0,
                cur_limb_bits_left : 0,
                cur_limb: 0,
                idx : 0,
            }
        }
        let min_index = vec.min_index();
        let pair = limb_bit_index_pair(vec.prime(), min_index);
        let cur_limb = limbs[pair.limb] >> pair.bit_index;
        let cur_limb_entries_left = ENTRIES_PER_LIMB - (min_index % ENTRIES_PER_LIMB);
        let cur_limb_bits_left = cur_limb_entries_left * BITS_PER_ENTRY;
        Self {
            limbs,
            limb_index : pair.limb,
            cur_limb_entries_left,
            cur_limb_bits_left,
            cur_limb,
            idx : 0,
        }
    }
}

impl<'a> Iterator for FpVector3IteratorNonzero<'a> {
    type Item = (usize, u32);
    fn next(&mut self) -> Option<Self::Item> {
        const BITS_PER_ENTRY : usize = 3;
        const MASK : u64 = 0b111;
        const ENTRIES_PER_LIMB : usize = 21;
        const USABLE_BITS_PER_LIMB : usize = ENTRIES_PER_LIMB * BITS_PER_ENTRY;
        loop {
            let tz_real = (self.cur_limb | 1u64.checked_shl(self.cur_limb_bits_left as u32).unwrap_or(0)).trailing_zeros();
            let tz_rem = ((tz_real as u8) % (BITS_PER_ENTRY as u8)) as u32;
            let tz_div = ((tz_real as u8) / (BITS_PER_ENTRY as u8)) as u32;
            let tz = tz_real - tz_rem;
            self.idx += tz_div as usize;
            self.cur_limb_entries_left -= tz_div as usize;
            self.cur_limb_bits_left -= tz as usize;
            if self.cur_limb_entries_left == 0 {
                self.limb_index += 1;
                self.cur_limb_entries_left = ENTRIES_PER_LIMB;
                self.cur_limb_bits_left = USABLE_BITS_PER_LIMB;
                if self.limb_index < self.limbs.len() {
                    self.cur_limb = self.limbs[self.limb_index];
                } else {
                    return None;
                }
                continue;
            } 
            self.cur_limb >>= tz;
            if tz == 0 {
                break;
            }
        }
        let result = (self.idx, (self.cur_limb & MASK) as u32);
        self.idx += 1;
        self.cur_limb_entries_left -= 1;
        self.cur_limb_bits_left -= BITS_PER_ENTRY;
        self.cur_limb >>= BITS_PER_ENTRY;
        Some(result)
    }
}

pub struct FpVector5IteratorNonzero<'a> {
    limbs : &'a Vec<u64>,
    limb_index : usize,
    cur_limb_entries_left : usize,
    cur_limb_bits_left : usize,
    cur_limb : u64,
    idx : usize
}

impl<'a> FpVector5IteratorNonzero<'a> {
    fn new(vec : &'a FpVector) -> Self {
        const BITS_PER_ENTRY : usize = 5;
        const ENTRIES_PER_LIMB : usize = 12;
        // const USABLE_BITS_PER_LIMB = ENTRIES_PER_LIMB * BITS_PER_ENTRY;
        let dim = vec.dimension() as isize;
        let limbs = vec.limbs();

        if dim == 0 {
            return Self {
                limbs,
                limb_index : 0,
                cur_limb_entries_left : 0,
                cur_limb_bits_left : 0,
                cur_limb: 0,
                idx : 0,
            }
        }
        let min_index = vec.min_index();
        let pair = limb_bit_index_pair(vec.prime(), min_index);
        let cur_limb = limbs[pair.limb] >> pair.bit_index;
        let cur_limb_entries_left = ENTRIES_PER_LIMB - (min_index % ENTRIES_PER_LIMB);
        let cur_limb_bits_left = cur_limb_entries_left * BITS_PER_ENTRY;
        Self {
            limbs,
            limb_index : pair.limb,
            cur_limb_entries_left,
            cur_limb_bits_left,
            cur_limb,
            idx : 0,
        }
    }
}

impl<'a> Iterator for FpVector5IteratorNonzero<'a> {
    type Item = (usize, u32);
    fn next(&mut self) -> Option<Self::Item> {
        const BITS_PER_ENTRY : usize = 5;
        const MASK : u64 = 0b11111;
        const ENTRIES_PER_LIMB : usize = 12;
        const USABLE_BITS_PER_LIMB : usize = ENTRIES_PER_LIMB * BITS_PER_ENTRY;
        loop {
            let tz_real = (self.cur_limb | 1u64.checked_shl(self.cur_limb_bits_left as u32).unwrap_or(0)).trailing_zeros();
            let tz_rem = ((tz_real as u8) % (BITS_PER_ENTRY as u8)) as u32;
            let tz_div = ((tz_real as u8) / (BITS_PER_ENTRY as u8)) as u32;
            let tz = tz_real - tz_rem;
            self.idx += tz_div as usize;
            self.cur_limb_entries_left -= tz_div as usize;
            self.cur_limb_bits_left -= tz as usize;
            if self.cur_limb_entries_left == 0 {
                self.limb_index += 1;
                self.cur_limb_entries_left = ENTRIES_PER_LIMB;
                self.cur_limb_bits_left = USABLE_BITS_PER_LIMB;
                if self.limb_index < self.limbs.len() {
                    self.cur_limb = self.limbs[self.limb_index];
                } else {
                    return None;
                }
                continue;
            } 
            self.cur_limb >>= tz;
            if tz == 0 {
                break;
            }
        }
        let result = (self.idx, (self.cur_limb & MASK) as u32);
        self.idx += 1;
        self.cur_limb_entries_left -= 1;
        self.cur_limb_bits_left -= BITS_PER_ENTRY;
        self.cur_limb >>= BITS_PER_ENTRY;
        Some(result)
    }
}


#[allow(dead_code)]
pub struct FpVectorGenericIteratorNonzero<'a> {
    limbs : &'a Vec<u64>,
    limb_index : usize,
    cur_limb_entries_left : usize,
    cur_limb : u64,
    counter : isize,
    idx : usize
}

#[allow(unused_variables)]
impl<'a> FpVectorGenericIteratorNonzero<'a> {
    fn new(vec : &'a FpVector) -> Self {
        unimplemented!()
    }
}

impl<'a> Iterator for FpVectorGenericIteratorNonzero<'a> {
    type Item = (usize, u32);
    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}

pub enum FpVectorIteratorNonzero<'a> {
    FpVec2(FpVector2IteratorNonzero<'a>),
    FpVec3(FpVector3IteratorNonzero<'a>),
    FpVec5(FpVector5IteratorNonzero<'a>),
    FpVecGeneric(FpVectorGenericIteratorNonzero<'a>)
}

impl<'a> FpVectorIteratorNonzero<'a> {
    fn new(vec : &'a FpVector) -> Self {
        match *vec.prime() {
            2 => Self::FpVec2(FpVector2IteratorNonzero::new(vec)),
            3 => Self::FpVec3(FpVector3IteratorNonzero::new(vec)),
            5 => Self::FpVec5(FpVector5IteratorNonzero::new(vec)),
            _ => Self::FpVecGeneric(FpVectorGenericIteratorNonzero::new(vec)),
        }
    }
}

impl<'a> Iterator for FpVectorIteratorNonzero<'a> {
    type Item = (usize, u32);
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::FpVec2(vec) => vec.next(),
            Self::FpVec3(vec) => vec.next(),
            Self::FpVec5(vec) => vec.next(),
            Self::FpVecGeneric(vec) => vec.next(),
        }
    }
}



impl fmt::Display for FpVector {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "[{}]", self.iter().join(", "))?;
        Ok(())
    }
}

#[cfg(feature = "json")]
impl Serialize for FpVector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S : Serializer,
    {
        self.to_vector().serialize(serializer)
    }
}

#[cfg(feature = "json")]
impl<'de> Deserialize<'de> for FpVector {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
        where D : Deserializer<'de>
    {
        panic!("Deserializing FpVector not supported");
        // This is needed for ext-websocket/actions to be happy
    }
}

pub struct FpVectorSlice<'a> {
    old_slice: (usize, usize),
    inner: &'a mut FpVector
}

impl<'a> Drop for FpVectorSlice<'a> {
    fn drop(&mut self) {
        self.inner.restore_slice(self.old_slice);
    }
}

impl std::ops::Deref for FpVectorSlice<'_> {
    type Target = FpVector;

    fn deref(&self) -> &FpVector {
        &self.inner
    }
}

impl std::ops::DerefMut for FpVectorSlice<'_> {
    fn deref_mut(&mut self) -> &mut FpVector {
        &mut self.inner
    }
}
/// An FpVectorMask encodes a subset of the basis elements of an Fp vector space. This is used to
/// project onto the subspace spanned by the selected basis elements.
#[derive(Debug)]
pub struct FpVectorMask {
    p : ValidPrime,
    dimension : usize,
    masks : Vec<u64>
}

impl FpVectorMask {
    pub fn new(p : ValidPrime, dimension : usize) -> Self {
        let number_of_limbs = FpVector::number_of_limbs(p, dimension);
        Self {
            p,
            dimension,
            masks : vec![!0; number_of_limbs]
        }
    }

    pub fn set_zero(&mut self) {
        for limb in &mut self.masks {
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
    #[allow(clippy::needless_range_loop)]
    pub fn apply(&self, target : &mut FpVector) {
        debug_assert_eq!(self.dimension, target.dimension());
        debug_assert_eq!(target.vector_container().slice_start, 0);
        debug_assert_eq!(target.vector_container().slice_end, target.dimension());

        let target = &mut target.vector_container_mut().limbs;
        for i in 0 .. self.masks.len() {
            target[i] &= self.masks[i];
        }
    }

    #[allow(clippy::needless_range_loop)]
    pub fn contains(&self, target : &FpVector) -> bool {
        debug_assert_eq!(self.dimension, target.dimension());
        debug_assert_eq!(target.vector_container().slice_start, 0);
        debug_assert_eq!(target.vector_container().slice_end, target.dimension());

        let target = &target.vector_container().limbs;
        for i in 0 .. self.masks.len() {
            if target[i] & self.masks[i] != target[i] {
                return false;
            }
        }
        return true;
    }    
}

pub struct VectorDiffEntry {
    pub index : usize,
    pub left : u32,
    pub right : u32
}

impl FpVector {
    pub fn diff_list(&self, other : &Vec<u32>) -> Vec<VectorDiffEntry> {
        assert!(self.dimension() == other.len());
        let mut result = Vec::new();
        for index in 0 .. self.dimension() {
            let left = self.entry(index);
            let right = other[index];
            if left != right {
                result.push(VectorDiffEntry{
                    index,
                    left,
                    right
                });
            }
        }
        result
    }

    pub fn diff_vec(&self, other : &FpVector) -> Vec<VectorDiffEntry> {
        assert!(self.dimension() == other.dimension());
        let mut result = Vec::new();
        for index in 0 .. self.dimension() {
            let left = self.entry(index);
            let right = other.entry(index);
            if left != right {
                result.push(VectorDiffEntry{
                    index,
                    left,
                    right
                });
            }
        }
        result
    }
    
    pub fn format_diff(diff : Vec<VectorDiffEntry>) -> String {
        let data_formatter = diff.iter().format_with("\n ", |VectorDiffEntry {index, left, right}, f| 
            f(&format_args!("  At index {}: {}!={}", index, left, right))
        );
        format!("{}", data_formatter)
    }

    pub fn assert_list_eq(&self, other : &Vec<u32>){
        let diff = self.diff_list(other);
        if diff.len() == 0 {
            return;
        }
        println!("assert {} == {:?}", self,other);
        println!("{}", FpVector::format_diff(diff));
    }

    pub fn assert_vec_eq(&self, other : &FpVector){
        let diff = self.diff_vec(other);
        if diff.len() == 0 {
            return;
        }
        println!("assert {} == {}", self,other);
        println!("{}", FpVector::format_diff(diff));
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
    type AuxData = ValidPrime;

    fn load(buffer : &mut impl Read, p : &ValidPrime) -> io::Result<Self> {
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

        #[cfg(feature = "prime-two")]
        let result = FpVector::from(FpVector2 { vector_container });

        #[cfg(not(feature = "prime-two"))]
        let result = match *p  {
            2 => FpVector::from(FpVector2 { vector_container }),
            3 => FpVector::from(FpVector3 { vector_container }),
            5 => FpVector::from(FpVector5 { vector_container }),
            _ => FpVector::from(FpVectorGeneric { p, vector_container })
        };

        Ok(result)
    }
}


impl Hash for FpVector {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dimension().hash(state);
        if self.dimension() == 0 {
            return;
        }
        let dat = AddShiftNoneData::new(self, self);
        let mut i = 0; {
            dat.mask_first_limb(self, i).hash(state);
        }
        for i in 1 .. dat.number_of_limbs - 1 {
            dat.mask_middle_limb(self, i).hash(state);
        }
        i = dat.number_of_limbs - 1;
        if i > 0 {
            dat.mask_last_limb(self, i).hash(state);
        }
    }
}

#[cfg(test)]
#[allow(clippy::needless_range_loop)]
mod tests {
    use super::*;
    use rand::Rng;
    use rstest::rstest;

    fn random_vector(p : u32, dimension : usize) -> Vec<u32> {
        let mut result = Vec::with_capacity(dimension);
        let mut rng = rand::thread_rng();
        for _ in 0..dimension {
            result.push(rng.gen::<u32>() % p);
        }
        result
    }

    #[rstest(p, case(3), case(5), case(7))]
    fn test_reduce_limb(p : u32){
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        for &dim in &[10, 20, 70, 100, 1000] {
            println!("p: {}, dim: {}", p, dim);
            let mut v = FpVector::new(p_, dim);
            let mut v_arr = random_vector(p*(p-1), dim);
            v.pack(&v_arr);
            v.reduce_limbs(v.min_limb(), v.max_limb());
            for i in 0..dim {
                v_arr[i] = v_arr[i] % p;
            }
            v.assert_list_eq(&v_arr);
        }
    }

    #[rstest(p,  case(2), case(3), case(5))]//, case(7))]
    fn test_add(p : u32){
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        for &dim in &[10, 20, 70, 100, 1000] {
            println!("p: {}, dim: {}", p, dim);
            let mut v = FpVector::new(p_, dim);
            let mut w = FpVector::new(p_, dim);
            let mut v_arr = random_vector(p, dim);
            let w_arr = random_vector(p, dim);
            v.pack(&v_arr);
            w.pack(&w_arr);
            v.add(&w, 1);
            for i in 0..dim {
                v_arr[i] = (v_arr[i] + w_arr[i]) % p;
            }
            v.assert_list_eq(&v_arr);
        }
    }

    #[rstest(p,  case(2), case(3), case(5), case(7))]
    fn test_scale(p : u32){
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        for &dim in &[10, 20, 70, 100, 1000] {
            println!("p: {}, dim: {}", p, dim);
            let mut v = FpVector::new(p_, dim);
            let mut v_arr = random_vector(p, dim);
            let mut rng = rand::thread_rng();
            let c = rng.gen::<u32>() % p;
            v.pack(&v_arr);
            v.scale(c);
            for entry in &mut v_arr {
                *entry = (*entry * c) % p;
            }
            v.assert_list_eq(&v_arr);
        }
    }

    #[rstest(p, case(2), case(3), case(5), case(7))]
    fn test_entry(p : u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for &dim in &dim_list {
            let mut v = FpVector::new(p_, dim);
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

    #[rstest(p,  case(2), case(3), case(5), case(7))]//
    fn test_entry_slice(p : u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let mut v = FpVector::new(p_, dim);
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

    #[rstest(p,  case(2), case(3), case(5), case(7))]
    fn test_set_entry(p : u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for &dim in &dim_list {
            let mut v = FpVector::new(p_, dim);
            let v_arr = random_vector(p, dim);
            for i in 0..dim {
                v.set_entry(i, v_arr[i]);
            }
            v.assert_list_eq(&v_arr);
        }
    }

    #[rstest(p,  case(2), case(3), case(5), case(7))]//
    fn test_set_entry_slice(p : u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let mut v = FpVector::new(p_, dim);
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
    #[rstest(p,  case(2), case(3), case(5), case(7))]
    fn test_set_to_zero_slice(p : u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            println!("slice_start : {}, slice_end : {}", slice_start, slice_end);
            let mut v_arr = random_vector(p, dim);
            v_arr[0] = 1; // make sure that v isn't zero
            let mut v = FpVector::new(p_, dim);
            v.pack(&v_arr);
            v.set_slice(slice_start, slice_end);
            v.set_to_zero();
            assert!(v.is_zero());
            v.clear_slice();
            assert!(!v.is_zero()); // The first entry is 1, so it's not zero.
            for i in slice_start .. slice_end {
                v_arr[i] = 0;
            }            
            v.assert_list_eq(&v_arr);
        }
    }

    #[rstest(p, case(2), case(3), case(5))]//, case(7))]//
    fn test_add_slice_to_slice(p : u32) {
        let p_ = ValidPrime::new(p);
        println!("p : {}", p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let mut v_arr = random_vector(p, dim);
            let mut v = FpVector::new(p_, dim);
            v.pack(&v_arr);
            let w_arr = random_vector(p, dim);
            let mut w = FpVector::new(p_, dim);
            w.pack(&w_arr);
            v.set_slice(slice_start, slice_end);
            w.set_slice(slice_start, slice_end);
            v.add(&w, 1);
            v.clear_slice();
            for i in slice_start .. slice_end {
                v_arr[i] = (v_arr[i] + w_arr[i]) % p;
            }
            v.assert_list_eq(&v_arr);
        }
    }

    // Tests assign and Eq
    #[rstest(p, case(2), case(3), case(5), case(7))]//
    fn test_assign(p : u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        for &dim in &[10, 20, 70, 100, 1000] {
            println!("p: {}, dim: {}", p, dim);
            let mut v = FpVector::new(p_, dim);
            let mut w = FpVector::new(p_, dim);
            let v_arr = random_vector(p, dim);
            let w_arr = random_vector(p, dim);
            v.pack(&v_arr);
            w.pack(&w_arr);
            v.assign(&w);
            v.assert_vec_eq(&w);
        }
    }

    #[rstest(p, case(2), case(3), case(5))]//, case(7))]//
    fn test_assign_slice_to_slice(p : u32) {
        let p_ = ValidPrime::new(p);
        println!("p : {}", p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let mut v_arr = random_vector(p, dim);
            v_arr[0] = 1; // Ensure v != w.
            let mut v = FpVector::new(p_, dim);
            v.pack(&v_arr);
            let mut w_arr = random_vector(p, dim);
            w_arr[0] = 0; // Ensure v != w.
            let mut w = FpVector::new(p_, dim);
            w.pack(&w_arr);
            v.set_slice(slice_start, slice_end);
            w.set_slice(slice_start, slice_end);
            v.assign(&w);
            v.clear_slice();
            w.clear_slice();
            for i in slice_start .. slice_end {
                v_arr[i] = w_arr[i];
            }
            v.assert_list_eq(&v_arr);
        }
    }

    #[rstest(p, case(2), case(3), case(5))]//, case(7))]
    fn test_add_shift_right(p : u32) {
        let p_ = ValidPrime::new(p);
        println!("p : {}", p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let mut v_arr = random_vector(p, dim);
            let mut v = FpVector::new(p_, dim);
            v.pack(&v_arr);
            let w_arr = random_vector(p, dim);
            let mut w = FpVector::new(p_, dim);
            w.pack(&w_arr);
            v.set_slice(slice_start + 2, slice_end + 2);
            w.set_slice(slice_start, slice_end);
            v.add(&w, 1);
            v.clear_slice();
            println!("v : {}", v);
            for i in slice_start + 2 .. slice_end + 2 {
                v_arr[i] = (v_arr[i] + w_arr[i - 2]) % p;
            }
            v.assert_list_eq(&v_arr);            
        }
    }

    #[rstest(p, case(2), case(3), case(5))]//, case(7))]
    fn test_add_shift_left(p : u32) {
        let p_ = ValidPrime::new(p);
        println!("p : {}", p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let mut v_arr = random_vector(p, dim);
            let mut v = FpVector::new(p_, dim);
            v.pack(&v_arr);
            let w_arr = random_vector(p, dim);
            let mut w = FpVector::new(p_, dim);
            w.pack(&w_arr);
            v.set_slice(slice_start - 2, slice_end - 2);
            w.set_slice(slice_start, slice_end);
            v.add(&w, 1);
            v.clear_slice();
            for i in slice_start - 2 .. slice_end - 2 {
                v_arr[i] = (v_arr[i] + w_arr[i + 2]) % p;
            }
            v.assert_list_eq(&v_arr);
        }
    }

    #[rstest(p, case(2), case(3), case(5))]//, case(7))]
    fn test_iterator_slice(p : u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let ep = entries_per_64_bits(p_);
        for &dim in &[5, 10, ep, ep - 1, ep + 1, 3 * ep, 3 * ep - 1, 3 * ep + 1] {
            let mut v = FpVector::new(p_, dim);
            let v_arr = random_vector(p, dim);
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

    #[rstest(p, case(2), case(3), case(5), case(7))]
    fn test_iterator_skip(p : u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let ep = entries_per_64_bits(p_);
        let dim = 5 * ep;
        for &num_skip in &[ep, ep - 1, ep + 1, 3 * ep, 3 * ep - 1, 3 * ep + 1, 6 * ep] {
            let mut v = FpVector::new(p_, dim);
            let v_arr = random_vector(p, dim);
            v.pack(&v_arr);

            let mut w = v.iter();
            w.skip_n(num_skip);
            let mut counter = 0;
            for (i, x) in w.enumerate() {
                assert_eq!(v.entry(i + num_skip), x);
                counter += 1;
            }
            if num_skip == 6 * ep {
                assert_eq!(counter, 0);
            } else {
                assert_eq!(counter, v.dimension() - num_skip);
            }
        }
    }

    #[rstest(p, case(2), case(3), case(5), case(7))]
    fn test_iterator(p : u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let ep = entries_per_64_bits(p_);
        for &dim in &[0, 5, 10, ep, ep - 1, ep + 1, 3 * ep, 3 * ep - 1, 3 * ep + 1] {
            let mut v = FpVector::new(p_, dim);
            let v_arr = random_vector(p, dim);
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
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        assert_eq!(vec.len(), mask.len());
        let mut v = FpVector::from_vec(p_, vec);
        let mut m = FpVectorMask::new(p_, vec.len());
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
    
    #[rstest(p, case(2), case(3), case(5))]
    fn test_add_truncate(p : u32){
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        for &dim in &[10, 20, 70, 100, 1000] {
            println!("p: {}, dim: {}", p, dim);
            let mut v = FpVector::new(p_, dim);
            let mut w = FpVector::new(p_, dim);
            let mut v_arr = random_vector(p, dim);
            let w_arr = random_vector(p, dim);
            v.pack(&v_arr);
            w.pack(&w_arr);
            let ok_q = v.add_truncate(&w, 1).is_ok();
            v.clear_slice();
            if ok_q {
                for i in 0..dim {
                    v_arr[i] = (v_arr[i] + w_arr[i]) % p;
                }
                v.assert_list_eq(&v_arr);
            } else {
                let mut carried = false;
                for i in 0..dim {
                    if (v_arr[i] + w_arr[i]) >= p {
                        carried = true;
                        break;
                    }
                }
                assert!(carried);
            }
        }
    }

    #[rstest(p, case(2), case(3), case(5))]//, case(7))]
    fn test_add_truncate_shift_right(p : u32) {
        let p_ = ValidPrime::new(p);
        println!("p : {}", p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let mut v_arr = random_vector(p, dim);
            let mut v = FpVector::new(p_, dim);
            v.pack(&v_arr);
            let w_arr = random_vector(p, dim);
            let mut w = FpVector::new(p_, dim);
            w.pack(&w_arr);
            v.set_slice(slice_start + 2, slice_end + 2);
            w.set_slice(slice_start, slice_end);
            let ok_q = v.add_truncate(&w, 1).is_ok();
            v.clear_slice();
            println!("\nok_q: {}\n" , ok_q);
            if ok_q {
                for i in slice_start + 2 .. slice_end + 2 {
                    v_arr[i] = (v_arr[i] + w_arr[i - 2]) % p;
                }
                v.assert_list_eq(&v_arr);
            } else {
                let mut carried = false;
                for i in slice_start + 2 .. slice_end + 2 {
                    if (v_arr[i] + w_arr[i - 2]) >= p {
                        carried = true; 
                        break;
                    }
                }
                assert!(carried);
            }
        }
    }

    #[rstest(p, case(2), case(3), case(5))]//, case(7))]
    fn test_add_truncate_shift_left(p : u32) {
        let p_ = ValidPrime::new(p);
        println!("p : {}", p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let mut v_arr = random_vector(p, dim);
            let mut v = FpVector::new(p_, dim);
            v.pack(&v_arr);
            let w_arr = random_vector(p, dim);
            let mut w = FpVector::new(p_, dim);
            w.pack(&w_arr);
            v.set_slice(slice_start - 2, slice_end - 2);
            w.set_slice(slice_start, slice_end);
            let ok_q = v.add_truncate(&w, 1).is_ok();
            v.clear_slice();
            if ok_q {
                for i in slice_start - 2 .. slice_end - 2 {
                    v_arr[i] = (v_arr[i] + w_arr[i + 2]) % p;
                }
                v.assert_list_eq(&v_arr);
            } else {
                let mut carried = false;
                for i in slice_start - 2 .. slice_end - 2 {
                    if (v_arr[i] + w_arr[i + 2]) >= p {
                        carried = true;
                        break;
                    }
                }
                assert!(carried);
            }
            v.clear_slice();
        }
    }




    #[rstest(p, case(2), case(3), case(5))]
    fn test_add_carry(p : u32){
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        for &dim in &[10, 20, 70, 100, 1000] {
            println!("p: {}, dim: {}", p, dim);
            const E_MAX : usize = 4;
            let p_to_the_e_max = (p*p*p*p)*p;
            let mut v = Vec::with_capacity(E_MAX + 1);
            let mut w = Vec::with_capacity(E_MAX + 1);
            for _ in 0 ..= E_MAX {
                v.push(FpVector::new(p_, dim));
                w.push(FpVector::new(p_, dim));
            }
            let v_arr = random_vector(p_to_the_e_max, dim);
            let w_arr = random_vector(p_to_the_e_max, dim);
            for i in 0 .. dim {
                let mut ev = v_arr[i];
                let mut ew = w_arr[i];
                for e in 0..=E_MAX {
                    v[e].set_entry(i, ev % p);
                    w[e].set_entry(i, ew % p);
                    ev /= p;
                    ew /= p;
                }
            }
            
            println!("in  : {:?}", v_arr);
            for e in 0 ..= E_MAX {
                println!("in {}: {}", e, v[e]);
            }
            println!("");
            
            println!("in  : {:?}", w_arr);
            for e in 0 ..= E_MAX {
                println!("in {}: {}", e, w[e]);
            }
            println!("");

            for e in 0 ..= E_MAX {
                let (first, rest) = v[e..].split_at_mut(1);
                first[0].add_carry(&w[e], 1, rest);
            }

            let mut vec_result = vec![0; dim];
            for i in 0 .. dim {
                for e in (0 ..= E_MAX).rev() {
                    vec_result[i] *= p;
                    vec_result[i] += v[e].entry(i);
                }
            }

            for e in 0 ..= E_MAX {
                println!("out{}: {}", e, v[e]);
            }
            println!("");

            let mut comparison_result = vec![0; dim];
            for i in 0 .. dim {
                comparison_result[i] = (v_arr[i] + w_arr[i]) % p_to_the_e_max;
            }
            println!("out : {:?}", comparison_result);


            let mut diffs = Vec::new();
            let mut diffs_str = String::new();
            for i in 0..dim {
                if vec_result[i] != comparison_result[i] {
                    diffs.push((i, comparison_result[i], vec_result[i]));
                    diffs_str.push_str(&format!(
                        "\nIn position {} expected {} got {}. v[i] = {}, w[i] = {}.", 
                        i, comparison_result[i], vec_result[i],
                        v_arr[i], w_arr[i]
                    ));
                }
            }
            assert!(diffs == [], "{}", diffs_str);
        }
    }

    #[rstest(p, case(2))]//, case(3), case(5))]//, case(7))]
    fn test_iter_nonzero_empty(p : u32) {
        let p_ = ValidPrime::new(p);
        let v = FpVector::new(p_, 0);
        for (_idx, _v) in v.iter_nonzero() {
            assert!(false);
        }
    }

    #[rstest(p, case(2))]//, case(7))]
    fn test_iter_nonzero_slice(p : u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let mut v = FpVector::new(p_, 5);
        v.set_entry(0, 1);
        v.set_entry(1, 1);
        v.set_entry(2, 1);
        v.set_slice(0, 1);
        for (i, _) in v.iter_nonzero() {
            assert!(i == 0);
        }
    }

    #[rstest(p, case(2), case(3), case(5))]//, case(7))]
    fn test_iter_nonzero(p : u32) {
        let p_ = ValidPrime::new(p);
        println!("p : {}", p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [20, 66, 100, 270, 1000];
        for i in 0..dim_list.len() {
            let dim = dim_list[i];
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start)/2;
            let v_arr = random_vector(p, dim);
            let mut v = FpVector::new(p_, dim);
            v.pack(&v_arr);
            v.set_slice(slice_start, slice_end);
            let mut result = Vec::new();
            for (idx, e) in v.iter_nonzero() {
                result.push((idx, e));
            }
            let mut comparison_result = Vec::new();
            for i in slice_start..slice_end {
                if v_arr[i] != 0 {
                    comparison_result.push((i - slice_start, v_arr[i]));
                }
            }

            // println!("v    : {}", v);
            // println!("v_arr: {:?}", v_arr);

            let mut i = 0;
            let mut j = 0;
            let mut diffs_str = String::new();
            while i < result.len() && j < comparison_result.len() {
                if result[i] != comparison_result[j] {
                    if result[i].0 < comparison_result[j].0 {
                        diffs_str.push_str(&format!(
                            "\n({:?}) present in result, missing from comparison_result", result[i]
                        ));
                        i += 1;
                    } else {
                        diffs_str.push_str(&format!(
                            "\n({:?}) present in comparison_result, missing from result", comparison_result[j]
                        ));
                        j += 1;                        
                    }
                } else {
                    i += 1;
                    j += 1;
                }
            }
            // for i in 0 .. std::cmp::min(result.len(), comparison_result.len()) {
            //     println!("res : {:?}, comp : {:?}", result[i], comparison_result[i]);
            // }
            assert!(diffs_str == "", "{}", diffs_str);
        }
    }

    #[test]
    fn test_sign_rule_limb(){
        let p = 2;
        let p_ = ValidPrime::new(p);
        let dummy_vec = FpVector::new(p_, 0);
        assert!(dummy_vec.sign_rule_limb(1, 0b10) == 1);
        assert!(dummy_vec.sign_rule_limb(0b10, 1) == 0);
        assert!(dummy_vec.sign_rule_limb(0x84012c02,0x6b920241) == 1);
        assert!(dummy_vec.sign_rule_limb(0x6b920241, 0x84012c02) == 0);
    }

    #[test]
    fn test_sign_rule(){
        let p = 2;
        let p_ = ValidPrime::new(p);        
        let mut in1 = FpVector::new(p_, 128);
        let mut in2 = FpVector::new(p_, 128);
        let tests = [
            (0x181e20846a820820, 0x2122a1a08c1a0069, 0xe30140608100e540, 0xd2180e4350008004, false, false),
            (0x2090400020017044, 0xa04e0802080000e1, 0x18298a0a85080089, 0x050020311030411a, false, false),
            (0x082080022408d510, 0x538a000802078210, 0x2355308c4a920002, 0x00058130800000a2, true, true),
            (0x33a0824922050704, 0x00400520a0800404, 0x00090836000a980b, 0x4801d005064b9840, false, false),
            (0x290c14040154a01b, 0x38014102810a0245, 0x0093281a620a1060, 0x029014cd0684080a, true, true),
            (0x240255b490b0e040, 0x0815414130548881, 0x8ad4880a00000416, 0xb660a4b84cab002c, true, true),
            (0x010c000060840540, 0x8008001480104028, 0x8842938396233a31, 0x5e20400311059a41, true, true),
            (0x02012141008e5081, 0x2829060241920a00, 0xe0208a1a47102310, 0x051240010e6c4008, false, false),
            (0x200812011081880f, 0x100661c082625864, 0x48840c76c03a2380, 0x861088274000060a, false, false),
            (0x84000f5490449008, 0x00891820f4623401, 0x107490a964b802a4, 0x40024487008800b0, false, false),
            (0x080448a2db282c41, 0x2c100011e00097dd, 0x0131024124844028, 0x8329600202440002, false, false),
            (0x441c60a208c2e206, 0x00a4210b50049281, 0x0842020160091158, 0x48131424846a6868, true, true),
            (0xc2743ad490a21411, 0x0150221280868050, 0x1082402043040888, 0xdc070000021128a0, true, true),
            (0x0614030849072140, 0x0e7a710422002540, 0x300904418240c422, 0x80850ccad8a10200, false, true),
            (0x90080028402bc624, 0x215002cf204840a0, 0x6373f01012001042, 0x420b111008350859, false, true),
            (0x4220c41100513301, 0x332c050498c21102, 0x0c0c206c8a008044, 0xc0024840461484d0, true, false),
            (0x0353a04b08000010, 0x3e00045295202851, 0x60040810a42a1284, 0x001d680860800080, true, false),
            (0x084801c0c2100581, 0x1820090035001080, 0x3111121b0522185c, 0x01404209002c080c, true, false),
            (0x414800000823a20e, 0x008074081080a214, 0x1a12852095d040c0, 0x8119003425575408, false, true),
            (0x210c730112098440, 0x01c0b106111483d0, 0x920004486810020c, 0xb614405084c30004, true, true),
            (0x60210168b8802094, 0x2a10021a4b08420c, 0x1554000102241028, 0x04048d0000349000, true, true),
            (0x81200240041188c8, 0x148008c1c6220818, 0x0082a92c10000010, 0x0050500800100084, true, false),
            (0x4593105c94090408, 0x820029daa0026830, 0x1864242101429200, 0x1822060103290348, true, false),
            (0x551a0002870e6000, 0x0040a00040353a00, 0x200409c110101589, 0x28870e620a488442, true, false),
            (0x8a0200806440124b, 0x9c6000904e824800, 0x5150404003022c84, 0x2014452420012031, true, false),
            (0x840216c970c02c10, 0x16490c8222011000, 0x4a6040120034800b, 0x09008001d4166827, false, true),
            (0x042040900809589c, 0x4102064021804040, 0x98903b221480a523, 0x964840081847130e, false, false),
            (0xa005ed201240a002, 0x580903106014a842, 0x16680288c4321521, 0x2030400608021010, true, true),
            (0x405008860b020123, 0x2100052200602aee, 0xb809422040018014, 0x0a21a20090041001, true, true),
            (0x3108541538030498, 0x014302a04a20a081, 0x0080806005804804, 0xdc00700020cc405c, true, true),
            (0x6020490087030a00, 0x008a11c320049998, 0x069512591824a091, 0x4a300a0808002006, true, true),
            (0x206e90b404108a02, 0x4a0408221400b022, 0x0580040201607498, 0x0131d21d80080b08, false, false),
            (0x84811204041e00bd, 0x011410092c824801, 0x0162802203216100, 0xd8200844514c8040, false, false),
            (0x0020000005800845, 0x4c19021081244589, 0x56026e803008012a, 0x916081a350103000, true, true),
            (0x407050c08808e102, 0x1102095040020904, 0x000187005245184c, 0x28104485228804e3, true, true),
            (0x6d20550000808446, 0x4008211019808425, 0x804e20c004212381, 0x02305c0542603848, false, false),
            (0x8010400016110202, 0x5a40a22409e0220c, 0x04e20103604a3980, 0x80181142f20a9103, false, true),
            (0x002c12089073280e, 0x80c8680090b66020, 0xd8c12d02488850a0, 0x010217794101901c, false, true),
            (0x290c01102e12800c, 0x4c881498c852154e, 0x86c0142101a810b2, 0x31420a2623a40091, false, true),
            (0xe08400012018c888, 0x020204c23b0a1010, 0x0301230249420426, 0x01340a3084204282, false, true),
            (0x4038ea62022e8480, 0x4098130044062cf8, 0x2400009810006028, 0xb200606800900100, true, true),
            (0x502000190002d410, 0x0438100a01024d00, 0x2217c2025085020a, 0xa302e11110002008, false, false),
            (0x4200400240411212, 0xb816804201c00229, 0x94401924308a01c8, 0x41203911e0009114, true, true),
            (0x00181012e8048110, 0xa040200b8c000504, 0xe2c08424148b3621, 0x04a6473461be288b, false, false),
            (0x118930450a104281, 0x601aa1629118e100, 0x0072c190b1208908, 0x8125461c400018cd, false, true),
            (0x6420649001148862, 0xb8140a29851b311c, 0x93c9180820881088, 0x014040400a000040, true, true),
            (0x080622a043c60190, 0x2103c10f04000312, 0x1120404098087809, 0x00000090f8918000, false, false),
            (0xc19e4204800b0b88, 0x008040504c102020, 0x3000844216406441, 0x4e450203006dc014, false, false),
            (0xc0204c082c200c01, 0x13046c600e0044c1, 0x01cb111600005240, 0x8012028130c18800, false, false),
            (0x80e1850014a56020, 0x20055110c8011012, 0x240422904200918e, 0x10d02c21213442a0, true, true)
        ];
        let mut diffs = Vec::new();
        for &(in1_limb1, in1_limb2, in2_limb1, in2_limb2, res1, res2) in tests.iter() {
            in1.limbs_mut()[1] = in1_limb1;
            in1.limbs_mut()[0] = in1_limb2;
            in2.limbs_mut()[1] = in2_limb1;
            in2.limbs_mut()[0] = in2_limb2;
            let test_res1 = in1.sign_rule(&in2);
            let test_res2 = in2.sign_rule(&in1);
            let res = (res1, res2);
            let test_res = (test_res1, test_res2);
            let tuple = (in1_limb1, in1_limb2, in2_limb1, in2_limb2);
            let popcnts = (in1_limb1.count_ones() %2, in1_limb2.count_ones()%2, in2_limb1.count_ones()%2, in2_limb2.count_ones()%2);
            if res != test_res {
                diffs.push((tuple, popcnts, res, test_res))
            }
        }
        if !diffs.is_empty() {
            let formatter = diffs.iter().format_with("\n",
                |(tuple, popcnts, res, test_res), f| f(&format_args!(
                    "   Inputs: {:x?}\n      expected {:?}, got {:?}. popcnts: {:?}", tuple,  res, test_res, popcnts
                ))
            );
            assert!(false, "\nFailed test cases:\n {}", formatter);
        }
    }
}
