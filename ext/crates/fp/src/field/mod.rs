use self::{element::MultiplicativeFieldElement, limb::LimbMethods};
use crate::prime::{Prime, ValidPrime};

pub mod element;
pub(crate) mod limb;

pub trait Field: Clone + Copy + LimbMethods + Sized {
    type Characteristic: Prime;

    fn characteristic(self) -> Self::Characteristic;
    fn degree(self) -> u32;

    fn zero(self) -> Self::Element;
    fn one(self) -> Self::Element;

    fn add(self, a: Self::Element, b: Self::Element) -> Self::Element;
    fn sub(self, a: Self::Element, b: Self::Element) -> Self::Element;
    fn mul(self, a: Self::Element, b: Self::Element) -> Self::Element;
    fn div(self, a: Self::Element, b: Self::Element) -> Self::Element;

    fn inv(self, a: Self::Element) -> Self::Element;
    fn neg(self, a: Self::Element) -> Self::Element;

    fn frobenius(self, a: Self::Element) -> Self::Element;
}

/// A prime field. This is just a wrapper around a prime.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Fp<P>(pub P);

/// A field of order `q = p^d`, where `q < 2^16` and `d > 1`. Fields of that size are small enough
/// that we can cache their Zech logarithms.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SmallFq {
    p: ValidPrime,
    d: u32,
}

/// A field of order `q = prime^degree`, where `q >= 2^16` and `d > 1`. Fields of that size are too
/// large to cache their Zech logarithms, so we represent their elements as polynomials over the
/// prime field.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LargeFq {
    p: ValidPrime,
    d: u32,
}

impl SmallFq {
    pub fn new(p: ValidPrime, degree: u32) -> Self {
        assert!(degree > 0);
        assert!(degree <= 16);
        Self { p, d: degree }
    }
}

impl<P: Prime> Field for Fp<P> {
    type Characteristic = P;

    fn characteristic(self) -> Self::Characteristic {
        self.0
    }

    fn degree(self) -> u32 {
        1
    }

    fn zero(self) -> Self::Element {
        0
    }

    fn one(self) -> Self::Element {
        1
    }

    fn add(self, a: Self::Element, b: Self::Element) -> Self::Element {
        ((a as u64 + b as u64) % self.0.as_u32() as u64) as u32
    }

    fn sub(self, a: Self::Element, b: Self::Element) -> Self::Element {
        ((a as u64 - b as u64) % self.0.as_u32() as u64) as u32
    }

    fn mul(self, a: Self::Element, b: Self::Element) -> Self::Element {
        ((a as u64 * b as u64) % self.0.as_u32() as u64) as u32
    }

    fn div(self, a: Self::Element, b: Self::Element) -> Self::Element {
        self.mul(a, self.inv(b))
    }

    fn inv(self, a: Self::Element) -> Self::Element {
        self.0.pow_mod(a, self.0.as_u32() - 2)
    }

    fn neg(self, a: Self::Element) -> Self::Element {
        self.0.as_u32() - a
    }

    fn frobenius(self, a: Self::Element) -> Self::Element {
        a
    }
}

impl Field for SmallFq {
    type Characteristic = ValidPrime;

    fn characteristic(self) -> Self::Characteristic {
        self.p
    }

    fn degree(self) -> u32 {
        self.d
    }

    fn zero(self) -> Self::Element {
        MultiplicativeFieldElement(None)
    }

    fn one(self) -> Self::Element {
        todo!()
    }

    fn add(self, a: Self::Element, b: Self::Element) -> Self::Element {
        todo!()
    }

    fn sub(self, a: Self::Element, b: Self::Element) -> Self::Element {
        todo!()
    }

    fn mul(self, a: Self::Element, b: Self::Element) -> Self::Element {
        todo!()
    }

    fn div(self, a: Self::Element, b: Self::Element) -> Self::Element {
        todo!()
    }

    fn inv(self, a: Self::Element) -> Self::Element {
        todo!()
    }

    fn neg(self, a: Self::Element) -> Self::Element {
        todo!()
    }

    fn frobenius(self, a: Self::Element) -> Self::Element {
        todo!()
    }
}

impl Field for LargeFq {
    type Characteristic = ValidPrime;

    fn characteristic(self) -> Self::Characteristic {
        self.p
    }

    fn degree(self) -> u32 {
        self.d
    }

    fn zero(self) -> Self::Element {
        todo!()
    }

    fn one(self) -> Self::Element {
        todo!()
    }

    fn add(self, a: Self::Element, b: Self::Element) -> Self::Element {
        todo!()
    }

    fn sub(self, a: Self::Element, b: Self::Element) -> Self::Element {
        todo!()
    }

    fn mul(self, a: Self::Element, b: Self::Element) -> Self::Element {
        todo!()
    }

    fn div(self, a: Self::Element, b: Self::Element) -> Self::Element {
        todo!()
    }

    fn inv(self, a: Self::Element) -> Self::Element {
        todo!()
    }

    fn neg(self, a: Self::Element) -> Self::Element {
        todo!()
    }

    fn frobenius(self, a: Self::Element) -> Self::Element {
        todo!()
    }
}
