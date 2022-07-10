use std::cmp::Ordering;

use super::generic::{FpVectorIterator, FpVectorNonZeroIteratorP, FpVectorP, SliceMutP, SliceP};
use crate::{
    constants,
    limb::{self, Limb, LimbLength},
    prime::ValidPrime,
};

mod impl_internal;

pub trait InternalBaseVectorP<const P: u32>: Sized {
    /// Returns a pointer to the allocation containing the actual data. This is a raw pointer and
    /// does not take lifetimes into account. It is the responsibility of the caller to ensure that
    /// the pointer is not dereferenced after the allocation is freed.
    ///
    /// We use a pointer instead of a slice because otherwise handling the lifetimes is a huge mess.
    /// In practice it is almost always better to use [`InternalBaseVectorP::_limbs`] to manipulate
    /// the underlying data.
    fn _as_ptr(&self) -> *const Limb;

    /// Returns a description of the vector as a [`LimbLength`]. See there for the available
    /// information.
    fn _len(&self) -> LimbLength<P>;

    fn _limbs(&self) -> &[Limb] {
        unsafe { std::slice::from_raw_parts(self._as_ptr(), self._len().limbs()) }
    }

    fn _prime(&self) -> ValidPrime {
        ValidPrime::new(P)
    }

    fn _is_empty(&self) -> bool {
        self._len().logical() == 0
    }

    fn _is_zero(&self) -> bool {
        let limb_range = self._len().limb_range();
        if limb_range.is_empty() {
            return true;
        }
        let (min_mask, max_mask) = self._len().limb_masks();
        if self._limbs()[limb_range.start] & min_mask != 0 {
            return false;
        }

        let inner_range = self._len().limb_range_inner();
        if self._limbs()[inner_range].iter().any(|&x| x != 0) {
            return false;
        }
        if self._limbs()[limb_range.end - 1] & max_mask != 0 {
            return false;
        }
        true
    }

    fn _slice<'a>(&self, range: LimbLength<P>) -> SliceP<'a, P>
    where
        Self: 'a,
    {
        let (new_len, offset) = self._len().restrict_to(range).apply_shift();
        let limbs_ptr = unsafe { self._as_ptr().add(offset) };
        let limbs = unsafe { std::slice::from_raw_parts(limbs_ptr, new_len.limbs()) };
        SliceP {
            limbs,
            range: new_len,
        }
    }

    fn _as_slice(&self) -> SliceP<P> {
        SliceP {
            limbs: self._limbs(),
            range: self._len(),
        }
    }

    fn _entry(&self, index: usize) -> u32 {
        debug_assert!(
            index < self._len().logical(),
            "Index {} too large, length of vector is only {}.",
            index,
            self._len().logical()
        );
        let bit_mask = limb::bitmask::<P>();
        let limb_index = limb::limb_bit_index_pair::<P>(index + self._len().start);
        let mut result = self._limbs()[limb_index.limb];
        result >>= limb_index.bit_index;
        result &= bit_mask;
        result as u32
    }

    fn _iter(&self) -> FpVectorIterator {
        FpVectorIterator::new(self)
    }

    fn _iter_nonzero(&self) -> FpVectorNonZeroIteratorP<P> {
        FpVectorNonZeroIteratorP::new(self)
    }

    fn _first_nonzero(&self) -> Option<(usize, u32)> {
        todo!();
    }

    fn _sign_rule<S: InternalBaseVectorP<P>>(&self, other: S) -> bool {
        assert_eq!(P, 2);
        let mut result = 0;
        for target_limb_idx in 0..self._limbs().len() {
            let target_limb = other._limbs()[target_limb_idx];
            let source_limb = self._limbs()[target_limb_idx];
            result ^= limb::sign_rule(target_limb, source_limb);
            if target_limb.count_ones() % 2 == 0 {
                continue;
            }
            for _ in 0..target_limb_idx {
                result ^= source_limb.count_ones() % 2;
            }
        }
        result == 1
    }

