//! Bit-sliced arithmetic kernels for prime fields.
//!
//! In the bit-sliced layout, a group of [`BITS_PER_LIMB`] (64) field elements occupies
//! `k = ceil(log2 p)` consecutive limbs (the *planes*): plane `j` holds bit `j` of all 64
//! elements, with element `i` living at bit `i` of each plane. Addition and scalar
//! multiplication then reduce to short branch-free boolean circuits over the planes that
//! act on 64 lanes at once, with no separate reduction step.
//!
//! Addition is a ripple-carry adder over the `k` planes (producing a `(k+1)`-bit sum in
//! `[0, 2p)`) followed by a single conditional subtraction of `p`. Scalar multiplication is
//! double-and-add with a modular reduction at each step. The number of planes `k` is
//! dispatched to a const-generic implementation so that, for each prime, the arrays are
//! exactly sized and the loops fully unrolled; a heap-scratch fallback covers the rare
//! primes with `k` beyond the dispatch range.

use crate::{constants::BITS_PER_LIMB, limb::Limb};

/// Largest `k` that the const-generic dispatch covers directly (`p < 2^16`). Larger primes
/// fall back to the heap-scratch path.
const MAX_DISPATCH_K: usize = 16;

/// The number of planes `k = ceil(log2 p)` needed to bit-slice an element of `F_p`.
pub(crate) fn planes(p: u32) -> usize {
    debug_assert!(p >= 2);
    (u32::BITS - (p - 1).leading_zeros()) as usize
}

/// The bits of `p` as full-width lane masks: `out[j]` is all-ones iff bit `j` of `p` is set,
/// for `j` in `0..=k`.
fn p_masks(p: u32, k: usize) -> [Limb; BITS_PER_LIMB + 1] {
    let mut masks = [0; BITS_PER_LIMB + 1];
    for (i, m) in masks.iter_mut().enumerate().take(k + 1) {
        if (p >> i) & 1 == 1 {
            *m = !0;
        }
    }
    masks
}

/// The full-width lane mask for bit `j` of `p`: all-ones if set, zero otherwise.
#[inline(always)]
fn pmask(p: u32, j: usize) -> Limb {
    // Widen bit `j` of `p` to the limb width, then broadcast it: `1 -> !0`, `0 -> 0` via
    // two's-complement negation.
    Limb::from((p >> j) & 1).wrapping_neg()
}

/// `dst += c * src` (mod p) over every group, where `dst` and `src` hold the same number of
/// whole groups of `k` planes. Assumes both are reduced; the result is reduced.
pub(crate) fn add_groups(p: u32, k: usize, dst: &mut [Limb], src: &[Limb], c: u32) {
    if c == 0 {
        return;
    }
    if p == 2 {
        // One plane (k = 1); the only nonzero scalar is 1, so addition is XOR.
        for (d, s) in dst.iter_mut().zip(src) {
            *d ^= *s;
        }
        return;
    }
    if p == 3 {
        return f3_add_groups(dst, src, c);
    }
    if p == 5 {
        return f5_add_groups(dst, src, c);
    }
    macro_rules! dispatch {
        ($($k:literal),*) => {
            match k {
                $($k => add_groups_k::<$k>(dst, src, c, p),)*
                _ => add_groups_dyn(k, dst, src, c, &p_masks(p, k)),
            }
        };
    }
    dispatch!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
}

/// `dst += c * src` (mod p) for a single group of `k` planes, restricted to the lanes set in
/// `lane_mask`. Lanes outside the mask are unchanged. `dst` and `src` are each exactly `k`
/// limbs.
pub(crate) fn add_group_masked(
    p: u32,
    k: usize,
    dst: &mut [Limb],
    src: &[Limb],
    c: u32,
    lane_mask: Limb,
) {
    if c == 0 {
        return;
    }
    if p == 2 {
        // One plane (k = 1); XOR in only the in-range lanes.
        dst[0] ^= src[0] & lane_mask;
        return;
    }
    if p == 3 {
        return f3_add_group_masked(dst, src, c, lane_mask);
    }
    if p == 5 {
        return f5_add_group_masked(dst, src, c, lane_mask);
    }
    macro_rules! dispatch {
        ($($k:literal),*) => {
            match k {
                $($k => add_group_masked_k::<$k>(dst, src, c, p, lane_mask),)*
                _ => {
                    // Masking `src` to the in-range lanes makes the circuit a no-op elsewhere.
                    let mut masked = vec![0; k];
                    for j in 0..k {
                        masked[j] = src[j] & lane_mask;
                    }
                    add_groups(p, k, dst, &masked, c);
                }
            }
        };
    }
    dispatch!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
}

