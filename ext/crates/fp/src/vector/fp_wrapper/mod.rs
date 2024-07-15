//! This module provides convenience wrappers around the contents of [`crate::vector::inner`] in the
//! special case where the field is a prime field. The main purpose is to put [`FqVector`] for
//! different fields `Fp<P>` into a single enum, and to simplify scalars to just `u32`s instead of
//! rather unwieldy `FieldElement<Fp<P>>`s. It does the same for the various slice structs.
//!
//! The main magic occurs in the macros, such as `dispatch_vector_inner`, which we use to provide
//! wrapper functions around the `FqVector` functions. To maintain consistency, we define the API
//! in this file irrespective of whether the `odd-primes` feature is enabled or not, and it is the
//! macros that will take care of making the distinction.
//!
//! Note: Since we still want to have scalars simply be `u32`s, even when `odd-primes` is disabled,
//! we can't simply define `type FpVector = FqVector<Fp<2>>` like we previously did: we need to use
//! a transparent wrapper.

use std::{
    convert::TryInto,
    io::{Read, Write},
    mem::size_of,
};

use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::iter::{FqVectorIteratorP, FqVectorNonZeroIteratorP};
use crate::{
    field::{field_internal::FieldInternal, Fp},
    limb::Limb,
    prime::Prime,
    vector::inner::{FqVector, SliceMutP, SliceP},
};

mod helpers;

#[cfg(feature = "odd-primes")]
#[macro_use]
mod macros_generic;
#[cfg(feature = "odd-primes")]
use macros_generic::{dispatch_struct, dispatch_vector, impl_try_into, use_primes};

#[cfg(not(feature = "odd-primes"))]
#[macro_use]
mod macros_2;
#[cfg(not(feature = "odd-primes"))]
use macros_2::{dispatch_struct, dispatch_vector, impl_try_into, use_primes};

use_primes!();

dispatch_struct! {
    #[derive(Debug, Hash, Eq, PartialEq, Clone)]
    pub FpVector from FqVector
}

dispatch_struct! {
    #[derive(Debug, Copy, Clone)]
    pub Slice<'a> from SliceP
}

dispatch_struct! {
    #[derive(Debug)]
    pub SliceMut<'a> from SliceMutP
}

dispatch_struct! {
    pub FpVectorIterator<'a> from FqVectorIteratorP
}

dispatch_struct! {
    pub FpVectorNonZeroIterator<'a> from FqVectorNonZeroIteratorP
}

impl FpVector {
    dispatch_vector! {
        pub fn prime(&self) -> ValidPrime;
        pub fn len(&self) -> usize;
        pub fn is_empty(&self) -> bool;
        pub fn @scale(&mut self, c: u32);
        pub fn set_to_zero(&mut self);
        pub fn @entry(&self, index: usize) -> u32;
        pub fn @set_entry(&mut self, index: usize, value: u32);
        pub fn assign(&mut self, other: &Self);
        pub fn assign_partial(&mut self, other: &Self);
        pub fn @add(&mut self, other: &Self, c: u32);
        pub fn @add_offset(&mut self, other: &Self, c: u32, offset: usize);
        pub fn slice(&self, start: usize, end: usize) -> (dispatch Slice);
        pub fn as_slice(&self) -> (dispatch Slice);
        pub fn slice_mut(&mut self, start: usize, end: usize) -> (dispatch SliceMut);
        pub fn as_slice_mut(&mut self) -> (dispatch SliceMut);
        pub fn is_zero(&self) -> bool;
        pub fn iter(&self) -> (dispatch FpVectorIterator);
        pub fn iter_nonzero(&self) -> (dispatch FpVectorNonZeroIterator);
        pub fn extend_len(&mut self, dim: usize);
        pub fn set_scratch_vector_size(&mut self, dim: usize);
        pub fn @add_basis_element(&mut self, index: usize, value: u32);
        pub fn @copy_from_slice(&mut self, slice: &[u32]);
        pub(crate) fn trim_start(&mut self, n: usize);
        pub fn @add_truncate(&mut self, other: &Self, c: u32) -> (Option<()>);
        pub fn sign_rule(&self, other: &Self) -> bool;
        pub fn @add_carry(&mut self, other: &Self, c: u32, rest: &mut [Self]) -> bool;
        pub fn @first_nonzero(&self) -> (Option<(usize, u32)>);
        pub fn density(&self) -> f32;

        pub(crate) fn limbs(&self) -> (&[Limb]);
        pub(crate) fn limbs_mut(&mut self) -> (&mut [Limb]);

        pub fn new<P: Prime>(p: P, len: usize) -> (from FqVector);
        pub fn new_with_capacity<P: Prime>(p: P, len: usize, capacity: usize) -> (from FqVector);
    }

    pub fn from_slice<P: Prime>(p: P, slice: &[u32]) -> Self {
        let mut v = Self::new(p, slice.len());
        v.copy_from_slice(slice);
        v
    }

    // Convenient for some matrix methods
    pub(crate) fn num_limbs(p: ValidPrime, len: usize) -> usize {
        Fp::new(p).number(len)
    }

    // Convenient for some matrix methods
    pub(crate) fn padded_len(p: ValidPrime, len: usize) -> usize {
        Self::num_limbs(p, len) * Fp::new(p).entries_per_limb()
    }

    pub fn update_from_bytes(&mut self, data: &mut impl Read) -> std::io::Result<()> {
        let limbs = self.limbs_mut();

        if cfg!(target_endian = "little") {
            let num_bytes = std::mem::size_of_val(limbs);
            unsafe {
                let buf: &mut [u8] =
                    std::slice::from_raw_parts_mut(limbs.as_mut_ptr() as *mut u8, num_bytes);
                data.read_exact(buf).unwrap();
            }
        } else {
            for entry in limbs {
                let mut bytes: [u8; size_of::<Limb>()] = [0; size_of::<Limb>()];
                data.read_exact(&mut bytes)?;
                *entry = Limb::from_le_bytes(bytes);
            }
        };
        Ok(())
    }

