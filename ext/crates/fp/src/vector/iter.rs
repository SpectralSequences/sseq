use super::inner::SliceP;
use crate::{
    limb::{self, Limb},
    prime::Prime,
};

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
    pub(super) fn new<P: Prime>(vec: SliceP<'a, P>) -> Self {
        let counter = vec.len();
        let limbs = &vec.limbs;

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
        let pair = limb::limb_bit_index_pair(vec.p, vec.start);

        let bit_length = limb::bit_length(vec.p);
        let cur_limb = limbs[pair.limb] >> pair.bit_index;

        let entries_per_limb = limb::entries_per_limb(vec.p);
        Self {
            limbs,
            bit_length,
            entries_per_limb_m_1: entries_per_limb - 1,
            bit_mask: limb::bitmask(vec.p),
            limb_index: pair.limb,
            entries_left: entries_per_limb - (vec.start % entries_per_limb),
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
pub struct FpVectorNonZeroIteratorP<'a, P> {
    p: P,
    limbs: &'a [Limb],
    limb_index: usize,
    cur_limb_entries_left: usize,
    cur_limb: Limb,
    idx: usize,
    dim: usize,
}

impl<'a, P: Prime> FpVectorNonZeroIteratorP<'a, P> {
    pub(super) fn new(vec: SliceP<'a, P>) -> Self {
        let entries_per_limb = limb::entries_per_limb(vec.p);

        let dim = vec.len();
        let limbs = vec.limbs;

        if dim == 0 {
            return Self {
                p: vec.p,
                limbs,
                limb_index: 0,
                cur_limb_entries_left: 0,
                cur_limb: 0,
                idx: 0,
                dim: 0,
            };
        }
        let min_index = vec.start;
        let pair = limb::limb_bit_index_pair(vec.p, min_index);
        let cur_limb = limbs[pair.limb] >> pair.bit_index;
        let cur_limb_entries_left = entries_per_limb - (min_index % entries_per_limb);
        Self {
            p: vec.p,
            limbs,
            limb_index: pair.limb,
            cur_limb_entries_left,
            cur_limb,
            idx: 0,
            dim,
        }
    }
}

impl<'a, P: Prime> Iterator for FpVectorNonZeroIteratorP<'a, P> {
    type Item = (usize, u32);

    fn next(&mut self) -> Option<Self::Item> {
        let bit_length: usize = limb::bit_length(self.p);
        let bitmask: Limb = limb::bitmask(self.p);
        let entries_per_limb: usize = limb::entries_per_limb(self.p);
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
