use super::Subspace;
use crate::vector::{FpVector, Slice};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffineSubspace {
    offset: FpVector,
    linear_part: Subspace,
}

impl AffineSubspace {
    pub fn new(mut offset: FpVector, linear_part: Subspace) -> Self {
        assert_eq!(offset.len(), linear_part.dimension());
        linear_part.reduce(offset.as_slice_mut());
        Self {
            offset,
            linear_part,
        }
    }

    pub fn offset(&self) -> &FpVector {
        &self.offset
    }

    pub fn linear_part(&self) -> &Subspace {
        &self.linear_part
    }

    pub fn sum(&self, other: &Self) -> Self {
        let linear_part = self.linear_part.sum(&other.linear_part);

        let mut offset = self.offset.clone();
        offset.add(&other.offset, 1);

        Self::new(offset, linear_part)
    }

    pub fn contains(&self, vector: Slice) -> bool {
        let mut vector = vector.to_owned();
        vector.add(&self.offset, vector.prime() - 1);
        self.linear_part.contains(vector.as_slice())
    }

    pub fn contains_space(&self, other: &Self) -> bool {
        self.linear_part.contains_space(&other.linear_part)
            && self.contains(other.offset.as_slice())
    }
}

impl From<Subspace> for AffineSubspace {
    fn from(subspace: Subspace) -> Self {
        Self::new(
            FpVector::new(subspace.prime(), subspace.ambient_dimension()),
            subspace,
        )
    }
}

impl std::fmt::Display for AffineSubspace {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} + {{{:#}}}", self.offset, self.linear_part)
    }
}
