use std::cmp::Ordering;

use itertools::Itertools;

use crate::{
    constants,
    limb::{self, Limb},
    prime::{Prime, ValidPrime, P2},
};

use super::inner::{FpVectorP, SliceMutP, SliceP};

impl<'a, P: Prime> SliceMutP<'a, P> {
    pub fn prime(&self) -> ValidPrime {
        self.p.to_dyn()
    }

    pub fn add_basis_element(&mut self, index: usize, value: u32) {
        if self.p == 2 {
            // Checking for value % 2 == 0 appears to be less performant
            let pair = limb::limb_bit_index_pair(P2, index + self.start);
            self.limbs[pair.limb] ^= (value as Limb % 2) << pair.bit_index;
        } else {
            let mut x = self.as_slice().entry(index);
            x += value;
            x %= self.p.as_u32();
            self.set_entry(index, x);
        }
    }

    pub fn set_entry(&mut self, index: usize, value: u32) {
        debug_assert!(index < self.as_slice().len());
        let bit_mask = limb::bitmask(self.p);
        let limb_index = limb::limb_bit_index_pair(self.p, index + self.start);
        let mut result = self.limbs[limb_index.limb];
        result &= !(bit_mask << limb_index.bit_index);
        result |= (value as Limb) << limb_index.bit_index;
        self.limbs[limb_index.limb] = result;
    }

    fn reduce_limbs(&mut self) {
        if self.p != 2 {
            let limb_range = self.as_slice().limb_range();

            for limb in &mut self.limbs[limb_range] {
                *limb = limb::reduce(self.p, *limb);
            }
        }
    }

    pub fn scale(&mut self, c: u32) {
        if self.p == 2 {
            if c == 0 {
                self.set_to_zero();
            }
            return;
        }

        let c = c as Limb;
        let limb_range = self.as_slice().limb_range();
        if limb_range.is_empty() {
            return;
        }
        let (min_mask, max_mask) = self.as_slice().limb_masks();

        let limb = self.limbs[limb_range.start];
        let masked_limb = limb & min_mask;
        let rest_limb = limb & !min_mask;
        self.limbs[limb_range.start] = (masked_limb * c) | rest_limb;

        let inner_range = self.as_slice().limb_range_inner();
        for limb in &mut self.limbs[inner_range] {
            *limb *= c;
        }
        if limb_range.len() > 1 {
            let full_limb = self.limbs[limb_range.end - 1];
            let masked_limb = full_limb & max_mask;
            let rest_limb = full_limb & !max_mask;
            self.limbs[limb_range.end - 1] = (masked_limb * c) | rest_limb;
        }
        self.reduce_limbs();
    }

    pub fn set_to_zero(&mut self) {
        let limb_range = self.as_slice().limb_range();
        if limb_range.is_empty() {
            return;
        }
        let (min_mask, max_mask) = self.as_slice().limb_masks();
        self.limbs[limb_range.start] &= !min_mask;

        let inner_range = self.as_slice().limb_range_inner();
        for limb in &mut self.limbs[inner_range] {
            *limb = 0;
        }
        self.limbs[limb_range.end - 1] &= !max_mask;
    }

    pub fn add(&mut self, other: SliceP<'_, P>, c: u32) {
        debug_assert!(self.p > c);
        if self.as_slice().is_empty() {
            return;
        }

        if self.p == 2 {
            if c != 0 {
                match self.as_slice().offset().cmp(&other.offset()) {
                    Ordering::Equal => self.add_shift_none(other, 1),
                    Ordering::Less => self.add_shift_left(other, 1),
                    Ordering::Greater => self.add_shift_right(other, 1),
                };
            }
        } else {
            match self.as_slice().offset().cmp(&other.offset()) {
                Ordering::Equal => self.add_shift_none(other, c),
                Ordering::Less => self.add_shift_left(other, c),
                Ordering::Greater => self.add_shift_right(other, c),
            };
        }
    }

    /// `coeff` need not be reduced mod p.
    /// Adds v otimes w to self.
    pub fn add_tensor(&mut self, offset: usize, coeff: u32, left: SliceP<P>, right: SliceP<P>) {
        let right_dim = right.len();

        for (i, v) in left.iter_nonzero() {
            let entry = (v * coeff) % self.prime();
            self.slice_mut(offset + i * right_dim, offset + (i + 1) * right_dim)
                .add(right, entry);
        }
    }