    fn _into_owned(self) -> FpVectorP<P> {
        let mut new = FpVectorP::<P>::new_(self._len().logical());
        if self._len().start % limb::entries_per_limb_const::<P>() == 0 {
            let limb_range = self._len().limb_range();
            new.limbs[0..limb_range.len()].copy_from_slice(&self._limbs()[limb_range]);
            if !new.limbs.is_empty() {
                let len = new.limbs.len();
                new.limbs[len - 1] &= self._len().limb_masks().1;
            }
        } else {
            new._assign(self);
        }
        new
    }

    fn _density(&self) -> f32 {
        self._iter_nonzero().count() as f32 / self._len().logical() as f32
    }
}

pub trait InternalBaseVectorMutP<const P: u32>: InternalBaseVectorP<P> {
    fn _as_mut_ptr(&mut self) -> *mut Limb;

    fn _limbs_mut(&mut self) -> &mut [Limb] {
        unsafe { std::slice::from_raw_parts_mut(self._as_mut_ptr(), self._len().limbs()) }
    }

    fn _slice_mut(&mut self, range: LimbLength<P>) -> SliceMutP<P> {
        let (new_len, offset) = self._len().restrict_to(range).apply_shift();
        let limbs_ptr = unsafe { self._as_mut_ptr().add(offset) };
        let limbs = unsafe { std::slice::from_raw_parts_mut(limbs_ptr, new_len.limbs()) };
        SliceMutP {
            limbs,
            range: new_len,
        }
    }

    fn _as_slice_mut(&mut self) -> SliceMutP<P> {
        let range = self._len();
        SliceMutP {
            limbs: self._limbs_mut(),
            range,
        }
    }

    fn _add<T: InternalBaseVectorP<P>>(&mut self, other: T, c: u32) {
        debug_assert!(c < P);
        if self._is_empty() {
            return;
        }

        if P == 2 {
            if c != 0 {
                match self._len().bit_offset().cmp(&other._len().bit_offset()) {
                    Ordering::Equal => self._add_shift_none(other, 1),
                    Ordering::Less => self._add_shift_left(other, 1),
                    Ordering::Greater => self._add_shift_right(other, 1),
                };
            }
        } else {
            match self._len().bit_offset().cmp(&other._len().bit_offset()) {
                Ordering::Equal => self._add_shift_none(other, c),
                Ordering::Less => self._add_shift_left(other, c),
                Ordering::Greater => self._add_shift_right(other, c),
            };
        }
    }

    fn _add_shift_none<T: InternalBaseVectorP<P>>(&mut self, other: T, c: u32) {
        let target_range = self._len().limb_range();
        let source_range = other._len().limb_range();

        let (min_mask, max_mask) = other._len().limb_masks();
        let other_limbs = other._limbs();

        self._limbs_mut()[target_range.start] = limb::add::<P>(
            self._limbs_mut()[target_range.start],
            other_limbs[source_range.start] & min_mask,
            c,
        );
        self._limbs_mut()[target_range.start] =
            limb::reduce::<P>(self._limbs_mut()[target_range.start]);

        let target_inner_range = self._len().limb_range_inner();
        let source_inner_range = other._len().limb_range_inner();
        if !source_inner_range.is_empty() {
            limb::add_all::<P>(
                &mut self._limbs_mut()[target_inner_range],
                &other_limbs[source_inner_range],
                c,
            );
        }
        if source_range.len() > 1 {
            // The first and last limbs are distinct, so we process the last.
            self._limbs_mut()[target_range.end - 1] = limb::add::<P>(
                self._limbs_mut()[target_range.end - 1],
                other_limbs[source_range.end - 1] & max_mask,
                c,
            );
            self._limbs_mut()[target_range.end - 1] =
                limb::reduce::<P>(self._limbs_mut()[target_range.end - 1]);
        }
    }

