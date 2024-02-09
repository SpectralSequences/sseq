use std::sync::Arc;

use dashmap::DashMap as HashMap;
use once_cell::sync::Lazy;

use crate::{
    constants::BITS_PER_LIMB,
    limb::Limb,
    prime::{log2, Prime, ValidPrime},
    vector::FpVector,
    PRIME_TO_INDEX_MAP,
};

use super::{limb::LimbMethods, Field, FieldElement, Fp};

static SMALL_CONWAY_POLYS: [[[u32; 17]; 15]; 54] = include!("small_conway_polys.txt");

type ZechTable = HashMap<SmallFqElement, SmallFqElement>;

/// A table of lazily initialized [Zech logarithms][zech_logs].
///
/// Key is the field, value is a fully initialized table of Zech logarithms.
///
/// [zech_logs]: https://en.wikipedia.org/wiki/Zech%27s_logarithm
static ZECH_LOGS: Lazy<HashMap<(ValidPrime, u32), Arc<ZechTable>>> = Lazy::new(HashMap::new);

/// Return the Zech logarithm table for the given field. If it does not exist yet, initialize it.
/// The initialization might be fairly expensive (several ms).
fn zech_logs<P: Prime>(fq: SmallFq<P>) -> Arc<ZechTable> {
    let table = ZECH_LOGS.entry((fq.p.to_dyn(), fq.d)).or_insert_with(|| {
        let conway_poly = {
            let v = SMALL_CONWAY_POLYS[PRIME_TO_INDEX_MAP[fq.p.as_usize()]][fq.d as usize - 2]
                .iter()
                .copied()
                .take(fq.d as usize + 1)
                .collect::<Vec<_>>();
            FpVector::from_slice(fq.characteristic(), &v)
        };
        let mul_by_a = |cur: FpVector| {
            // Shift all entries to the right by one. We're assuming that cur is a polynomial
            // representing an element of the field, so the leading coefficient is zero, and there
            // is no overflow.
            let mut next = FpVector::from_slice(
                cur.prime(),
                &std::iter::once(0)
                    .chain(cur.iter())
                    .take(cur.len())
                    .collect::<Vec<_>>(),
            );
            let leading_coeff = next.entry(next.len() - 1);
            next.add(&conway_poly, Fp(cur.prime()).neg(leading_coeff));
            next
        };

        // Generate a lookup table. For every element represented as a polynomial, we store the
        // power of `a` that corresponds to it.
        let poly_to_power: HashMap<FpVector, u32> = HashMap::new();
        let mut cur = FpVector::new(fq.characteristic(), conway_poly.len());
        cur.set_entry(0, 1);
        poly_to_power.insert(cur.clone(), 0);

        for i in 1..fq.q() - 1 {
            cur = mul_by_a(cur);
            poly_to_power.insert(cur.clone(), i);
        }

        // Loop over all elements again, but now recording logarithms.
        let table = HashMap::new();
        table.insert(fq.zero(), fq.one());

        let mut cur = FpVector::new(fq.characteristic(), conway_poly.len());
        cur.set_entry(0, 1);
        for i in 0..fq.q() - 1 {
            let cur_plus_1 = {
                let mut cur_plus_1 = cur.clone();
                cur_plus_1.add_basis_element(0, 1);
                cur_plus_1
            };
            cur = mul_by_a(cur);

            table.insert(
                SmallFqElement(Some(i)),
                SmallFqElement(poly_to_power.get(&cur_plus_1).as_deref().cloned()),
            );
        }
        Arc::new(table)
    });
    Arc::clone(&table)
}

/// A field of order `q = p^d`, where `q < 2^16` and `d > 1`. Fields of that size are small enough
/// that we can cache their Zech logarithms.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SmallFq<P> {
    p: P,
    d: u32,
}

impl<P: Prime> SmallFq<P> {
    pub fn new(p: P, d: u32) -> Self {
        assert!(d > 1);
        assert!(log2(p.pow(d) as usize) < 16);
        Self { p, d }
    }

    /// Return the element `-1`. If `p = 2`, this is `a^0 = 1`. Otherwise, it is `a^((q - 1) / 2)`.
    pub fn negative_one(self) -> SmallFqElement {
        let e = if self.p == 2 { 0 } else { (self.q() - 1) / 2 };
        SmallFqElement(Some(e))
    }

