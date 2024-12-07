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

use std::{convert::TryInto, io};

use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::iter::{FqVectorIterator, FqVectorNonZeroIterator};
use crate::{
    field::{field_internal::FieldInternal, Fp},
    limb::Limb,
    prime::Prime,
    vector::inner::{FqSlice, FqSliceMut, FqVector},
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
    pub FpSlice<'a> from FqSlice
}

dispatch_struct! {
    #[derive(Debug)]
    pub FpSliceMut<'a> from FqSliceMut
}

dispatch_struct! {
    pub FpVectorIterator<'a> from FqVectorIterator
}

dispatch_struct! {
    pub FpVectorNonZeroIterator<'a> from FqVectorNonZeroIterator
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
        pub fn slice(&self, start: usize, end: usize) -> (dispatch FpSlice);
        pub fn as_slice(&self) -> (dispatch FpSlice);
        pub fn slice_mut(&mut self, start: usize, end: usize) -> (dispatch FpSliceMut);
        pub fn as_slice_mut(&mut self) -> (dispatch FpSliceMut);
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

        pub fn update_from_bytes(&mut self, data: &mut impl io::Read) -> (io::Result<()>);
        pub fn from_bytes<P: Prime>(p: P, len: usize, data: &mut impl io::Read) -> (from io FqVector);
        pub fn to_bytes(&self, buffer: &mut impl io::Write) -> (io::Result<()>);
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
}

impl<'a> FpSlice<'a> {
    dispatch_vector! {
        pub fn prime(&self) -> ValidPrime;
        pub fn len(&self) -> usize;
        pub fn is_empty(&self) -> bool;
        pub fn @entry(&self, index: usize) -> u32;
        pub fn iter(self) -> (dispatch FpVectorIterator 'a);
        pub fn iter_nonzero(self) -> (dispatch FpVectorNonZeroIterator 'a);
        pub fn is_zero(&self) -> bool;
        pub fn slice(self, start: usize, end: usize) -> (dispatch FpSlice 'a);
        pub fn to_owned(self) -> (dispatch FpVector);
    }
}

impl FpSliceMut<'_> {
    dispatch_vector! {
        pub fn prime(&self) -> ValidPrime;
        pub fn @scale(&mut self, c: u32);
        pub fn set_to_zero(&mut self);
        pub fn @add(&mut self, other: FpSlice, c: u32);
        pub fn assign(&mut self, other: FpSlice);
        pub fn @set_entry(&mut self, index: usize, value: u32);
        pub fn as_slice(&self) -> (dispatch FpSlice);
        pub fn slice_mut(&mut self, start: usize, end: usize) -> (dispatch FpSliceMut);
        pub fn @add_basis_element(&mut self, index: usize, value: u32);
        pub fn copy(&mut self) -> (dispatch FpSliceMut);
        pub fn @add_masked(&mut self, other: FpSlice, c: u32, mask: &[usize]);
        pub fn @add_unmasked(&mut self, other: FpSlice, c: u32, mask: &[usize]);
        pub fn @add_tensor(&mut self, offset: usize, coeff: u32, @left: FpSlice, right: FpSlice);
    }
}

impl FpVectorIterator<'_> {
    dispatch_vector! {
        pub fn skip_n(&mut self, n: usize);
    }
}

impl std::fmt::Display for FpVector {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl std::fmt::Display for FpSlice<'_> {
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

impl Iterator for FpVectorIterator<'_> {
    type Item = u32;

    dispatch_vector! {
        fn @next(&mut self) -> (Option<u32>);
    }
}

impl Iterator for FpVectorNonZeroIterator<'_> {
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

impl_from!();
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

impl<'a, 'b> From<&'a mut FpSliceMut<'b>> for FpSliceMut<'a> {
    fn from(slice: &'a mut FpSliceMut<'b>) -> Self {
        slice.copy()
    }
}

impl<'a, 'b> From<&'a FpSlice<'b>> for FpSlice<'a> {
    fn from(slice: &'a FpSlice<'b>) -> Self {
        *slice
    }
}

impl<'a, 'b> From<&'a FpSliceMut<'b>> for FpSlice<'a> {
    fn from(slice: &'a FpSliceMut<'b>) -> Self {
        slice.as_slice()
    }
}

impl<'a> From<&'a FpVector> for FpSlice<'a> {
    fn from(v: &'a FpVector) -> Self {
        v.as_slice()
    }
}

impl<'a> From<&'a mut FpVector> for FpSliceMut<'a> {
    fn from(v: &'a mut FpVector) -> Self {
        v.as_slice_mut()
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use rstest::rstest;

    use crate::{prime::ValidPrime, vector::FpVector};

    fn random_vector(p: u32, dimension: usize) -> Vec<u32> {
        let mut rng = rand::thread_rng();
        (0..dimension).map(|_| rng.gen_range(0..p)).collect()
    }

    #[rstest]
    #[trace]
    fn test_add_carry(#[values(2)] p: u32, #[values(10, 20, 70, 100, 1000)] dim: usize) {
        use std::fmt::Write;

        let p = ValidPrime::new(p);
        const E_MAX: usize = 4;
        let pto_the_e_max = (p * p * p * p) * p;
        let mut v = Vec::with_capacity(E_MAX + 1);
        let mut w = Vec::with_capacity(E_MAX + 1);
        for _ in 0..=E_MAX {
            v.push(FpVector::new(p, dim));
            w.push(FpVector::new(p, dim));
        }
        let v_arr = random_vector(pto_the_e_max, dim);
        let w_arr = random_vector(pto_the_e_max, dim);
        for i in 0..dim {
            let mut ev = v_arr[i];
            let mut ew = w_arr[i];
            for e in 0..=E_MAX {
                v[e].set_entry(i, ev % p);
                w[e].set_entry(i, ew % p);
                ev /= p;
                ew /= p;
            }
        }

        println!("in  : {v_arr:?}");
        for (e, val) in v.iter().enumerate() {
            println!("in {e}: {val}");
        }
        println!();

        println!("in  : {w_arr:?}");
        for (e, val) in w.iter().enumerate() {
            println!("in {e}: {val}");
        }
        println!();

        for e in 0..=E_MAX {
            let (first, rest) = v[e..].split_at_mut(1);
            first[0].add_carry(&w[e], 1, rest);
        }

        let mut vec_result = vec![0; dim];
        for (i, entry) in vec_result.iter_mut().enumerate() {
            for e in (0..=E_MAX).rev() {
                *entry *= p;
                *entry += v[e].entry(i);
            }
        }

        for (e, val) in v.iter().enumerate() {
            println!("out{e}: {val}");
        }
        println!();

        let mut comparison_result = vec![0; dim];
        for i in 0..dim {
            comparison_result[i] = (v_arr[i] + w_arr[i]) % pto_the_e_max;
        }
        println!("out : {comparison_result:?}");

        let mut diffs = Vec::new();
        let mut diffs_str = String::new();
        for i in 0..dim {
            if vec_result[i] != comparison_result[i] {
                diffs.push((i, comparison_result[i], vec_result[i]));
                let _ = write!(
                    diffs_str,
                    "\nIn position {} expected {} got {}. v[i] = {}, w[i] = {}.",
                    i, comparison_result[i], vec_result[i], v_arr[i], w_arr[i]
                );
            }
        }
        assert!(diffs.is_empty(), "{}", diffs_str);
    }
}