    /// TODO: improve efficiency
    pub fn assign(&mut self, other: SliceP<'_, P>) {
        if self.as_slice().offset() != other.offset() {
            self.set_to_zero();
            self.add(other, 1);
            return;
        }
        let target_range = self.as_slice().limb_range();
        let source_range = other.limb_range();

        if target_range.is_empty() {
            return;
        }

        let (min_mask, max_mask) = other.limb_masks();

        let result = other.limbs[source_range.start] & min_mask;
        self.limbs[target_range.start] &= !min_mask;
        self.limbs[target_range.start] |= result;

        let target_inner_range = self.as_slice().limb_range_inner();
        let source_inner_range = other.limb_range_inner();
        if !target_inner_range.is_empty() && !source_inner_range.is_empty() {
            self.limbs[target_inner_range].clone_from_slice(&other.limbs[source_inner_range]);
        }

        let result = other.limbs[source_range.end - 1] & max_mask;
        self.limbs[target_range.end - 1] &= !max_mask;
        self.limbs[target_range.end - 1] |= result;
    }

    /// Adds `c` * `other` to `self`. `other` must have the same length, offset, and prime as self, and `c` must be between `0` and `p - 1`.
    pub fn add_shift_none(&mut self, other: SliceP<'_, P>, c: u32) {
        let target_range = self.as_slice().limb_range();
        let source_range = other.limb_range();

        let (min_mask, max_mask) = other.limb_masks();

        self.limbs[target_range.start] = limb::add(
            self.p,
            self.limbs[target_range.start],
            other.limbs[source_range.start] & min_mask,
            c,
        );
        self.limbs[target_range.start] = limb::reduce(self.p, self.limbs[target_range.start]);

        let target_inner_range = self.as_slice().limb_range_inner();
        let source_inner_range = other.limb_range_inner();
        if !source_inner_range.is_empty() {
            for (left, right) in self.limbs[target_inner_range]
                .iter_mut()
                .zip_eq(&other.limbs[source_inner_range])
            {
                *left = limb::add(self.p, *left, *right, c);
                *left = limb::reduce(self.p, *left);
            }
        }
        if source_range.len() > 1 {
            // The first and last limbs are distinct, so we process the last.
            self.limbs[target_range.end - 1] = limb::add(
                self.p,
                self.limbs[target_range.end - 1],
                other.limbs[source_range.end - 1] & max_mask,
                c,
            );
            self.limbs[target_range.end - 1] =
                limb::reduce(self.p, self.limbs[target_range.end - 1]);
        }
    }

    fn add_shift_left(&mut self, other: SliceP<'_, P>, c: u32) {
        struct AddShiftLeftData {
            offset_shift: usize,
            tail_shift: usize,
            zero_bits: usize,
            min_source_limb: usize,
            min_target_limb: usize,
            number_of_source_limbs: usize,
            number_of_target_limbs: usize,
            min_mask: Limb,
            max_mask: Limb,
        }

        impl AddShiftLeftData {
            fn new<P: Prime>(p: P, target: SliceP<'_, P>, source: SliceP<'_, P>) -> Self {
                debug_assert!(target.prime() == source.prime());
                debug_assert!(target.offset() <= source.offset());
                debug_assert!(
                    target.len() == source.len(),
                    "self.dim {} not equal to other.dim {}",
                    target.len(),
                    source.len()
                );
                let offset_shift = source.offset() - target.offset();
                let bit_length = limb::bit_length(p);
                let entries_per_limb = limb::entries_per_limb(p);
                let usable_bits_per_limb = bit_length * entries_per_limb;
                let tail_shift = usable_bits_per_limb - offset_shift;
                let zero_bits = constants::BITS_PER_LIMB - usable_bits_per_limb;
                let source_range = source.limb_range();
                let target_range = target.limb_range();
                let min_source_limb = source_range.start;
                let min_target_limb = target_range.start;
                let number_of_source_limbs = source_range.len();
                let number_of_target_limbs = target_range.len();
                let (min_mask, max_mask) = source.limb_masks();

                Self {
                    offset_shift,
                    tail_shift,
                    zero_bits,
                    min_source_limb,
                    min_target_limb,
                    number_of_source_limbs,
                    number_of_target_limbs,
                    min_mask,
                    max_mask,
                }
            }

            fn mask_first_limb<P: Prime>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
                (other.limbs[i] & self.min_mask) >> self.offset_shift
            }

            fn mask_middle_limb_a<P: Prime>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
                other.limbs[i] >> self.offset_shift
            }

