use std::cmp::Ordering;

use itertools::Itertools;

use super::inner::{FqSlice, FqSliceMut, FqVector};
use crate::{
    constants,
    field::{Field, element::FieldElement},
    limb::Limb,
    prime::{Prime, ValidPrime},
};

impl<'a, F: Field> FqSliceMut<'a, F> {
    pub fn prime(&self) -> ValidPrime {
        self.fq().characteristic().to_dyn()
    }

    pub fn add_basis_element(&mut self, index: usize, value: FieldElement<F>) {
        assert_eq!(self.fq(), value.field());
        if self.fq().q() == 2 {
            let pair = self.fq().limb_bit_index_pair(index + self.start());
            self.limbs_mut()[pair.limb] ^= self.fq().encode(value) << pair.bit_index;
        } else {
            let mut x = self.as_slice().entry(index);
            x += value;
            self.set_entry(index, x);
        }
    }

    pub fn set_entry(&mut self, index: usize, value: FieldElement<F>) {
        assert_eq!(self.fq(), value.field());
        assert!(index < self.as_slice().len());
        let fq = self.fq();
        let idx = index + self.start();
        let lpg = fq.limbs_per_group();
        let base = fq.group_of(idx) * lpg;
        let lane = fq.lane_of(idx);
        fq.scatter(&mut self.limbs_mut()[base..base + lpg], lane, value);
    }

    fn reduce_limbs(&mut self) {
        let fq = self.fq();
        if fq.q() != 2 {
            let limb_range = self.as_slice().limb_range();

            for limb in self.limbs_mut()[limb_range].iter_mut() {
                *limb = fq.reduce(*limb);
            }
        }
    }

    pub fn scale(&mut self, c: FieldElement<F>) {
        assert_eq!(self.fq(), c.field());
        let fq = self.fq();

        if fq.q() == 2 {
            if c == fq.zero() {
                self.set_to_zero();
            }
            return;
        }

        if fq.is_bitsliced() {
            // The packed bit-offset masking does not apply to the bit-sliced layout; scale
            // each in-range entry through the layout-aware gather/scatter.
            if c == fq.zero() {
                self.set_to_zero();
                return;
            }
            for i in 0..self.as_slice().len() {
                let x = self.as_slice().entry(i) * c.clone();
                self.set_entry(i, x);
            }
            return;
        }

        let limb_range = self.as_slice().limb_range();
        if limb_range.is_empty() {
            return;
        }
        let (min_mask, max_mask) = self.as_slice().limb_masks();

        let limb = self.limbs()[limb_range.start];
        let masked_limb = limb & min_mask;
        let rest_limb = limb & !min_mask;
        self.limbs_mut()[limb_range.start] = fq.fma_limb(0, masked_limb, c.clone()) | rest_limb;

        let inner_range = self.as_slice().limb_range_inner();
        for limb in self.limbs_mut()[inner_range].iter_mut() {
            *limb = fq.fma_limb(0, *limb, c.clone());
        }
        if limb_range.len() > 1 {
            let full_limb = self.limbs()[limb_range.end - 1];
            let masked_limb = full_limb & max_mask;
            let rest_limb = full_limb & !max_mask;
            self.limbs_mut()[limb_range.end - 1] = fq.fma_limb(0, masked_limb, c) | rest_limb;
        }
        self.reduce_limbs();
    }

    pub fn set_to_zero(&mut self) {
        if self.fq().is_bitsliced() {
            let zero = self.fq().zero();
            for i in 0..self.as_slice().len() {
                self.set_entry(i, zero.clone());
            }
            return;
        }
        let limb_range = self.as_slice().limb_range();
        if limb_range.is_empty() {
            return;
        }
        let (min_mask, max_mask) = self.as_slice().limb_masks();
        self.limbs_mut()[limb_range.start] &= !min_mask;

        let inner_range = self.as_slice().limb_range_inner();
        for limb in self.limbs_mut()[inner_range].iter_mut() {
            *limb = 0;
        }
        self.limbs_mut()[limb_range.end - 1] &= !max_mask;
    }

