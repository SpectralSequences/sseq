use itertools::Itertools;

use super::{InternalBaseVectorMutP, InternalBaseVectorP};
use crate::{
    limb::{self, Limb, LimbLength},
    simd,
    vector::generic::{FpVectorP, SliceMutP, SliceP},
};

impl<const P: u32> InternalBaseVectorP<P> for FpVectorP<P> {
    fn _as_ptr(&self) -> *const Limb {
        self.limbs.as_ptr()
    }

    fn _len(&self) -> LimbLength<P> {
        self.len
    }

    fn _is_zero(&self) -> bool {
        self.limbs.iter().all(|&x| x == 0)
    }

    fn _limbs(&self) -> &[Limb] {
        &self.limbs
    }

    fn _into_owned(self) -> FpVectorP<P> {
        self
    }

    fn _first_nonzero(&self) -> Option<(usize, u32)> {
        let entries_per_limb = limb::entries_per_limb_const::<P>();
        let bit_length = limb::bit_length_const::<P>();
        let bitmask = limb::bitmask::<P>();
        for (i, &limb) in self._limbs().iter().enumerate() {
            if limb == 0 {
                continue;
            }
            let index = limb.trailing_zeros() as usize / bit_length;
            return Some((
                i * entries_per_limb + index,
                ((limb >> (index * bit_length)) & bitmask) as u32,
            ));
        }
        None
    }

    fn _density(&self) -> f32 {
        (if P == 2 {
            self.limbs
                .iter()
                .copied()
                .map(Limb::count_ones)
                .sum::<u32>() as usize
        } else {
            self._iter_nonzero().count()
        }) as f32
            / self._len().logical() as f32
    }
}

impl<const P: u32> InternalBaseVectorMutP<P> for FpVectorP<P> {
    fn _as_mut_ptr(&mut self) -> *mut Limb {
        self.limbs.as_mut_ptr()
    }

    fn _set_to_zero(&mut self) {
        self.limbs.fill(0);
    }

    fn _scale(&mut self, c: u32) {
        match P {
            2 => {
                if c == 0 {
                    self._set_to_zero()
                }
            }
            3 | 5 => {
                for limb in &mut self.limbs {
                    *limb = limb::reduce::<P>(*limb * c as Limb);
                }
            }
            _ => {
                for limb in &mut self.limbs {
                    *limb = limb::pack::<_, P>(limb::unpack::<P>(*limb).map(|x| (x * c) % P));
                }
            }
        }
    }

    fn _add_offset<T: InternalBaseVectorP<P>>(&mut self, other: T, c: u32, offset: usize) {
        debug_assert_eq!(
            other._len().start,
            0,
            "`FpVector::add_offset` only supports limb-aligned arguments"
        );
        debug_assert_eq!(self._len().logical(), other._len().logical());
        let min_limb = offset / limb::entries_per_limb_const::<P>();
        if P == 2 {
            if c != 0 {
                simd::add_simd(&mut self.limbs, other._limbs(), min_limb);
            }
        } else {
            for (left, right) in self.limbs.iter_mut().zip(other._limbs()).skip(min_limb) {
                *left = limb::add::<P>(*left, *right, c);
            }
            for limb in &mut self.limbs[min_limb..] {
                *limb = limb::reduce::<P>(*limb);
            }
        }
    }

    fn _assign<T: InternalBaseVectorP<P>>(&mut self, other: T) {
        debug_assert_eq!(self._len().logical(), other._len().logical());
        let other_num_limbs = other._len().limbs();
        let shift = other._len().bit_offset();

        self.limbs.resize(other_num_limbs, 0);
        self.limbs
            .copy_from_slice(&other._limbs()[..other_num_limbs]);

        if shift > 0 {
            let mut borrow = 0;
            let borrow_shift = limb::used_bits::<P>() - shift;
            for elem in self.limbs.iter_mut().rev() {
                let new_borrow = *elem << borrow_shift;
                *elem = ((*elem >> shift) | borrow) & limb::used_mask::<P>();
                borrow = new_borrow;
            }
        }

        // Potentially useless, but otherwise we can end up with nonzero limbs past the end of the
        // vector. That doesn't seem to cause a problem for now but it might down the road.
        self.limbs.truncate(self._len().limbs());

        for limb in self.limbs.iter() {
            debug_assert_eq!(limb & !limb::used_mask::<P>(), 0);
        }
    }

    fn _limbs_mut(&mut self) -> &mut [Limb] {
        &mut self.limbs
    }

    fn _copy_from_slice(&mut self, slice: &[u32]) {
        assert_eq!(self._len().logical(), slice.len());

        self.limbs.clear();
        self.limbs.extend(
            slice
                .chunks(limb::entries_per_limb_const::<P>())
                .map(|x| limb::pack::<_, P>(x.iter().copied())),
        );
    }

    fn _add_truncate<T: InternalBaseVectorP<P>>(&mut self, other: T, c: u32) -> Option<()> {
        // We require `other` to start on a limb boundary. In practice we only ever call this
        // function with `other: FpVectorP`, which satisfies this condition by definition.
        debug_assert_eq!(other._len().start, 0);
        for (left, right) in self.limbs.iter_mut().zip_eq(other._limbs()) {
            *left = limb::add::<P>(*left, *right, c);
            *left = limb::truncate::<P>(*left)?;
        }
        Some(())
    }
}

impl<'a, const P: u32> InternalBaseVectorP<P> for SliceP<'a, P> {
    fn _as_ptr(&self) -> *const Limb {
        self.limbs.as_ptr()
    }

    fn _len(&self) -> LimbLength<P> {
        self.range
    }

    fn _limbs(&self) -> &[Limb] {
        self.limbs
    }
}

impl<'a, const P: u32> InternalBaseVectorP<P> for SliceMutP<'a, P> {
    fn _as_ptr(&self) -> *const Limb {
        self.limbs.as_ptr()
    }

    fn _len(&self) -> LimbLength<P> {
        self.range
    }

    fn _limbs(&self) -> &[Limb] {
        self.limbs
    }
}

impl<'a, const P: u32> InternalBaseVectorMutP<P> for SliceMutP<'a, P> {
    fn _as_mut_ptr(&mut self) -> *mut Limb {
        self.limbs.as_mut_ptr()
    }

    fn _limbs_mut(&mut self) -> &mut [Limb] {
        self.limbs
    }
}

// Tautological impls

impl<T: InternalBaseVectorP<P>, const P: u32> InternalBaseVectorP<P> for &T {
    fn _as_ptr(&self) -> *const Limb {
        T::_as_ptr(self)
    }

    fn _len(&self) -> LimbLength<P> {
        T::_len(self)
    }
}

impl<T: InternalBaseVectorP<P>, const P: u32> InternalBaseVectorP<P> for &mut T {
    fn _as_ptr(&self) -> *const Limb {
        T::_as_ptr(self)
    }

    fn _len(&self) -> LimbLength<P> {
        T::_len(self)
    }
}

impl<T: InternalBaseVectorMutP<P>, const P: u32> InternalBaseVectorMutP<P> for &mut T {
    fn _as_mut_ptr(&mut self) -> *mut Limb {
        T::_as_mut_ptr(self)
    }
}
