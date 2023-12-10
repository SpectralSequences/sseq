use std::ops::Range;

use crate::{
    constants::BITS_PER_LIMB,
    limb::{Limb, LimbBitIndexPair},
    prime::Prime,
};

use super::{
    element::{FieldElement, MultiplicativeFieldElement, PolynomialFieldElement},
    Field, Fp, LargeFq, SmallFq,
};

/// Methods that lets us interact with the underlying `Limb` type.
///
/// In practice this is an extension trait of a `Field`, so we treat it as such. We can't make it a
/// supertrait of `Field` because `Field` is already a supertrait of `LimbMethods`.
pub trait LimbMethods: Clone + Copy + Sized {
    type Element: FieldElement;

    /// Encode a field element into a `Limb`. The limbs of an `FpVectorP<Self>` will consist of the
    /// coordinates of the vector, packed together using this method. It is assumed that the output
    /// value occupies at most `self.bit_length()` bits with the rest padded with zeros, and that
    /// the limb is reduced.
    ///
    /// It is required that `self.encode(self.zero()) == 0` (whenever `Self` implements `Field`).
    fn encode(self, element: Self::Element) -> Limb;

    /// Decode a `Limb` into a field element. The argument will always contain a single encoded
    /// field element, padded with zeros. This is the inverse of [`encode`].
    fn decode(self, element: Limb) -> Self::Element;

    /// Return the number of bits a `Self::Element` occupies in a limb.
    fn bit_length(self) -> usize;

    /// Fused multiply-add. Return the `Limb` whose `i`th entry is `limb_a[i] + coeff * limb_b[i]`.
    /// Both `limb_a` and `limb_b` are assumed to be reduced, and the result does not have to be
    /// reduced.
    fn fma_limb(self, limb_a: Limb, limb_b: Limb, coeff: Self::Element) -> Limb;

    /// Return the `Limb` whose entries are the entries of `limb` reduced modulo `P`.
    fn reduce(self, limb: Limb) -> Limb;

    /// If `l` is a limb of `Self::Element`s, then `l & F.bitmask()` is the value of the
    /// first entry of `l`.
    fn bitmask(self) -> Limb {
        (1 << self.bit_length()) - 1
    }

    /// The number of `Self::Element`s that fit in a single limb.
    fn entries_per_limb(self) -> usize {
        BITS_PER_LIMB / self.bit_length()
    }

    fn limb_bit_index_pair(self, idx: usize) -> LimbBitIndexPair {
        LimbBitIndexPair {
            limb: idx / self.entries_per_limb(),
            bit_index: (idx % self.entries_per_limb() * self.bit_length()),
        }
    }

    /// Check whether or not a limb is reduced, i.e. whether every entry is a value in the range `0..P`.
    /// This is currently **not** faster than calling [`reduce`] directly.
    fn is_reduced(self, limb: Limb) -> bool {
        limb == self.reduce(limb)
    }

    /// Given an interator of `Self::Element`s, pack all of them into a single limb in order.
    /// It is assumed that
    ///  - The values of the iterator are less than P
    ///  - The values of the iterator fit into a single limb
    ///
    /// If these assumptions are violated, the result will be nonsense.
    fn pack<T: Iterator<Item = Self::Element>>(self, entries: T) -> Limb {
        let bit_length = self.bit_length();
        let mut result: Limb = 0;
        let mut shift = 0;
        for entry in entries {
            result += self.encode(entry) << shift;
            shift += bit_length;
        }
        result
    }

    /// Give an iterator over the entries of `limb`.
    fn unpack(self, limb: Limb) -> LimbIterator<Self> {
        LimbIterator {
            fq: self,
            limb,
            bit_length: self.bit_length(),
            bit_mask: self.bitmask(),
        }
    }

    /// Return the number of limbs required to hold `dim` entries.
    fn number(self, dim: usize) -> usize {
        if dim == 0 {
            0
        } else {
            self.limb_bit_index_pair(dim - 1).limb + 1
        }
    }

    /// Return the `Range<usize>` starting at the index of the limb containing the `start`th entry, and
    /// ending at the index of the limb containing the `end`th entry (including the latter).
    fn range(self, start: usize, end: usize) -> Range<usize> {
        let min = self.limb_bit_index_pair(start).limb;
        let max = if end > 0 {
            self.limb_bit_index_pair(end - 1).limb + 1
        } else {
            0
        };
        min..max
    }

    /// Return either `Some(sum)` if no carries happen in the limb, or `None` if some carry does happen.
    fn truncate(self, sum: Limb) -> Option<Limb> {
        if self.is_reduced(sum) {
            Some(sum)
        } else {
            None
        }
    }
}

struct LimbIterator<F> {
    fq: F,
    limb: Limb,
    bit_length: usize,
    bit_mask: Limb,
}

impl<F: LimbMethods> Iterator for LimbIterator<F> {
    type Item = F::Element;

    fn next(&mut self) -> Option<Self::Item> {
        if self.limb == 0 {
            return None;
        }
        let result = self.limb & self.bit_mask;
        self.limb >>= self.bit_length;
        Some(self.fq.decode(result))
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

impl LimbMethods for SmallFq {
    type Element = MultiplicativeFieldElement;

    fn encode(self, element: Self::Element) -> Limb {
        element.0.map(|x| (x as Limb) << 1 | 1).unwrap_or(0)
    }

    fn decode(self, element: Limb) -> Self::Element {
        if element & 1 == 0 {
            MultiplicativeFieldElement(None)
        } else {
            MultiplicativeFieldElement(Some((element >> 1) as u32))
        }
    }

    fn bit_length(self) -> usize {
        todo!()
    }

    fn fma_limb(self, limb_a: Limb, limb_b: Limb, coeff: Self::Element) -> Limb {
        todo!()
    }

    fn reduce(self, limb: Limb) -> Limb {
        todo!()
    }
}

impl LimbMethods for LargeFq {
    type Element = PolynomialFieldElement<Self>;

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
        todo!()
    }
}
