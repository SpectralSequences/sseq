use super::{
    inner::{FpVectorP, SliceP},
    iter::{FpVectorIterator, FpVectorNonZeroIteratorP},
};
use crate::{
    constants,
    limb::{self, Limb},
    prime::{Prime, ValidPrime},
};

// Public methods

impl<'a, P: Prime> SliceP<'a, P> {
    pub fn prime(&self) -> ValidPrime {
        self.p.to_dyn()
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn entry(&self, index: usize) -> u32 {
        debug_assert!(
            index < self.len(),
            "Index {} too large, length of vector is only {}.",
            index,
            self.len()
        );
        let bit_mask = limb::bitmask(self.p);
        let limb_index = limb::limb_bit_index_pair(self.p, index + self.start);
        let mut result = self.limbs[limb_index.limb];
        result >>= limb_index.bit_index;
        result &= bit_mask;
        result as u32
    }

    /// TODO: implement prime 2 version
    pub fn iter(self) -> FpVectorIterator<'a> {
        FpVectorIterator::new(self)
    }

    pub fn iter_nonzero(self) -> FpVectorNonZeroIteratorP<'a, P> {
        FpVectorNonZeroIteratorP::new(self)
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

        Self {
            p: self.p,
            limbs: self.limbs,
            start: self.start + start,
            end: self.start + end,
        }
    }

    /// Converts a slice to an owned FpVectorP. This is vastly more efficient if the start of the vector is aligned.
    #[must_use]
    pub fn to_owned(self) -> FpVectorP<P> {
        let mut new = FpVectorP::new(self.p, self.len());
        if self.start % limb::entries_per_limb(self.p) == 0 {
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
impl<'a, P: Prime> SliceP<'a, P> {
    #[inline]
    pub(super) fn offset(&self) -> usize {
        let bit_length = limb::bit_length(self.p);
        let entries_per_limb = limb::entries_per_limb(self.p);
        (self.start % entries_per_limb) * bit_length
    }

    #[inline]
    pub(super) fn limb_range(&self) -> std::ops::Range<usize> {
        limb::range(self.p, self.start, self.end)
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
        let num_entries = 1 + (self.end - 1) % limb::entries_per_limb(self.p);
        let bit_max = num_entries * limb::bit_length(self.p);

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

impl<'a, P: Prime> From<&'a FpVectorP<P>> for SliceP<'a, P> {
    fn from(v: &'a FpVectorP<P>) -> Self {
        v.slice(0, v.len)
    }
}
