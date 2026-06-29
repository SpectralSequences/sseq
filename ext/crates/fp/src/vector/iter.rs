use super::inner::FqSlice;
use crate::{
    field::{Field, element::FieldElement},
    limb::Limb,
};

/// Read entry `idx` (an absolute index into `limbs`) under the bit-sliced layout, by
/// gathering its bit from each plane of its group.
#[inline]
fn gather_at<F: Field>(fq: F, limbs: &[Limb], idx: usize) -> FieldElement<F> {
    let lpg = fq.limbs_per_group();
    let base = fq.group_of(idx) * lpg;
    fq.gather(&limbs[base..base + lpg], fq.lane_of(idx))
}

pub struct FqVectorIterator<'a, F> {
    fq: F,
    limbs: &'a [Limb],
    // Bit-sliced path: `pos` is the absolute index of the next entry to emit.
    bitsliced: bool,
    pos: usize,
    // Packed path state.
    bit_length: usize,
    bit_mask: Limb,
    entries_per_limb_m_1: usize,
    limb_index: usize,
    entries_left: usize,
    cur_limb: Limb,
    counter: usize,
}

impl<'a, F: Field> FqVectorIterator<'a, F> {
    pub(super) fn new(vec: FqSlice<'a, F>) -> Self {
        let counter = vec.len();
        let fq = vec.fq();
        let start = vec.start();
        let limbs = vec.into_limbs();

        if counter == 0 {
            return Self {
                fq,
                limbs,
                bitsliced: fq.is_bitsliced(),
                pos: start,
                bit_length: 0,
                entries_per_limb_m_1: 0,
                bit_mask: 0,
                limb_index: 0,
                entries_left: 0,
                cur_limb: 0,
                counter,
            };
        }

        if fq.is_bitsliced() {
            return Self {
                fq,
                limbs,
                bitsliced: true,
                pos: start,
                bit_length: 0,
                entries_per_limb_m_1: 0,
                bit_mask: 0,
                limb_index: 0,
                entries_left: 0,
                cur_limb: 0,
                counter,
            };
        }

        let pair = fq.limb_bit_index_pair(start);
        let bit_length = fq.bit_length();
        let cur_limb = limbs[pair.limb] >> pair.bit_index;
        let entries_per_limb = fq.entries_per_limb();
        Self {
            fq,
            limbs,
            bitsliced: false,
            pos: start,
            bit_length,
            entries_per_limb_m_1: entries_per_limb - 1,
            bit_mask: fq.bitmask(),
            limb_index: pair.limb,
            entries_left: entries_per_limb - (start % entries_per_limb),
            cur_limb,
            counter,
        }
    }

    pub fn skip_n(&mut self, mut n: usize) {
        if n >= self.counter {
            self.pos += self.counter;
            self.counter = 0;
            return;
        }
        if self.bitsliced {
            self.pos += n;
            self.counter -= n;
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

impl<F: Field> Iterator for FqVectorIterator<'_, F> {
    type Item = FieldElement<F>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.counter == 0 {
            return None;
        }

        if self.bitsliced {
            let result = gather_at(self.fq, self.limbs, self.pos);
            self.pos += 1;
            self.counter -= 1;
            return Some(result);
        }

        if self.entries_left == 0 {
            self.limb_index += 1;
            self.cur_limb = self.limbs[self.limb_index];
            self.entries_left = self.entries_per_limb_m_1;
        } else {
            self.entries_left -= 1;
        }

        let result = self.cur_limb & self.bit_mask;
        self.counter -= 1;
        self.cur_limb >>= self.bit_length;

        Some(self.fq.decode(result))
    }
}

impl<F: Field> ExactSizeIterator for FqVectorIterator<'_, F> {
    fn len(&self) -> usize {
        self.counter
    }
}

/// Iterator over non-zero entries of an FpVector. This is monomorphized over the ground field for
/// significant performance gains.
pub struct FqVectorNonZeroIterator<'a, F> {
    fq: F,
    limbs: &'a [Limb],
    // Bit-sliced path: absolute index of the group base, and the relative cursor.
    bitsliced: bool,
    start: usize,
    // Shared/packed path state.
    limb_index: usize,
    cur_limb_entries_left: usize,
    cur_limb: Limb,
    idx: usize,
    dim: usize,
}

impl<'a, F: Field> FqVectorNonZeroIterator<'a, F> {
    pub(super) fn new(vec: FqSlice<'a, F>) -> Self {
        let fq = vec.fq();
        let dim = vec.len();
        let start = vec.start();
        let limbs = vec.into_limbs();

        if dim == 0 || fq.is_bitsliced() {
            return Self {
                fq,
                limbs,
                bitsliced: fq.is_bitsliced(),
                start,
                limb_index: 0,
                cur_limb_entries_left: 0,
                cur_limb: 0,
                idx: 0,
                dim,
            };
        }

        let entries_per_limb = fq.entries_per_limb();
        let pair = fq.limb_bit_index_pair(start);
        let cur_limb = limbs[pair.limb] >> pair.bit_index;
        let cur_limb_entries_left = entries_per_limb - (start % entries_per_limb);
        Self {
            fq,
            limbs,
            bitsliced: false,
            start,
            limb_index: pair.limb,
            cur_limb_entries_left,
            cur_limb,
            idx: 0,
            dim,
        }
    }
}

impl<F: Field> Iterator for FqVectorNonZeroIterator<'_, F> {
    type Item = (usize, FieldElement<F>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.bitsliced {
            let zero = self.fq.zero();
            while self.idx < self.dim {
                let value = gather_at(self.fq, self.limbs, self.start + self.idx);
                let cur = self.idx;
                self.idx += 1;
                if value != zero {
                    return Some((cur, value));
                }
            }
            return None;
        }

        let bit_length: usize = self.fq.bit_length();
        let bitmask: Limb = self.fq.bitmask();
        let entries_per_limb: usize = self.fq.entries_per_limb();
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
        let result = (self.idx, self.fq.decode(self.cur_limb & bitmask));
        self.idx += 1;
        self.cur_limb_entries_left -= 1;
        self.cur_limb >>= bit_length;
        Some(result)
    }
}
