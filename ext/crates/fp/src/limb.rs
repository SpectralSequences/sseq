use std::ops::Range;

pub(crate) use crate::constants::Limb;
use crate::{constants::BITS_PER_LIMB, prime::ValidPrime, simd};

/// A struct containing the information required to access a specific entry in an array of `Limb`s.
#[derive(Copy, Clone)]
pub(crate) struct LimbBitIndexPair {
    pub(crate) limb: usize,
    pub(crate) bit_index: usize,
}

/// A struct that defines a range of entries in a slice of limbs.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct LimbLength<const P: u32> {
    /// The index of the first entry. We do not assume that this value is less than
    /// `entries_per_limb_const::<P>()` in general, but some functions require it. See
    /// [`apply_shift`].
    pub(crate) start: usize,

    /// The index of the last entry.
    pub(crate) end: usize,

    /// The total number of limbs in the range.
    ///
    /// We store this value instead of computing it on the fly because benchmarks tend to show that
    /// the tradeoff is beneficial in high dimensions (>1000). We might want to only enable this
    /// when odd-primes is enabled, since computing this number is easier when `p == 2`, so the
    /// tradeoff is potentially worse.
    limbs: usize,
}

impl<const P: u32> LimbLength<P> {
    pub(crate) const fn from_logical(logical: usize) -> Self {
        let limbs = number::<P>(logical);
        Self {
            start: 0,
            end: logical,
            limbs,
        }
    }

    /// Returns a `LimbLength` describing a vector starting at entry `start` and ending at entry
    /// `end`.
    pub(crate) const fn from_start_end(start: usize, end: usize) -> Self {
        let limb_range = range::<P>(start, end);
        Self {
            start,
            end,
            limbs: limb_range.end - limb_range.start,
        }
    }

    #[inline]
    pub(crate) const fn limbs(&self) -> usize {
        self.limbs
    }

    #[inline]
    pub(crate) const fn logical(&self) -> usize {
        self.end - self.start
    }

    pub(crate) const fn contains(&self, other: &Self) -> bool {
        self.start + other.end <= self.end
    }

    /// Shift the entire `LimbLength` backwards so that the start of the range belongs to the first
    /// limb, and return it together with the number of limbs shifted.
    pub(crate) const fn apply_shift(&self) -> (Self, usize) {
        let entries_per = entries_per_limb_const::<P>();
        let offset = self.start / entries_per;
        let start = self.start - offset * entries_per;
        let end = self.end - offset * entries_per;
        (LimbLength::from_start_end(start, end), offset)
    }

    pub(crate) const fn restrict_to(&self, other: Self) -> Self {
        debug_assert!(self.contains(&other));
        Self::from_start_end(other.start + self.start, other.end + self.start)
    }

    /// This function panics if `self.start != 0`. The `LimbLength` that is returned also satisfies
    /// `self.start == 0`.
    ///
    /// It would be possible to make it work if we only assume `self.start %
    /// entries_per_limb_const::<P>() == 0`, but this introduces slight complications, e.g.
    /// depending on which of self.start or offset is bigger. While this can be solved by a
    /// `saturating_sub`, the reason we leave it that way is because we only use it when trimming
    /// the front of an `FpVector`, where the start is 0 by definition.
    pub(crate) fn trim_start(&self, offset: usize) -> Self {
        debug_assert_eq!(self.start, 0);
        assert_eq!(offset % entries_per_limb_const::<P>(), 0);
        let limb_shift = offset / entries_per_limb_const::<P>();
        Self {
            start: self.start,
            end: self.end - offset,
            limbs: self.limbs - limb_shift,
        }
    }

    /// This function assumes that `self.start < entries_per_limb_const::<P>()`. A `LimbLength`
    /// equivalent to `self` that does satisfy this condition can be obtained by calling
    /// [`apply_shift`].
    #[inline]
    pub(crate) const fn bit_offset(&self) -> usize {
        self.start * bit_length_const::<P>()
    }

    #[inline]
    pub(crate) const fn limb_range(&self) -> Range<usize> {
        range::<P>(self.start, self.end)
    }

    /// # Panics
    ///
    /// This function underflows if `self.start + self.logical() == 0`, which happens if and only if
    /// we are taking a slice of width 0 at the start of a limb. This should be a very rare edge
    /// case. Dealing with the underflow properly would probably require using `saturating_sub` or
    /// something of that nature, and that has a nontrivial (10%) performance hit.
    #[inline]
    pub(crate) fn limb_range_inner(&self) -> Range<usize> {
        let range = self.limb_range();
        (range.start + 1)..(usize::max(range.start + 1, range.end - 1))
    }