    fn _add_shift_left<T: InternalBaseVectorP<P>>(&mut self, other: T, c: u32) {
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
            fn new<T, S, const P: u32>(target: T, source: S) -> Self
            where
                T: InternalBaseVectorP<P>,
                S: InternalBaseVectorP<P>,
            {
                debug_assert!(target._prime() == source._prime());
                debug_assert!(target._len().bit_offset() <= source._len().bit_offset());
                debug_assert!(
                    target._len().logical() == source._len().logical(),
                    "self.dim {} not equal to other.dim {}",
                    target._len().logical(),
                    source._len().logical()
                );
                let offset_shift = source._len().bit_offset() - target._len().bit_offset();
                let bit_length = limb::bit_length_const::<P>();
                let entries_per_limb = limb::entries_per_limb_const::<P>();
                let usable_bits_per_limb = bit_length * entries_per_limb;
                let tail_shift = usable_bits_per_limb - offset_shift;
                let zero_bits = constants::BITS_PER_LIMB - usable_bits_per_limb;
                let source_range = source._len().limb_range();
                let target_range = target._len().limb_range();
                let min_source_limb = source_range.start;
                let min_target_limb = target_range.start;
                let number_of_source_limbs = source_range.len();
                let number_of_target_limbs = target_range.len();
                let (min_mask, max_mask) = source._len().limb_masks();

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

            fn mask_first_limb<T: InternalBaseVectorP<P>, const P: u32>(
                &self,
                other: T,
                i: usize,
            ) -> Limb {
                (other._limbs()[i] & self.min_mask) >> self.offset_shift
            }

            fn mask_middle_limb_a<T: InternalBaseVectorP<P>, const P: u32>(
                &self,
                other: T,
                i: usize,
            ) -> Limb {
                other._limbs()[i] >> self.offset_shift
            }

            fn mask_middle_limb_b<T: InternalBaseVectorP<P>, const P: u32>(
                &self,
                other: T,
                i: usize,
            ) -> Limb {
                (other._limbs()[i] << (self.tail_shift + self.zero_bits)) >> self.zero_bits
            }

            fn mask_last_limb_a<T: InternalBaseVectorP<P>, const P: u32>(
                &self,
                other: T,
                i: usize,
            ) -> Limb {
                let source_limb_masked = other._limbs()[i] & self.max_mask;
                source_limb_masked << self.tail_shift
            }

            fn mask_last_limb_b<T: InternalBaseVectorP<P>, const P: u32>(
                &self,
                other: T,
                i: usize,
            ) -> Limb {
                let source_limb_masked = other._limbs()[i] & self.max_mask;
                source_limb_masked >> self.offset_shift
            }
        }

        let dat = AddShiftLeftData::new(&self, &other);
        let mut i = 0;
        let limbs_mut = self._limbs_mut();

        {
            limbs_mut[i + dat.min_target_limb] = limb::add::<P>(
                limbs_mut[i + dat.min_target_limb],
                dat.mask_first_limb(&other, i + dat.min_source_limb),
                c,
            );
        }
        for i in 1..dat.number_of_source_limbs - 1 {
            limbs_mut[i + dat.min_target_limb] = limb::add::<P>(
                limbs_mut[i + dat.min_target_limb],
                dat.mask_middle_limb_a(&other, i + dat.min_source_limb),
                c,
            );
            limbs_mut[i + dat.min_target_limb - 1] = limb::add::<P>(
                limbs_mut[i + dat.min_target_limb - 1],
                dat.mask_middle_limb_b(&other, i + dat.min_source_limb),
                c,
            );
            limbs_mut[i + dat.min_target_limb - 1] =
                limb::reduce::<P>(limbs_mut[i + dat.min_target_limb - 1]);
        }
        i = dat.number_of_source_limbs - 1;
        if i > 0 {
            limbs_mut[i + dat.min_target_limb - 1] = limb::add::<P>(
                limbs_mut[i + dat.min_target_limb - 1],
                dat.mask_last_limb_a(&other, i + dat.min_source_limb),
                c,
            );
            limbs_mut[i + dat.min_target_limb - 1] =
                limb::reduce::<P>(limbs_mut[i + dat.min_target_limb - 1]);
            if dat.number_of_source_limbs == dat.number_of_target_limbs {
                limbs_mut[i + dat.min_target_limb] = limb::add::<P>(
                    limbs_mut[i + dat.min_target_limb],
                    dat.mask_last_limb_b(&other, i + dat.min_source_limb),
                    c,
                );
                limbs_mut[i + dat.min_target_limb] =
                    limb::reduce::<P>(limbs_mut[i + dat.min_target_limb]);
            }
        } else {
            limbs_mut[i + dat.min_target_limb] =
                limb::reduce::<P>(limbs_mut[i + dat.min_target_limb]);
        }
    }

