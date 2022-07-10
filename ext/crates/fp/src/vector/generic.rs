use std::io::{Read, Write};

use super::{
    base_generic::{BaseVectorMutP, BaseVectorP},
    internal::{InternalBaseVectorMutP, InternalBaseVectorP},
};
use crate::{
    constants,
    limb::{self, Limb, LimbLength},
    prime::ValidPrime,
};

/// An `FpVectorP` is a vector over $\mathbb{F}_p$ for a fixed prime, implemented using const
/// generics. Due to limitations with const generics, we cannot constrain P to actually be a prime,
/// so we allow it to be any u32. However, most functions will panic if P is not a prime.
///
/// Interally, it packs entries of the vectors into limbs. However, this is an abstraction that
/// must not leave the `fp` library.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct FpVectorP<const P: u32> {
    /// The limbs containing the entries of the (mathematical) vector. At all times, `limbs` must be
    /// at least `len.limbs()` long, but is allowed to be larger.
    pub(crate) limbs: Vec<Limb>,
    pub(crate) len: LimbLength<P>,
}

/// A SliceP is a slice of an FpVectorP. This immutably borrows the vector and implements Copy.
#[derive(Debug, Copy, Clone)]
pub struct SliceP<'a, const P: u32> {
    pub(crate) limbs: &'a [Limb],
    pub(crate) range: LimbLength<P>,
}

/// A `SliceMutP` is a mutable slice of an `FpVectorP`. This mutably borrows the vector. Since it
/// is a mutable borrow, it cannot implement `Copy`. However, it has a [`SliceMutP::copy`] function
/// that imitates the reborrowing, that mutably borrows `SliceMutP` and returns a `SliceMutP` with
/// a shorter lifetime.
#[derive(Debug)]
pub struct SliceMutP<'a, const P: u32> {
    pub(crate) limbs: &'a mut [Limb],
    pub(crate) range: LimbLength<P>,
}

impl<const P: u32> FpVectorP<P> {
    pub fn new_(len: usize) -> Self {
        let length = LimbLength::<P>::from_logical(len);
        Self {
            limbs: vec![0; length.limbs()],
            len: length,
        }
    }

    pub fn new_with_capacity_(len: usize, capacity: usize) -> Self {
        let length = LimbLength::<P>::from_logical(len);
        let mut limbs = Vec::with_capacity(limb::number::<P>(capacity));
        limbs.resize(length.limbs(), 0);
        Self { limbs, len: length }
    }

    // /// A version of [`FpVectorP::assign`] that allows `other` to be shorter than `self`.
    pub fn assign_partial(&mut self, other: &Self) {
        debug_assert!(other.len() <= self.len());
        self.limbs[0..other.limbs.len()].copy_from_slice(&other.limbs);
        for limb in self.limbs[other.limbs.len()..].iter_mut() {
            *limb = 0;
        }
    }

    /// This function ensures the length of the vector is at least `len`. See also
    /// `set_scratch_vector_size`.
    pub fn extend_len(&mut self, len: usize) {
        if self.len() >= len {
            return;
        }
        self.len = LimbLength::<P>::from_logical(len);
        self.limbs.resize(self.len.limbs(), 0);
    }

    /// This clears the vector and sets the length to `len`. This is useful for reusing
    /// allocations of temporary vectors.
    pub fn set_scratch_vector_size(&mut self, len: usize) {
        self.len = LimbLength::<P>::from_logical(len);
        self.limbs.clear();
        self.limbs.resize(self.len.limbs(), 0);
    }

    /// Permanently remove the first `n` elements in the vector. `n` must be a multiple of
    /// the number of entries per limb
    pub(crate) fn trim_start(&mut self, n: usize) {
        assert!(n <= self.len.logical());
        let entries_per = limb::entries_per_limb_const::<P>();
        assert_eq!(n % entries_per, 0);
        let num_limbs = n / entries_per;
        self.limbs.drain(0..num_limbs);
        self.len = self.len.trim_start(n);
    }

