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

impl<'a> FpSliceMut<'a> {
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

impl<'a> std::fmt::Display for FpSlice<'a> {
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
    use itertools::Itertools;
    use proptest::prelude::*;
    use rand::Rng;
    use rstest::rstest;

    use crate::{
        prime::{Prime, ValidPrime},
        vector::{tests::MAX_TEST_VEC_LEN, FpVector},
    };

    pub struct VectorDiffEntry {
        pub index: usize,
        pub left: u32,
        pub right: u32,
    }

    impl FpVector {
        pub fn diff_list(&self, other: &[u32]) -> Vec<VectorDiffEntry> {
            assert!(self.len() == other.len());
            let mut result = Vec::new();
            #[allow(clippy::needless_range_loop)]
            for index in 0..self.len() {
                let left = self.entry(index);
                let right = other[index].clone();
                if left != right {
                    result.push(VectorDiffEntry { index, left, right });
                }
            }
            result
        }

        pub fn diff_vec(&self, other: &Self) -> Vec<VectorDiffEntry> {
            assert!(self.len() == other.len());
            let mut result = Vec::new();
            for index in 0..self.len() {
                let left = self.entry(index);
                let right = other.entry(index);
                if left != right {
                    result.push(VectorDiffEntry { index, left, right });
                }
            }
            result
        }

        pub fn format_diff(diff: Vec<VectorDiffEntry>) -> String {
            let data_formatter =
                diff.iter()
                    .format_with("\n ", |VectorDiffEntry { index, left, right }, f| {
                        f(&format_args!("  At index {index}: {left}!={right}"))
                    });
            format!("{data_formatter}")
        }

        pub fn assert_list_eq(&self, other: &[u32]) {
            let diff = self.diff_list(other);
            if diff.is_empty() {
                return;
            }
            panic!(
                "assert {} == {:?}\n{}",
                self,
                other,
                Self::format_diff(diff)
            );
        }

        pub fn assert_vec_eq(&self, other: &Self) {
            let diff = self.diff_vec(other);
            if diff.is_empty() {
                return;
            }
            panic!(
                "assert {} == {:?}\n{}",
                self,
                other,
                Self::format_diff(diff)
            );
        }
    }

    fn random_vector(p: u32, dimension: usize) -> Vec<u32> {
        let mut rng = rand::thread_rng();
        (0..dimension).map(|_| rng.gen_range(0..p)).collect()
    }

    fn arb_vec_u32() -> impl Strategy<Value = (ValidPrime, Vec<u32>)> {
        any::<ValidPrime>().prop_flat_map(|p| {
            (
                Just(p),
                proptest::collection::vec(0..p.as_u32(), 1..=MAX_TEST_VEC_LEN),
            )
        })
    }

    proptest! {
        #[test]
        fn test_serialize((p, v_arr) in arb_vec_u32()) {
            use std::io::{Seek, Cursor};

            let v = FpVector::from_slice(p, &v_arr);

            let mut cursor = Cursor::new(Vec::<u8>::new());
            v.to_bytes(&mut cursor).unwrap();
            cursor.rewind().unwrap();

            let w = FpVector::from_bytes(v.prime(), v.len(), &mut cursor).unwrap();
            v.assert_vec_eq(&w);
        }
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