    fn _add_shift_right<T: InternalBaseVectorP<P>>(&mut self, other: T, c: u32) {
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
            fn new<T, S, const P: u32>(target: T, source: S) -> Self
            where
                T: InternalBaseVectorP<P>,
                S: InternalBaseVectorP<P>,
            {
                debug_assert!(target._prime() == source._prime());
                debug_assert!(target._len().bit_offset() >= source._len().bit_offset());
                debug_assert!(
                    target._len().logical() == source._len().logical(),
                    "self.dim {} not equal to other.dim {}",
                    target._len().logical(),
                    source._len().logical()
                );
                let offset_shift = target._len().bit_offset() - source._len().bit_offset();
                let bit_length = limb::bit_length_const::<P>();
                let entries_per_limb = limb::entries_per_limb_const::<P>();
                let usable_bits_per_limb = bit_length * entries_per_limb;
                let tail_shift = usable_bits_per_limb - offset_shift;
                let zero_bits = constants::BITS_PER_LIMB - usable_bits_per_limb;
                let source_range = source._len().limb_range();
                let target_range = target._len().limb_range();
                let min_source_limb = source_range.start;
                let min_target_limb = target_range.start;
                let number_of_source_limbs = source_range.len();
                let number_of_target_limbs = target_range.len();
                let (min_mask, max_mask) = source._len().limb_masks();
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

            fn mask_first_limb_a<T: InternalBaseVectorP<P>, const P: u32>(
                &self,
                other: T,
                i: usize,
            ) -> Limb {
                let source_limb_masked = other._limbs()[i] & self.min_mask;
                (source_limb_masked << (self.offset_shift + self.zero_bits)) >> self.zero_bits
            }

            fn mask_first_limb_b<T: InternalBaseVectorP<P>, const P: u32>(
                &self,
                other: T,
                i: usize,
            ) -> Limb {
                let source_limb_masked = other._limbs()[i] & self.min_mask;
                source_limb_masked >> self.tail_shift
            }

            fn mask_middle_limb_a<T: InternalBaseVectorP<P>, const P: u32>(
                &self,
                other: T,
                i: usize,
            ) -> Limb {
                (other._limbs()[i] << (self.offset_shift + self.zero_bits)) >> self.zero_bits
            }

            fn mask_middle_limb_b<T: InternalBaseVectorP<P>, const P: u32>(
                &self,
                other: T,
                i: usize,
            ) -> Limb {
                other._limbs()[i] >> self.tail_shift
            }

            fn mask_last_limb_a<T: InternalBaseVectorP<P>, const P: u32>(
                &self,
                other: T,
                i: usize,
            ) -> Limb {
                let source_limb_masked = other._limbs()[i] & self.max_mask;
                source_limb_masked << self.offset_shift
            }

            fn mask_last_limb_b<T: InternalBaseVectorP<P>, const P: u32>(
                &self,
                other: T,
                i: usize,
            ) -> Limb {
                let source_limb_masked = other._limbs()[i] & self.max_mask;
                source_limb_masked >> self.tail_shift
            }
        }

        let dat = AddShiftRightData::new(&self, &other);
        let mut i = 0;
        let limbs_mut = self._limbs_mut();

        {
            limbs_mut[i + dat.min_target_limb] = limb::add::<P>(
                limbs_mut[i + dat.min_target_limb],
                dat.mask_first_limb_a(&other, i + dat.min_source_limb),
                c,
            );
            limbs_mut[i + dat.min_target_limb] =
                limb::reduce::<P>(limbs_mut[i + dat.min_target_limb]);
            if dat.number_of_target_limbs > 1 {
                limbs_mut[i + dat.min_target_limb + 1] = limb::add::<P>(
                    limbs_mut[i + dat.min_target_limb + 1],
                    dat.mask_first_limb_b(&other, i + dat.min_source_limb),
                    c,
                );
            }
        }
        for i in 1..dat.number_of_source_limbs - 1 {
            limbs_mut[i + dat.min_target_limb] = limb::add::<P>(
                limbs_mut[i + dat.min_target_limb],
                dat.mask_middle_limb_a(&other, i + dat.min_source_limb),
                c,
            );
            limbs_mut[i + dat.min_target_limb] =
                limb::reduce::<P>(limbs_mut[i + dat.min_target_limb]);
            limbs_mut[i + dat.min_target_limb + 1] = limb::add::<P>(
                limbs_mut[i + dat.min_target_limb + 1],
                dat.mask_middle_limb_b(&other, i + dat.min_source_limb),
                c,
            );
        }
        i = dat.number_of_source_limbs - 1;
        if i > 0 {
            limbs_mut[i + dat.min_target_limb] = limb::add::<P>(
                limbs_mut[i + dat.min_target_limb],
                dat.mask_last_limb_a(&other, i + dat.min_source_limb),
                c,
            );
            limbs_mut[i + dat.min_target_limb] =
                limb::reduce::<P>(limbs_mut[i + dat.min_target_limb]);
            if dat.number_of_target_limbs > dat.number_of_source_limbs {
                limbs_mut[i + dat.min_target_limb + 1] = limb::add::<P>(
                    limbs_mut[i + dat.min_target_limb + 1],
                    dat.mask_last_limb_b(&other, i + dat.min_source_limb),
                    c,
                );
            }
        }
        if dat.number_of_target_limbs > dat.number_of_source_limbs {
            limbs_mut[i + dat.min_target_limb + 1] =
                limb::reduce::<P>(limbs_mut[i + dat.min_target_limb + 1]);
        }
    }