    fn add_carry_limb<T>(&mut self, idx: usize, source: Limb, c: u32, rest: &mut [T]) -> bool
    where
        for<'a> &'a mut T: TryInto<&'a mut Self>,
    {
        if P == 2 {
            if c == 0 {
                return false;
            }
            let mut cur_vec = self;
            let mut carry = source;
            for carry_vec in rest.iter_mut() {
                let carry_vec = carry_vec
                    .try_into()
                    .ok()
                    .expect("rest vectors in add_carry must be of the same prime");
                let rem = cur_vec.limbs[idx] ^ carry;
                let quot = cur_vec.limbs[idx] & carry;
                cur_vec.limbs[idx] = rem;
                carry = quot;
                cur_vec = carry_vec;
                if quot == 0 {
                    return false;
                }
            }
            cur_vec.limbs[idx] ^= carry;
            true
        } else {
            unimplemented!()
        }
    }

    pub fn add_carry<T>(&mut self, other: &Self, c: u32, rest: &mut [T]) -> bool
    where
        for<'a> &'a mut T: TryInto<&'a mut Self>,
    {
        let mut result = false;
        for i in 0..self.limbs.len() {
            result |= self.add_carry_limb(i, other.limbs[i], c, rest);
        }
        result
    }

    pub fn update_from_bytes(&mut self, data: &mut impl Read) -> std::io::Result<()> {
        let limbs = &mut self.limbs;
        let num_limbs = limbs.len();

        if cfg!(target_endian = "little") {
            let num_bytes = num_limbs * constants::BYTES_PER_LIMB;
            unsafe {
                let buf: &mut [u8] =
                    std::slice::from_raw_parts_mut(limbs.as_mut_ptr() as *mut u8, num_bytes);
                data.read_exact(buf).unwrap();
            }
        } else {
            for entry in limbs {
                let mut bytes: [u8; constants::BYTES_PER_LIMB] = [0; constants::BYTES_PER_LIMB];
                data.read_exact(&mut bytes)?;
                *entry = Limb::from_le_bytes(bytes);
            }
        };
        Ok(())
    }

    pub fn from_bytes(_p: ValidPrime, len: usize, data: &mut impl Read) -> std::io::Result<Self> {
        let mut v = Self::new_(len);
        v.update_from_bytes(data)?;
        Ok(v)
    }

    pub fn to_bytes(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        // self.limbs is allowed to have more limbs than necessary, but we only save the
        // necessary ones.
        let num_limbs = limb::number::<P>(self.len());

        if cfg!(target_endian = "little") {
            let num_bytes = num_limbs * constants::BYTES_PER_LIMB;
            unsafe {
                let buf: &[u8] =
                    std::slice::from_raw_parts_mut(self.limbs.as_ptr() as *mut u8, num_bytes);
                buffer.write_all(buf)?;
            }
        } else {
            for limb in &self.limbs[0..num_limbs] {
                let bytes = limb.to_le_bytes();
                buffer.write_all(&bytes)?;
            }
        }
        Ok(())
    }

    pub(crate) fn limbs(&self) -> &[Limb] {
        self._limbs()
    }

    pub(crate) fn limbs_mut(&mut self) -> &mut [Limb] {
        self._limbs_mut()
    }
}

impl<'a, const P: u32> From<&'a FpVectorP<P>> for SliceP<'a, P> {
    fn from(v: &'a FpVectorP<P>) -> Self {
        v.as_slice()
    }
}

impl<'a, const P: u32> From<&'a mut FpVectorP<P>> for SliceP<'a, P> {
    fn from(v: &'a mut FpVectorP<P>) -> Self {
        (v as &'a FpVectorP<P>).as_slice()
    }
}

impl<'a, const P: u32> From<&'a mut FpVectorP<P>> for SliceMutP<'a, P> {
    fn from(v: &'a mut FpVectorP<P>) -> Self {
        v.as_slice_mut()
    }
}