    /// The distinguished primitive element that generates the multiplicative group of the field.
    pub fn a(self) -> SmallFqElement {
        SmallFqElement(Some(1))
    }
}

impl<P: Prime> std::fmt::Display for SmallFq<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "F_({}^{})", self.p, self.d)
    }
}

impl<P: Prime> Field for SmallFq<P> {
    #[cfg(feature = "odd-primes")]
    type Characteristic = P;

    #[cfg(feature = "odd-primes")]
    fn characteristic(self) -> Self::Characteristic {
        self.p
    }

    fn degree(self) -> u32 {
        self.d
    }

    fn zero(self) -> Self::Element {
        SmallFqElement(None)
    }

    fn one(self) -> Self::Element {
        SmallFqElement(Some(0))
    }

    fn add(self, a: Self::Element, b: Self::Element) -> Self::Element {
        match (a, b) {
            (SmallFqElement(None), b) => b,
            (a, SmallFqElement(None)) => a,
            (SmallFqElement(Some(a)), SmallFqElement(Some(b))) => {
                // a^m + a^n = a^m (1 + a^(n - m)) = a^(m + Zech(n - m))
                let table = zech_logs(self);
                let (a, b) = if a >= b { (a, b) } else { (b, a) };
                let zech = table.get(&SmallFqElement(Some(a - b))).unwrap();
                if let Some(zech) = zech.0 {
                    SmallFqElement(Some((b + zech) % (self.q() - 1)))
                } else {
                    SmallFqElement(None)
                }
            }
        }
    }

    fn mul(self, a: Self::Element, b: Self::Element) -> Self::Element {
        if let (Some(a), Some(b)) = (a.0, b.0) {
            // Both `a` and `b` are less than 2^16, so there is no overflow.
            SmallFqElement(Some((a + b) % (self.q() - 1)))
        } else {
            SmallFqElement(None)
        }
    }

    fn neg(self, a: Self::Element) -> Self::Element {
        self.mul(a, self.negative_one())
    }

    fn inv(self, a: Self::Element) -> Option<Self::Element> {
        let complement = match a.0? {
            0 => 0,
            x => self.q() - 1 - x,
        };
        Some(SmallFqElement(Some(complement)))
    }

    fn frobenius(self, a: Self::Element) -> Self::Element {
        SmallFqElement(a.0.map(|x| (x * self.characteristic().as_u32()) % (self.q() - 1)))
    }
}

impl<P: Prime> LimbMethods for SmallFq<P> {
    type Element = SmallFqElement;

    /// This is 2n + 1 if `element` is a^n, and 0 otherwise.
    fn encode(self, element: Self::Element) -> Limb {
        element.0.map(|x| (x as Limb) << 1 | 1).unwrap_or(0)
    }

    fn decode(self, element: Limb) -> Self::Element {
        if element & 1 == 0 {
            // This only checks that the element is even, but by the definition of `encode`, this
            // only happens if the element is zero.
            SmallFqElement(None)
        } else {
            SmallFqElement(Some((element >> 1) as u32))
        }
    }

    fn bit_length(self) -> usize {
        // A field has q - 1 units, so SmallFqElement is either Some(a) where a is in [0, q - 2], or
        // None. We add 1 bit to account for encoding the None case.
        BITS_PER_LIMB - (self.q() - 1).leading_zeros() as usize + 1
    }

    fn fma_limb(self, limb_a: Limb, limb_b: Limb, coeff: Self::Element) -> Limb {
        let bit_length = self.bit_length();
        let mut result: Limb = 0;
        let mut shift = 0;
        for (a, b) in self.unpack(limb_a).zip(self.unpack(limb_b)) {
            result += self.encode(self.add(a, self.mul(coeff, b))) << shift;
            shift += bit_length;
        }
        result
    }

    fn reduce(self, limb: Limb) -> Limb {
        limb
    }
}

/// A field element, stored as the exponent of a distinguished generator of the group of units.
/// `None` if the element is zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SmallFqElement(pub(super) Option<u32>);

impl FieldElement for SmallFqElement {
    fn is_zero(&self) -> bool {
        self.0.is_none()
    }
}

impl std::fmt::Display for SmallFqElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            None => write!(f, "0"),
            Some(0) => write!(f, "1"),
            Some(1) => write!(f, "a"),
            Some(x) => write!(f, "a^{}", x),
        }
    }
}