    pub fn add(&mut self, other: FqSlice<'_, F>, c: FieldElement<F>) {
        assert_eq!(self.fq(), c.field());
        assert_eq!(self.fq(), other.fq());

        if self.as_slice().is_empty() {
            return;
        }

        if self.fq().q() == 2 {
            if c != self.fq().zero() {
                match self.as_slice().offset().cmp(&other.offset()) {
                    Ordering::Equal => self.add_shift_none(other, self.fq().one()),
                    Ordering::Less => self.add_shift_left(other, self.fq().one()),
                    Ordering::Greater => self.add_shift_right(other, self.fq().one()),
                };
            }
        } else if self.fq().is_bitsliced() {
            self.add_bitsliced(other, c);
        } else {
            match self.as_slice().offset().cmp(&other.offset()) {
                Ordering::Equal => self.add_shift_none(other, c),
                Ordering::Less => self.add_shift_left(other, c),
                Ordering::Greater => self.add_shift_right(other, c),
            };
        }
    }

    /// Add `c * other` to `self` in the bit-sliced layout. When both slices begin at a group
    /// boundary (lane 0 — the common case, e.g. whole vectors and matrix rows), the complete
    /// groups are added with the fast plane kernel ([`add_groups`](crate::field::field_internal));
    /// the fewer-than-64 trailing entries, and any non-group-aligned slice, fall back to
    /// entry-wise addition.
    ///
    /// [`add_groups`]: crate::field::field_internal::FieldInternal::add_groups
    fn add_bitsliced(&mut self, other: FqSlice<'_, F>, c: FieldElement<F>) {
        let fq = self.fq();
        if c == fq.zero() {
            return;
        }
        let epg = fq.entries_per_group();
        let s_start = self.start();
        let o_start = other.start();
        let len = self.as_slice().len();
        if len == 0 {
            return;
        }

        // The fast plane kernel needs the two slices to share a lane offset within their
        // groups (so group `g` of one lines up with group `g` of the other). This holds for
        // whole vectors and for matrix-row adds that start at the same pivot column. The
        // partial leading/trailing groups (and any mismatched-offset slice) are added
        // entry-wise.
        if s_start % epg != o_start % epg {
            // Mismatched lane offsets: the planes don't line up, so fall back to entry-wise.
            for (i, v) in other.iter_nonzero() {
                self.add_basis_element(i, v * c.clone());
            }
            return;
        }

        let k = fq.limbs_per_group();
        let s_end = s_start + len;
        let first_g = s_start / epg;
        let last_g = (s_end - 1) / epg;
        // Group `g` of `self` lines up with group `g - first_g + o_first_g` of `other`.
        let o_first_g = o_start / epg;
        let group_limbs = |self_g: usize| {
            let s = self_g * k;
            let o = (o_first_g + (self_g - first_g)) * k;
            (s, o)
        };
        // Lane mask selecting bits `[lo, hi)`.
        let lane_mask = |lo: usize, hi: usize| -> Limb {
            let high: Limb = if hi >= epg { !0 } else { (1 << hi) - 1 };
            let low: Limb = (1 << lo) - 1;
            high & !low
        };

        if first_g == last_g {
            // Single (partial) group.
            let (s, o) = group_limbs(first_g);
            let mask = lane_mask(s_start - first_g * epg, s_end - first_g * epg);
            let src = &other.limbs()[o..o + k];
            fq.add_group_masked(&mut self.limbs_mut()[s..s + k], src, c, mask);
            return;
        }

        // Leading partial group: lanes [s_start mod 64, 64).
        {
            let (s, o) = group_limbs(first_g);
            let mask = lane_mask(s_start - first_g * epg, epg);
            let src = &other.limbs()[o..o + k];
            fq.add_group_masked(&mut self.limbs_mut()[s..s + k], src, c.clone(), mask);
        }
        // Interior full groups, via the plane kernel in one contiguous call.
        if last_g > first_g + 1 {
            let (s, o) = group_limbs(first_g + 1);
            let nlimbs = (last_g - first_g - 1) * k;
            let src = &other.limbs()[o..o + nlimbs];
            fq.add_groups(&mut self.limbs_mut()[s..s + nlimbs], src, c.clone());
        }
        // Trailing partial group: lanes [0, s_end mod 64).
        {
            let (s, o) = group_limbs(last_g);
            let mask = lane_mask(0, s_end - last_g * epg);
            let src = &other.limbs()[o..o + k];
            fq.add_group_masked(&mut self.limbs_mut()[s..s + k], src, c, mask);
        }
    }