    /// Given a mask v, add the `v[i]`th entry of `other` to the `i`th entry of `self`.
    fn _add_masked<T: InternalBaseVectorP<P>>(&mut self, other: T, c: u32, mask: &[usize]) {
        // TODO: If this ends up being a bottleneck, try to use PDEP/PEXT
        assert_eq!(self._as_slice()._len().logical(), mask.len());
        for (i, &x) in mask.iter().enumerate() {
            let entry = other._entry(x);
            if entry != 0 {
                self._add_basis_element(i, entry * c);
            }
        }
    }

    /// Given a mask v, add the `i`th entry of `other` to the `v[i]`th entry of `self`.
    fn _add_unmasked<T: InternalBaseVectorP<P>>(&mut self, other: T, c: u32, mask: &[usize]) {
        assert!(other._len().logical() <= mask.len());
        for (i, v) in other._iter_nonzero() {
            self._add_basis_element(mask[i], v * c);
        }
    }

    fn _add_basis_element(&mut self, index: usize, value: u32) {
        if P == 2 {
            // Checking for value % 2 == 0 appears to be less performant
            let pair = limb::limb_bit_index_pair::<2>(index + self._len().start);
            self._limbs_mut()[pair.limb] ^= (value as Limb % 2) << pair.bit_index;
        } else {
            let mut x = self._entry(index);
            x += value;
            x %= P;
            self._set_entry(index, x);
        }
    }

    fn _add_offset<T: InternalBaseVectorP<P>>(&mut self, _other: T, _c: u32, _offset: usize) {
        todo!();
    }

