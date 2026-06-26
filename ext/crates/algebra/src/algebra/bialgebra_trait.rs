use crate::algebra::Algebra;

/// Why [`Bialgebra::try_coproduct`] could not compute a coproduct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoproductError {
    /// `op_deg` is negative, or `op_idx` is not a valid basis index in that degree.
    OutOfRange,
    /// The Milnor coproduct is only implemented at the prime 2.
    OddPrimeUnsupported,
    /// The generic Adem coproduct is only defined when the degree is divisible by `q = 2p - 2`
    /// (the degree-1 Bockstein aside).
    IndivisibleDegree {
        /// The modulus `q = 2p - 2` the degree must be divisible by.
        q: u32,
        /// The offending degree.
        degree: i32,
    },
    /// The Adem coproduct at the prime 2 is only defined for index 0.
    NonzeroIndex,
}

impl std::fmt::Display for CoproductError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfRange => f.write_str("basis element index out of range"),
            Self::OddPrimeUnsupported => f.write_str("coproduct is only supported at p = 2"),
            Self::IndivisibleDegree { q, degree } => {
                write!(f, "coproduct expects a degree divisible by {q}, got {degree}")
            }
            Self::NonzeroIndex => f.write_str("at p = 2 the coproduct expects index 0"),
        }
    }
}

impl std::error::Error for CoproductError {}

/// An [`Algebra`] equipped with a coproduct operation that makes it into a
/// bialgebra.
#[enum_dispatch::enum_dispatch]
pub trait Bialgebra: Algebra {
    /// Computes a coproduct $\Delta(x)$, expressed as
    ///
    /// $$ Delta(x)_i = \sum_j A_{ij} \otimes B_{ij}. $$
    ///
    /// The return value is a list of these pairs of basis elements.
    ///
    /// `x` must have been returned by [`Bialgebra::decompose()`].
    fn coproduct(&self, op_deg: i32, op_idx: usize) -> Vec<(i32, usize, i32, usize)>;

    /// Non-panicking variant of [`Self::coproduct`]. Computes the basis up to `op_deg` first and
    /// returns [`CoproductError::OutOfRange`] when `op_deg`/`op_idx` do not name a basis element.
    ///
    /// This default only guards the out-of-range preconditions shared by every implementation, so
    /// it is correct for [`Self::coproduct`] implementations that are total on valid basis
    /// elements (such as [`crate::algebra::Field`]). Implementations whose `coproduct` is only
    /// partially defined — e.g. [`crate::algebra::MilnorAlgebra`] (prime 2 only) and
    /// [`crate::algebra::AdemAlgebra`] (degree/index restrictions) — override this method to guard
    /// their extra preconditions, so the delegation below never reaches the panicking path.
    fn try_coproduct(
        &self,
        op_deg: i32,
        op_idx: usize,
    ) -> Result<Vec<(i32, usize, i32, usize)>, CoproductError> {
        if op_deg < 0 {
            return Err(CoproductError::OutOfRange);
        }
        self.compute_basis(op_deg);
        if op_idx >= self.dimension(op_deg) {
            return Err(CoproductError::OutOfRange);
        }
        Ok(self.coproduct(op_deg, op_idx))
    }

    /// Decomposes an element of the algebra into a product of elements, each of
    /// which we can compute a coproduct on efficiently.
    ///
    /// The product is laid out such that the first element of the vector is
    /// applied to a module element first when acting on it.
    ///
    /// This function is to be used with [`Bialgebra::coproduct()`].
    ///
    /// This API is motivated by the fact that, in the admissible basis for the Adem algebra,
    /// an element naturally decomposes into a product of Steenrod squares, each of which has an
    /// easy coproduct formula.
    fn decompose(&self, op_deg: i32, op_idx: usize) -> Vec<(i32, usize)>;
}