    pub fn add_offset(&mut self, other: FqSlice<'_, F>, c: FieldElement<F>, offset: usize) {
        self.slice_mut(offset, self.as_slice().len())
            .add(other.restrict(offset, other.len()), c)
    }

    /// Adds v otimes w to self.
    pub fn add_tensor(
        &mut self,
        offset: usize,
        coeff: FieldElement<F>,
        left: FqSlice<F>,
        right: FqSlice<F>,
    ) {
        assert_eq!(self.fq(), coeff.field());
        assert_eq!(self.fq(), left.fq());
        assert_eq!(self.fq(), right.fq());

        let right_dim = right.len();

        for (i, v) in left.iter_nonzero() {
            let entry = v * coeff.clone();
            self.slice_mut(offset + i * right_dim, offset + (i + 1) * right_dim)
                .add(right, entry);
        }
    }

    /// TODO: improve efficiency
    pub fn assign(&mut self, other: FqSlice<'_, F>) {
        assert_eq!(self.fq(), other.fq());
        if self.fq().is_bitsliced() || self.as_slice().offset() != other.offset() {
            self.set_to_zero();
            self.add(other, self.fq().one());
            return;
        }
        let target_range = self.as_slice().limb_range();
        let source_range = other.limb_range();

        if target_range.is_empty() {
            return;
        }

        let (min_mask, max_mask) = other.limb_masks();

        let result = other.limbs()[source_range.start] & min_mask;
        self.limbs_mut()[target_range.start] &= !min_mask;
        self.limbs_mut()[target_range.start] |= result;

        let target_inner_range = self.as_slice().limb_range_inner();
        let source_inner_range = other.limb_range_inner();
        if !target_inner_range.is_empty() && !source_inner_range.is_empty() {
            self.limbs_mut()[target_inner_range]
                .clone_from_slice(&other.limbs()[source_inner_range]);
        }

        let result = other.limbs()[source_range.end - 1] & max_mask;
        self.limbs_mut()[target_range.end - 1] &= !max_mask;
        self.limbs_mut()[target_range.end - 1] |= result;
    }

    /// Shifts the entries of `self` to the left by `shift` entries.
    pub fn shl_assign(&mut self, shift: usize) {
        if shift == 0 {
            return;
        }
        if self.fq().is_bitsliced() {
            // The packed limb-move trick assumes an entry is a contiguous bitfield, which the
            // bit-sliced layout breaks. Move entries down one at a time via gather/scatter:
            // reading `i + shift` strictly ahead of writing `i` keeps it correct in place.
            let new_len = self.as_slice().len() - shift;
            for i in 0..new_len {
                let v = self.as_slice().entry(i + shift);
                self.set_entry(i, v);
            }
            *self.end_mut() -= shift;
            return;
        }
        if self.start() == 0 && shift.is_multiple_of(self.fq().entries_per_limb()) {
            let limb_shift = shift / self.fq().entries_per_limb();
            *self.end_mut() -= shift;
            let new_num_limbs = self.fq().number(self.end());
            for idx in 0..new_num_limbs {
                self.limbs_mut()[idx] = self.limbs()[idx + limb_shift];
            }
        } else {
            unimplemented!()
        }
    }