/// `dst *= c` (mod p) over every group.
pub(crate) fn scale_groups(p: u32, k: usize, dst: &mut [Limb], c: u32) {
    if c == 1 {
        return;
    }
    if c == 0 {
        dst.fill(0);
        return;
    }
    if p == 3 {
        return f3_scale_groups(dst, c);
    }
    if p == 5 {
        return f5_scale_groups(dst, c);
    }
    macro_rules! dispatch {
        ($($k:literal),*) => {
            match k {
                $($k => scale_groups_k::<$k>(dst, c, p),)*
                _ => scale_groups_dyn(k, dst, c, &p_masks(p, k)),
            }
        };
    }
    dispatch!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
}

// ---------------------------------------------------------------------------------------
// Const-generic kernels: `K` planes known at compile time, so every array is exactly sized
// and every loop is fully unrolled.
// ---------------------------------------------------------------------------------------

/// Reduce a `(K+1)`-bit unreduced sum (`s` low planes + `s_top`) in `[0, 2p)` to `s mod p`.
/// The per-plane mask is computed inline from `p` (no per-call mask array).
#[inline(always)]
fn cond_sub_k<const K: usize>(s: &[Limb; K], s_top: Limb, p: u32) -> [Limb; K] {
    let mut d = [0 as Limb; K];
    let mut borrow: Limb = 0;
    for j in 0..K {
        let sj = s[j];
        let pj = pmask(p, j);
        let sxp = sj ^ pj;
        d[j] = sxp ^ borrow;
        borrow = (!sj & pj) | (borrow & !sxp);
    }
    // Top bit only affects the borrow-out (the result fits in K planes since result < p).
    let pj = pmask(p, K);
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
fn add_mod_k<const K: usize>(a: &[Limb; K], b: &[Limb; K], p: u32) -> [Limb; K] {
    let mut s = [0 as Limb; K];
    let mut carry: Limb = 0;
    for j in 0..K {
        let aj = a[j];
        let bj = b[j];
        let axb = aj ^ bj;
        s[j] = axb ^ carry;
        carry = (aj & bj) | (carry & axb);
    }
    cond_sub_k::<K>(&s, carry, p)
}

/// `(2 * a) mod p` over `K` planes (doubling is a one-position plane shift).
#[inline(always)]
fn double_mod_k<const K: usize>(a: &[Limb; K], p: u32) -> [Limb; K] {
    let mut s = [0 as Limb; K];
    s[1..K].copy_from_slice(&a[..K - 1]);
    let s_top = a[K - 1];
    cond_sub_k::<K>(&s, s_top, p)
}

/// `(c * b) mod p` over `K` planes, via double-and-add.
#[inline(always)]
fn scalar_mul_k<const K: usize>(b: &[Limb; K], c: u32, p: u32) -> [Limb; K] {
    let mut result = [0 as Limb; K];
    let mut temp = *b;
    let mut cc = c;
    loop {
        if cc & 1 == 1 {
            result = add_mod_k::<K>(&result, &temp, p);
        }
        cc >>= 1;
        if cc == 0 {
            break;
        }
        temp = double_mod_k::<K>(&temp, p);
    }
    result
}

#[inline]
fn add_groups_k<const K: usize>(dst: &mut [Limb], src: &[Limb], c: u32, p: u32) {
    for (dg, sg) in dst
        .as_chunks_mut::<K>()
        .0
        .iter_mut()
        .zip(src.as_chunks::<K>().0)
    {
        let addend = if c == 1 {
            *sg
        } else {
            scalar_mul_k::<K>(sg, c, p)
        };
        *dg = add_mod_k::<K>(dg, &addend, p);
    }
}

/// `dst += c * src` (mod p) for a single `K`-plane group, restricted to lanes in `lane_mask`.
#[inline]
fn add_group_masked_k<const K: usize>(
    dst: &mut [Limb],
    src: &[Limb],
    c: u32,
    p: u32,
    lane_mask: Limb,
) {
    let mut a = [0 as Limb; K];
    let mut b = [0 as Limb; K];
    for j in 0..K {
        a[j] = dst[j];
        b[j] = src[j] & lane_mask;
    }
    let addend = if c == 1 {
        b
    } else {
        scalar_mul_k::<K>(&b, c, p)
    };
    let sum = add_mod_k::<K>(&a, &addend, p);
    dst[..K].copy_from_slice(&sum);
}

#[inline]
fn scale_groups_k<const K: usize>(dst: &mut [Limb], c: u32, p: u32) {
    if c == 1 {
        return;
    }
    if c == 0 {
        dst.fill(0);
        return;
    }
    for dg in dst.as_chunks_mut::<K>().0 {
        *dg = scalar_mul_k::<K>(dg, c, p);
    }
}

// ---------------------------------------------------------------------------------------
// F3 specialization (k = 2). Plane 0 is the low bit, plane 1 the high bit, so an element
// `v in {0,1,2}` is stored as `(hi, lo)` with `v = 2*hi + lo`. Addition is a flat boolean
// circuit (no ripple-carry or borrow chain), and multiplication by 2 = negation just swaps
// the two planes — both avoid the sequential dependencies that make the generic circuit lose
// to the packed SWAR reduce at small primes.
// ---------------------------------------------------------------------------------------

/// `(a + b) mod 3` as a flat 6-gate circuit on the `(lo, hi)` planes (each lane independent).
///
/// Three parallel layers — two XORs, two XORs, two AND-NOTs — so it maps onto x86 `andn`
/// and has very short dependency chains. Verified exhaustively against the 9 valid input
/// pairs (the `(hi, lo) = (1, 1)` encoding never occurs for reduced inputs).
#[inline(always)]
fn f3_add_planes(a_lo: Limb, a_hi: Limb, b_lo: Limb, b_hi: Limb) -> (Limb, Limb) {
    let t_hi = a_hi ^ b_hi;
    let t_lo = a_lo ^ b_lo;
    let u_hi = b_hi ^ t_lo;
    let u_lo = b_lo ^ t_hi;
    let r_hi = u_lo & !t_lo;
    let r_lo = u_hi & !t_hi;
    (r_lo, r_hi)
}

/// Negation in F3 swaps 1 <-> 2 (and fixes 0), i.e. swaps the two planes.
#[inline(always)]
fn f3_addend(sg: &[Limb], c: u32) -> (Limb, Limb) {
    // c is 1 or 2 here; c == 2 means add (-other), i.e. negate by swapping planes.
    if c == 1 {
        (sg[0], sg[1])
    } else {
        (sg[1], sg[0])
    }
}

fn f3_add_groups(dst: &mut [Limb], src: &[Limb], c: u32) {
    for (dg, sg) in dst
        .as_chunks_mut::<2>()
        .0
        .iter_mut()
        .zip(src.as_chunks::<2>().0)
    {
        let (b_lo, b_hi) = f3_addend(sg, c);
        let (r_lo, r_hi) = f3_add_planes(dg[0], dg[1], b_lo, b_hi);
        dg[0] = r_lo;
        dg[1] = r_hi;
    }
}

fn f3_add_group_masked(dst: &mut [Limb], src: &[Limb], c: u32, lane_mask: Limb) {
    let (b_lo, b_hi) = f3_addend(src, c);
    // Masking the addend to the in-range lanes makes the circuit a no-op (adds 0) elsewhere.
    let (r_lo, r_hi) = f3_add_planes(dst[0], dst[1], b_lo & lane_mask, b_hi & lane_mask);
    dst[0] = r_lo;
    dst[1] = r_hi;
}

fn f3_scale_groups(dst: &mut [Limb], c: u32) {
    // c == 2 is negation (plane swap); c == 1 is a no-op; c == 0 is handled by the caller.
    if c == 2 {
        for dg in dst.as_chunks_mut::<2>().0 {
            dg.swap(0, 1);
        }
    }
}

// ---------------------------------------------------------------------------------------
// F5 specialization (k = 3). Planes are bits 0,1,2 of the value `v in {0,..,4}`. Both the
// add and the scalar multiply are built as flat "indicator" circuits — one-hot lane masks
// `is_v` for each operand value, recombined into the result with no carry/borrow chain — so
// they keep the wide instruction-level parallelism the sequential generic circuit loses.
// ---------------------------------------------------------------------------------------

/// One-hot lane masks: `out[v]` has the bits of the lanes whose value is `v` (for `v in 0..5`).
#[inline(always)]
fn f5_indicators(p0: Limb, p1: Limb, p2: Limb) -> [Limb; 5] {
    let n0 = !p0;
    let n1 = !p1;
    let n2 = !p2;
    [
        n0 & n1 & n2, // 0 = 000
        p0 & n1 & n2, // 1 = 001
        n0 & p1 & n2, // 2 = 010
        p0 & p1 & n2, // 3 = 011
        n0 & n1 & p2, // 4 = 100
    ]
}

/// Reassemble the three planes from per-value selection masks (`sel[v]` selects value `v`).
#[inline(always)]
fn f5_compose(sel: [Limb; 5]) -> (Limb, Limb, Limb) {
    // bit 0 set for values {1,3}; bit 1 for {2,3}; bit 2 for {4}.
    (sel[1] | sel[3], sel[2] | sel[3], sel[4])
}

/// `c * v mod 5` on the three planes (`c in 1..5`).
#[inline(always)]
fn f5_mul_planes(p0: Limb, p1: Limb, p2: Limb, c: u32) -> (Limb, Limb, Limb) {
    let ind = f5_indicators(p0, p1, p2);
    let mut sel = [0 as Limb; 5];
    for v in 0..5u32 {
        sel[((c * v) % 5) as usize] |= ind[v as usize];
    }
    f5_compose(sel)
}

/// `(a + b) mod 5` on the three planes, as a flat indicator circuit.
#[inline(always)]
fn f5_add_planes(a0: Limb, a1: Limb, a2: Limb, b0: Limb, b1: Limb, b2: Limb) -> (Limb, Limb, Limb) {
    let ia = f5_indicators(a0, a1, a2);
    let ib = f5_indicators(b0, b1, b2);
    let mut sel = [0 as Limb; 5];
    for av in 0..5usize {
        for bv in 0..5usize {
            sel[(av + bv) % 5] |= ia[av] & ib[bv];
        }
    }
    f5_compose(sel)
}

fn f5_add_groups(dst: &mut [Limb], src: &[Limb], c: u32) {
    for (dg, sg) in dst
        .as_chunks_mut::<3>()
        .0
        .iter_mut()
        .zip(src.as_chunks::<3>().0)
    {
        let (b0, b1, b2) = if c == 1 {
            (sg[0], sg[1], sg[2])
        } else {
            f5_mul_planes(sg[0], sg[1], sg[2], c)
        };
        let (r0, r1, r2) = f5_add_planes(dg[0], dg[1], dg[2], b0, b1, b2);
        dg[0] = r0;
        dg[1] = r1;
        dg[2] = r2;
    }
}

fn f5_add_group_masked(dst: &mut [Limb], src: &[Limb], c: u32, lane_mask: Limb) {
    let (mut b0, mut b1, mut b2) = if c == 1 {
        (src[0], src[1], src[2])
    } else {
        f5_mul_planes(src[0], src[1], src[2], c)
    };
    // Zeroing the addend outside the mask leaves those lanes unchanged (adds 0).
    b0 &= lane_mask;
    b1 &= lane_mask;
    b2 &= lane_mask;
    let (r0, r1, r2) = f5_add_planes(dst[0], dst[1], dst[2], b0, b1, b2);
    dst[0] = r0;
    dst[1] = r1;
    dst[2] = r2;
}

fn f5_scale_groups(dst: &mut [Limb], c: u32) {
    for dg in dst.as_chunks_mut::<3>().0 {
        let (r0, r1, r2) = f5_mul_planes(dg[0], dg[1], dg[2], c);
        dg[0] = r0;
        dg[1] = r1;
        dg[2] = r2;
    }
}

// ---------------------------------------------------------------------------------------
// Heap-scratch fallback for `k > MAX_DISPATCH_K` (very large primes).
// ---------------------------------------------------------------------------------------

fn cond_sub_into(dst: &mut [Limb], s: &[Limb], masks: &[Limb], d: &mut [Limb]) {
    let k = dst.len();
    let mut borrow: Limb = 0;
    for j in 0..=k {
        let sj = s[j];
        let pj = masks[j];
        let sxp = sj ^ pj;
        d[j] = sxp ^ borrow;
        borrow = (!sj & pj) | (borrow & !sxp);
    }
    let ge = !borrow;
    for j in 0..k {
        dst[j] = (d[j] & ge) | (s[j] & !ge);
    }
}

fn add_mod_into(dst: &mut [Limb], b: &[Limb], masks: &[Limb], s: &mut [Limb], d: &mut [Limb]) {
    let k = dst.len();
    let mut carry: Limb = 0;
    for j in 0..k {
        let aj = dst[j];
        let bj = b[j];
        let axb = aj ^ bj;
        s[j] = axb ^ carry;
        carry = (aj & bj) | (carry & axb);
    }
    s[k] = carry;
    cond_sub_into(dst, s, masks, d);
}

fn double_mod_into(dst: &mut [Limb], masks: &[Limb], s: &mut [Limb], d: &mut [Limb]) {
    let k = dst.len();
    s[0] = 0;
    s[1..=k].copy_from_slice(&dst[..k]);
    cond_sub_into(dst, s, masks, d);
}

fn scalar_mul_into(
    acc: &mut [Limb],
    b: &[Limb],
    c: u32,
    masks: &[Limb],
    temp: &mut [Limb],
    s: &mut [Limb],
    d: &mut [Limb],
) {
    temp.copy_from_slice(b);
    acc.fill(0);
    let mut cc = c;
    loop {
        if cc & 1 == 1 {
            add_mod_into(acc, temp, masks, s, d);
        }
        cc >>= 1;
        if cc == 0 {
            break;
        }
        double_mod_into(temp, masks, s, d);
    }
}

fn add_groups_dyn(k: usize, dst: &mut [Limb], src: &[Limb], c: u32, masks: &[Limb]) {
    let mut s = vec![0; k + 1];
    let mut d = vec![0; k + 1];
    let mut acc = vec![0; k];
    let mut temp = vec![0; k];
    for (dg, sg) in dst.chunks_exact_mut(k).zip(src.chunks_exact(k)) {
        if c == 1 {
            add_mod_into(dg, sg, masks, &mut s, &mut d);
        } else {
            scalar_mul_into(&mut acc, sg, c, masks, &mut temp, &mut s, &mut d);
            add_mod_into(dg, &acc, masks, &mut s, &mut d);
        }
    }
}

fn scale_groups_dyn(k: usize, dst: &mut [Limb], c: u32, masks: &[Limb]) {
    if c == 1 {
        return;
    }
    if c == 0 {
        dst.fill(0);
        return;
    }
    let mut s = vec![0; k + 1];
    let mut d = vec![0; k + 1];
    let mut acc = vec![0; k];
    let mut temp = vec![0; k];
    for dg in dst.chunks_exact_mut(k) {
        scalar_mul_into(&mut acc, dg, c, masks, &mut temp, &mut s, &mut d);
        dg.copy_from_slice(&acc);
    }
}

const _: () = assert!(MAX_DISPATCH_K <= BITS_PER_LIMB);