    fn _set_entry(&mut self, index: usize, value: u32) {
        debug_assert!(index < self._len().logical());
        let bit_mask = limb::bitmask::<P>();
        let limb_index = limb::limb_bit_index_pair::<P>(index + self._len().start);
        let mut result = self._limbs()[limb_index.limb];
        result &= !(bit_mask << limb_index.bit_index);
        result |= (value as Limb) << limb_index.bit_index;
        self._limbs_mut()[limb_index.limb] = result;
    }

    fn _set_to_zero(&mut self) {
        let limb_range = self._len().limb_range();
        if limb_range.is_empty() {
            return;
        }
        let (min_mask, max_mask) = self._len().limb_masks();
        self._limbs_mut()[limb_range.start] &= !min_mask;

        let inner_range = self._len().limb_range_inner();
        for limb in &mut self._limbs_mut()[inner_range] {
            *limb = 0;
        }
        self._limbs_mut()[limb_range.end - 1] &= !max_mask;
    }

    fn _reduce_limbs(&mut self) {
        if P != 2 {
            let limb_range = self._len().limb_range();

            for limb in &mut self._limbs_mut()[limb_range] {
                *limb = limb::reduce::<P>(*limb);
            }
        }
    }

    fn _scale(&mut self, c: u32) {
        if P == 2 {
            if c == 0 {
                self._set_to_zero();
            }
            return;
        }

        let c = c as Limb;
        let limb_range = self._len().limb_range();
        if limb_range.is_empty() {
            return;
        }
        let (min_mask, max_mask) = self._len().limb_masks();

        let limb = self._limbs()[limb_range.start];
        let masked_limb = limb & min_mask;
        let rest_limb = limb & !min_mask;
        self._limbs_mut()[limb_range.start] = (masked_limb * c) | rest_limb;

        let inner_range = self._len().limb_range_inner();
        for limb in &mut self._limbs_mut()[inner_range] {
            *limb *= c;
        }
        if limb_range.len() > 1 {
            let full_limb = self._limbs()[limb_range.end - 1];
            let masked_limb = full_limb & max_mask;
            let rest_limb = full_limb & !max_mask;
            self._limbs_mut()[limb_range.end - 1] = (masked_limb * c) | rest_limb;
        }
        self._reduce_limbs();
    }

    fn _assign<T: InternalBaseVectorP<P>>(&mut self, other: T) {
        debug_assert_eq!(self._len().logical(), other._len().logical());
        if self._len().bit_offset() != other._len().bit_offset() {
            self._set_to_zero();
            self._add(other, 1);
            return;
        }
        let target_range = self._len().limb_range();
        let source_range = other._len().limb_range();

        if target_range.is_empty() {
            return;
        }

        let (min_mask, max_mask) = other._len().limb_masks();

        let result = other._limbs()[source_range.start] & min_mask;
        self._limbs_mut()[target_range.start] &= !min_mask;
        self._limbs_mut()[target_range.start] |= result;

        let target_inner_range = self._len().limb_range_inner();
        let source_inner_range = other._len().limb_range_inner();
        if !target_inner_range.is_empty() && !source_inner_range.is_empty() {
            self._limbs_mut()[target_inner_range]
                .clone_from_slice(&other._limbs()[source_inner_range]);
        }

        let result = other._limbs()[source_range.end - 1] & max_mask;
        self._limbs_mut()[target_range.end - 1] &= !max_mask;
        self._limbs_mut()[target_range.end - 1] |= result;
    }

    /// This replaces the contents of the vector with the contents of the slice. The two must have
    /// the same length.
    ///
    /// This method is only implemented on `FpVectorP` right now. This is the only use case so far,
    /// so I don't feel too bad about marking it as unimplemented in the general case.
    fn _copy_from_slice(&mut self, _slice: &[u32]) {
        unimplemented!();
    }

    /// This method is only implemented on `FpVectorP` right now. This is the only use case so far,
    /// so I don't feel too bad about marking it as unimplemented in the general case.
    fn _add_truncate<T: InternalBaseVectorP<P>>(&mut self, _other: T, _c: u32) -> Option<()> {
        unimplemented!();
    }
}
