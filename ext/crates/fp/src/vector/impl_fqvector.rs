use std::io;

use itertools::Itertools;

use super::{
    inner::{FqSlice, FqSliceMut, FqVector},
    iter::{FqVectorIterator, FqVectorNonZeroIterator},
};
use crate::{
    field::{Field, element::FieldElement},
    limb::Limb,
    prime::{Prime, ValidPrime},
};

impl<F: Field> FqVector<F> {
    pub fn new(fq: F, len: usize) -> Self {
        let number_of_limbs = fq.number(len);

        Self::from_raw_parts(fq, len, vec![0; number_of_limbs])
    }

    pub fn new_with_capacity(fq: F, len: usize, capacity: usize) -> Self {
        let mut limbs = Vec::with_capacity(fq.number(capacity));
        limbs.resize(fq.number(len), 0);

        Self::from_raw_parts(fq, len, limbs)
    }

    pub fn from_slice(fq: F, slice: &[FieldElement<F>]) -> Self {
        assert!(slice.iter().all(|x| x.field() == fq));
        let len = slice.len();
        let mut v = Self::new(fq, len);
        v.copy_from_slice(slice);
        v
    }

    pub fn from_bytes(fq: F, len: usize, data: &mut impl io::Read) -> io::Result<Self> {
        let mut v = Self::new(fq, len);
        v.update_from_bytes(data)?;
        Ok(v)
    }

