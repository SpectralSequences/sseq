use itertools::Itertools;

use crate::{
    limb::{self, Limb},
    prime::ValidPrime,
    simd,
};

use super::{
    inner::{FpVectorP, SliceMutP, SliceP},
    iter::{FpVectorIterator, FpVectorNonZeroIteratorP},
};

impl<const P: u32> FpVectorP<P> {
    pub fn new_(len: usize) -> Self {
        let number_of_limbs = limb::number::<P>(len);
        Self {
            len,
            limbs: vec![0; number_of_limbs],
        }
    }

    pub fn from_raw_parts(len: usize, limbs: Vec<Limb>) -> Self {
        debug_assert_eq!(limbs.len(), limb::number::<P>(len));
        Self { len, limbs }
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

    #[must_use]
    pub fn slice(&self, start: usize, end: usize) -> SliceP<'_, P> {
        assert!(start <= end && end <= self.len);
        SliceP {
            limbs: &self.limbs,
            start,
            end,
        }
    }

    #[must_use]
    pub fn slice_mut(&mut self, start: usize, end: usize) -> SliceMutP<'_, P> {
        assert!(start <= end && end <= self.len);
        SliceMutP {
            limbs: &mut self.limbs,
            start,
            end,
        }
    }

    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> SliceP<'_, P> {
        self.into()
    }

    #[inline]
    #[must_use]
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
        let min_limb = offset / limb::entries_per_limb_const::<P>();
        if P == 2 {
            if c != 0 {
                simd::add_simd(&mut self.limbs, &other.limbs, min_limb);
            }
        } else {
            for (left, right) in self.limbs.iter_mut().zip_eq(&other.limbs).skip(min_limb) {
                *left = limb::add::<P>(*left, *right, c);
            }
            for limb in &mut self.limbs[min_limb..] {
                *limb = limb::reduce::<P>(*limb);
            }
        }
    }

    /// Add `other` to `self` on the assumption that the first `offset` entries of `other` are
    /// empty.
    pub fn add_offset_nosimd(&mut self, other: &FpVectorP<P>, c: u32, offset: usize) {
        assert_eq!(self.len(), other.len());
        let min_limb = offset / limb::entries_per_limb_const::<P>();
        if P == 2 {
            if c != 0 {
                for i in 0..self.limbs.len() {
                    self.limbs[i] ^= other.limbs[i];
                }
            }
        } else {
            for (left, right) in self.limbs.iter_mut().zip_eq(&other.limbs).skip(min_limb) {
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

    pub fn add_nosimd(&mut self, other: &FpVectorP<P>, c: u32) {
        self.add_offset_nosimd(other, c, 0);
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
                .chunks(limb::entries_per_limb_const::<P>())
                .map(|x| limb::pack::<_, P>(x.iter().copied())),
        );
    }

    /// Permanently remove the first `n` elements in the vector. `n` must be a multiple of
    /// the number of entries per limb
    pub(crate) fn trim_start(&mut self, n: usize) {
        assert!(n <= self.len);
        let entries_per = limb::entries_per_limb_const::<P>();
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
        for (left, right) in self.limbs.iter_mut().zip_eq(&other.limbs) {
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
        let entries_per_limb = limb::entries_per_limb_const::<P>();
        let bit_length = limb::bit_length_const::<P>();
        let bitmask = limb::bitmask::<P>();
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

    pub fn density(&self) -> f32 {
        let num_nonzero = if P == 2 {
            self.limbs
                .iter()
                .copied()
                .map(Limb::count_ones)
                .sum::<u32>() as usize
        } else {
            self.iter_nonzero().count()
        };
        num_nonzero as f32 / self.len() as f32
    }
}

impl<T: AsRef<[u32]>, const P: u32> From<&T> for FpVectorP<P> {
    fn from(slice: &T) -> Self {
        let mut v = Self::new_(slice.as_ref().len());
        v.limbs.clear();
        v.limbs.extend(
            slice
                .as_ref()
                .chunks(limb::entries_per_limb_const::<P>())
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
