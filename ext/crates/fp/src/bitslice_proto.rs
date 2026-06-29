//! **PHASE 0 PROTOTYPE — to be removed in Phase 5.**
//!
//! A standalone, self-contained prototype of bit-sliced (bit-plane) storage for vectors
//! over a prime field `F_p`. This is *not* wired into [`crate::vector::FqVector`]; it
//! exists only to validate the performance claim before the larger refactor (see the
//! approved plan). It deliberately re-implements the minimum needed to benchmark the
//! `add`/`scale` kernels against the existing packed representation.
//!
//! # Layout
//!
//! An element of `F_p` is represented with `k = ceil(log2 p)` bits. A *group* of 64
//! elements occupies `k` consecutive [`Limb`]s (the *planes*): plane `j` of a group holds
//! bit `j` of all 64 elements, with element `i` living at bit `i` of each plane. A vector
//! of length `len` has `ceil(len / 64)` groups, so `k * ceil(len / 64)` limbs total.
//!
//! # Arithmetic
//!
//! - The **generic** kernels work for any prime: addition is a ripple-carry adder over the
//!   `k` planes followed by a single conditional subtraction of `p` (the sum of two reduced
//!   values is `< 2p`), and scalar multiplication is double-and-add with modular reduction
//!   at each step. No lookup tables, fully branch-free, operating on 64 lanes at once.
//! - The **F3 fast path** uses a hand-written 2-plane circuit (addition and negation),
//!   demonstrating the kind of speedup a per-prime specialization can give.

#![allow(dead_code)]

use crate::{constants::BITS_PER_LIMB, limb::Limb};

/// Maximum number of bit-planes (`k`) the prototype supports. `k = ceil(log2 p)`, so this
/// covers primes up to `2^24` — plenty for the benchmark, which only needs a handful of
/// representative primes.
const MAX_K: usize = 24;

/// Number of field elements packed into one group.
const ENTRIES_PER_GROUP: usize = BITS_PER_LIMB; // 64

/// `k = ceil(log2 p)`: the number of bit-planes needed to store an element of `F_p`.
const fn bit_planes(p: u32) -> usize {
    // Smallest k with 2^k >= p.
    let mut k = 0;
    while (1u64 << k) < p as u64 {
        k += 1;
    }
    if k == 0 { 1 } else { k }
}

/// A vector over `F_p` in bit-sliced layout. Prototype only.
#[derive(Clone, Debug)]
pub struct BitSlicedVec {
    p: u32,
    k: usize,
    len: usize,
    /// `k * ceil(len / 64)` limbs, group-major: group `g`'s plane `j` is `limbs[g * k + j]`.
    limbs: Vec<Limb>,
}

impl BitSlicedVec {
    pub fn new(p: u32, len: usize) -> Self {
        let k = bit_planes(p);
        let groups = len.div_ceil(ENTRIES_PER_GROUP);
        Self {
            p,
            k,
            len,
            limbs: vec![0; k * groups],
        }
    }