    pub fn update_from_bytes(&mut self, data: &mut impl io::Read) -> io::Result<()> {
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

    pub fn to_bytes(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        // self.limbs is allowed to have more limbs than necessary, but we only save the
        // necessary ones.
        let num_limbs = self.fq().number(self.len());

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

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn prime(&self) -> ValidPrime {
        self.fq().characteristic().to_dyn()
    }

    #[must_use]
    pub fn slice(&self, start: usize, end: usize) -> FqSlice<'_, F> {
        assert!(start <= end && end <= self.len());
        FqSlice::new(self.fq(), self.limbs(), start, end)
    }

    #[must_use]
    pub fn slice_mut(&mut self, start: usize, end: usize) -> FqSliceMut<'_, F> {
        assert!(start <= end && end <= self.len());
        FqSliceMut::new(self.fq(), self.limbs_mut(), start, end)
    }

    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> FqSlice<'_, F> {
        self.into()
    }

    #[inline]
    #[must_use]
    pub fn as_slice_mut(&mut self) -> FqSliceMut<'_, F> {
        self.into()
    }

    pub fn add_basis_element(&mut self, index: usize, value: FieldElement<F>) {
        assert_eq!(self.fq(), value.field());
        self.as_slice_mut().add_basis_element(index, value);
    }

    pub fn entry(&self, index: usize) -> FieldElement<F> {
        self.as_slice().entry(index)
    }

    pub fn set_entry(&mut self, index: usize, value: FieldElement<F>) {
        assert_eq!(self.fq(), value.field());
        self.as_slice_mut().set_entry(index, value);
    }

    pub fn iter(&self) -> FqVectorIterator<'_, F> {
        self.as_slice().iter()
    }

    pub fn iter_nonzero(&self) -> FqVectorNonZeroIterator<'_, F> {
        self.as_slice().iter_nonzero()
    }

    pub fn set_to_zero(&mut self) {
        // This is sound because `fq.encode(fq.zero())` is always zero.
        for limb in self.limbs_mut() {
            *limb = 0;
        }
    }

    pub fn scale(&mut self, c: FieldElement<F>) {
        assert_eq!(self.fq(), c.field());
        let fq = self.fq();

        if c == fq.zero() {
            self.set_to_zero();
        }
        if fq.q() != 2 {
            for limb in self.limbs_mut() {
                *limb = fq.reduce(fq.fma_limb(0, *limb, c.clone()));
            }
        }
    }

    /// Add `other` to `self` on the assumption that the first `offset` entries of `other` are
    /// empty.
    pub fn add_offset(&mut self, other: &Self, c: FieldElement<F>, offset: usize) {
        assert_eq!(self.fq(), c.field());
        assert_eq!(self.fq(), other.fq());
        assert_eq!(self.len(), other.len());

        let fq = self.fq();
        let min_limb = offset / fq.entries_per_limb();

        if fq.q() == 2 {
            if c != fq.zero() {
                crate::simd::add_simd(self.limbs_mut(), other.limbs(), min_limb);
            }
        } else {
            for (left, right) in self
                .limbs_mut()
                .iter_mut()
                .zip_eq(other.limbs())
                .skip(min_limb)
            {
                *left = fq.fma_limb(*left, *right, c.clone());
            }
            for limb in self.limbs_mut()[min_limb..].iter_mut() {
                *limb = fq.reduce(*limb);
            }
        }
    }

    pub fn add(&mut self, other: &Self, c: FieldElement<F>) {
        self.add_offset(other, c, 0);
    }

    pub fn assign(&mut self, other: &Self) {
        assert_eq!(self.fq(), other.fq());
        assert_eq!(self.len(), other.len());
        self.limbs_mut().copy_from_slice(other.limbs())
    }

    /// A version of [`FqVector::assign`] that allows `other` to be shorter than `self`.
    pub fn assign_partial(&mut self, other: &Self) {
        assert_eq!(self.fq(), other.fq());
        assert!(other.len() <= self.len());

        self.limbs_mut()[0..other.limbs().len()].copy_from_slice(other.limbs());
        for limb in self.limbs_mut()[other.limbs().len()..].iter_mut() {
            *limb = 0;
        }
    }

    pub fn is_zero(&self) -> bool {
        self.limbs().iter().all(|&x| x == 0)
    }

    /// This function ensures the length of the vector is at least `len`. See also
    /// `set_scratch_vector_size`.
    pub fn extend_len(&mut self, len: usize) {
        if self.len() >= len {
            return;
        }
        *self.len_mut() = len;
        let new_len = self.fq().number(len);
        self.vec_mut().resize(new_len, 0);
    }

    /// This clears the vector and sets the length to `len`. This is useful for reusing
    /// allocations of temporary vectors.
    pub fn set_scratch_vector_size(&mut self, len: usize) {
        self.vec_mut().clear();
        let new_len = self.fq().number(len);
        self.vec_mut().resize(new_len, 0);
        *self.len_mut() = len;
    }

    /// This replaces the contents of the vector with the contents of the slice. The two must have
    /// the same length.
    pub fn copy_from_slice(&mut self, slice: &[FieldElement<F>]) {
        assert!(slice.iter().all(|x| x.field() == self.fq()));
        assert_eq!(self.len(), slice.len());

        let fq = self.fq();
        self.vec_mut().clear();
        self.vec_mut().extend(
            slice
                .chunks(fq.entries_per_limb())
                .map(|x| fq.pack(x.iter().cloned())),
        );
    }

    /// Permanently remove the first `n` elements in the vector. `n` must be a multiple of
    /// the number of entries per limb
    pub(crate) fn trim_start(&mut self, n: usize) {
        assert!(n <= self.len());
        let entries_per = self.fq().entries_per_limb();
        assert_eq!(n % entries_per, 0);
        let num_limbs = n / entries_per;
        self.vec_mut().drain(0..num_limbs);
        *self.len_mut() -= n;
    }

    pub fn sign_rule(&self, other: &Self) -> bool {
        assert_eq!(self.fq(), other.fq());
        assert_eq!(self.fq().q(), 2);

        let mut result = 0;
        for target_limb_idx in 0..self.limbs().len() {
            let target_limb = other.limbs()[target_limb_idx];
            let source_limb = self.limbs()[target_limb_idx];
            result ^= crate::limb::sign_rule(target_limb, source_limb);
            if target_limb.count_ones().is_multiple_of(2) {
                continue;
            }
            for _ in 0..target_limb_idx {
                result ^= source_limb.count_ones() % 2;
            }
        }
        result == 1
    }

    pub fn add_truncate(&mut self, other: &Self, c: FieldElement<F>) -> Option<()> {
        assert_eq!(self.fq(), other.fq());
        let fq = self.fq();
        for (left, right) in self.limbs_mut().iter_mut().zip_eq(other.limbs()) {
            *left = fq.fma_limb(*left, *right, c.clone());
            *left = fq.truncate(*left)?;
        }
        Some(())
    }

    fn add_carry_limb<T>(
        &mut self,
        idx: usize,
        source: Limb,
        c: FieldElement<F>,
        rest: &mut [T],
    ) -> bool
    where
        for<'a> &'a mut T: TryInto<&'a mut Self>,
    {
        assert_eq!(self.fq(), c.field());
        if self.fq().q() == 2 {
            let c = self.fq().encode(c);
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
                let rem = cur_vec.limbs()[idx] ^ carry;
                let quot = cur_vec.limbs()[idx] & carry;
                cur_vec.limbs_mut()[idx] = rem;
                carry = quot;
                cur_vec = carry_vec;
                if quot == 0 {
                    return false;
                }
            }
            cur_vec.limbs_mut()[idx] ^= carry;
            true
        } else {
            unimplemented!()
        }
    }

    pub fn add_carry<T>(&mut self, other: &Self, c: FieldElement<F>, rest: &mut [T]) -> bool
    where
        for<'a> &'a mut T: TryInto<&'a mut Self>,
    {
        assert_eq!(self.fq(), other.fq());
        let mut result = false;
        for i in 0..self.limbs().len() {
            result |= self.add_carry_limb(i, other.limbs()[i], c.clone(), rest);
        }
        result
    }

    /// Find the index and value of the first non-zero entry of the vector. `None` if the vector is zero.
    pub fn first_nonzero(&self) -> Option<(usize, FieldElement<F>)> {
        let entries_per_limb = self.fq().entries_per_limb();
        let bit_length = self.fq().bit_length();
        let bitmask = self.fq().bitmask();
        for (i, &limb) in self.limbs().iter().enumerate() {
            if limb == 0 {
                continue;
            }
            let index = limb.trailing_zeros() as usize / bit_length;
            return Some((
                i * entries_per_limb + index,
                self.fq().decode((limb >> (index * bit_length)) & bitmask),
            ));
        }
        None
    }

    pub fn density(&self) -> f32 {
        let num_nonzero = if self.fq().q() == 2 {
            self.limbs()
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

impl<T: AsRef<[FieldElement<F>]>, F: Field> From<(F, T)> for FqVector<F> {
    fn from(data: (F, T)) -> Self {
        let (fq, slice) = data;
        assert!(slice.as_ref().iter().all(|x| x.field() == fq));
        let mut v = Self::new(fq, slice.as_ref().len());
        v.copy_from_slice(slice.as_ref());
        v
    }
}

impl<F: Field> From<&FqVector<F>> for Vec<FieldElement<F>> {
    fn from(vec: &FqVector<F>) -> Self {
        vec.iter().collect()
    }
}

impl<F: Field> std::fmt::Display for FqVector<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_slice().fmt(f)
    }
}

#[cfg(feature = "proptest")]
pub mod arbitrary {
    use proptest::prelude::*;

    use super::*;

    pub const MAX_LEN: usize = 10_000;

    #[derive(Debug, Clone)]
    pub struct FqVectorArbParams<F> {
        pub fq: Option<F>,
        pub len: BoxedStrategy<usize>,
    }

    impl<F> Default for FqVectorArbParams<F> {
        fn default() -> Self {
            Self {
                fq: None,
                len: (0..=MAX_LEN).boxed(),
            }
        }
    }

    impl<F: Field> Arbitrary for FqVector<F> {
        type Parameters = FqVectorArbParams<F>;
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
            let fq = match args.fq {
                Some(fq) => Just(fq).boxed(),
                None => any::<F>().boxed(),
            };
            (fq, args.len)
                .prop_flat_map(|(fq, len)| {
                    (Just(fq), proptest::collection::vec(fq.arb_element(), len))
                })
                .prop_map(|(fq, v)| Self::from_slice(fq, &v))
                .boxed()
        }
    }
}