impl<'a, const P: u32> SliceMutP<'a, P> {
    /// `coeff` need not be reduced mod p.
    /// Adds v otimes w to self.
    pub fn add_tensor(&mut self, offset: usize, coeff: u32, left: SliceP<P>, right: SliceP<P>) {
        let right_dim = right.len();

        for (i, v) in left.iter_nonzero() {
            let entry = (v * coeff) % *self.prime();
            self.slice_mut(offset + i * right_dim, offset + (i + 1) * right_dim)
                .add(right, entry);
        }
    }

    /// Generates a version of itself with a shorter lifetime
    #[inline]
    pub fn copy(&mut self) -> SliceMutP<'_, P> {
        SliceMutP {
            limbs: self.limbs,
            range: self.range,
        }
    }
}

impl<'a, 'b, const P: u32> From<&'a mut SliceMutP<'b, P>> for SliceMutP<'a, P> {
    fn from(slice: &'a mut SliceMutP<'b, P>) -> SliceMutP<'a, P> {
        slice.copy()
    }
}

impl<'a, 'b, const P: u32> From<&'a SliceP<'b, P>> for SliceP<'a, P> {
    fn from(slice: &'a SliceP<'b, P>) -> SliceP<'a, P> {
        *slice
    }
}

impl<'a, 'b, const P: u32> From<&'a SliceMutP<'b, P>> for SliceP<'a, P> {
    fn from(slice: &'a SliceMutP<'b, P>) -> SliceP<'a, P> {
        slice.as_slice()
    }
}

impl<T: AsRef<[u32]>, const P: u32> From<&T> for FpVectorP<P> {
    fn from(slice: &T) -> Self {
        let mut v = Self::new_(slice.as_ref().len());
        v.limbs.clear();
        v.limbs.extend(
            slice
                .as_ref()
                .chunks(limb::entries_per_limb_const::<P>())
                .map(|x| limb::pack::<_, P>(x.iter().copied())),
        );
        v
    }
}

impl<const P: u32> From<&FpVectorP<P>> for Vec<u32> {
    fn from(vec: &FpVectorP<P>) -> Vec<u32> {
        vec.iter().collect()
    }
}

// Iterators

pub struct FpVectorIterator<'a> {
    limbs: &'a [Limb],
    bit_length: usize,
    bit_mask: Limb,
    entries_per_limb_m_1: usize,
    limb_index: usize,
    entries_left: usize,
    cur_limb: Limb,
    counter: usize,
}

impl<'a> FpVectorIterator<'a> {
    pub(crate) fn new<T: InternalBaseVectorP<P> + 'a, const P: u32>(vec: &'a T) -> Self {
        let counter = vec._len().logical();
        let limbs = vec._limbs();

        if counter == 0 {
            return Self {
                limbs,
                bit_length: 0,
                entries_per_limb_m_1: 0,
                bit_mask: 0,
                limb_index: 0,
                entries_left: 0,
                cur_limb: 0,
                counter,
            };
        }
        let pair = limb::limb_bit_index_pair::<P>(vec._len().start);

        let bit_length = limb::bit_length_const::<P>();
        let cur_limb = limbs[pair.limb] >> pair.bit_index;

        let entries_per_limb = limb::entries_per_limb_const::<P>();
        Self {
            limbs,
            bit_length,
            entries_per_limb_m_1: entries_per_limb - 1,
            bit_mask: limb::bitmask::<P>(),
            limb_index: pair.limb,
            entries_left: entries_per_limb - (vec._len().start % entries_per_limb),
            cur_limb,
            counter,
        }
    }

    pub fn skip_n(&mut self, mut n: usize) {
        if n >= self.counter {
            self.counter = 0;
            return;
        }
        let entries_per_limb = self.entries_per_limb_m_1 + 1;
        if n < self.entries_left {
            self.entries_left -= n;
            self.counter -= n;
            self.cur_limb >>= self.bit_length * n;
            return;
        }

        n -= self.entries_left;
        self.counter -= self.entries_left;
        self.entries_left = 0;

        let skip_limbs = n / entries_per_limb;
        self.limb_index += skip_limbs;
        self.counter -= skip_limbs * entries_per_limb;
        n -= skip_limbs * entries_per_limb;

        if n > 0 {
            self.entries_left = entries_per_limb - n;
            self.limb_index += 1;
            self.cur_limb = self.limbs[self.limb_index] >> (n * self.bit_length);
            self.counter -= n;
        }
    }
}

