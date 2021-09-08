use std::ops::Range;

pub(crate) use crate::constants::Limb;

use crate::{
    constants::{
        BITMASKS, BITS_PER_LIMB, BIT_LENGTHS, ENTRIES_PER_LIMB, MAX_LEN, PRIME_TO_INDEX_MAP,
    },
    prime::ValidPrime,
};

#[derive(Copy, Clone)]
pub(crate) struct LimbBitIndexPair {
    pub(crate) limb: usize,
    pub(crate) bit_index: usize,
}

pub(crate) const fn bit_length<const P: u32>() -> usize {
    BIT_LENGTHS[PRIME_TO_INDEX_MAP[P as usize]]
}

/// TODO: Would it be simpler to just compute this at "runtime"? It's going to be inlined anyway.
pub(crate) const fn bitmask<const P: u32>() -> Limb {
    BITMASKS[PRIME_TO_INDEX_MAP[P as usize]]
}

// this function is never called if `odd-primes` is disabled
#[allow(dead_code)]
pub(crate) const fn entries_per_limb(p: ValidPrime) -> usize {
    ENTRIES_PER_LIMB[PRIME_TO_INDEX_MAP[p.value() as usize]]
}

pub(crate) const fn entries_per_limb_const<const P: u32>() -> usize {
    ENTRIES_PER_LIMB[PRIME_TO_INDEX_MAP[P as usize]]
}

pub(crate) const fn limb_bit_index_pair<const P: u32>(idx: usize) -> LimbBitIndexPair {
    match P {
        2 => LimbBitIndexPair {
            limb: idx / BITS_PER_LIMB,
            bit_index: idx % BITS_PER_LIMB,
        },
        _ => {
            LimbBitIndexPair {
                limb: idx / entries_per_limb_const::<P>(),
                bit_index: (idx % entries_per_limb_const::<P>() * bit_length::<P>()),
            }
            // let prime_idx = PRIME_TO_INDEX_MAP[*p as usize];
            // debug_assert!(idx < MAX_LEN);
            // unsafe {
            //     let table = &LIMB_BIT_INDEX_TABLE[prime_idx];
            //     debug_assert!(table.is_some());
            //     *table
            //         .as_ref()
            //         .unwrap_or_else(|| std::hint::unreachable_unchecked())
            //         .get_unchecked(idx)
            // }
        }
    }
}

// /// This table tells us which limb and which bitfield of that limb to look for a given index of
// /// the vector in.
// static mut LIMB_BIT_INDEX_TABLE: [Option<Vec<LimbBitIndexPair>>; NUM_PRIMES] =
//     [None, None, None, None, None, None, None, None];

// static mut LIMB_BIT_INDEX_ONCE_TABLE: [Once; NUM_PRIMES] = [
//     Once::new(),
//     Once::new(),
//     Once::new(),
//     Once::new(),
//     Once::new(),
//     Once::new(),
//     Once::new(),
//     Once::new(),
// ];

// pub fn initialize_limb_bit_index_table(p: ValidPrime) {
//     if *p == 2 {
//         return;
//     }
//     unsafe {
//         LIMB_BIT_INDEX_ONCE_TABLE[PRIME_TO_INDEX_MAP[*p as usize]].call_once(|| {
//             let entries_per_limb = entries_per_limb(p);
//             let bit_length = bit_length(p);
//             let mut table: Vec<LimbBitIndexPair> = Vec::with_capacity(MAX_LEN);
//             for i in 0..MAX_LEN {
//                 table.push(LimbBitIndexPair {
//                     limb: i / entries_per_limb,
//                     bit_index: (i % entries_per_limb) * bit_length,
//                 })
//             }
//             LIMB_BIT_INDEX_TABLE[PRIME_TO_INDEX_MAP[*p as usize]] = Some(table);
//         });
//     }
// }

pub(crate) const fn add<const P: u32>(limb_a: Limb, limb_b: Limb, coeff: u32) -> Limb {
    if P == 2 {
        limb_a ^ (coeff as Limb * limb_b)
    } else {
        limb_a + (coeff as Limb) * limb_b
    }
}

/// Contbuted by Robert Burklund
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

pub(crate) fn is_reduced<const P: u32>(limb: Limb) -> bool {
    limb == reduce::<P>(limb)
}

/// Given an interator of u32's, pack all of them into a single limb in order.
/// It is assumed that
///  - The values of the iterator are less than P
///  - The values of the iterator fit into a single limb
///
/// If these assumptions are violated, the result will be nonsense.
pub(crate) fn pack<T: Iterator<Item = u32>, const P: u32>(entries: T) -> Limb {
    let bit_length = bit_length::<P>();
    let mut result: Limb = 0;
    let mut shift = 0;
    for entry in entries {
        result += (entry as Limb) << shift;
        shift += bit_length;
    }
    result
}

/// Give an iterator over the entries of a limb.
pub(crate) fn unpack<const P: u32>(mut limb: Limb) -> impl Iterator<Item = u32> {
    let entries = entries_per_limb_const::<P>();
    let bit_length = bit_length::<P>();
    let bit_mask = bitmask::<P>();

    (0..entries).map(move |_| {
        let result = (limb & bit_mask) as u32;
        limb >>= bit_length;
        result
    })
}

pub(crate) fn number<const P: u32>(dim: usize) -> usize {
    debug_assert!(dim < MAX_LEN);
    if dim == 0 {
        0
    } else {
        limb_bit_index_pair::<P>(dim - 1).limb + 1
    }
}

pub(crate) fn range<const P: u32>(start: usize, end: usize) -> Range<usize> {
    let min = limb_bit_index_pair::<P>(start).limb;
    let max = if end > 0 {
        limb_bit_index_pair::<P>(end - 1).limb + 1
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

/// Returns: either Some(sum) if no carries happen in the limb or None if some carry does
/// happen.
pub(crate) fn truncate<const P: u32>(sum: Limb) -> Option<Limb> {
    if is_reduced::<P>(sum) {
        Some(sum)
    } else {
        None
    }
}
