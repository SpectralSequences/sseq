use super::Subspace;
use crate::{
    prime::Prime,
    vector::{FpSlice, FpVector},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffineSubspace {
    offset: FpVector,
    linear_part: Subspace,
}

/// Why [`AffineSubspace::try_new`] rejected an `(offset, linear_part)` pair.
///
/// [`AffineSubspace::new`] `assert_eq!`s that `offset.len()` matches `linear_part`'s ambient
/// dimension and then reduces `offset` against `linear_part`, which additionally requires the two
/// to share a prime (otherwise the reduction's vector addition panics). The variants below name
/// those two rejection modes for callers (such as the Python bindings) handling untrusted input
/// that must not panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffineSubspaceError {
    /// `offset` and `linear_part` are defined over different primes.
    PrimeMismatch { offset: u32, linear_part: u32 },
    /// `offset.len()` does not match `linear_part`'s ambient dimension.
    LengthMismatch { offset: usize, ambient: usize },
}

impl std::fmt::Display for AffineSubspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PrimeMismatch {
                offset,
                linear_part,
            } => write!(f, "prime mismatch: {offset} != {linear_part}"),
            Self::LengthMismatch { offset, ambient } => {
                write!(f, "length mismatch: {offset} != {ambient}")
            }
        }
    }
}

impl std::error::Error for AffineSubspaceError {}

impl AffineSubspace {
    pub fn new(mut offset: FpVector, linear_part: Subspace) -> Self {
        assert_eq!(offset.len(), linear_part.ambient_dimension());
        linear_part.reduce(offset.as_slice_mut());
        Self {
            offset,
            linear_part,
        }
    }

    /// Construct an affine subspace `offset + linear_part`, validating compatibility.
    ///
    /// Unlike [`Self::new`], which `assert_eq!`s on the ambient dimension (and can panic inside
    /// the offset reduction when the operands disagree on the prime), this checks both conditions
    /// without panicking: `offset` and `linear_part` must share a prime and `offset.len()` must
    /// equal `linear_part`'s ambient dimension, returning the matching [`AffineSubspaceError`]
    /// otherwise. Intended for callers handling untrusted input, such as the Python bindings.
    pub fn try_new(offset: FpVector, linear_part: Subspace) -> Result<Self, AffineSubspaceError> {
        let offset_prime = offset.prime().as_u32();
        let linear_prime = linear_part.prime().as_u32();
        if offset_prime != linear_prime {
            return Err(AffineSubspaceError::PrimeMismatch {
                offset: offset_prime,
                linear_part: linear_prime,
            });
        }
        if offset.len() != linear_part.ambient_dimension() {
            return Err(AffineSubspaceError::LengthMismatch {
                offset: offset.len(),
                ambient: linear_part.ambient_dimension(),
            });
        }
        Ok(Self::new(offset, linear_part))
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

    /// Whether the origin lies in this coset, equivalently whether it is a linear subspace.
    ///
    /// The offset is kept reduced modulo the linear part (see [`AffineSubspace::new`]), so this
    /// holds exactly when the stored offset is zero.
    pub fn contains_zero(&self) -> bool {
        self.offset.is_zero()
    }

    pub fn contains(&self, vector: FpSlice) -> bool {
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
    /// # Example
    /// ```
    /// # use fp::{matrix::{AffineSubspace, Matrix, Subspace}, prime::TWO, vector::FpVector};
    /// let linear_part = Subspace::from_matrix(Matrix::from_vec(TWO, &[vec![0, 1, 0], vec![0, 0, 1]]));
    /// let offset = FpVector::from_slice(TWO, &[1, 0, 0]);
    /// let subspace = AffineSubspace::new(offset, linear_part);
    ///
    /// assert_eq!(
    ///     format!("{}", subspace),
    ///     "[1, 0, 0] + {[0, 1, 0], [0, 0, 1]}"
    /// );
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} + {{{:#}}}", self.offset, self.linear_part)
    }
}
