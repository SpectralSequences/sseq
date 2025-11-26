use itertools::Itertools;

use super::{
    inner::{FqSlice, FqVector},
    iter::{FqVectorIterator, FqVectorNonZeroIterator},
};
use crate::field::{Field, element::FieldElement};

// Public methods

impl<'a, F: Field> FqSlice<'a, F> {
    /// TODO: implement prime 2 version
    pub fn iter(self) -> FqVectorIterator<'a, F> {
        FqVectorIterator::new(self)
    }

    pub fn iter_nonzero(self) -> FqVectorNonZeroIterator<'a, F> {
        FqVectorNonZeroIterator::new(self)
    }

    pub fn first_nonzero(&self) -> Option<(usize, FieldElement<F>)> {
        self.iter_nonzero().next()
    }

    #[must_use]
    pub fn restrict(self, start: usize, end: usize) -> Self {
        assert!(start <= end && end <= self.len());

        FqSlice::_new(
            self.fq(),
            self.into_limbs(),
            self.start() + start,
            self.start() + end,
        )
    }

    /// Converts a slice to an owned FqVector. This is vastly more efficient if the start of the vector is aligned.
    #[must_use]
    pub fn to_owned(self) -> FqVector<F> {
        let mut new = FqVector::new(self.fq(), self.len());
        if self.start().is_multiple_of(self.fq().entries_per_limb()) {
            let limb_range = self.limb_range();
            new.limbs_mut()[0..limb_range.len()].copy_from_slice(&self.limbs()[limb_range]);
            if !new.limbs().is_empty() {
                let len = new.limbs().len();
                new.limbs_mut()[len - 1] &= self.limb_masks().1;
            }
        } else {
            new.as_slice_mut().assign(self);
        }
        new
    }
}

impl<'a, F: Field> From<&'a FqVector<F>> for FqSlice<'a, F> {
    fn from(v: &'a FqVector<F>) -> Self {
        v.slice(0, v.len())
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
