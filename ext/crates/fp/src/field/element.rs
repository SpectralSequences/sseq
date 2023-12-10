use super::{Field, Fp};
use crate::vector::inner::FqVectorP;

pub trait FieldElement: Clone {
    fn is_zero(&self) -> bool;
}

/// A field element, stored as the exponent of a chosen generator of the group of units. `None` if
/// the element is zero.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MultiplicativeFieldElement(pub(super) Option<u32>);

/// A field element as a polynomial over the prime field. This is used when the order of the
/// field is large, since otherwise caching Zech logarithms uses too much memory.
///
/// This is backed by an `FpVectorP` consisting of the coefficients of the polynomial.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PolynomialFieldElement<F: Field>(pub(super) FqVectorP<Fp<F::Characteristic>>);

impl FieldElement for u32 {
    fn is_zero(&self) -> bool {
        *self == 0
    }
}

impl FieldElement for MultiplicativeFieldElement {
    fn is_zero(&self) -> bool {
        self.0.is_none()
    }
}

impl<F: Field> FieldElement for PolynomialFieldElement<F> {
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}