    /// Adds `c` * `other` to `self`. `other` must have the same length, offset, and prime as self.
    pub fn add_shift_none(&mut self, other: FqSlice<'_, F>, c: FieldElement<F>) {
        assert_eq!(self.fq(), c.field());
        assert_eq!(self.fq(), other.fq());
        let fq = self.fq();

        let target_range = self.as_slice().limb_range();
        let source_range = other.limb_range();

        let (min_mask, max_mask) = other.limb_masks();

        self.limbs_mut()[target_range.start] = fq.fma_limb(
            self.limbs()[target_range.start],
            other.limbs()[source_range.start] & min_mask,
            c.clone(),
        );
        self.limbs_mut()[target_range.start] = fq.reduce(self.limbs()[target_range.start]);

        let target_inner_range = self.as_slice().limb_range_inner();
        let source_inner_range = other.limb_range_inner();
        if !source_inner_range.is_empty() {
            for (left, right) in self.limbs_mut()[target_inner_range]
                .iter_mut()
                .zip_eq(&other.limbs()[source_inner_range])
            {
                *left = fq.fma_limb(*left, *right, c.clone());
                *left = fq.reduce(*left);
            }
        }
        if source_range.len() > 1 {
            // The first and last limbs are distinct, so we process the last.
            self.limbs_mut()[target_range.end - 1] = fq.fma_limb(
                self.limbs()[target_range.end - 1],
                other.limbs()[source_range.end - 1] & max_mask,
                c,
            );
            self.limbs_mut()[target_range.end - 1] = fq.reduce(self.limbs()[target_range.end - 1]);
        }
    }

    fn add_shift_left(&mut self, other: FqSlice<'_, F>, c: FieldElement<F>) {
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
            fn new<F: Field>(fq: F, target: FqSlice<'_, F>, source: FqSlice<'_, F>) -> Self {
                debug_assert!(target.prime() == source.prime());
                debug_assert!(target.offset() <= source.offset());
                debug_assert!(
                    target.len() == source.len(),
                    "self.dim {} not equal to other.dim {}",
                    target.len(),
                    source.len()
                );
                let offset_shift = source.offset() - target.offset();
                let bit_length = fq.bit_length();
                let entries_per_limb = fq.entries_per_limb();
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

            fn mask_first_limb<F: Field>(&self, other: FqSlice<'_, F>, i: usize) -> Limb {
                (other.limbs()[i] & self.min_mask) >> self.offset_shift
            }

            fn mask_middle_limb_a<F: Field>(&self, other: FqSlice<'_, F>, i: usize) -> Limb {
                other.limbs()[i] >> self.offset_shift
            }

            fn mask_middle_limb_b<F: Field>(&self, other: FqSlice<'_, F>, i: usize) -> Limb {
                (other.limbs()[i] << (self.tail_shift + self.zero_bits)) >> self.zero_bits
            }

            fn mask_last_limb_a<F: Field>(&self, other: FqSlice<'_, F>, i: usize) -> Limb {
                let source_limb_masked = other.limbs()[i] & self.max_mask;
                source_limb_masked << self.tail_shift
            }

            fn mask_last_limb_b<F: Field>(&self, other: FqSlice<'_, F>, i: usize) -> Limb {
                let source_limb_masked = other.limbs()[i] & self.max_mask;
                source_limb_masked >> self.offset_shift
            }
        }

        let dat = AddShiftLeftData::new(self.fq(), self.as_slice(), other);
        let mut i = 0;
        {
            self.limbs_mut()[i + dat.min_target_limb] = self.fq().fma_limb(
                self.limbs()[i + dat.min_target_limb],
                dat.mask_first_limb(other, i + dat.min_source_limb),
                c.clone(),
            );
        }
        for i in 1..dat.number_of_source_limbs - 1 {
            self.limbs_mut()[i + dat.min_target_limb] = self.fq().fma_limb(
                self.limbs()[i + dat.min_target_limb],
                dat.mask_middle_limb_a(other, i + dat.min_source_limb),
                c.clone(),
            );
            self.limbs_mut()[i + dat.min_target_limb - 1] = self.fq().fma_limb(
                self.limbs()[i + dat.min_target_limb - 1],
                dat.mask_middle_limb_b(other, i + dat.min_source_limb),
                c.clone(),
            );
            self.limbs_mut()[i + dat.min_target_limb - 1] =
                self.fq().reduce(self.limbs()[i + dat.min_target_limb - 1]);
        }
        i = dat.number_of_source_limbs - 1;
        if i > 0 {
            self.limbs_mut()[i + dat.min_target_limb - 1] = self.fq().fma_limb(
                self.limbs()[i + dat.min_target_limb - 1],
                dat.mask_last_limb_a(other, i + dat.min_source_limb),
                c.clone(),
            );
            self.limbs_mut()[i + dat.min_target_limb - 1] =
                self.fq().reduce(self.limbs()[i + dat.min_target_limb - 1]);
            if dat.number_of_source_limbs == dat.number_of_target_limbs {
                self.limbs_mut()[i + dat.min_target_limb] = self.fq().fma_limb(
                    self.limbs()[i + dat.min_target_limb],
                    dat.mask_last_limb_b(other, i + dat.min_source_limb),
                    c,
                );
                self.limbs_mut()[i + dat.min_target_limb] =
                    self.fq().reduce(self.limbs()[i + dat.min_target_limb]);
            }
        } else {
            self.limbs_mut()[i + dat.min_target_limb] =
                self.fq().reduce(self.limbs()[i + dat.min_target_limb]);
        }
    }

