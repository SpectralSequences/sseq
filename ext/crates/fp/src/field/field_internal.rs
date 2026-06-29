// According to
// https://doc.rust-lang.org/stable/rustc/lints/listing/warn-by-default.html#private-interfaces:
//
// "Having something private in primary interface guarantees that the item will be unusable from
// outer modules due to type privacy."
//
// In our case, this is a feature. We want to be able to use the `FieldInternal` trait in this crate
// and we also want it to be inaccessible from outside the crate.
#![allow(private_interfaces)]

use std::{hash::Hash, ops::Range};

use super::element::{FieldElement, FieldElementContainer};
use crate::{
    constants::BITS_PER_LIMB,
    limb::{Limb, LimbBitIndexPair},
};

macro_rules! normal_from_assign {
    ($fn_normal:ident, $fn_assign:ident) => {
        fn $fn_normal(
            self,
            mut a: FieldElement<Self>,
            b: FieldElement<Self>,
        ) -> FieldElement<Self> {
            self.$fn_assign(&mut a, b);
            a
        }
    };
}

/// Internal methods required for fields.
///
/// A field has several responsibilities. It must define:
/// - what its elements "look like", i.e. how they are represented in memory;
/// - how to perform finite field operations on those elements, namely addition, subtraction,
///   multiplication, division (except by zero), and the Frobenius endomorphism;
/// - how to pack and unpack elements into and from `Limb`s, so that `FqVector` can handle them.
///
/// We want a trait that makes all those definitions. However, we don't want to expose these
/// implementation details to the outside world. Therefore, we define a public trait that defines
/// public field methods (e.g. constructing the zero element) and an internal trait that takes care
/// of the details. The latter trait is `FieldInternal`.
///
/// The fact that each field defines its own element type means that we can define a single struct
/// that packages both a field and one of its elements, and this struct will be how we expose field
/// operations to the outside world.
#[allow(private_bounds)]
pub trait FieldInternal:
    std::fmt::Debug + Copy + PartialEq + Eq + Hash + Sized + crate::MaybeArbitrary<()> + 'static
{
    /// The internal representation of a field element.
    type ElementContainer: FieldElementContainer;

    /// Create a new field element. This is the method responsible for ensuring that the returned
    /// value is in a consistent state. For example, for a prime field of characteristic `p`, this
    /// function is responsible for ensuring that the `FieldElement` that is returned contains a
    /// value in the range `0..p`.
    fn el(self, value: Self::ElementContainer) -> FieldElement<Self>;

    // # Field operations
    // ## Mendatory methods

    fn add_assign(self, a: &mut FieldElement<Self>, b: FieldElement<Self>);
    fn mul_assign(self, a: &mut FieldElement<Self>, b: FieldElement<Self>);

    fn neg(self, a: FieldElement<Self>) -> FieldElement<Self>;
    fn inv(self, a: FieldElement<Self>) -> Option<FieldElement<Self>>;

    fn frobenius(self, a: FieldElement<Self>) -> FieldElement<Self>;

    // ## Default implementations

    fn sub_assign(self, a: &mut FieldElement<Self>, b: FieldElement<Self>) {
        self.add_assign(a, self.neg(b));
    }

    normal_from_assign!(add, add_assign);
    normal_from_assign!(sub, sub_assign);
    normal_from_assign!(mul, mul_assign);

    fn div(self, a: FieldElement<Self>, b: FieldElement<Self>) -> Option<FieldElement<Self>> {
        Some(self.mul(a, self.inv(b)?))
    }

    // # Limb operations

    /// Encode a field element into a `Limb`. The limbs of an `FqVector<Self>` will consist of the
    /// coordinates of the vector, packed together using this method. It is assumed that the output
    /// value occupies at most `self.bit_length()` bits with the rest padded with zeros, and that
    /// the limb is reduced.
    ///
    /// It is required that `self.encode(self.zero()) == 0` (whenever `Self` implements `Field`).
    fn encode(self, element: FieldElement<Self>) -> Limb;

    /// Decode a `Limb` into a field element. The argument will always contain a single encoded
    /// field element, padded with zeros. This is the inverse of [`encode`](FieldInternal::encode).
    fn decode(self, element: Limb) -> FieldElement<Self>;

    /// Return the number of bits a `Self::Element` occupies in a limb.
    fn bit_length(self) -> usize;

    /// Fused multiply-add. Return the `Limb` whose `i`th entry is `limb_a[i] + coeff * limb_b[i]`.
    /// Both `limb_a` and `limb_b` are assumed to be reduced, and the result does not have to be
    /// reduced.
    fn fma_limb(self, limb_a: Limb, limb_b: Limb, coeff: FieldElement<Self>) -> Limb;

    /// Reduce a limb, i.e. make it "canonical". For example, in [`Fp`](super::Fp), this replaces
    /// every entry by its value modulo p.
    ///
    /// Many functions assume that the input limbs are reduced, but it's useful to allow the
    /// existence of non-reduced limbs for performance reasons. Some functions like `fma_limb` can
    /// be very quick compared to the reduction step, so finishing a computation by reducing all
    /// limbs in sequence may allow the compiler to play some tricks with, for example, loop
    /// unrolling and SIMD.
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

    // # Group layout (bit-sliced storage)
    //
    // Storage is organized into *groups*: a group holds [`entries_per_group`] = 64 consecutive
    // entries and occupies [`limbs_per_group`] = `k` consecutive limbs, the *bit-planes*. Plane
    // `j` of a group holds bit `j` of all 64 entries, so entry `i` lives at bit `i` of each of
    // the `k` planes. Every field uses this layout, with `k = ceil(log2 q)` (the bits needed to
    // store an encoded value in `0..q`). For `q = 2` this is `k = 1`, which coincides exactly
    // with the old packed layout — so `F_2` (and its SIMD / matrix machinery) is byte-identical
    // and unaffected. Entry access goes through [`gather`]/[`scatter`]; the sizing helpers
    // [`number`]/[`range`] are expressed in terms of groups.
    //
    // [`entries_per_group`]: FieldInternal::entries_per_group
    // [`limbs_per_group`]: FieldInternal::limbs_per_group
    // [`gather`]: FieldInternal::gather
    // [`scatter`]: FieldInternal::scatter
    // [`number`]: FieldInternal::number
    // [`range`]: FieldInternal::range

    /// The number of entries stored in a single group: one per bit of a [`Limb`].
    fn entries_per_group(self) -> usize {
        BITS_PER_LIMB
    }

    /// The number of bit-planes per group, `k = ceil(log2 q)`. Each field defines this; `q = 2`
    /// gives `k = 1` (packed-compatible).
    fn limbs_per_group(self) -> usize;

    /// The index of the group containing entry `idx`.
    fn group_of(self, idx: usize) -> usize {
        idx / self.entries_per_group()
    }

    /// The position of entry `idx` within its group, in `0..entries_per_group()`.
    fn lane_of(self, idx: usize) -> usize {
        idx % self.entries_per_group()
    }

    /// Read entry `lane` (in `0..entries_per_group()`) out of a single group's `k` planes (a
    /// slice of length [`limbs_per_group`](FieldInternal::limbs_per_group)) by reassembling its
    /// bit from each plane.
    fn gather(self, group: &[Limb], lane: usize) -> FieldElement<Self> {
        let mut value: Limb = 0;
        for (j, plane) in group.iter().enumerate() {
            value |= ((plane >> lane) & 1) << j;
        }
        self.decode(value)
    }

    /// Write `value` into entry `lane` of a single group's `k` planes, dispersing the encoded
    /// value's bits one per plane. Assumes the stored value fits in `k` bits.
    fn scatter(self, group: &mut [Limb], lane: usize, value: FieldElement<Self>) {
        let encoded = self.encode(value);
        let lane_mask: Limb = 1 << lane;
        for (j, plane) in group.iter_mut().enumerate() {
            let bit = (encoded >> j) & 1;
            *plane = (*plane & !lane_mask) | (bit << lane);
        }
    }

    /// Whether this field uses a genuinely multi-plane layout (`k > 1`). Only `F_2` has `k = 1`,
    /// where the bit-sliced layout coincides with the packed one and the `F_2`-specific fast
    /// paths (`offset`, `limb_masks`, SIMD, m4ri) apply.
    fn is_bitsliced(self) -> bool {
        self.limbs_per_group() > 1
    }

    /// `dst += coeff * src` (mod p) over a span of whole groups (`dst` and `src` have equal,
    /// group-aligned length). Both are assumed reduced; the result is reduced.
    ///
    /// Default: element-wise over lanes via [`Self::gather`]/[`Self::scatter`] and the field's own
    /// arithmetic — correct for any bit-sliced field (used by [`SmallFq`](super::SmallFq)). The
    /// prime fields [`Fp`](super::Fp) override this with a branch-free plane circuit.
    fn add_groups(self, dst: &mut [Limb], src: &[Limb], coeff: FieldElement<Self>) {
        let lpg = self.limbs_per_group();
        let epg = self.entries_per_group();
        for (dgroup, sgroup) in dst.chunks_exact_mut(lpg).zip(src.chunks_exact(lpg)) {
            for lane in 0..epg {
                let a = self.gather(dgroup, lane);
                let b = self.gather(sgroup, lane);
                let result = self.add(a, self.mul(coeff.clone(), b));
                self.scatter(dgroup, lane, result);
            }
        }
    }

    /// `dst *= coeff` (mod p) over a span of whole groups. Default: element-wise; overridden by
    /// [`Fp`](super::Fp).
    fn scale_groups(self, dst: &mut [Limb], coeff: FieldElement<Self>) {
        let lpg = self.limbs_per_group();
        let epg = self.entries_per_group();
        for dgroup in dst.chunks_exact_mut(lpg) {
            for lane in 0..epg {
                let a = self.gather(dgroup, lane);
                self.scatter(dgroup, lane, self.mul(a, coeff.clone()));
            }
        }
    }

    /// `dst += coeff * src` (mod p) for a single group (each `limbs_per_group()` limbs),
    /// restricted to the lanes set in `lane_mask`; other lanes are unchanged. Used for the
    /// partial boundary groups of a slice add. Default: element-wise; overridden by
    /// [`Fp`](super::Fp) with a masked plane circuit.
    fn add_group_masked(
        self,
        dst: &mut [Limb],
        src: &[Limb],
        coeff: FieldElement<Self>,
        lane_mask: Limb,
    ) {
        for lane in 0..self.entries_per_group() {
            if (lane_mask >> lane) & 1 == 1 {
                let a = self.gather(dst, lane);
                let b = self.gather(src, lane);
                self.scatter(dst, lane, self.add(a, self.mul(coeff.clone(), b)));
            }
        }
    }

    /// Check whether or not a limb is reduced. This may potentially not be faster than calling
    /// [`reduce`](FieldInternal::reduce) directly.
    fn is_reduced(self, limb: Limb) -> bool {
        limb == self.reduce(limb)
    }

    /// Given an interator of `FieldElement<Self>`s, pack all of them into a single limb in order.
    /// It is assumed that the values of the iterator fit into a single limb. If this assumption is
    /// violated, the result will be nonsense.
    fn pack<T: Iterator<Item = FieldElement<Self>>>(self, entries: T) -> Limb {
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
            entries: self.entries_per_limb(),
            bit_length: self.bit_length(),
            bit_mask: self.bitmask(),
        }
    }

    /// Return the number of limbs required to hold `dim` entries.
    fn number(self, dim: usize) -> usize {
        // Whole groups needed to hold `dim` entries, times the limbs in each group. For the
        // packed layout (1 limb/group, `entries_per_limb` entries/group) this is `ceil(dim /
        // entries_per_limb)`, matching the previous definition.
        self.limbs_per_group() * dim.div_ceil(self.entries_per_group())
    }

    /// Return the `Range<usize>` of limbs spanning entries `start..end`: from the first limb of
    /// the group containing `start` to the last limb of the group containing `end - 1`.
    fn range(self, start: usize, end: usize) -> Range<usize> {
        let min = self.group_of(start) * self.limbs_per_group();
        let max = self.number(end);
        min..max
    }

    /// Return either `Some(sum)` if no carries happen in the limb, or `None` if some carry does happen.
    // TODO: maybe name this something clearer
    fn truncate(self, sum: Limb) -> Option<Limb> {
        if self.is_reduced(sum) {
            Some(sum)
        } else {
            None
        }
    }
}

pub(crate) struct LimbIterator<F> {
    fq: F,
    limb: Limb,
    entries: usize,
    bit_length: usize,
    bit_mask: Limb,
}

impl<F: FieldInternal> Iterator for LimbIterator<F> {
    type Item = FieldElement<F>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.entries == 0 {
            return None;
        }
        self.entries -= 1;
        let result = self.limb & self.bit_mask;
        self.limb >>= self.bit_length;
        Some(self.fq.decode(result))
    }
}