    /// This function assumes that `self.start < entries_per_limb_const::<P>()`. A `LimbLength`
    /// equivalent to `self` that does satisfy this condition can be obtained by calling
    /// [`apply_shift`].
    #[inline(always)]
    pub(crate) const fn min_limb_mask(&self) -> Limb {
        !0 << self.bit_offset()
    }

    #[inline(always)]
    pub(crate) const fn max_limb_mask(&self) -> Limb {
        let num_entries = 1 + (self.end - 1) % entries_per_limb_const::<P>();
        let bit_max = num_entries * bit_length_const::<P>();

        (!0) >> (BITS_PER_LIMB - bit_max)
    }

    /// This function assumes that `self.start < entries_per_limb_const::<P>()`. A `LimbLength`
    /// equivalent to `self` that does satisfy this condition can be obtained by calling
    /// [`apply_shift`].
    #[inline(always)]
    pub(crate) fn limb_masks(&self) -> (Limb, Limb) {
        if self.limb_range().len() == 1 {
            (
                self.min_limb_mask() & self.max_limb_mask(),
                self.min_limb_mask() & self.max_limb_mask(),
            )
        } else {
            (self.min_limb_mask(), self.max_limb_mask())
        }
    }
}

/// Return the number of bits an element of $\mathbb{F}_P$ occupies in a limb.
pub(crate) const fn bit_length(p: ValidPrime) -> usize {
    let p = p.value();
    match p {
        2 => 1,
        _ => (32 - (p * (p - 1)).leading_zeros()) as usize,
    }
}

/// Return the number of bits an element of $\mathbb{F}_P$ occupies in a limb.
pub(crate) const fn bit_length_const<const P: u32>() -> usize {
    match P {
        2 => 1,
        _ => (32 - (P * (P - 1)).leading_zeros()) as usize,
    }
}

/// If `l` is a limb of elements of $\\mathbb{F}_p$, then `l & bitmask::<P>()` is the value of the
/// first entry of `l`.
pub(crate) const fn bitmask<const P: u32>() -> Limb {
    (1 << bit_length_const::<P>()) - 1
}

// this function is never called if `odd-primes` is disabled
#[allow(dead_code)]
/// The number of elements of $\\mathbb{F}_p$ that fit in a single limb.
pub(crate) const fn entries_per_limb(p: ValidPrime) -> usize {
    BITS_PER_LIMB / bit_length(p)
}

/// The number of elements of $\\mathbb{F}_p$ that fit in a single limb.
pub(crate) const fn entries_per_limb_const<const P: u32>() -> usize {
    BITS_PER_LIMB / bit_length_const::<P>()
}

/// This is identical to [`limb::number`], except that it's not const. Hopefully almost every method
/// in the limb crate can be const once the matrix rewrite is in place.
pub(crate) const fn num_limbs(p: ValidPrime, len: usize) -> usize {
    let entries_per_limb = entries_per_limb(p);
    (len + entries_per_limb - 1) / entries_per_limb
}

pub(crate) const fn padded_len(p: ValidPrime, len: usize) -> usize {
    num_limbs(p, len) * entries_per_limb(p)
}

/// The number of bits that the entries occupy in total. This number is close to [`BITS_PER_LIMB`],
/// but often slightly lower unless `P == 2`.
pub(crate) const fn used_bits<const P: u32>() -> usize {
    entries_per_limb_const::<P>() * bit_length_const::<P>()
}

/// A mask on the region that contains entries. Limbs are usually assumed to satisfy the condition
/// `limb & !used_mask() == 0`.
pub(crate) const fn used_mask<const P: u32>() -> Limb {
    !0 >> (BITS_PER_LIMB - used_bits::<P>())
}

pub(crate) const fn limb_bit_index_pair<const P: u32>(idx: usize) -> LimbBitIndexPair {
    LimbBitIndexPair {
        limb: idx / entries_per_limb_const::<P>(),
        bit_index: (idx % entries_per_limb_const::<P>() * bit_length_const::<P>()),
    }
}