    fn add_shift_right(&mut self, other: FqSlice<'_, F>, c: FieldElement<F>) {
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
            fn new<F: Field>(fq: F, target: FqSlice<'_, F>, source: FqSlice<'_, F>) -> Self {
                debug_assert!(target.prime() == source.prime());
                debug_assert!(target.offset() >= source.offset());
                debug_assert!(
                    target.len() == source.len(),
                    "self.dim {} not equal to other.dim {}",
                    target.len(),
                    source.len()
                );
                let offset_shift = target.offset() - source.offset();
                let bit_length = fq.bit_length();
                let entries_per_limb = fq.entries_per_limb();
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

            fn mask_first_limb_a<F: Field>(&self, other: FqSlice<'_, F>, i: usize) -> Limb {
                let source_limb_masked = other.limbs()[i] & self.min_mask;
                (source_limb_masked << (self.offset_shift + self.zero_bits)) >> self.zero_bits
            }

            fn mask_first_limb_b<F: Field>(&self, other: FqSlice<'_, F>, i: usize) -> Limb {
                let source_limb_masked = other.limbs()[i] & self.min_mask;
                source_limb_masked >> self.tail_shift
            }

            fn mask_middle_limb_a<F: Field>(&self, other: FqSlice<'_, F>, i: usize) -> Limb {
                (other.limbs()[i] << (self.offset_shift + self.zero_bits)) >> self.zero_bits
            }

            fn mask_middle_limb_b<F: Field>(&self, other: FqSlice<'_, F>, i: usize) -> Limb {
                other.limbs()[i] >> self.tail_shift
            }

            fn mask_last_limb_a<F: Field>(&self, other: FqSlice<'_, F>, i: usize) -> Limb {
                let source_limb_masked = other.limbs()[i] & self.max_mask;
                source_limb_masked << self.offset_shift
            }

            fn mask_last_limb_b<F: Field>(&self, other: FqSlice<'_, F>, i: usize) -> Limb {
                let source_limb_masked = other.limbs()[i] & self.max_mask;
                source_limb_masked >> self.tail_shift
            }
        }

        let dat = AddShiftRightData::new(self.fq(), self.as_slice(), other);
        let mut i = 0;
        {
            self.limbs_mut()[i + dat.min_target_limb] = self.fq().fma_limb(
                self.limbs()[i + dat.min_target_limb],
                dat.mask_first_limb_a(other, i + dat.min_source_limb),
                c.clone(),
            );
            self.limbs_mut()[i + dat.min_target_limb] =
                self.fq().reduce(self.limbs()[i + dat.min_target_limb]);
            if dat.number_of_target_limbs > 1 {
                self.limbs_mut()[i + dat.min_target_limb + 1] = self.fq().fma_limb(
                    self.limbs()[i + dat.min_target_limb + 1],
                    dat.mask_first_limb_b(other, i + dat.min_source_limb),
                    c.clone(),
                );
            }
        }
        for i in 1..dat.number_of_source_limbs - 1 {
            self.limbs_mut()[i + dat.min_target_limb] = self.fq().fma_limb(
                self.limbs()[i + dat.min_target_limb],
                dat.mask_middle_limb_a(other, i + dat.min_source_limb),
                c.clone(),
            );
            self.limbs_mut()[i + dat.min_target_limb] =
                self.fq().reduce(self.limbs()[i + dat.min_target_limb]);
            self.limbs_mut()[i + dat.min_target_limb + 1] = self.fq().fma_limb(
                self.limbs()[i + dat.min_target_limb + 1],
                dat.mask_middle_limb_b(other, i + dat.min_source_limb),
                c.clone(),
            );
        }
        i = dat.number_of_source_limbs - 1;
        if i > 0 {
            self.limbs_mut()[i + dat.min_target_limb] = self.fq().fma_limb(
                self.limbs()[i + dat.min_target_limb],
                dat.mask_last_limb_a(other, i + dat.min_source_limb),
                c.clone(),
            );
            self.limbs_mut()[i + dat.min_target_limb] =
                self.fq().reduce(self.limbs()[i + dat.min_target_limb]);
            if dat.number_of_target_limbs > dat.number_of_source_limbs {
                self.limbs_mut()[i + dat.min_target_limb + 1] = self.fq().fma_limb(
                    self.limbs()[i + dat.min_target_limb + 1],
                    dat.mask_last_limb_b(other, i + dat.min_source_limb),
                    c.clone(),
                );
            }
        }
        if dat.number_of_target_limbs > dat.number_of_source_limbs {
            self.limbs_mut()[i + dat.min_target_limb + 1] =
                self.fq().reduce(self.limbs()[i + dat.min_target_limb + 1]);
        }
    }