            fn mask_middle_limb_b<P: Prime>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
                (other.limbs[i] << (self.tail_shift + self.zero_bits)) >> self.zero_bits
            }

            fn mask_last_limb_a<P: Prime>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
                let source_limb_masked = other.limbs[i] & self.max_mask;
                source_limb_masked << self.tail_shift
            }

            fn mask_last_limb_b<P: Prime>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
                let source_limb_masked = other.limbs[i] & self.max_mask;
                source_limb_masked >> self.offset_shift
            }
        }

        let dat = AddShiftLeftData::new(self.p, self.as_slice(), other);
        let mut i = 0;
        {
            self.limbs[i + dat.min_target_limb] = limb::add(
                self.p,
                self.limbs[i + dat.min_target_limb],
                dat.mask_first_limb(other, i + dat.min_source_limb),
                c,
            );
        }
        for i in 1..dat.number_of_source_limbs - 1 {
            self.limbs[i + dat.min_target_limb] = limb::add(
                self.p,
                self.limbs[i + dat.min_target_limb],
                dat.mask_middle_limb_a(other, i + dat.min_source_limb),
                c,
            );
            self.limbs[i + dat.min_target_limb - 1] = limb::add(
                self.p,
                self.limbs[i + dat.min_target_limb - 1],
                dat.mask_middle_limb_b(other, i + dat.min_source_limb),
                c,
            );
            self.limbs[i + dat.min_target_limb - 1] =
                limb::reduce(self.p, self.limbs[i + dat.min_target_limb - 1]);
        }
        i = dat.number_of_source_limbs - 1;
        if i > 0 {
            self.limbs[i + dat.min_target_limb - 1] = limb::add(
                self.p,
                self.limbs[i + dat.min_target_limb - 1],
                dat.mask_last_limb_a(other, i + dat.min_source_limb),
                c,
            );
            self.limbs[i + dat.min_target_limb - 1] =
                limb::reduce(self.p, self.limbs[i + dat.min_target_limb - 1]);
            if dat.number_of_source_limbs == dat.number_of_target_limbs {
                self.limbs[i + dat.min_target_limb] = limb::add(
                    self.p,
                    self.limbs[i + dat.min_target_limb],
                    dat.mask_last_limb_b(other, i + dat.min_source_limb),
                    c,
                );
                self.limbs[i + dat.min_target_limb] =
                    limb::reduce(self.p, self.limbs[i + dat.min_target_limb]);
            }
        } else {
            self.limbs[i + dat.min_target_limb] =
                limb::reduce(self.p, self.limbs[i + dat.min_target_limb]);
        }
    }

    fn add_shift_right(&mut self, other: SliceP<'_, P>, c: u32) {
        struct AddShiftRightData {
            offset_shift: usize,
            tail_shift: usize,
            zero_bits: usize,
            min_source_limb: usize,
            min_target_limb: usize,
            number_of_source_limbs: usize,
            number_of_target_limbs: usize,
            min_mask: Limb,
            max_mask: Limb,
        }

        impl AddShiftRightData {
            fn new<P: Prime>(p: P, target: SliceP<'_, P>, source: SliceP<'_, P>) -> Self {
                debug_assert!(target.prime() == source.prime());
                debug_assert!(target.offset() >= source.offset());
                debug_assert!(
                    target.len() == source.len(),
                    "self.dim {} not equal to other.dim {}",
                    target.len(),
                    source.len()
                );
                let offset_shift = target.offset() - source.offset();
                let bit_length = limb::bit_length(p);
                let entries_per_limb = limb::entries_per_limb(p);
                let usable_bits_per_limb = bit_length * entries_per_limb;
                let tail_shift = usable_bits_per_limb - offset_shift;
                let zero_bits = constants::BITS_PER_LIMB - usable_bits_per_limb;
                let source_range = source.limb_range();
                let target_range = target.limb_range();
                let min_source_limb = source_range.start;
                let min_target_limb = target_range.start;
                let number_of_source_limbs = source_range.len();
                let number_of_target_limbs = target_range.len();
                let (min_mask, max_mask) = source.limb_masks();
                Self {
                    offset_shift,
                    tail_shift,
                    zero_bits,
                    min_source_limb,
                    min_target_limb,
                    number_of_source_limbs,
                    number_of_target_limbs,
                    min_mask,
                    max_mask,
                }
            }

            fn mask_first_limb_a<P: Prime>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
                let source_limb_masked = other.limbs[i] & self.min_mask;
                (source_limb_masked << (self.offset_shift + self.zero_bits)) >> self.zero_bits
            }

            fn mask_first_limb_b<P: Prime>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
                let source_limb_masked = other.limbs[i] & self.min_mask;
                source_limb_masked >> self.tail_shift
            }

            fn mask_middle_limb_a<P: Prime>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
                (other.limbs[i] << (self.offset_shift + self.zero_bits)) >> self.zero_bits
            }

            fn mask_middle_limb_b<P: Prime>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
                other.limbs[i] >> self.tail_shift
            }

            fn mask_last_limb_a<P: Prime>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
                let source_limb_masked = other.limbs[i] & self.max_mask;
                source_limb_masked << self.offset_shift
            }

            fn mask_last_limb_b<P: Prime>(&self, other: SliceP<'_, P>, i: usize) -> Limb {
                let source_limb_masked = other.limbs[i] & self.max_mask;
                source_limb_masked >> self.tail_shift
            }
        }

        let dat = AddShiftRightData::new(self.p, self.as_slice(), other);
        let mut i = 0;
        {
            self.limbs[i + dat.min_target_limb] = limb::add(
                self.p,
                self.limbs[i + dat.min_target_limb],
                dat.mask_first_limb_a(other, i + dat.min_source_limb),
                c,
            );
            self.limbs[i + dat.min_target_limb] =
                limb::reduce(self.p, self.limbs[i + dat.min_target_limb]);
            if dat.number_of_target_limbs > 1 {
                self.limbs[i + dat.min_target_limb + 1] = limb::add(
                    self.p,
                    self.limbs[i + dat.min_target_limb + 1],
                    dat.mask_first_limb_b(other, i + dat.min_source_limb),
                    c,
                );
            }
        }
        for i in 1..dat.number_of_source_limbs - 1 {
            self.limbs[i + dat.min_target_limb] = limb::add(
                self.p,
                self.limbs[i + dat.min_target_limb],
                dat.mask_middle_limb_a(other, i + dat.min_source_limb),
                c,
            );
            self.limbs[i + dat.min_target_limb] =
                limb::reduce(self.p, self.limbs[i + dat.min_target_limb]);
            self.limbs[i + dat.min_target_limb + 1] = limb::add(
                self.p,
                self.limbs[i + dat.min_target_limb + 1],
                dat.mask_middle_limb_b(other, i + dat.min_source_limb),
                c,
            );
        }
        i = dat.number_of_source_limbs - 1;
        if i > 0 {
            self.limbs[i + dat.min_target_limb] = limb::add(
                self.p,
                self.limbs[i + dat.min_target_limb],
                dat.mask_last_limb_a(other, i + dat.min_source_limb),
                c,
            );
            self.limbs[i + dat.min_target_limb] =
                limb::reduce(self.p, self.limbs[i + dat.min_target_limb]);
            if dat.number_of_target_limbs > dat.number_of_source_limbs {
                self.limbs[i + dat.min_target_limb + 1] = limb::add(
                    self.p,
                    self.limbs[i + dat.min_target_limb + 1],
                    dat.mask_last_limb_b(other, i + dat.min_source_limb),
                    c,
                );
            }
        }
        if dat.number_of_target_limbs > dat.number_of_source_limbs {
            self.limbs[i + dat.min_target_limb + 1] =
                limb::reduce(self.p, self.limbs[i + dat.min_target_limb + 1]);
        }
    }

    /// Given a mask v, add the `v[i]`th entry of `other` to the `i`th entry of `self`.
    pub fn add_masked(&mut self, other: SliceP<'_, P>, c: u32, mask: &[usize]) {
        // TODO: If this ends up being a bottleneck, try to use PDEP/PEXT
        assert_eq!(self.as_slice().len(), mask.len());
        for (i, &x) in mask.iter().enumerate() {
            let entry = other.entry(x);
            if entry != 0 {
                self.add_basis_element(i, entry * c);
            }
        }
    }

    /// Given a mask v, add the `i`th entry of `other` to the `v[i]`th entry of `self`.
    pub fn add_unmasked(&mut self, other: SliceP<'_, P>, c: u32, mask: &[usize]) {
        assert!(other.len() <= mask.len());
        for (i, v) in other.iter_nonzero() {
            self.add_basis_element(mask[i], v * c);
        }
    }

    pub fn slice_mut(&mut self, start: usize, end: usize) -> SliceMutP<'_, P> {
        assert!(start <= end && end <= self.as_slice().len());

        SliceMutP {
            p: self.p,
            limbs: &mut *self.limbs,
            start: self.start + start,
            end: self.start + end,
        }
    }

    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> SliceP<'_, P> {
        SliceP {
            p: self.p,
            limbs: &*self.limbs,
            start: self.start,
            end: self.end,
        }
    }

    /// Generates a version of itself with a shorter lifetime
    #[inline]
    #[must_use]
    pub fn copy(&mut self) -> SliceMutP<'_, P> {
        SliceMutP {
            p: self.p,
            limbs: self.limbs,
            start: self.start,
            end: self.end,
        }
    }
}

impl<'a, P: Prime> From<&'a mut FpVectorP<P>> for SliceMutP<'a, P> {
    fn from(v: &'a mut FpVectorP<P>) -> Self {
        v.slice_mut(0, v.len)
    }
}
