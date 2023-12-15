use itertools::Itertools;

use super::{
    inner::{FqVectorP, SliceMutP, SliceP},
    iter::{FpVectorIteratorP, FpVectorNonZeroIteratorP},
};
use crate::{
    field::{Field, FieldElement},
    limb::{self, Limb},
    prime::{Prime, ValidPrime},
    simd,
};

impl<F: Field> FqVectorP<F> {
    pub fn new(fq: impl Into<F>, len: usize) -> Self {
        let fq = fq.into();
        let number_of_limbs = fq.number(len);
        Self {
            fq,
            len,
            limbs: vec![0; number_of_limbs],
        }
    }

    pub fn from_raw_parts(fq: F, len: usize, limbs: Vec<Limb>) -> Self {
        debug_assert_eq!(limbs.len(), fq.number(len));
        Self { fq, len, limbs }
    }

    pub fn new_with_capacity(fq: impl Into<F>, len: usize, capacity: usize) -> Self {
        let fq = fq.into();
        let mut limbs = Vec::with_capacity(fq.number(capacity));
        limbs.resize(fq.number(len), 0);
        Self { fq, len, limbs }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn prime(&self) -> ValidPrime {
        self.fq.characteristic().to_dyn()
    }

    #[must_use]
    pub fn slice(&self, start: usize, end: usize) -> SliceP<'_, F> {
        assert!(start <= end && end <= self.len);
        SliceP {
            fq: self.fq,
            limbs: &self.limbs,
            start,
            end,
        }
    }

