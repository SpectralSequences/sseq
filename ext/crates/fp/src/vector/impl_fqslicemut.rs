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
        assert_eq!(self.as_slice().len(), other.len());

        if self.as_slice().is_empty() {
            return;
        }

        // Every field uses the bit-sliced layout (`F_2` is just the `k = 1` case), so a single
        // code path handles them all.
        self.add_bitsliced(other, c);
    }

    /// Add `c * other` to `self` in the bit-sliced layout. Interior full groups are added with
    /// the fast plane kernel ([`add_groups`](crate::field::field_internal)); the leading/trailing
    /// partial groups go through the masked plane circuit
    /// ([`add_group_masked`](crate::field::field_internal::FieldInternal::add_group_masked)). When
    /// the two slices have different lane offsets within their groups, the planes are realigned
    /// first via [`add_bitsliced_shifted`](Self::add_bitsliced_shifted).
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

        // The fast plane kernel needs the two slices to share a lane offset within their groups
        // (so group `g` of one lines up with group `g` of the other). This holds for whole
        // vectors and for matrix-row adds that start at the same pivot column. When the offsets
        // differ, the planes must be realigned first — see `add_bitsliced_shifted`.
        if s_start % epg != o_start % epg {
            self.add_bitsliced_shifted(other, c);
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

    /// Add `c * other` to `self` when the two slices have different lane offsets within their
    /// groups, so the planes don't line up. A bit-sliced vector is `k` independent single-bit
    /// planes; realigning it is a per-plane bit shift (this is exactly what the old F_2-only
    /// `add_shift_*` did, generalized from one plane to `k`). For each target group we build the
    /// `k` source planes shifted into alignment, then add them in with the masked group circuit.
    fn add_bitsliced_shifted(&mut self, other: FqSlice<'_, F>, c: FieldElement<F>) {
        let fq = self.fq();
        let k = fq.limbs_per_group();
        let epg = fq.entries_per_group();
        let ts = self.start();
        let len = self.as_slice().len();
        // Source lane = target lane + shift (`other`'s entry i sits `shift` lanes from `self`'s).
        let shift = other.start() as isize - ts as isize;
        let src_limbs = other.limbs();

        // Plane `j` of (absolute) group `g` of the source, or 0 if out of range. The whole-limb
        // reads can stray outside the valid lane range, but those bits are masked off below.
        let plane_limb = |g: isize, j: usize| -> Limb {
            if g < 0 {
                return 0;
            }
            let idx = g as usize * k + j;
            if idx < src_limbs.len() {
                src_limbs[idx]
            } else {
                0
            }
        };
        let lane_mask = |lo: usize, hi: usize| -> Limb {
            let high: Limb = if hi >= epg { !0 } else { (1 << hi) - 1 };
            let low: Limb = (1 << lo) - 1;
            high & !low
        };

        debug_assert!(k <= epg);
        let mut shifted = [0 as Limb; constants::BITS_PER_LIMB];

        let first_g = ts / epg;
        let last_g = (ts + len - 1) / epg;
        let epg_i = epg as isize;
        for g in first_g..=last_g {
            let lo = if g == first_g { ts - g * epg } else { 0 };
            let hi = if g == last_g {
                (ts + len) - g * epg
            } else {
                epg
            };
            let mask = lane_mask(lo, hi);

            // Source bit `b` of target group `g` lives at absolute source lane `g*epg + b + shift`.
            let src_base = g as isize * epg_i + shift;
            let sg = src_base.div_euclid(epg_i);
            let bs = src_base.rem_euclid(epg_i) as u32;
            for (j, s) in shifted[..k].iter_mut().enumerate() {
                *s = if bs == 0 {
                    plane_limb(sg, j)
                } else {
                    (plane_limb(sg, j) >> bs) | (plane_limb(sg + 1, j) << (epg as u32 - bs))
                };
            }
            fq.add_group_masked(
                &mut self.limbs_mut()[g * k..g * k + k],
                &shifted[..k],
                c.clone(),
                mask,
            );
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
            let len = self.as_slice().len();
            if shift >= len {
                *self.end_mut() = self.start();
                return;
            }
            let new_len = len - shift;
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