impl<'a> Iterator for FpVectorIterator<'a> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.counter == 0 {
            return None;
        } else if self.entries_left == 0 {
            self.limb_index += 1;
            self.cur_limb = self.limbs[self.limb_index];
            self.entries_left = self.entries_per_limb_m_1;
        } else {
            self.entries_left -= 1;
        }

        let result = (self.cur_limb & self.bit_mask) as u32;
        self.counter -= 1;
        self.cur_limb >>= self.bit_length;

        Some(result)
    }
}

impl<'a> ExactSizeIterator for FpVectorIterator<'a> {
    fn len(&self) -> usize {
        self.counter
    }
}

/// Iterator over non-zero entries of an FpVector. This is monomorphized over P for significant
/// performance gains.
pub struct FpVectorNonZeroIteratorP<'a, const P: u32> {
    limbs: &'a [Limb],
    limb_index: usize,
    cur_limb_entries_left: usize,
    cur_limb: Limb,
    idx: usize,
    dim: usize,
}

impl<'a, const P: u32> FpVectorNonZeroIteratorP<'a, P> {
    pub(crate) fn new<T: InternalBaseVectorP<P> + 'a>(vec: &'a T) -> Self {
        let entries_per_limb = limb::entries_per_limb_const::<P>();

        let dim = vec._len().logical();
        let limbs = vec._limbs();

        if dim == 0 {
            return Self {
                limbs,
                limb_index: 0,
                cur_limb_entries_left: 0,
                cur_limb: 0,
                idx: 0,
                dim: 0,
            };
        }
        let min_index = vec._len().start;
        let pair = limb::limb_bit_index_pair::<P>(vec._len().start);
        let cur_limb = limbs[pair.limb] >> pair.bit_index;
        let cur_limb_entries_left = entries_per_limb - (min_index % entries_per_limb);
        Self {
            limbs,
            limb_index: pair.limb,
            cur_limb_entries_left,
            cur_limb,
            idx: 0,
            dim,
        }
    }
}

impl<'a, const P: u32> Iterator for FpVectorNonZeroIteratorP<'a, P> {
    type Item = (usize, u32);

    fn next(&mut self) -> Option<Self::Item> {
        let bit_length: usize = limb::bit_length_const::<P>();
        let bitmask: Limb = limb::bitmask::<P>();
        let entries_per_limb: usize = limb::entries_per_limb_const::<P>();
        loop {
            let bits_left = (self.cur_limb_entries_left * bit_length) as u32;
            #[allow(clippy::unnecessary_cast)]
            let tz_real = (self.cur_limb | (1 as Limb).checked_shl(bits_left as u32).unwrap_or(0))
                .trailing_zeros();
            let tz_rem = ((tz_real as u8) % (bit_length as u8)) as u32;
            let tz_div = ((tz_real as u8) / (bit_length as u8)) as u32;
            let tz = tz_real - tz_rem;
            self.idx += tz_div as usize;
            if self.idx >= self.dim {
                return None;
            }
            self.cur_limb_entries_left -= tz_div as usize;
            if self.cur_limb_entries_left == 0 {
                self.limb_index += 1;
                self.cur_limb_entries_left = entries_per_limb;
                self.cur_limb = self.limbs[self.limb_index];
                continue;
            }
            self.cur_limb >>= tz;
            if tz == 0 {
                break;
            }
        }
        let result = (self.idx, (self.cur_limb & bitmask) as u32);
        self.idx += 1;
        self.cur_limb_entries_left -= 1;
        self.cur_limb >>= bit_length;
        Some(result)
    }
}
