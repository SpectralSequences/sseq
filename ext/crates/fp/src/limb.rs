use std::ops::Range;

pub(crate) use crate::constants::Limb;
use crate::{constants::BITS_PER_LIMB, prime::Prime};

/// A struct containing the information required to access a specific entry in an array of `Limb`s.
#[derive(Copy, Clone)]
pub(crate) struct LimbBitIndexPair {
    pub(crate) limb: usize,
    pub(crate) bit_index: usize,
}

/// Return the number of bits an element of $\mathbb{F}_P$ occupies in a limb.
pub(crate) fn bit_length<P: Prime>(p: P) -> usize {
    let p = p.as_u32() as u64;
    match p {
        2 => 1,
        _ => (BITS_PER_LIMB as u32 - (p * (p - 1)).leading_zeros()) as usize,
    }
}

/// If `l` is a limb of elements of $\\mathbb{F}_p$, then `l & bitmask::<P>()` is the value of the
/// first entry of `l`.
pub(crate) fn bitmask<P: Prime>(p: P) -> Limb {
    (1 << bit_length(p)) - 1
}

// this function is never called if `odd-primes` is disabled
#[allow(dead_code)]
/// The number of elements of $\\mathbb{F}_p$ that fit in a single limb.
pub(crate) fn entries_per_limb<P: Prime>(p: P) -> usize {
    BITS_PER_LIMB / bit_length(p)
}

pub(crate) fn limb_bit_index_pair<P: Prime>(p: P, idx: usize) -> LimbBitIndexPair {
    LimbBitIndexPair {
        limb: idx / entries_per_limb(p),
        bit_index: (idx % entries_per_limb(p) * bit_length(p)),
    }
}

/// Return the `Limb` whose `i`th entry is `limb_a[i] + coeff * limb_b[i]` mod P. Both `limb_a` and
/// `limb_b` are assumed to be reduced.
pub(crate) fn add<P: Prime>(p: P, limb_a: Limb, limb_b: Limb, coeff: u32) -> Limb {
    if p == 2 {
        limb_a ^ (coeff as Limb * limb_b)
    } else {
        limb_a + (coeff as Limb) * limb_b
    }
}

/// Return the `Limb` whose entries are the entries of `limb` reduced modulo `P`.
///
/// Contributed by Robert Burklund.
pub(crate) fn reduce<P: Prime>(p: P, limb: Limb) -> Limb {
    match p.as_u32() {
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
        _ => pack(p, unpack(p, limb).map(|x| (x % (p.as_u32() as u64)) as u32)),
    }
}

/// Check whether or not a limb is reduced, i.e. whether every entry is a value in the range `0..P`.
/// This is currently **not** faster than calling [`reduce`] directly.
pub(crate) fn is_reduced<P: Prime>(p: P, limb: Limb) -> bool {
    limb == reduce(p, limb)
}

/// Given an interator of `u32`'s, pack all of them into a single limb in order.
/// It is assumed that
///  - The values of the iterator are less than P
///  - The values of the iterator fit into a single limb
///
/// If these assumptions are violated, the result will be nonsense.
pub(crate) fn pack<T: Iterator<Item = u32>, P: Prime>(p: P, entries: T) -> Limb {
    let bit_length = bit_length(p);
    let mut result: Limb = 0;
    let mut shift = 0;
    for entry in entries {
        result += (entry as Limb) << shift;
        shift += bit_length;
    }
    result
}

/// Give an iterator over the entries of `limb`. We return `u64` instead of `u32` because the
/// entries might not be reduced. In general we can only guarantee that the entries are less than p
/// * (p - 1), so they might not fit into a single `u32`.
pub(crate) fn unpack<P: Prime>(p: P, mut limb: Limb) -> impl Iterator<Item = u64> {
    let entries = entries_per_limb(p);
    let bit_length = bit_length(p);
    let bit_mask = bitmask(p);

    (0..entries).map(move |_| {
        let result = limb & bit_mask;
        limb >>= bit_length;
        result
    })
}

/// Return the number of limbs required to hold `dim` entries.
pub(crate) fn number<P: Prime>(p: P, dim: usize) -> usize {
    if dim == 0 {
        0
    } else {
        limb_bit_index_pair(p, dim - 1).limb + 1
    }
}

/// Return the `Range<usize>` starting at the index of the limb containing the `start`th entry, and
/// ending at the index of the limb containing the `end`th entry (including the latter).
pub(crate) fn range<P: Prime>(p: P, start: usize, end: usize) -> Range<usize> {
    let min = limb_bit_index_pair(p, start).limb;
    let max = if end > 0 {
        limb_bit_index_pair(p, end - 1).limb + 1
    } else {
        0
    };
    min..max
}

pub(crate) fn sign_rule(mut target: Limb, mut source: Limb) -> u32 {
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
pub(crate) fn truncate<P: Prime>(p: P, sum: Limb) -> Option<Limb> {
    if is_reduced(p, sum) {
        Some(sum)
    } else {
        None
    }
}
