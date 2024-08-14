use super::{
    element::{FieldElement, FieldElementContainer},
    field_internal::FieldInternal,
    Field,
};
// Reexport the prime fields in a more logical place
pub use crate::prime::fp::*;
use crate::{constants::BITS_PER_LIMB, limb::Limb, prime::Prime};

/// A prime field. This is just a wrapper around a prime.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Fp<P> {
    p: P,
}

impl<P: Prime> Fp<P> {
    pub const fn new(p: P) -> Self {
        Self { p }
    }

    pub fn element(&self, value: u32) -> FieldElement<Self> {
        self.el(value)
    }
}

impl<P: Prime> Field for Fp<P> {
    type Characteristic = P;

    fn characteristic(self) -> Self::Characteristic {
        self.p
    }

    fn degree(self) -> u32 {
        1
    }

    fn zero(self) -> FieldElement<Self> {
        self.el(0)
    }

    fn one(self) -> FieldElement<Self> {
        self.el(1)
    }
}

impl<P: Prime> FieldInternal for Fp<P> {
    type ElementContainer = u32;

    fn el(self, value: Self::ElementContainer) -> FieldElement<Self> {
        FieldElement::new(self, value % self.p.as_u32())
    }

    fn add_assign(self, a: &mut FieldElement<Self>, b: FieldElement<Self>) {
        a.value = self.p.sum(**a, *b);
    }

    fn mul_assign(self, a: &mut FieldElement<Self>, b: FieldElement<Self>) {
        a.value = self.p.product(**a, *b);
    }

    fn inv(self, a: FieldElement<Self>) -> Option<FieldElement<Self>> {
        if *a == 0 {
            None
        } else {
            Some(self.el(crate::prime::inverse(self.p, *a)))
        }
    }

    fn neg(self, a: FieldElement<Self>) -> FieldElement<Self> {
        self.el(if *a == 0 { 0 } else { self.p.as_u32() - *a })
    }

    fn frobenius(self, a: FieldElement<Self>) -> FieldElement<Self> {
        a
    }

    fn encode(self, element: FieldElement<Self>) -> Limb {
        element.value as Limb
    }

    fn decode(self, element: Limb) -> FieldElement<Self> {
        // We have to pass in the already reduced value to `Self::el` because we have no guarantee
        // that this Limb fits in a u32. For example, `element` could be the result of `fma_limb(0,
        // 1_000_000, 1_000_000)`, if the prime is large enough.
        let prime_limb = self.p.as_u32() as Limb;
        self.el((element % prime_limb) as u32)
    }

    fn bit_length(self) -> usize {
        let p = self.characteristic().as_u32() as u64;
        match p {
            // 2 is a special case b/c bitwise xor does the add and reduce together so we only need enough bits to fit p-1.
            2 => 1,
            _ => (BITS_PER_LIMB as u32 - (p * (p - 1)).leading_zeros()) as usize,
        }
    }

    fn fma_limb(self, limb_a: Limb, limb_b: Limb, coeff: FieldElement<Self>) -> Limb {
        if self.characteristic() == 2 {
            limb_a ^ (coeff.value as Limb * limb_b)
        } else {
            limb_a + (coeff.value as Limb) * limb_b
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
            // Slow generic fallback. If anyone cares enough about some larger prime, they can add a faster implementation
            _ => self.pack(self.unpack(limb)),
        }
    }
}

impl<P> std::ops::Deref for Fp<P> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.p
    }
}

impl<P: Prime> From<P> for Fp<P> {
    fn from(p: P) -> Self {
        Self { p }
    }
}

impl FieldElementContainer for u32 {}

impl<P: Prime> From<FieldElement<Fp<P>>> for u32 {
    fn from(element: FieldElement<Fp<P>>) -> Self {
        element.value
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::Fp;
    use crate::{
        field::{element::FieldElement, Field},
        prime::Prime,
    };

    fn arb_element<P: Prime>(f: Fp<P>) -> impl Strategy<Value = FieldElement<Fp<P>>> {
        (0..f.characteristic().as_u32()).prop_map(move |v| f.element(v))
    }

    fn arb_elements<P: Prime, const N: usize>(
        p: P,
    ) -> impl Strategy<Value = (Fp<P>, [FieldElement<Fp<P>>; N])> {
        let f = Fp::new(p);

        let elements: [_; N] = (0..N)
            .map(|_| arb_element(f))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        (Just(f), elements)
    }

    mod validprime {
        use super::*;
        use crate::{field_tests, prime::ValidPrime};

        fn arb_elements<const N: usize>(
        ) -> impl Strategy<Value = (Fp<ValidPrime>, [FieldElement<Fp<ValidPrime>>; N])> {
            crate::prime::tests::arb_prime().prop_flat_map(super::arb_elements)
        }

        field_tests!();
    }

    macro_rules! static_fp_tests {
        ($p:tt) => {
            paste::paste! {
                static_fp_tests!(@ [<$p:lower>], $p, $p, $p);
            }
        };
        (@ $mod_name:ident, $p_expr:expr, $p_ident:ident, $p_ty:ty) => {
            mod $mod_name {
                use super::*;
                use crate::{field_tests, prime::$p_ident};

                fn arb_elements<const N: usize>(
                ) -> impl Strategy<Value = (Fp<$p_ty>, [FieldElement<Fp<$p_ty>>; N])> {
                    super::arb_elements($p_expr)
                }

                field_tests!();
            }
        };
    }

    static_fp_tests!(P2);
    cfg_if::cfg_if! { if #[cfg(feature = "odd-primes")] {
        static_fp_tests!(P3);
        static_fp_tests!(P5);
        static_fp_tests!(P7);
    }}
}
