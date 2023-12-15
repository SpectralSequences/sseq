use dashmap::DashMap as HashMap;
use once_cell::sync::Lazy;

use crate::{
    limb::Limb,
    matrix::Matrix,
    prime::{Prime, ValidPrime},
    vector::inner::FqVectorP,
};

use super::{limb::LimbMethods, Field, FieldElement, Fp};

static MULT_MATRICES: Lazy<HashMap<(ValidPrime, u32), Matrix>> = Lazy::new(HashMap::new);
static FROB_MATRICES: Lazy<HashMap<(ValidPrime, u32), Matrix>> = Lazy::new(HashMap::new);

/// A field of order `q = p^d`, where `q >= 2^16` and `d > 1`. Fields of that size are too large to
/// cache their Zech logarithms, so we represent their elements as polynomials over the prime field.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LargeFq<P> {
    p: P,
    d: u32,
}

impl<P: Prime> Field for LargeFq<P> {
    type Characteristic = P;

    fn characteristic(self) -> Self::Characteristic {
        self.p
    }

    fn degree(self) -> u32 {
        self.d
    }

    fn zero(self) -> Self::Element {
        LargeFqElement(FqVectorP::new(Fp(self.p), self.d as usize))
    }

    fn one(self) -> Self::Element {
        todo!()
    }

    fn add(self, a: Self::Element, b: Self::Element) -> Self::Element {
        todo!()
    }

    fn mul(self, a: Self::Element, b: Self::Element) -> Self::Element {
        todo!()
    }

    fn neg(self, a: Self::Element) -> Self::Element {
        todo!()
    }

    fn inv(self, a: Self::Element) -> Option<Self::Element> {
        todo!()
    }

    fn frobenius(self, a: Self::Element) -> Self::Element {
        todo!()
    }
}

impl<P: Prime> LimbMethods for LargeFq<P> {
    type Element = LargeFqElement<P>;

    fn encode(self, element: Self::Element) -> Limb {
        todo!()
    }

    fn decode(self, element: Limb) -> Self::Element {
        todo!()
    }

    fn bit_length(self) -> usize {
        todo!()
    }

    fn fma_limb(self, limb_a: Limb, limb_b: Limb, coeff: Self::Element) -> Limb {
        todo!()
    }

    fn reduce(self, limb: Limb) -> Limb {
        limb
    }
}

/// A field element as a polynomial over the prime field. This is used when the order of the
/// field is large, since otherwise caching Zech logarithms uses too much memory.
///
/// This is backed by an `FpVectorP` consisting of the coefficients of the polynomial.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LargeFqElement<P: Prime>(pub(super) FqVectorP<Fp<P>>);

impl<P: Prime> FieldElement for LargeFqElement<P> {
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}