    pub fn from_bytes(p: ValidPrime, len: usize, data: &mut impl Read) -> std::io::Result<Self> {
        let mut v = Self::new(p, len);
        v.update_from_bytes(data)?;
        Ok(v)
    }

    pub fn to_bytes(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        // self.limbs is allowed to have more limbs than necessary, but we only save the
        // necessary ones.
        let num_limbs = Self::num_limbs(self.prime(), self.len());

        if cfg!(target_endian = "little") {
            let num_bytes = num_limbs * size_of::<Limb>();
            unsafe {
                let buf: &[u8] =
                    std::slice::from_raw_parts_mut(self.limbs().as_ptr() as *mut u8, num_bytes);
                buffer.write_all(buf)?;
            }
        } else {
            for limb in &self.limbs()[0..num_limbs] {
                let bytes = limb.to_le_bytes();
                buffer.write_all(&bytes)?;
            }
        }
        Ok(())
    }
}

impl<'a> Slice<'a> {
    dispatch_vector! {
        pub fn prime(&self) -> ValidPrime;
        pub fn len(&self) -> usize;
        pub fn is_empty(&self) -> bool;
        pub fn @entry(&self, index: usize) -> u32;
        pub fn iter(self) -> (dispatch FpVectorIterator 'a);
        pub fn iter_nonzero(self) -> (dispatch FpVectorNonZeroIterator 'a);
        pub fn is_zero(&self) -> bool;
        pub fn slice(self, start: usize, end: usize) -> (dispatch Slice 'a);
        pub fn to_owned(self) -> (dispatch FpVector);
    }
}

impl<'a> SliceMut<'a> {
    dispatch_vector! {
        pub fn prime(&self) -> ValidPrime;
        pub fn @scale(&mut self, c: u32);
        pub fn set_to_zero(&mut self);
        pub fn @add(&mut self, other: Slice, c: u32);
        pub fn assign(&mut self, other: Slice);
        pub fn @set_entry(&mut self, index: usize, value: u32);
        pub fn as_slice(&self) -> (dispatch Slice);
        pub fn slice_mut(&mut self, start: usize, end: usize) -> (dispatch SliceMut);
        pub fn @add_basis_element(&mut self, index: usize, value: u32);
        pub fn copy(&mut self) -> (dispatch SliceMut);
        pub fn @add_masked(&mut self, other: Slice, c: u32, mask: &[usize]);
        pub fn @add_unmasked(&mut self, other: Slice, c: u32, mask: &[usize]);
        pub fn @add_tensor(&mut self, offset: usize, coeff: u32, @left: Slice, right: Slice);
    }
}

impl<'a> FpVectorIterator<'a> {
    dispatch_vector! {
        pub fn skip_n(&mut self, n: usize);
    }
}

impl std::fmt::Display for FpVector {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl<'a> std::fmt::Display for Slice<'a> {
    /// # Example
    /// ```
    /// # use fp::vector::FpVector;
    /// # use fp::prime::ValidPrime;
    /// let v = FpVector::from_slice(ValidPrime::new(2), &[0, 1, 0]);
    /// assert_eq!(&format!("{v}"), "[0, 1, 0]");
    /// assert_eq!(&format!("{v:#}"), "010");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            for v in self.iter() {
                // If self.p >= 11, this will look funky
                write!(f, "{v}")?;
            }
            Ok(())
        } else {
            write!(f, "[{}]", self.iter().format(", "))
        }
    }
}

impl From<&FpVector> for Vec<u32> {
    fn from(v: &FpVector) -> Self {
        v.iter().collect()
    }
}

impl std::ops::AddAssign<&Self> for FpVector {
    fn add_assign(&mut self, other: &Self) {
        self.add(other, 1);
    }
}

impl<'a> Iterator for FpVectorIterator<'a> {
    type Item = u32;

    dispatch_vector! {
        fn @next(&mut self) -> (Option<u32>);
    }
}

impl<'a> Iterator for FpVectorNonZeroIterator<'a> {
    type Item = (usize, u32);

    dispatch_vector! {
        fn @next(&mut self) -> (Option<(usize, u32)>);
    }
}

impl<'a> IntoIterator for &'a FpVector {
    type IntoIter = FpVectorIterator<'a>;
    type Item = u32;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl_try_into!();

impl Serialize for FpVector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Vec::<u32>::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FpVector {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        panic!("Deserializing FpVector not supported");
        // This is needed for ext-websocket/actions to be happy
    }
}

impl<'a, 'b> From<&'a mut SliceMut<'b>> for SliceMut<'a> {
    fn from(slice: &'a mut SliceMut<'b>) -> Self {
        slice.copy()
    }
}

impl<'a, 'b> From<&'a Slice<'b>> for Slice<'a> {
    fn from(slice: &'a Slice<'b>) -> Self {
        *slice
    }
}

impl<'a, 'b> From<&'a SliceMut<'b>> for Slice<'a> {
    fn from(slice: &'a SliceMut<'b>) -> Self {
        slice.as_slice()
    }
}

impl<'a> From<&'a FpVector> for Slice<'a> {
    fn from(v: &'a FpVector) -> Self {
        v.as_slice()
    }
}

impl<'a> From<&'a mut FpVector> for SliceMut<'a> {
    fn from(v: &'a mut FpVector) -> Self {
        v.as_slice_mut()
    }
}
