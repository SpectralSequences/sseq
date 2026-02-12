use super::{FqSlice, FqVector, FqVectorBase, Repr};
use crate::field::{Field, element::FieldElement};

// Public methods

impl<'a, F: Field> FqSlice<'a, F> {
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

impl<'a, const A: bool, R: Repr, F: Field> From<&'a FqVectorBase<A, R, F>> for FqSlice<'a, F> {
    fn from(v: &'a FqVectorBase<A, R, F>) -> Self {
        v.as_slice()
    }
}