    #[must_use]
    pub fn slice_mut(&mut self, start: usize, end: usize) -> SliceMutP<'_, F> {
        assert!(start <= end && end <= self.len);
        SliceMutP {
            fq: self.fq,
            limbs: &mut self.limbs,
            start,
            end,
        }
    }

    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> SliceP<'_, F> {
        self.into()
    }

    #[inline]
    #[must_use]
    pub fn as_slice_mut(&mut self) -> SliceMutP<'_, F> {
        self.into()
    }

    pub fn add_basis_element(&mut self, index: usize, value: F::Element) {
        self.as_slice_mut().add_basis_element(index, value);
    }

    pub fn entry(&self, index: usize) -> F::Element {
        self.as_slice().entry(index)
    }

    pub fn set_entry(&mut self, index: usize, value: F::Element) {
        self.as_slice_mut().set_entry(index, value);
    }

    pub fn iter(&self) -> FpVectorIteratorP<'_, F> {
        self.as_slice().iter()
    }

    pub fn iter_nonzero(&self) -> FpVectorNonZeroIteratorP<'_, F> {
        self.as_slice().iter_nonzero()
    }

    pub fn set_to_zero(&mut self) {
        // This is sound because `fq.encode(fq.zero())` is always zero.
        for limb in &mut self.limbs {
            *limb = 0;
        }
    }

    pub fn scale(&mut self, c: F::Element) {
        if self.fq.characteristic() == 2 {
            if c.is_zero() {
                self.set_to_zero();
            }
        } else {
            for limb in &mut self.limbs {
                *limb = self.fq.reduce(self.fq.fma_limb(0, *limb, c.clone()));
            }
        }
    }

    /// Add `other` to `self` on the assumption that the first `offset` entries of `other` are
    /// empty.
    pub fn add_offset(&mut self, other: &Self, c: F::Element, offset: usize) {
        assert_eq!(self.len(), other.len());
        let fq = self.fq;
        let min_limb = offset / fq.entries_per_limb();
        if fq.characteristic() == 2 && fq.degree() == 1 {
            if !c.is_zero() {
                simd::add_simd(&mut self.limbs, &other.limbs, min_limb);
            }
        } else {
            for (left, right) in self.limbs.iter_mut().zip_eq(&other.limbs).skip(min_limb) {
                *left = fq.fma_limb(*left, *right, c.clone());
            }
            for limb in &mut self.limbs[min_limb..] {
                *limb = fq.reduce(*limb);
            }
        }
    }

    pub fn add(&mut self, other: &Self, c: F::Element) {
        self.add_offset(other, c, 0);
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
        self.limbs.resize(self.fq.number(len), 0);
    }

    /// This clears the vector and sets the length to `len`. This is useful for reusing
    /// allocations of temporary vectors.
    pub fn set_scratch_vector_size(&mut self, len: usize) {
        self.limbs.clear();
        self.limbs.resize(self.fq.number(len), 0);
        self.len = len;
    }

    /// This replaces the contents of the vector with the contents of the slice. The two must have
    /// the same length.
    pub fn copy_from_slice(&mut self, slice: &[F::Element]) {
        assert_eq!(self.len, slice.len());

        self.limbs.clear();
        self.limbs.extend(
            slice
                .chunks(self.fq.entries_per_limb())
                .map(|x| self.fq.pack(x.iter().cloned())),
        );
    }

    /// Permanently remove the first `n` elements in the vector. `n` must be a multiple of
    /// the number of entries per limb
    pub(crate) fn trim_start(&mut self, n: usize) {
        assert!(n <= self.len);
        let entries_per = self.fq.entries_per_limb();
        assert_eq!(n % entries_per, 0);
        let num_limbs = n / entries_per;
        self.limbs.drain(0..num_limbs);
        self.len -= n;
    }

    pub fn sign_rule(&self, other: &Self) -> bool {
        assert_eq!(self.fq.characteristic(), 2);
        assert_eq!(self.fq.degree(), 1);

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

    pub fn add_truncate(&mut self, other: &Self, c: F::Element) -> Option<()> {
        for (left, right) in self.limbs.iter_mut().zip_eq(&other.limbs) {
            *left = self.fq.fma_limb(*left, *right, c.clone());
            *left = self.fq.truncate(*left)?;
        }
        Some(())
    }

    fn add_carry_limb<T>(&mut self, idx: usize, source: Limb, c: F::Element, rest: &mut [T]) -> bool
    where
        for<'a> &'a mut T: TryInto<&'a mut Self>,
    {
        if self.fq.characteristic() == 2 && self.fq.degree() == 1 {
            let c = self.fq.encode(c);
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

    pub fn add_carry<T>(&mut self, other: &Self, c: F::Element, rest: &mut [T]) -> bool
    where
        for<'a> &'a mut T: TryInto<&'a mut Self>,
    {
        let mut result = false;
        for i in 0..self.limbs.len() {
            result |= self.add_carry_limb(i, other.limbs[i], c.clone(), rest);
        }
        result
    }

    /// Find the index and value of the first non-zero entry of the vector. `None` if the vector is zero.
    pub fn first_nonzero(&self) -> Option<(usize, F::Element)> {
        let entries_per_limb = self.fq.entries_per_limb();
        let bit_length = self.fq.bit_length();
        let bitmask = self.fq.bitmask();
        for (i, &limb) in self.limbs.iter().enumerate() {
            if limb == 0 {
                continue;
            }
            let index = limb.trailing_zeros() as usize / bit_length;
            return Some((
                i * entries_per_limb + index,
                self.fq.decode((limb >> (index * bit_length)) & bitmask),
            ));
        }
        None
    }

    pub fn density(&self) -> f32 {
        let num_nonzero = if self.fq.characteristic() == 2 {
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

impl<T: AsRef<[F::Element]>, F: Field> From<(F, T)> for FqVectorP<F> {
    fn from(data: (F, T)) -> Self {
        let (fq, slice) = data;
        let mut v = Self::new(fq, slice.as_ref().len());
        v.limbs.clear();
        v.limbs.extend(
            slice
                .as_ref()
                .chunks(fq.entries_per_limb())
                .map(|x| fq.pack(x.iter().cloned())),
        );
        v
    }
}

impl<F: Field> From<&FqVectorP<F>> for Vec<F::Element> {
    fn from(vec: &FqVectorP<F>) -> Vec<F::Element> {
        vec.iter().collect()
    }
}
