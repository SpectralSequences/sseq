use std::sync::{Arc, LazyLock};

use dashmap::DashMap as HashMap;

use super::{
    element::{FieldElement, FieldElementContainer},
    field_internal::FieldInternal,
    Field, Fp,
};
use crate::{
    constants::BITS_PER_LIMB,
    limb::Limb,
    prime::{log2, Prime, ValidPrime},
    vector::inner::FqVector,
    PRIME_TO_INDEX_MAP,
};

static SMALL_CONWAY_POLYS: [[[u32; 17]; 15]; 54] = include!("small_conway_polys.txt");

type ZechTable = HashMap<SmallFqElement, SmallFqElement>;
type Polynomial<P> = FqVector<Fp<P>>;

/// A table of lazily initialized [Zech logarithms][zech_logs].
///
/// Key is the field, value is a fully initialized table of Zech logarithms.
///
/// [zech_logs]: https://en.wikipedia.org/wiki/Zech%27s_logarithm
static ZECH_LOGS: LazyLock<HashMap<(ValidPrime, u32), Arc<ZechTable>>> =
    LazyLock::new(HashMap::new);

/// Return the Zech logarithm table for the given field. If it does not exist yet, initialize it.
/// The initialization might be fairly expensive (several ms).
fn zech_logs<P: Prime>(fq: SmallFq<P>) -> Arc<ZechTable> {
    let table = ZECH_LOGS.entry((fq.p.to_dyn(), fq.d)).or_insert_with(|| {
        let prime_field = Fp::new(fq.characteristic());
        let conway_poly = {
            let v = SMALL_CONWAY_POLYS[PRIME_TO_INDEX_MAP[fq.p.as_usize()]][fq.d as usize - 2]
                .iter()
                .take(fq.d as usize + 1)
                .map(|c| prime_field.el(*c))
                .collect::<Vec<_>>();
            Polynomial::from_slice(prime_field, &v)
        };
        let mul_by_a = |current: Polynomial<P>| {
            // Shift all entries up by one. We're assuming that cur is a polynomial representing an
            // element of the field, so the top coefficient is zero, and there is no overflow.
            let mut next = Polynomial::from_slice(
                prime_field,
                &std::iter::once(prime_field.zero())
                    .chain(current.iter())
                    .take(current.len())
                    .collect::<Vec<_>>(),
            );
            let leading_coeff = next.entry(next.len() - 1);
            next.add(&conway_poly, -leading_coeff);
            next
        };

        // Generate a lookup table. For every element represented as a polynomial, we store the
        // power of `a` that corresponds to it.
        let poly_to_power: HashMap<Polynomial<P>, u32> = HashMap::new();
        let mut current = Polynomial::new(prime_field, conway_poly.len());
        current.set_entry(0, prime_field.one());
        poly_to_power.insert(current.clone(), 0);

        for i in 1..fq.q() - 1 {
            current = mul_by_a(current);
            poly_to_power.insert(current.clone(), i);
        }

        // Loop over all elements again, but now recording logarithms.
        let table = ZechTable::new();
        table.insert(fq.zero().val(), fq.one().val());

        let mut current = Polynomial::new(prime_field, conway_poly.len());
        current.set_entry(0, prime_field.one());
        for i in 0..fq.q() - 1 {
            let mut current_plus_1 = current.clone();
            current_plus_1.add_basis_element(0, prime_field.one());
            table.insert(
                SmallFqElement(Some(i)),
                SmallFqElement(poly_to_power.get(&current_plus_1).as_deref().copied()),
            );

            current = mul_by_a(current);
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
        assert!(d > 1, "Use Fp for prime fields");
        assert!(log2(p.pow(d) as usize) < 16, "Field too large");

        Self { p, d }
    }

    /// Return the element `-1`. If `p = 2`, this is `a^0 = 1`. Otherwise, it is `a^((q - 1) / 2)`.
    pub fn negative_one(self) -> FieldElement<Self> {
        let e = if self.p == 2 { 0 } else { (self.q() - 1) / 2 };
        self.el(SmallFqElement(Some(e)))
    }

    /// The distinguished primitive element that generates the multiplicative group of the field.
    pub fn a(self) -> FieldElement<Self> {
        self.el(SmallFqElement(Some(1)))
    }
}

impl<P: Prime> std::fmt::Display for SmallFq<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "F_({}^{})", self.p, self.d)
    }
}