/// Return the `Limb` whose `i`th entry is `limb_a[i] + coeff * limb_b[i]` mod P. Both `limb_a` and
/// `limb_b` are assumed to be reduced.
pub(crate) const fn add<const P: u32>(limb_a: Limb, limb_b: Limb, coeff: u32) -> Limb {
    if P == 2 {
        limb_a ^ (coeff as Limb * limb_b)
    } else {
        limb_a + (coeff as Limb) * limb_b
    }
}

/// Add (`c` times) all of the limbs in `rhs` to the limbs in `lhs`. This is optimized to use SIMD
/// when `P == 2`.
pub(crate) fn add_all<const P: u32>(lhs: &mut [Limb], rhs: &[Limb], c: u32) {
    if P == 2 {
        simd::add_simd(lhs, rhs, 0);
    } else {
        for (left, right) in lhs.iter_mut().zip(rhs) {
            *left = add::<P>(*left, *right, c);
            *left = reduce::<P>(*left);
        }
    }
}

/// Return the `Limb` whose entries are the entries of `limb` reduced modulo `P`.
///
/// Contributed by Robert Burklund.
pub(crate) fn reduce<const P: u32>(limb: Limb) -> Limb {
    match P {
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
        _ => pack::<_, P>(unpack::<P>(limb).map(|x| x % P)),
    }
}

/// Check whether or not a limb is reduced, i.e. whether every entry is a value in the range `0..P`.
/// This is currently **not** faster than calling [`reduce`] directly.
pub(crate) fn is_reduced<const P: u32>(limb: Limb) -> bool {
    limb == reduce::<P>(limb)
}

/// Given an interator of `u32`'s, pack all of them into a single limb in order. It is assumed that
///  - The values of the iterator are less than P
///  - The values of the iterator fit into a single limb
///
/// If these assumptions are violated, the result will be nonsense.
pub(crate) fn pack<T: Iterator<Item = u32>, const P: u32>(entries: T) -> Limb {
    let bit_length = bit_length_const::<P>();
    let mut result: Limb = 0;
    let mut shift = 0;
    for entry in entries {
        result += (entry as Limb) << shift;
        shift += bit_length;
    }
    result
}

/// Give an iterator over the entries of `limb`.
pub(crate) fn unpack<const P: u32>(mut limb: Limb) -> impl Iterator<Item = u32> {
    let entries = entries_per_limb_const::<P>();
    let bit_length = bit_length_const::<P>();
    let bit_mask = bitmask::<P>();

    (0..entries).map(move |_| {
        let result = (limb & bit_mask) as u32;
        limb >>= bit_length;
        result
    })
}

/// Return the number of limbs required to hold `dim` entries. This is identical to
/// [`limb::num_limbs`], except the latter is not const. Hopefully almost every method in the limb
/// crate can be const once the matrix rewrite is in place.
pub(crate) const fn number<const P: u32>(dim: usize) -> usize {
    let entries_per_limb = entries_per_limb_const::<P>();
    (dim + entries_per_limb - 1) / entries_per_limb
}

/// Return the `Range<usize>` starting at the index of the limb containing the `start`th entry, and
/// ending at the index of the limb containing the `end`th entry (including the latter).
pub(crate) const fn range<const P: u32>(start: usize, end: usize) -> Range<usize> {
    let min = limb_bit_index_pair::<P>(start).limb;
    let max = if end > 0 {
        limb_bit_index_pair::<P>(end - 1).limb + 1
    } else {
        0
    };
    min..max
}

pub(crate) const fn sign_rule(mut target: Limb, mut source: Limb) -> u32 {
    let mut result = 0;
    let mut n = 1;
    // Empirically, the compiler unrolls this loop because BITS_PER_LIMB is a constant.
    while 2 * n < BITS_PER_LIMB {
        // This is 1 every 2n bits.
        let mask: Limb = !0 / ((1 << (2 * n)) - 1);
        result ^= (mask & (source >> n) & target).count_ones() % 2;
        source = source ^ (source >> n);
        target = target ^ (target >> n);
        n *= 2;
    }
    result ^= (1 & (source >> (BITS_PER_LIMB / 2)) & target) as u32;
    result
}

/// Return either `Some(sum)` if no carries happen in the limb, or `None` if some carry does happen.
pub(crate) fn truncate<const P: u32>(sum: Limb) -> Option<Limb> {
    if is_reduced::<P>(sum) {
        Some(sum)
    } else {
        None
    }
}
