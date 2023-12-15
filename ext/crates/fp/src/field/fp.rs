use super::{limb::LimbMethods, Field, FieldElement};
use crate::{constants::BITS_PER_LIMB, limb::Limb, prime::Prime};

/// A prime field. This is just a wrapper around a prime.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Fp<P>(pub(crate) P);

impl<P: Prime> Field for Fp<P> {
    #[cfg(feature = "odd-primes")]
    type Characteristic = P;

    #[cfg(feature = "odd-primes")]
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
        self.0.sum(a, b)
    }

    fn mul(self, a: Self::Element, b: Self::Element) -> Self::Element {
        self.0.product(a, b)
    }

    fn inv(self, a: Self::Element) -> Option<Self::Element> {
        if a == 0 {
            None
        } else {
            Some(crate::prime::inverse(self.0, a))
        }
    }

    fn neg(self, a: Self::Element) -> Self::Element {
        if a > 0 {
            self.0.as_u32() - a
        } else {
            0
        }
    }

    fn frobenius(self, a: Self::Element) -> Self::Element {
        a
    }
}

impl<P: Prime> LimbMethods for Fp<P> {
    type Element = u32;

    fn encode(self, element: Self::Element) -> Limb {
        element as Limb
    }

    fn decode(self, element: Limb) -> Self::Element {
        (element % self.0.as_u32() as Limb) as u32
    }

    fn bit_length(self) -> usize {
        let p = self.characteristic().as_u32() as u64;
        match p {
            2 => 1,
            _ => (BITS_PER_LIMB as u32 - (p * (p - 1)).leading_zeros()) as usize,
        }
    }

    fn fma_limb(self, limb_a: Limb, limb_b: Limb, coeff: Self::Element) -> Limb {
        if self.characteristic() == 2 {
            limb_a ^ (coeff as Limb * limb_b)
        } else {
            limb_a + (coeff as Limb) * limb_b
        }
    }

    /// Contributed by Robert Burklund.
    fn reduce(self, limb: Limb) -> Limb {
        match self.characteristic().as_u32() {
            2 => limb,
            3 => {
                // Set top bit to 1 in every limb
                const TOP_BIT: Limb = (!0 / 7) << (2 - BITS_PER_LIMB % 3);
                let mut limb_2 = ((limb & TOP_BIT) >> 2) + (limb & (!TOP_BIT));
                let mut limb_3s = limb_2 & (limb_2 >> 1);
                limb_3s |= limb_3s << 1;
                limb_2 ^= limb_3s;
                limb_2
            }
            5 => {
                // Set bottom bit to 1 in every limb
                const BOTTOM_BIT: Limb = (!0 / 31) >> (BITS_PER_LIMB % 5);
                const BOTTOM_TWO_BITS: Limb = BOTTOM_BIT | (BOTTOM_BIT << 1);
                const BOTTOM_THREE_BITS: Limb = BOTTOM_BIT | (BOTTOM_TWO_BITS << 1);
                let a = (limb >> 2) & BOTTOM_THREE_BITS;
                let b = limb & BOTTOM_TWO_BITS;
                let m = (BOTTOM_BIT << 3) - a + b;
                let mut c = (m >> 3) & BOTTOM_BIT;
                c |= c << 1;
                let d = m & BOTTOM_THREE_BITS;
                d + c - BOTTOM_TWO_BITS
            }
            _ => self.pack(self.unpack(limb)),
        }
    }
}

impl<P> std::ops::Deref for Fp<P> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<P: Prime> From<P> for Fp<P> {
    fn from(p: P) -> Self {
        Self(p)
    }
}

impl FieldElement for u32 {
    fn is_zero(&self) -> bool {
        *self == 0
    }
}
