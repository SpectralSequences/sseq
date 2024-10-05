use itertools::Itertools;

use super::{
    inner::{FqSlice, FqVector},
    iter::{FqVectorIterator, FqVectorNonZeroIterator},
};
use crate::{
    constants,
    field::{element::FieldElement, Field},
    limb::Limb,
    prime::{Prime, ValidPrime},
};

// Public methods

impl<'a, F: Field> FqSlice<'a, F> {
    pub fn prime(&self) -> ValidPrime {
        self.fq.characteristic().to_dyn()
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn entry(&self, index: usize) -> FieldElement<F> {
        debug_assert!(
            index < self.len(),
            "Index {} too large, length of vector is only {}.",
            index,
            self.len()
        );
        let bit_mask = self.fq.bitmask();
        let limb_index = self.fq.limb_bit_index_pair(index + self.start);
        let mut result = self.limbs[limb_index.limb];
        result >>= limb_index.bit_index;
        result &= bit_mask;
        self.fq.decode(result)
    }

    /// TODO: implement prime 2 version
    pub fn iter(self) -> FqVectorIterator<'a, F> {
        FqVectorIterator::new(self)
    }

    pub fn iter_nonzero(self) -> FqVectorNonZeroIterator<'a, F> {
        FqVectorNonZeroIterator::new(self)
    }

    pub fn is_zero(&self) -> bool {
        let limb_range = self.limb_range();
        if limb_range.is_empty() {
            return true;
        }
        let (min_mask, max_mask) = self.limb_masks();
        if self.limbs[limb_range.start] & min_mask != 0 {
            return false;
        }

        let inner_range = self.limb_range_inner();
        if !inner_range.is_empty() && self.limbs[inner_range].iter().any(|&x| x != 0) {
            return false;
        }
        if self.limbs[limb_range.end - 1] & max_mask != 0 {
            return false;
        }
        true
    }

    #[must_use]
    pub fn slice(self, start: usize, end: usize) -> Self {
        assert!(start <= end && end <= self.len());

        FqSlice {
            fq: self.fq,
            limbs: self.limbs,
            start: self.start + start,
            end: self.start + end,
        }
    }

    /// Converts a slice to an owned FqVector. This is vastly more efficient if the start of the vector is aligned.
    #[must_use]
    pub fn to_owned(self) -> FqVector<F> {
        let mut new = FqVector::new(self.fq, self.len());
        if self.start % self.fq.entries_per_limb() == 0 {
            let limb_range = self.limb_range();
            new.limbs[0..limb_range.len()].copy_from_slice(&self.limbs[limb_range]);
            if !new.limbs.is_empty() {
                let len = new.limbs.len();
                new.limbs[len - 1] &= self.limb_masks().1;
            }
        } else {
            new.as_slice_mut().assign(self);
        }
        new
    }
}

// Limb methods
impl<F: Field> FqSlice<'_, F> {
    #[inline]
    pub(super) fn offset(&self) -> usize {
        let bit_length = self.fq.bit_length();
        let entries_per_limb = self.fq.entries_per_limb();
        (self.start % entries_per_limb) * bit_length
    }

    #[inline]
    pub(super) fn limb_range(&self) -> std::ops::Range<usize> {
        self.fq.range(self.start, self.end)
    }

    /// This function underflows if `self.end == 0`, which happens if and only if we are taking a
    /// slice of width 0 at the start of an `FpVector`. This should be a very rare edge case.
    /// Dealing with the underflow properly would probably require using `saturating_sub` or
    /// something of that nature, and that has a nontrivial (10%) performance hit.
    #[inline]
    pub(super) fn limb_range_inner(&self) -> std::ops::Range<usize> {
        let range = self.limb_range();
        (range.start + 1)..(usize::max(range.start + 1, range.end - 1))
    }

    #[inline(always)]
    pub(super) fn min_limb_mask(&self) -> Limb {
        !0 << self.offset()
    }

    #[inline(always)]
    pub(super) fn max_limb_mask(&self) -> Limb {
        let num_entries = 1 + (self.end - 1) % self.fq.entries_per_limb();
        let bit_max = num_entries * self.fq.bit_length();

        (!0) >> (constants::BITS_PER_LIMB - bit_max)
    }

    #[inline(always)]
    pub(super) fn limb_masks(&self) -> (Limb, Limb) {
        if self.limb_range().len() == 1 {
            (
                self.min_limb_mask() & self.max_limb_mask(),
                self.min_limb_mask() & self.max_limb_mask(),
            )
        } else {
            (self.min_limb_mask(), self.max_limb_mask())
        }
    }
}

impl<'a, F: Field> From<&'a FqVector<F>> for FqSlice<'a, F> {
    fn from(v: &'a FqVector<F>) -> Self {
        v.slice(0, v.len)
    }
}

impl<F: Field> std::fmt::Display for FqSlice<'_, F> {
    /// # Example
    /// ```
    /// # use fp::field::{Field, SmallFq};
    /// # use fp::prime::{P2, ValidPrime};
    /// # use fp::vector::FqVector;
    /// let fq = SmallFq::new(P2, 3);
    /// let v = FqVector::from_slice(fq, &[fq.zero(), fq.one(), fq.a(), fq.a() * fq.a()]);
    /// assert_eq!(&format!("{v}"), "[0, 1, a, a^2]");
    ///
    /// // This only looks reasonable over prime fields of order less than 10
    /// assert_eq!(&format!("{v:#}"), "01aa^2");
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