    /// Given a mask v, add the `v[i]`th entry of `other` to the `i`th entry of `self`.
    pub fn add_masked(&mut self, other: FqSlice<'_, F>, c: FieldElement<F>, mask: &[usize]) {
        // TODO: If this ends up being a bottleneck, try to use PDEP/PEXT
        assert_eq!(self.fq(), c.field());
        assert_eq!(self.fq(), other.fq());
        assert_eq!(self.as_slice().len(), mask.len());
        for (i, &x) in mask.iter().enumerate() {
            let entry = other.entry(x);
            if entry != self.fq().zero() {
                self.add_basis_element(i, entry * c.clone());
            }
        }
    }

    /// Given a mask v, add the `i`th entry of `other` to the `v[i]`th entry of `self`.
    pub fn add_unmasked(&mut self, other: FqSlice<'_, F>, c: FieldElement<F>, mask: &[usize]) {
        assert_eq!(self.fq(), c.field());
        assert_eq!(self.fq(), other.fq());
        assert!(other.len() <= mask.len());
        for (i, v) in other.iter_nonzero() {
            self.add_basis_element(mask[i], v * c.clone());
        }
    }

    pub fn slice_mut(&mut self, start: usize, end: usize) -> FqSliceMut<'_, F> {
        assert!(start <= end && end <= self.as_slice().len());
        let orig_start = self.start();

        FqSliceMut::new(
            self.fq(),
            self.limbs_mut(),
            orig_start + start,
            orig_start + end,
        )
    }

    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> FqSlice<'_, F> {
        FqSlice::new(self.fq(), self.limbs(), self.start(), self.end())
    }

    /// Generates a version of itself with a shorter lifetime
    #[inline]
    #[must_use]
    pub fn copy(&mut self) -> FqSliceMut<'_, F> {
        let start = self.start();
        let end = self.end();

        FqSliceMut::new(self.fq(), self.limbs_mut(), start, end)
    }
}

impl<'a, F: Field> From<&'a mut FqVector<F>> for FqSliceMut<'a, F> {
    fn from(v: &'a mut FqVector<F>) -> Self {
        v.slice_mut(0, v.len())
    }
}