    pub fn from_u32(p: u32, data: &[u32]) -> Self {
        let mut v = Self::new(p, data.len());
        for (i, &value) in data.iter().enumerate() {
            v.set_entry(i, value);
        }
        v
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn num_groups(&self) -> usize {
        self.len.div_ceil(ENTRIES_PER_GROUP)
    }

    pub fn entry(&self, index: usize) -> u32 {
        debug_assert!(index < self.len);
        let group = index / ENTRIES_PER_GROUP;
        let lane = index % ENTRIES_PER_GROUP;
        let base = group * self.k;
        let mut value = 0u32;
        for j in 0..self.k {
            let bit = (self.limbs[base + j] >> lane) & 1;
            value |= (bit as u32) << j;
        }
        value
    }

    pub fn set_entry(&mut self, index: usize, value: u32) {
        debug_assert!(index < self.len);
        debug_assert!(value < self.p);
        let group = index / ENTRIES_PER_GROUP;
        let lane = index % ENTRIES_PER_GROUP;
        let base = group * self.k;
        for j in 0..self.k {
            let bit = ((value >> j) & 1) as Limb;
            let mask = 1 << lane;
            let plane = &mut self.limbs[base + j];
            *plane = (*plane & !mask) | (bit << lane);
        }
    }

    pub fn to_u32(&self) -> Vec<u32> {
        (0..self.len).map(|i| self.entry(i)).collect()
    }

    /// Bits of `p` as full-width lane masks (`pbits[j]` is all-ones iff bit `j` of `p` is set),
    /// for `j` in `0..=k`. Since `p < 2^k` (except `p = 2`), bit `k` is normally zero.
    fn p_masks(&self) -> [Limb; MAX_K + 1] {
        let mut masks = [0; MAX_K + 1];
        for (j, m) in masks.iter_mut().enumerate().take(self.k + 1) {
            *m = if (self.p >> j) & 1 == 1 { !0 } else { 0 };
        }
        masks
    }

    /// `self += c * other` (mod p), generic kernel for any prime.
    ///
    /// Dispatches on the number of planes `k` to a const-generic implementation, so the
    /// per-group arithmetic uses exactly-`K`-sized stack arrays and fully-unrolled loops
    /// tuned to the prime, rather than runtime-bounded loops over heap scratch. `k` only
    /// takes a handful of values (`k = ceil(log2 p)`), so the dispatch covers them directly
    /// and falls back to a heap path only for very large primes.
    pub fn add_generic(&mut self, other: &Self, c: u32) {
        assert_eq!(self.p, other.p);
        assert_eq!(self.len, other.len);
        if c == 0 {
            return;
        }
        let p_masks = self.p_masks();
        macro_rules! dispatch {
            ($($k:literal),*) => {
                match self.k {
                    $($k => add_groups_k::<$k>(&mut self.limbs, &other.limbs, c, &p_masks),)*
                    _ => self.add_generic_dyn(other, c, &p_masks),
                }
            };
        }
        dispatch!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
    }

    /// `self *= c` (mod p), generic kernel. See [`add_generic`](Self::add_generic) for the
    /// const-generic dispatch rationale.
    pub fn scale_generic(&mut self, c: u32) {
        if c == 1 {
            return;
        }
        if c == 0 {
            for limb in &mut self.limbs {
                *limb = 0;
            }
            return;
        }
        let p_masks = self.p_masks();
        macro_rules! dispatch {
            ($($k:literal),*) => {
                match self.k {
                    $($k => scale_groups_k::<$k>(&mut self.limbs, c, &p_masks),)*
                    _ => self.scale_generic_dyn(c, &p_masks),
                }
            };
        }
        dispatch!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
    }

    /// Heap-scratch fallback for `add_generic` when `k` exceeds the const-dispatch range.
    fn add_generic_dyn(&mut self, other: &Self, c: u32, p_masks: &[Limb]) {
        let k = self.k;
        let mut s = vec![0; k + 1];
        let mut d = vec![0; k + 1];
        let mut acc = vec![0; k];
        let mut temp = vec![0; k];
        for g in 0..self.num_groups() {
            let base = g * k;
            let b = &other.limbs[base..base + k];
            if c == 1 {
                add_mod_into(&mut self.limbs[base..base + k], b, p_masks, &mut s, &mut d);
            } else {
                scalar_mul_into(&mut acc, b, c, p_masks, &mut temp, &mut s, &mut d);
                add_mod_into(&mut self.limbs[base..base + k], &acc, p_masks, &mut s, &mut d);
            }
        }
    }

    /// Heap-scratch fallback for `scale_generic` when `k` exceeds the const-dispatch range.
    fn scale_generic_dyn(&mut self, c: u32, p_masks: &[Limb]) {
        let k = self.k;
        let mut s = vec![0; k + 1];
        let mut d = vec![0; k + 1];
        let mut acc = vec![0; k];
        let mut temp = vec![0; k];
        for g in 0..self.num_groups() {
            let base = g * k;
            scalar_mul_into(
                &mut acc,
                &self.limbs[base..base + k],
                c,
                p_masks,
                &mut temp,
                &mut s,
                &mut d,
            );
            self.limbs[base..base + k].copy_from_slice(&acc);
        }
    }

    /// `self += c * other` (mod 3) using the hand-written F3 circuit. Requires `p == 3`.
    pub fn add_f3(&mut self, other: &Self, c: u32) {
        assert_eq!(self.p, 3);
        assert_eq!(self.k, 2);
        assert_eq!(self.len, other.len);
        if c == 0 {
            return;
        }
        for g in 0..self.num_groups() {
            let base = g * 2;
            let (a_lo, a_hi) = (self.limbs[base], self.limbs[base + 1]);
            let (mut b_lo, mut b_hi) = (other.limbs[base], other.limbs[base + 1]);
            if c == 2 {
                // Multiply other by 2 = negate: in the (hi, lo) encoding, negation swaps planes.
                std::mem::swap(&mut b_lo, &mut b_hi);
            }
            let (c_lo, c_hi) = f3_add(a_lo, a_hi, b_lo, b_hi);
            self.limbs[base] = c_lo;
            self.limbs[base + 1] = c_hi;
        }
    }

    /// `self *= c` (mod 3) using the F3 circuit. Requires `p == 3`.
    pub fn scale_f3(&mut self, c: u32) {
        assert_eq!(self.p, 3);
        if c == 1 {
            return;
        }
        if c == 0 {
            for limb in &mut self.limbs {
                *limb = 0;
            }
            return;
        }
        // c == 2: negate = swap the two planes of every group.
        for g in 0..self.num_groups() {
            let base = g * 2;
            self.limbs.swap(base, base + 1);
        }
    }
}

/// `dst += b` (mod p), where `dst` (the augend) and `b` are each `k = dst.len()` reduced
/// planes with independent lanes. `s`/`d` are reusable `(k+1)`-limb scratch buffers.
///
/// Ripple-carry adder over the `k` planes gives a `(k+1)`-bit sum in `[0, 2p)`, then a
/// single conditional subtraction of `p` brings each lane back into `[0, p)`.
#[inline]
fn add_mod_into(dst: &mut [Limb], b: &[Limb], p_masks: &[Limb], s: &mut [Limb], d: &mut [Limb]) {
    let k = dst.len();
    // s = dst + b as a (k+1)-bit number. `dst` is only written in the final select pass,
    // so passing `dst` as `b` (in-place doubling) is sound — handled by `double_mod_into`.
    let mut carry: Limb = 0;
    for j in 0..k {
        let aj = dst[j];
        let bj = b[j];
        let axb = aj ^ bj;
        s[j] = axb ^ carry;
        carry = (aj & bj) | (carry & axb);
    }
    s[k] = carry;

    conditional_subtract(dst, p_masks, s, d);
}

/// `dst = 2 * dst` (mod p). Doubling is a one-position plane shift (`s = dst << 1`) followed
/// by the conditional subtraction of `p`.
#[inline]
fn double_mod_into(dst: &mut [Limb], p_masks: &[Limb], s: &mut [Limb], d: &mut [Limb]) {
    let k = dst.len();
    s[0] = 0;
    for j in 1..=k {
        s[j] = dst[j - 1];
    }
    conditional_subtract(dst, p_masks, s, d);
}

/// Given a `(k+1)`-bit unreduced sum `s` in `[0, 2p)`, write `s mod p` into the `k` planes
/// of `dst`. `d` is `(k+1)`-limb scratch for the trial difference `s - p`.
#[inline]
fn conditional_subtract(dst: &mut [Limb], p_masks: &[Limb], s: &[Limb], d: &mut [Limb]) {
    let k = dst.len();
    // d = s - p over k+1 bits; the borrow-out marks lanes where s < p.
    let mut borrow: Limb = 0;
    for j in 0..=k {
        let sj = s[j];
        let pj = p_masks[j];
        let sxp = sj ^ pj;
        d[j] = sxp ^ borrow;
        borrow = (!sj & pj) | (borrow & !sxp);
    }
    let ge = !borrow; // lanes where s >= p
    // result = ge ? d : s, taking the low k planes (result < p < 2^k).
    for j in 0..k {
        dst[j] = (d[j] & ge) | (s[j] & !ge);
    }
}

/// `acc = c * b` (mod p) for a constant scalar `c`, via double-and-add with modular
/// reduction. `temp`/`s`/`d` are reusable scratch (`temp` is `k` limbs, `s`/`d` are `k+1`).
#[inline]
fn scalar_mul_into(
    acc: &mut [Limb],
    b: &[Limb],
    c: u32,
    p_masks: &[Limb],
    temp: &mut [Limb],
    s: &mut [Limb],
    d: &mut [Limb],
) {
    temp.copy_from_slice(b);
    acc.fill(0);
    let mut cc = c;
    loop {
        if cc & 1 == 1 {
            add_mod_into(acc, temp, p_masks, s, d);
        }
        cc >>= 1;
        if cc == 0 {
            break;
        }
        double_mod_into(temp, p_masks, s, d);
    }
}

// ---------------------------------------------------------------------------------------
// Const-generic kernels: `K` planes known at compile time, so every array is exactly sized
// and every loop is fully unrolled. Selected by a runtime dispatch on `k = ceil(log2 p)`.
// ---------------------------------------------------------------------------------------

/// Reduce a `(K+1)`-bit unreduced sum (`s` low planes + `s_top`) in `[0, 2p)` to `s mod p`.
#[inline(always)]
fn cond_sub_k<const K: usize>(s: &[Limb; K], s_top: Limb, p_masks: &[Limb]) -> [Limb; K] {
    let mut d = [0 as Limb; K];
    let mut borrow: Limb = 0;
    for j in 0..K {
        let sj = s[j];
        let pj = p_masks[j];
        let sxp = sj ^ pj;
        d[j] = sxp ^ borrow;
        borrow = (!sj & pj) | (borrow & !sxp);
    }
    // Top bit only affects the borrow-out (the result fits in K planes since result < p).
    let pj = p_masks[K];
    let sxp = s_top ^ pj;
    borrow = (!s_top & pj) | (borrow & !sxp);
    let ge = !borrow;
    let mut out = [0 as Limb; K];
    for j in 0..K {
        out[j] = (d[j] & ge) | (s[j] & !ge);
    }
    out
}

/// `(a + b) mod p` over `K` planes.
#[inline(always)]
fn add_mod_k<const K: usize>(a: &[Limb; K], b: &[Limb; K], p_masks: &[Limb]) -> [Limb; K] {
    let mut s = [0 as Limb; K];
    let mut carry: Limb = 0;
    for j in 0..K {
        let aj = a[j];
        let bj = b[j];
        let axb = aj ^ bj;
        s[j] = axb ^ carry;
        carry = (aj & bj) | (carry & axb);
    }
    cond_sub_k::<K>(&s, carry, p_masks)
}

/// `(2 * a) mod p` over `K` planes (doubling is a one-position plane shift).
#[inline(always)]
fn double_mod_k<const K: usize>(a: &[Limb; K], p_masks: &[Limb]) -> [Limb; K] {
    let mut s = [0 as Limb; K];
    for j in 1..K {
        s[j] = a[j - 1];
    }
    let s_top = a[K - 1];
    cond_sub_k::<K>(&s, s_top, p_masks)
}

/// `(c * b) mod p` over `K` planes, via double-and-add.
#[inline(always)]
fn scalar_mul_k<const K: usize>(b: &[Limb; K], c: u32, p_masks: &[Limb]) -> [Limb; K] {
    let mut result = [0 as Limb; K];
    let mut temp = *b;
    let mut cc = c;
    loop {
        if cc & 1 == 1 {
            result = add_mod_k::<K>(&result, &temp, p_masks);
        }
        cc >>= 1;
        if cc == 0 {
            break;
        }
        temp = double_mod_k::<K>(&temp, p_masks);
    }
    result
}

/// `dst += c * src` (mod p) over all groups, with `K` planes per group.
#[inline]
fn add_groups_k<const K: usize>(dst: &mut [Limb], src: &[Limb], c: u32, p_masks: &[Limb]) {
    for (dg, sg) in dst.chunks_exact_mut(K).zip(src.chunks_exact(K)) {
        let mut a = [0 as Limb; K];
        let mut b = [0 as Limb; K];
        a.copy_from_slice(dg);
        b.copy_from_slice(sg);
        let addend = if c == 1 { b } else { scalar_mul_k::<K>(&b, c, p_masks) };
        let sum = add_mod_k::<K>(&a, &addend, p_masks);
        dg.copy_from_slice(&sum);
    }
}

/// `dst *= c` (mod p) over all groups, with `K` planes per group.
#[inline]
fn scale_groups_k<const K: usize>(dst: &mut [Limb], c: u32, p_masks: &[Limb]) {
    for dg in dst.chunks_exact_mut(K) {
        let mut a = [0 as Limb; K];
        a.copy_from_slice(dg);
        let scaled = scalar_mul_k::<K>(&a, c, p_masks);
        dg.copy_from_slice(&scaled);
    }
}

/// F3 addition circuit on the `(lo, hi)` plane encoding (`value = 2*hi + lo`).
///
/// Output `c == 1` exactly for input value pairs `{(0,1),(1,0),(2,2)}` and `c == 2` for
/// `{(0,2),(1,1),(2,0)}`; everything else is `0`. The invalid encoding `(hi,lo) = (1,1)`
/// never occurs for reduced inputs.
#[inline]
fn f3_add(a_lo: Limb, a_hi: Limb, b_lo: Limb, b_hi: Limb) -> (Limb, Limb) {
    let is0_a = !(a_lo | a_hi);
    let is1_a = a_lo;
    let is2_a = a_hi;
    let is0_b = !(b_lo | b_hi);
    let is1_b = b_lo;
    let is2_b = b_hi;

    let c_lo = (is0_a & is1_b) | (is1_a & is0_b) | (is2_a & is2_b);
    let c_hi = (is0_a & is2_b) | (is1_a & is1_b) | (is2_a & is0_b);
    (c_lo, c_hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIMES: [u32; 6] = [2, 3, 5, 7, 251, 65521];

    #[test]
    fn bit_planes_correct() {
        assert_eq!(bit_planes(2), 1);
        assert_eq!(bit_planes(3), 2);
        assert_eq!(bit_planes(5), 3);
        assert_eq!(bit_planes(7), 3);
        assert_eq!(bit_planes(251), 8);
        assert_eq!(bit_planes(65521), 16);
    }

    #[test]
    fn pack_unpack_roundtrip() {
        for p in PRIMES {
            for len in [0, 1, 63, 64, 65, 130, 1000] {
                let data: Vec<u32> = (0..len).map(|i| (i as u32 * 7 + 1) % p).collect();
                let v = BitSlicedVec::from_u32(p, &data);
                assert_eq!(v.to_u32(), data, "p={p} len={len}");
            }
        }
    }

    /// Exhaustively check the generic add kernel against `(a + c*b) % p` for every pair of
    /// field elements and every scalar.
    #[test]
    fn generic_add_exhaustive() {
        for p in PRIMES {
            // Use one lane per (a, b) so a single group covers all pairs (p <= 64 cases for
            // small primes; for large primes sample instead).
            let pairs: Vec<(u32, u32)> = if p * p <= 64 {
                (0..p).flat_map(|a| (0..p).map(move |b| (a, b))).collect()
            } else {
                // Sample a spread of pairs into 64 lanes.
                (0..64u32)
                    .map(|i| ((i.wrapping_mul(2654435761) % p), (i.wrapping_mul(40503) % p)))
                    .collect()
            };
            for c in 0..p {
                let a_data: Vec<u32> = pairs.iter().map(|&(a, _)| a).collect();
                let b_data: Vec<u32> = pairs.iter().map(|&(_, b)| b).collect();
                let mut va = BitSlicedVec::from_u32(p, &a_data);
                let vb = BitSlicedVec::from_u32(p, &b_data);
                va.add_generic(&vb, c);
                let got = va.to_u32();
                for (idx, &(a, b)) in pairs.iter().enumerate() {
                    let expected = (a + c * b) % p;
                    assert_eq!(got[idx], expected, "p={p} a={a} b={b} c={c}");
                }
            }
        }
    }

    #[test]
    fn generic_scale_exhaustive() {
        for p in PRIMES {
            let data: Vec<u32> = (0..64).map(|i| (i as u32) % p).collect();
            for c in 0..p {
                let mut v = BitSlicedVec::from_u32(p, &data);
                v.scale_generic(c);
                let got = v.to_u32();
                for (i, &x) in data.iter().enumerate() {
                    assert_eq!(got[i], (x * c) % p, "p={p} x={x} c={c}");
                }
            }
        }
    }

    /// The F3 circuit must agree with `(a + c*b) % 3` for all inputs.
    #[test]
    fn f3_add_exhaustive() {
        let p = 3;
        let pairs: Vec<(u32, u32)> = (0..p).flat_map(|a| (0..p).map(move |b| (a, b))).collect();
        for c in 0..p {
            let a_data: Vec<u32> = pairs.iter().map(|&(a, _)| a).collect();
            let b_data: Vec<u32> = pairs.iter().map(|&(_, b)| b).collect();
            let mut va = BitSlicedVec::from_u32(p, &a_data);
            let vb = BitSlicedVec::from_u32(p, &b_data);
            va.add_f3(&vb, c);
            let got = va.to_u32();
            for (idx, &(a, b)) in pairs.iter().enumerate() {
                assert_eq!(got[idx], (a + c * b) % p, "a={a} b={b} c={c}");
            }
        }
    }

    #[test]
    fn f3_scale_exhaustive() {
        let data: Vec<u32> = (0..64).map(|i| (i as u32) % 3).collect();
        for c in 0..3 {
            let mut v = BitSlicedVec::from_u32(3, &data);
            v.scale_f3(c);
            let got = v.to_u32();
            for (i, &x) in data.iter().enumerate() {
                assert_eq!(got[i], (x * c) % 3, "x={x} c={c}");
            }
        }
    }

    /// The F3 fast path and the generic kernel must produce identical results on long vectors.
    #[test]
    fn f3_fast_matches_generic() {
        let len = 1000;
        let a_data: Vec<u32> = (0..len).map(|i| (i as u32 * 2 + 1) % 3).collect();
        let b_data: Vec<u32> = (0..len).map(|i| (i as u32 * 5 + 2) % 3).collect();
        for c in 0..3 {
            let mut fast = BitSlicedVec::from_u32(3, &a_data);
            let mut generic = BitSlicedVec::from_u32(3, &a_data);
            let b = BitSlicedVec::from_u32(3, &b_data);
            fast.add_f3(&b, c);
            generic.add_generic(&b, c);
            assert_eq!(fast.to_u32(), generic.to_u32(), "c={c}");
        }
    }
}