impl<P: Prime> Field for SmallFq<P> {
    type Characteristic = P;

    fn characteristic(self) -> Self::Characteristic {
        self.p
    }

    fn degree(self) -> u32 {
        self.d
    }

    fn zero(self) -> FieldElement<Self> {
        self.el(SmallFqElement(None))
    }

    fn one(self) -> FieldElement<Self> {
        self.el(SmallFqElement(Some(0)))
    }
}

impl<P: Prime> FieldInternal for SmallFq<P> {
    type ElementContainer = SmallFqElement;

    fn el(self, value: Self::ElementContainer) -> FieldElement<Self> {
        let reduced_value = value.0.map(|e| e % (self.q() - 1));
        FieldElement::new(self, SmallFqElement(reduced_value))
    }

    fn add_assign(self, a: &mut FieldElement<Self>, b: FieldElement<Self>) {
        // It's simpler to just define `add` directly, using the inherent symmetry. It doesn't
        // really matter since `SmallFqElement` is `Copy`.
        *a = self.add(*a, b);
    }

    fn add(self, a: FieldElement<Self>, b: FieldElement<Self>) -> FieldElement<Self> {
        self.el(match (a.value, b.value) {
            (SmallFqElement(None), b) => b,
            (a, SmallFqElement(None)) => a,
            (SmallFqElement(Some(a)), SmallFqElement(Some(b))) => {
                // a^m + a^n = a^m (1 + a^(n - m)) = a^(m + Zech(n - m))
                let table = zech_logs(self);
                let (a, b) = if a >= b { (a, b) } else { (b, a) };
                let zech = table.get(&SmallFqElement(Some(a - b))).unwrap();
                if let Some(zech) = zech.0 {
                    SmallFqElement(Some(b + zech))
                } else {
                    SmallFqElement(None)
                }
            }
        })
    }

    fn mul_assign(self, a: &mut FieldElement<Self>, b: FieldElement<Self>) {
        if let (SmallFqElement(Some(a)), SmallFqElement(Some(b))) = (&mut a.value, b.value) {
            *a += b;
            *a %= self.q() - 1;
        } else {
            a.value = SmallFqElement(None);
        }
    }

    fn neg(self, a: FieldElement<Self>) -> FieldElement<Self> {
        self.mul(a, self.negative_one())
    }

    fn inv(self, a: FieldElement<Self>) -> Option<FieldElement<Self>> {
        let complement = match a.0? {
            0 => 0,
            x => self.q() - 1 - x,
        };
        Some(self.el(SmallFqElement(Some(complement))))
    }

    fn frobenius(self, a: FieldElement<Self>) -> FieldElement<Self> {
        self.el(SmallFqElement(
            a.0.map(|x| x * self.characteristic().as_u32()),
        ))
    }

    /// This is 2n + 1 if `element` is a^n, and 0 otherwise.
    fn encode(self, element: FieldElement<Self>) -> Limb {
        element.value.0.map(|x| (x as Limb) << 1 | 1).unwrap_or(0)
    }

    fn decode(self, element: Limb) -> FieldElement<Self> {
        self.el(if element & 1 == 0 {
            // This only checks that the element is even, but by the definition of `encode`, this
            // only happens if the element is zero.
            SmallFqElement(None)
        } else {
            SmallFqElement(Some((element >> 1) as u32))
        })
    }

    fn bit_length(self) -> usize {
        // A field has q - 1 units, so SmallFqElement is either Some(a) where a is in [0, q - 2], or
        // None. We add 1 bit to account for encoding the None case.
        BITS_PER_LIMB - (self.q() - 1).leading_zeros() as usize + 1
    }

    fn fma_limb(self, limb_a: Limb, limb_b: Limb, coeff: FieldElement<Self>) -> Limb {
        let bit_length = self.bit_length();
        let mut result: Limb = 0;
        let mut shift = 0;
        for (a, b) in self.unpack(limb_a).zip(self.unpack(limb_b)) {
            result += self.encode(a + coeff * b) << shift;
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

impl FieldElementContainer for SmallFqElement {}

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