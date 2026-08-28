//! The C-motivic (over $\mathbb{C}$, prime 2) dual Steenrod algebra $A_{**}$ and
//! the Steenrod algebra $A_C$ in the Milnor basis, over $\mathbb{F}_2[\tau]$.
//!
//! This follows Kong–Lin, *Product formulas for motivic Milnor basis*
//! (arXiv:2411.12890), specialized to the C-motivic point where $\rho = 0$ (the
//! motivic cohomology of a point over $\mathbb{C}$ is $\mathbb{F}_2[\tau]$). We
//! use the *conjugate* generators of that paper, so the coproduct and
//! Milnor-matrix product match Milnor's classical formulas on the polynomial
//! (`ξ`) part.
//!
//! The dual algebra is $A_{**} = \mathbb{F}_2[\tau][\xi_1, \xi_2, \dots] \otimes
//! E(\tau_0, \tau_1, \dots)$ with $\tau_i^2 = \tau \xi_{i+1}$ (the $\rho = 0$
//! reduction of $\tau_i^2 = \tau\xi_{i+1} + \rho\tau_{i+1}$). Bidegrees:
//! $|\xi_i| = (2(2^i-1), 2^i-1)$, $|\tau_i| = (2^{i+1}-1, 2^i-1)$,
//! $|\tau| = (0, -1)$ in (stem, weight).
//!
//! Basis elements are $Q(E)P(R)$ with $E \in \mathrm{Seq}_1$ (entries in
//! $\{0,1\}$, encoded as a bitmask: bit $i$ is $\tau_i$) and $R \in \mathrm{Seq}$
//! (the exponent vector of the $\xi_i$).
//!
//! The product in the Steenrod algebra $A_C$ is computed by **duality**: the
//! dual algebra $A_{**}$ is a commutative $\mathbb{F}_2[\tau]$-algebra whose
//! multiplication ([`dual_mul`]) reduces $\tau_i^2 = \tau\xi_{i+1}$ via
//! [`rewrite_tau`] (Kong–Lin Theorem 3.4), and its coproduct $\psi$
//! ([`coproduct`], Kong–Lin §2.2) is a $\tau$-free algebra map. The product
//! $a \cdot b$ in $A_C$ is then read off from $\psi$: the coefficient of $z$ in
//! $a \cdot b$ is the coefficient of $\mathrm{mon}(a) \otimes \mathrm{mon}(b)$ in
//! $\psi(\mathrm{mon}(z))$ ([`multiply`]). This is a first, correctness-oriented
//! implementation; the closed-form product (Kong–Lin Theorem 5.1) can replace it
//! later for speed, validated against this one.
//!
//! Coefficients are homogeneous, so each one is a single power of $\tau$: a
//! [`Tau`] scalar (see [`super::tau`]). This is the $A_C$ product **engine**,
//! over $\mathbb{F}_2[\tau]$; the mod-$\tau$ reduction $A_C/\tau$ is presented to
//! the resolution engine as an ordinary $\mathbb{F}_2$-algebra in a follow-up
//! layer built on top of this engine.
//!
//! Weight convention: the motivic weight of an algebra basis element is the
//! *negative* of the weight of the dual monomial it pairs with, so that products
//! are weight-homogeneous with $\tau$ (weight $-1$) absorbing the difference.

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, OnceLock, RwLock},
};

use fp::prime::{Binomial, iter::BitflagIterator};
use itertools::Itertools;
use maybe_rayon::prelude::*;
use once::OnceVec;
use rustc_hash::FxHashMap;

use super::Tau;
use crate::algebra::milnor_algebra::PPart;

/// Drop the trailing zeros from an exponent vector.
///
/// Exponent vectors are compared and used as map keys throughout, so they are kept trimmed to
/// make that comparison canonical: `ξ(R)` does not see the slots past the last non-zero one.
fn trim(mut v: Vec<u32>) -> Vec<u32> {
    while let Some(&0) = v.last() {
        v.pop();
    }
    v
}

/// A basis monomial $\tau(E)\xi(R)$ of the dual algebra.
///
/// The derived ordering is lexicographic in `(q_part, p_part)`, which is what
/// [`MotivicMilnorAlgebra`] sorts and binary-searches its per-degree bases by.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Monomial {
    /// The square-free exterior part $E$ as a bitmask: bit `i` is $\tau_i$.
    pub q_part: u32,
    /// The $\xi$ exponents, as the classical bit-packed [`PPart`]: entry `i` is the exponent of
    /// $\xi_{i+1}$.
    ///
    /// The Kong–Lin formulas index $R$ from $\xi_0 = 1$, so their `R[0]` is always zero for a
    /// monomial and carries no information. Dropping it is what lets the exponents share the
    /// classical representation; [`Self::from_paper`] and [`Self::paper_p_part`] convert at the
    /// boundaries where the paper's indexing is the natural one.
    pub p_part: PPart,
}

impl Monomial {
    /// The monomial $\tau(E)\xi(R)$.
    pub fn new(q_part: u32, p_part: PPart) -> Self {
        Self { q_part, p_part }
    }

    /// The monomial whose $\xi$ exponents are given in the paper's indexing, `r[j]` being the
    /// exponent of $\xi_j$. `r[0]` must be zero, since $\xi_0 = 1$.
    ///
    /// Returns `None` if an exponent is too large to pack, which for the degrees this engine
    /// reaches means the caller has gone out of range.
    pub fn from_paper(q_part: u32, r: &[u32]) -> Option<Self> {
        debug_assert_eq!(
            r.first().copied().unwrap_or(0),
            0,
            "xi_0 = 1 slot must be 0"
        );
        Some(Self::new(
            q_part,
            PPart::try_from_slice(r.get(1..).unwrap_or_default())?,
        ))
    }

    /// The $\xi$ exponents in the paper's indexing, with the $\xi_0$ slot restored.
    pub fn paper_p_part(&self) -> Vec<u32> {
        if self.p_part.is_empty() {
            return Vec::new();
        }
        std::iter::once(0).chain(self.p_part.iter()).collect()
    }

    /// The unit monomial $1$.
    pub fn one() -> Self {
        Self::default()
    }

    /// The C-motivic (prime 2) bidegree `(t, w)`: `|τ_i| = (2^{i+1}-1, 2^i-1)` and
    /// `|ξ_{i+1}| = (2^{i+2}-2, 2^{i+1}-1)`.
    ///
    /// [`MotivicMilnorAlgebra::bidegree`] negates `w` for the algebra's own weight convention.
    pub fn bidegree(&self) -> (i32, i32) {
        let (mut t, mut w) = (0i32, 0i32);
        for i in BitflagIterator::set_bit_iterator(self.q_part as u64) {
            t += (1 << (i + 1)) - 1;
            w += (1 << i) - 1;
        }
        for (i, r) in self.p_part.iter().enumerate() {
            t += r as i32 * ((1 << (i + 2)) - 2);
            w += r as i32 * ((1 << (i + 1)) - 1);
        }
        (t, w)
    }
}

/// $\Sigma(R) = \sum_i r_i$.
fn sigma(r: &[u32]) -> u32 {
    r.iter().sum()
}

/// $\Sigma_2(R) = \sum_i r_i 2^i$.
fn sigma2(r: &[u32]) -> u64 {
    r.iter().enumerate().map(|(i, &v)| (v as u64) << i).sum()
}

/// The coefficient $c(S, R)$ of Notation 3.3 / Theorem 3.4, reduced mod 2.
///
/// $c(S, R) = \prod_{n \ge 1} \binom{\lfloor \sum_{i=0}^{n-1} 2^{i-n}(s_i - r_i)\rfloor}{r_n}$
/// when $r_0 = 0$, and $0$ otherwise. The inner floor is computed exactly with integer
/// (Euclidean) division: $\lfloor \sum_{i=0}^{n-1} 2^{i-n}(s_i-r_i)\rfloor
/// = \lfloor (\sum_{i=0}^{n-1} (s_i-r_i)2^i) / 2^n \rfloor$. Each factor is
/// [`Binomial::binomial2`], which is $0$ when the top is negative or below the bottom.
fn c_coeff(s: &[u32], r: &[u32]) -> u32 {
    if r.first().copied().unwrap_or(0) > 0 {
        return 0;
    }
    let get = |seq: &[u32], i: usize| seq.get(i).copied().unwrap_or(0) as i32;
    let len = s.len().max(r.len());
    let mut prod = 1u32;
    for n in 1..len {
        let mut num: i32 = 0;
        for i in 0..n {
            num += (get(s, i) - get(r, i)) << i;
        }
        let floor = num.div_euclid(1i32 << n);
        prod *= i32::binomial2(floor, get(r, n)) as u32;
        if prod == 0 {
            return 0;
        }
    }
    prod
}

/// A term $\tau^{\text{tau\_pow}} \cdot Q(E) P(R)$ (mod 2, C-motivic so no $\rho$), the output
/// of rewriting a monomial into the Milnor basis. `e_mask` encodes $E \in \mathrm{Seq}_1$.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotivicTerm {
    pub tau_pow: u32,
    pub e_mask: u32,
    pub r: Vec<u32>,
}

/// Rewrite $\tau(S) = \tau_0^{s_0}\tau_1^{s_1}\cdots$ (with `s` an arbitrary exponent vector)
/// into the C-motivic Milnor basis, using $\tau_i^2 = \tau\xi_{i+1}$ (Theorem 3.4 at
/// $\rho = 0$):
/// $$\tau(S) = \sum_{\substack{E \in \mathrm{Seq}_1,\ R \in \mathrm{Seq}\\ \Sigma_2(E)+\Sigma_2(R)=\Sigma_2(S)\\ \Sigma(E)+2\Sigma(R)=\Sigma(S)}} c(S,R)\, \tau^{\Sigma(R)}\, \tau(E)\xi(R),$$
/// where the second constraint is the vanishing of the $\rho$-exponent
/// $\Sigma(S) - \Sigma(E) - 2\Sigma(R)$.
pub fn rewrite_tau(s: &[u32]) -> Vec<MotivicTerm> {
    let target2 = sigma2(s);
    let target_sum = sigma(s);
    // Highest index we must consider: any nonzero e_i or r_i contributes at least 2^i, so
    // 2^i <= target2.
    let max_idx = if target2 == 0 {
        0
    } else {
        (63 - target2.leading_zeros()) as usize
    };

    let mut out = Vec::new();
    let mut e = vec![0u32; max_idx + 1];
    let mut r = vec![0u32; max_idx + 1];
    rewrite_tau_dfs(s, 0, max_idx, target2, target_sum, &mut e, &mut r, &mut out);
    out
}

#[allow(clippy::too_many_arguments)]
fn rewrite_tau_dfs(
    s: &[u32],
    idx: usize,
    max_idx: usize,
    rem2: u64,       // remaining Σ2(E) + Σ2(R) budget
    target_sum: u32, // Σ(S), for the ρ = 0 constraint Σ(E) + 2Σ(R) = Σ(S)
    e: &mut [u32],
    r: &mut [u32],
    out: &mut Vec<MotivicTerm>,
) {
    if idx > max_idx {
        if rem2 != 0 {
            return;
        }
        // ρ-exponent must vanish (C-motivic): Σ(E) + 2Σ(R) = Σ(S).
        if sigma(e) + 2 * sigma(r) != target_sum {
            return;
        }
        let coeff = c_coeff(s, r);
        if coeff == 0 {
            return;
        }
        let mut e_mask = 0u32;
        for (i, &ei) in e.iter().enumerate() {
            if ei != 0 {
                e_mask |= 1 << i;
            }
        }
        out.push(MotivicTerm {
            tau_pow: sigma(r),
            e_mask,
            r: trim(r.to_vec()),
        });
        return;
    }

    let weight = 1u64 << idx;
    // e_idx ∈ {0, 1} (E ∈ Seq_1), r_idx ≥ 0, with (e_idx + r_idx) * 2^idx ≤ rem2.
    for e_idx in 0..=1u32 {
        if (e_idx as u64) * weight > rem2 {
            break;
        }
        let after_e = rem2 - (e_idx as u64) * weight;
        let max_r = (after_e / weight) as u32;
        for r_idx in 0..=max_r {
            e[idx] = e_idx;
            r[idx] = r_idx;
            rewrite_tau_dfs(
                s,
                idx + 1,
                max_idx,
                after_e - (r_idx as u64) * weight,
                target_sum,
                e,
                r,
                out,
            );
        }
    }
    e[idx] = 0;
    r[idx] = 0;
}

// ---------------------------------------------------------------------------
// The dual algebra A_** (used to compute the product in A_C by duality).
// ---------------------------------------------------------------------------

/// A sparse $\mathbb{F}_2[\tau]$-linear combination of `K`s, as a `Vec` sorted by key.
///
/// Everything we compute is homogeneous — $\psi$ and the products are graded — so every
/// coefficient is a single power of $\tau$: a [`Tau`] scalar. Zero coefficients are never stored
/// and the terms are kept sorted, so equality is canonical, and coefficient arithmetic is exactly
/// [`Tau`]'s arithmetic — `mul` adds valuations, `add` cancels equal powers mod 2 (unequal powers
/// would be inhomogeneous and cannot arise).
///
/// A sorted `Vec` rather than a `BTreeMap`: these combinations are small — a product of two basis
/// elements has a handful of terms — and the keys are now `Copy` words, so one contiguous
/// allocation searched by [`slice::binary_search_by`] beats a tree that allocates per node.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SparseSum<K> {
    terms: Vec<(K, Tau)>,
}

impl<K: Ord> SparseSum<K> {
    /// The zero combination.
    pub fn new() -> Self {
        Self { terms: Vec::new() }
    }

    /// The number of non-zero terms.
    pub fn len(&self) -> usize {
        self.terms.len()
    }

    /// Whether this is zero.
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// The coefficient of `key`, if non-zero.
    pub fn get(&self, key: &K) -> Option<&Tau> {
        self.position(key).ok().map(|i| &self.terms[i].1)
    }

    /// Add `coeff * key`, dropping the term if it cancels to zero.
    pub fn add_term(&mut self, key: K, coeff: Tau) {
        if coeff.is_zero() {
            return;
        }
        match self.position(&key) {
            Ok(i) => {
                let sum = self.terms[i].1 + coeff;
                if sum.is_zero() {
                    self.terms.remove(i);
                } else {
                    self.terms[i].1 = sum;
                }
            }
            Err(i) => self.terms.insert(i, (key, coeff)),
        }
    }

    /// The index of `key`, or where it would be inserted.
    fn position(&self, key: &K) -> Result<usize, usize> {
        self.terms.binary_search_by(|(k, _)| k.cmp(key))
    }

    /// The terms, in key order.
    pub fn iter(&self) -> std::slice::Iter<'_, (K, Tau)> {
        self.terms.iter()
    }
}

impl<K: Ord> FromIterator<(K, Tau)> for SparseSum<K> {
    fn from_iter<I: IntoIterator<Item = (K, Tau)>>(iter: I) -> Self {
        let mut out = Self::new();
        for (key, coeff) in iter {
            out.add_term(key, coeff);
        }
        out
    }
}

impl<K: Ord, const N: usize> From<[(K, Tau); N]> for SparseSum<K> {
    fn from(terms: [(K, Tau); N]) -> Self {
        terms.into_iter().collect()
    }
}

impl<K> IntoIterator for SparseSum<K> {
    type IntoIter = std::vec::IntoIter<(K, Tau)>;
    type Item = (K, Tau);

    fn into_iter(self) -> Self::IntoIter {
        self.terms.into_iter()
    }
}

impl<'a, K> IntoIterator for &'a SparseSum<K> {
    type IntoIter = std::slice::Iter<'a, (K, Tau)>;
    type Item = &'a (K, Tau);

    fn into_iter(self) -> Self::IntoIter {
        self.terms.iter()
    }
}

/// An element of the dual algebra $A_{**}$: [`Monomial`]s with $\mathbb{F}_2[\tau]$
/// coefficients.
pub type DualElement = SparseSum<Monomial>;

/// Elementwise sum of two exponent sequences, i.e. the product of the two $\xi$ monomials.
fn ppart_add(a: PPart, b: PPart) -> PPart {
    let mut out = PPart::zero();
    for i in 0..a.len().max(b.len()) {
        out.set(i, a.get(i) + b.get(i));
    }
    out
}

/// Multiply two basis monomials of $A_{**}$ and accumulate `coeff * (m1 * m2)` into `acc`.
///
/// $A_{**}$ is commutative, so $\tau(E_1)\xi(R_1)\cdot\tau(E_2)\xi(R_2)$ is obtained by adding
/// exponents: the exterior parts form $S = E_1 + E_2$ (entries in $\{0,1,2\}$), which is
/// rewritten into the square-free basis via [`rewrite_tau`] ($\tau_i^2 = \tau\xi_{i+1}$), and
/// the resulting $\xi$ exponents are added to $R_1 + R_2$.
fn mul_monomials(m1: Monomial, m2: Monomial, coeff: Tau, acc: &mut DualElement) {
    let (e1, e2) = (m1.q_part, m2.q_part);
    let bits = u32::BITS - (e1 | e2).leading_zeros();
    let s: Vec<u32> = (0..bits)
        .map(|i| ((e1 >> i) & 1) + ((e2 >> i) & 1))
        .collect();
    let r12 = ppart_add(m1.p_part, m2.p_part);
    for term in rewrite_tau(&s) {
        // `term.r` is paper-indexed and `term.r[0]` is zero (`c_coeff` vanishes otherwise); it
        // is also trimmed, so it can be empty.
        let term_r = PPart::try_from_slice(term.r.get(1..).unwrap_or_default())
            .expect("rewrite_tau exponent out of packing range");
        let r = ppart_add(term_r, r12);
        acc.add_term(
            Monomial::new(term.e_mask, r),
            coeff * Tau::power(term.tau_pow),
        );
    }
}

/// The product of two elements of $A_{**}$.
pub fn dual_mul(a: &DualElement, b: &DualElement) -> DualElement {
    let mut out = DualElement::new();
    for &(m1, c1) in a {
        for &(m2, c2) in b {
            mul_monomials(m1, m2, c1 * c2, &mut out);
        }
    }
    out
}

/// The generator $\tau_i \in A_{**}$.
pub fn tau_gen(i: usize) -> DualElement {
    DualElement::from([(Monomial::new(1 << i, PPart::zero()), Tau::one())])
}

/// The generator $\xi_i \in A_{**}$ (for $i \ge 1$).
pub fn xi_gen(i: usize) -> DualElement {
    DualElement::from([(xi_pow_mon(i, 1), Tau::one())])
}

/// The unit $1 \in A_{**}$.
pub fn dual_one() -> DualElement {
    DualElement::from([(Monomial::one(), Tau::one())])
}

// ---------------------------------------------------------------------------
// The coproduct ψ: A_** → A_** ⊗ A_**, an algebra map (Kong–Lin §2.2).
// ---------------------------------------------------------------------------

/// An element of $A_{**} \otimes A_{**}$: pairs of [`Monomial`]s → $\mathbb{F}_2[\tau]$
/// coefficients. Zero coefficients are never stored.
pub type TensorElement = SparseSum<(Monomial, Monomial)>;

/// The unit $1 \otimes 1$.
fn tensor_one() -> TensorElement {
    TensorElement::from([((Monomial::one(), Monomial::one()), Tau::one())])
}

/// Multiply in $A_{**} \otimes A_{**}$: $(a_L \otimes a_R)(b_L \otimes b_R) = (a_L b_L) \otimes (a_R b_R)$.
fn tensor_mul(t1: &TensorElement, t2: &TensorElement) -> TensorElement {
    let mut out = TensorElement::new();
    for &((al, ar), c1) in t1 {
        for &((bl, br), c2) in t2 {
            let mut left = DualElement::new();
            mul_monomials(al, bl, Tau::one(), &mut left);
            let mut right = DualElement::new();
            mul_monomials(ar, br, Tau::one(), &mut right);
            let c = c1 * c2;
            for &(ml, cl) in &left {
                for &(mr, cr) in &right {
                    out.add_term((ml, mr), c * (cl * cr));
                }
            }
        }
    }
    out
}

/// The monomial $\xi_j^p$ (with $\xi_0 = 1$).
fn xi_pow_mon(j: usize, p: u32) -> Monomial {
    let mut r = PPart::zero();
    if j > 0 && p > 0 {
        r.set(j - 1, p);
    }
    Monomial::new(0, r)
}

/// $\psi(\tau_k) = 1 \otimes \tau_k + \sum_{i=0}^{k} \tau_i \otimes \xi_{k-i}^{2^i}$.
fn coprod_tau(k: usize) -> TensorElement {
    let mut out = TensorElement::new();
    out.add_term(
        (Monomial::one(), Monomial::new(1 << k, PPart::zero())),
        Tau::one(),
    );
    for i in 0..=k {
        out.add_term(
            (
                Monomial::new(1 << i, PPart::zero()),
                xi_pow_mon(k - i, 1 << i),
            ),
            Tau::one(),
        );
    }
    out
}

/// $\psi(\xi_k) = \sum_{i=0}^{k} \xi_i \otimes \xi_{k-i}^{2^i}$ (with $\xi_0 = 1$).
fn coprod_xi(k: usize) -> TensorElement {
    let mut out = TensorElement::new();
    for i in 0..=k {
        out.add_term((xi_pow_mon(i, 1), xi_pow_mon(k - i, 1 << i)), Tau::one());
    }
    out
}

/// The coproduct of a basis monomial $\tau(E)\xi(R)$, computed as an algebra map: the product
/// of the coproducts of its generators.
fn coproduct_monomial(m: &Monomial) -> TensorElement {
    let mut acc = tensor_one();
    for i in BitflagIterator::set_bit_iterator(m.q_part as u64) {
        acc = tensor_mul(&acc, &coprod_tau(i));
    }
    for (i, r) in m.p_part.iter().enumerate() {
        for _ in 0..r {
            acc = tensor_mul(&acc, &coprod_xi(i + 1));
        }
    }
    acc
}

/// The coproduct of an arbitrary element of $A_{**}$ (extended $\mathbb{F}_2[\tau]$-linearly).
pub fn coproduct(elt: &DualElement) -> TensorElement {
    let mut out = TensorElement::new();
    for &(m, c) in elt {
        for (key, cc) in coproduct_monomial(&m) {
            out.add_term(key, c * cc);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The antipode χ: A_** → A_** (conjugate generators ⟷ standard Milnor generators).
// ---------------------------------------------------------------------------

/// $\xi_j^p \in A_{**}$ (with $\xi_0 = 1$).
fn xi_pow_elt(j: usize, p: u32) -> DualElement {
    DualElement::from([(xi_pow_mon(j, p), Tau::one())])
}

/// $\chi(\xi_k)$, from the antipode axiom with this module's coproduct:
/// $\chi(\xi_k) = \sum_{i=0}^{k-1} \chi(\xi_i)\,\xi_{k-i}^{2^i}$, $\chi(\xi_0) = 1$.
fn chi_xi(k: usize) -> DualElement {
    if k == 0 {
        return dual_one();
    }
    let mut acc = DualElement::new();
    for i in 0..k {
        let term = dual_mul(&chi_xi(i), &xi_pow_elt(k - i, 1 << i));
        for (mon, c) in term {
            acc.add_term(mon, c);
        }
    }
    acc
}

/// $\chi(\tau_k)$, from the antipode axiom:
/// $\chi(\tau_k) = \tau_k + \sum_{i=0}^{k-1} \chi(\tau_i)\,\xi_{k-i}^{2^i}$.
fn chi_tau(k: usize) -> DualElement {
    let mut acc = tau_gen(k);
    for i in 0..k {
        let term = dual_mul(&chi_tau(i), &xi_pow_elt(k - i, 1 << i));
        for (mon, c) in term {
            acc.add_term(mon, c);
        }
    }
    acc
}

/// The antipode $\chi$ of $A_{**}$ — an algebra map (since $A_{**}$ is commutative) extended
/// from `chi_tau`/`chi_xi` on the generators. It converts the paper's conjugate generators to
/// the standard Milnor/Voevodsky generators (`χ(ξ_2) = ξ_2 + ξ_1^3`, etc.) and is an
/// involution.
pub fn antipode(elt: &DualElement) -> DualElement {
    let mut out = DualElement::new();
    for &(mon, c) in elt {
        let mut acc = dual_one();
        for i in 0..u32::BITS {
            if (mon.q_part >> i) & 1 != 0 {
                acc = dual_mul(&acc, &chi_tau(i as usize));
            }
        }
        for (i, r) in mon.p_part.iter().enumerate() {
            for _ in 0..r {
                acc = dual_mul(&acc, &chi_xi(i + 1));
            }
        }
        for (out_mon, cc) in acc {
            out.add_term(out_mon, c * cc);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The product in the Steenrod algebra A_C, computed by dualizing ψ.
// ---------------------------------------------------------------------------

/// All basis monomials `τ(E)ξ(R)` of a given topological degree `target`.
pub fn enum_basis(target: i32) -> Vec<Monomial> {
    // Generators available up to `target`: τ_i of degree 2^{i+1}-1, ξ_j (j≥1) of degree
    // 2^{j+1}-2. Represented as (is_tau, index, degree).
    let mut gens: Vec<(bool, usize, i32)> = Vec::new();
    let mut i = 0;
    while (1i32 << (i + 1)) - 1 <= target {
        gens.push((true, i, (1 << (i + 1)) - 1));
        i += 1;
    }
    let mut j = 1;
    while (1i32 << (j + 1)) - 2 <= target {
        gens.push((false, j, (1 << (j + 1)) - 2));
        j += 1;
    }
    let r_len = gens
        .iter()
        .filter(|g| !g.0)
        .map(|g| g.1 + 1)
        .max()
        .unwrap_or(0);

    let mut out = Vec::new();
    let mut e = 0u32;
    let mut r = vec![0u32; r_len];
    enum_basis_dfs(&gens, 0, target, &mut e, &mut r, &mut out);
    out
}

fn enum_basis_dfs(
    gens: &[(bool, usize, i32)],
    idx: usize,
    rem: i32,
    e: &mut u32,
    r: &mut [u32],
    out: &mut Vec<Monomial>,
) {
    if idx == gens.len() {
        if rem == 0 {
            // `r` is paper-indexed: `r[0]` is the (always zero) xi_0 slot.
            out.push(Monomial::from_paper(*e, r).expect("basis exponent out of packing range"));
        }
        return;
    }
    let (is_tau, index, deg) = gens[idx];
    let max_mult = if is_tau { 1 } else { rem / deg };
    for m in 0..=max_mult {
        if m * deg > rem {
            break;
        }
        if is_tau && m == 1 {
            *e |= 1 << index;
        } else if !is_tau {
            r[index] = m as u32;
        }
        enum_basis_dfs(gens, idx + 1, rem - m * deg, e, r, out);
        if is_tau {
            *e &= !(1 << index);
        } else {
            r[index] = 0;
        }
    }
}

/// The product `a · b` in the C-motivic Steenrod algebra `A_C`, where `a`, `b` are Milnor basis
/// elements `Q(E)P(R)` given as the monomials `(E, R)` they are dual to. The result is a map
/// from basis monomials to their `𝔽₂[τ]` coefficients.
///
/// Computed by duality: the coefficient of `z` in `a · b` is the coefficient of
/// `mon(a) ⊗ mon(b)` in `ψ(mon(z))`, summed over the basis `z` of the appropriate
/// topological degree.
pub fn multiply(a: Monomial, b: Monomial) -> DualElement {
    let key = (a, b);
    let t = a.bidegree().0 + b.bidegree().0;
    let mut out = DualElement::new();
    for z in enum_basis(t) {
        if let Some(&c) = coproduct_monomial(&z).get(&key)
            && !c.is_zero()
        {
            out.add_term(z, c);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The closed-form product (Kong–Lin Theorem 5.1, C-motivic ρ = 0).
// ---------------------------------------------------------------------------
//
// Theorem 5.1 computes Q(E₁)P(R₁)·Q(E₂)P(R₂) as a sum over two Milnor-style
// matrices X, Y with the output monomial Q(E₂ + T(Y)) P(T(X)). The subtlety that
// makes the motivic case "much more complex than classical" (and that trips a
// naive reading) is the ξ₀ = 1 / τ₀ ≠ 1 asymmetry (Remark 4.3): **ξ-type**
// sequences (R₁, R₂, S(X), R(X), T(X)) are compared on indices ≥ 1 — index 0 is
// absorbed — while **τ-type** sequences (E, S(Y), T(Y)) keep index 0.
//
// The τ-rewriting piece of the formula — the coefficient c(S(Y), R₁−S(X)), the
// Σ₂ constraint, and the ρ = 0 filter — is exactly [`rewrite_tau`], which we
// reuse rather than re-derive.
//
// Mod τ this collapses towards the classical shape: A_C/τ ≅ 𝔽₂[ξᵢ] ⊗ E(τᵢ) has the
// odd-primary dual's structure (with 2ⁱ powers rather than pⁱ), and the X constraint
// tightens from S(X) ≤ R₁ to S(X) = R₁ — the classical admissible matrix. What remains
// over the classical product is the second matrix Y, carrying the τᵢ/Q-part interaction.

/// A list of candidate columns, flattened into one allocation.
///
/// The recursion walks these lists constantly, so the columns are stored end to end with an
/// offset table rather than as a `Vec<Vec<u32>>`: iterating that chased a pointer per column and
/// showed up as a fifth of the product's time.
#[derive(Debug, Default)]
struct Columns {
    entries: Vec<u32>,
    /// `offsets[k]..offsets[k + 1]` is column `k`. Always starts with a 0.
    offsets: Vec<u32>,
}

impl Columns {
    fn push(&mut self, col: &[u32]) {
        if self.offsets.is_empty() {
            self.offsets.push(0);
        }
        self.entries.extend_from_slice(col);
        self.offsets.push(self.entries.len() as u32);
    }

    fn iter(&self) -> impl Iterator<Item = &[u32]> {
        self.offsets
            .windows(2)
            .map(|w| &self.entries[w[0] as usize..w[1] as usize])
    }
}

impl FromIterator<Vec<u32>> for Columns {
    fn from_iter<I: IntoIterator<Item = Vec<u32>>>(iter: I) -> Self {
        let mut out = Self::default();
        for col in iter {
            out.push(&col);
        }
        out
    }
}

/// All columns `(v₀, v₁, …)` of non-negative integers with `Σᵢ 2ⁱ vᵢ == target`.
fn columns_eq(target: u32) -> Vec<Vec<u32>> {
    if target == 0 {
        return vec![vec![]];
    }
    let mut out = Vec::new();
    let mut v0 = target % 2;
    while v0 <= target {
        for rest in columns_eq((target - v0) / 2) {
            let mut col = vec![v0];
            col.extend(rest);
            out.push(trim(col));
        }
        v0 += 2;
    }
    out
}

thread_local! {
    /// Per-thread memo of [`columns_eq`] / [`columns_le`], keyed by target/bound.
    /// The candidate lists are pure functions of one integer and are requested
    /// over and over across products; caching them avoids re-enumerating and
    /// re-allocating. (`Rc` is confined to a single product's call stack.)
    static COLS_EQ: RefCell<FxHashMap<u32, Rc<Columns>>> = RefCell::new(FxHashMap::default());
    static COLS_LE: RefCell<FxHashMap<u32, Rc<Columns>>> = RefCell::new(FxHashMap::default());
}

fn columns_eq_cached(target: u32) -> Rc<Columns> {
    COLS_EQ.with(|c| {
        Rc::clone(
            c.borrow_mut()
                .entry(target)
                .or_insert_with(|| Rc::new(columns_eq(target).into_iter().collect())),
        )
    })
}

/// All columns with `Σᵢ 2ⁱ vᵢ ≤ bound` (distinct weighted sums, so no duplicates).
fn columns_le_cached(bound: u32) -> Rc<Columns> {
    COLS_LE.with(|c| {
        Rc::clone(
            c.borrow_mut()
                .entry(bound)
                .or_insert_with(|| Rc::new((0..=bound).flat_map(columns_eq).collect())),
        )
    })
}

/// The column-0 options for the `X` matrix: `x_{0,0} = 0`, and `x_{i,0} ≤ R₁[i]`
/// for `i ≥ 1` (bounded because it feeds the row sum `S(X)_i ≤ R₁[i]`).
fn col0_x_options(r1: &[u32]) -> Columns {
    let mut result = vec![vec![0u32]];
    for &bound in r1.iter().skip(1) {
        let mut next = Vec::new();
        for base in &result {
            for v in 0..=bound {
                let mut c = base.clone();
                c.push(v);
                next.push(c);
            }
        }
        result = next;
    }
    result.into_iter().map(trim).collect()
}

/// Buffer size for row / column / anti-diagonal indices.
///
/// Anti-diagonals are indexed by `i + j`, with the row index `i` bounded by a column's length
/// (at most `log2(MAX_DEGREE) + 1`) and the column index `j` by [`PPart::MAX_LEN`] + 1. That
/// caps the sum near 21, and this is the next power of two; [`Acc`] holds three of these, so the
/// size is paid on every matrix.
const NB: usize = 32;

/// Immutable context for the closed-form product recursion (Theorem 5.1, ρ = 0).
struct Closed<'a> {
    r1v: &'a [u32],
    r2v: &'a [u32],
    l: usize,
    e1_mask: u32,
    e2_mask: u32,
    sigma2_e1: i64,
    /// Per-column candidate columns for the `X` matrix.
    x_cands: &'a [Rc<Columns>],
}

/// Incrementally maintained matrix state: row sums `S`, a running XOR per
/// anti-diagonal (`or`, which equals the OR because we only keep disjoint entries)
/// for the `b`-coefficient pruning, and the anti-diagonal sums `T`.
struct Acc {
    rows: [u32; NB],
    or: [u32; NB],
    sum: [u32; NB],
}

impl Acc {
    fn zero() -> Self {
        Self {
            rows: [0; NB],
            or: [0; NB],
            sum: [0; NB],
        }
    }
}

impl<'a> Closed<'a> {
    /// Enumerate the `X` matrix column by column. A column is rejected — pruning
    /// the whole subtree — if a row sum would exceed `R₁` (rows ≥ 1; row 0 is
    /// absorbed by ξ₀ = 1) or if any entry collides on its anti-diagonal (which
    /// would make `b(X) = 0`). So only `b = 1` matrices are visited, and `b`/`T`/`S`
    /// never need recomputing. The accumulator `acc` and `xcols` are reused.
    fn enum_x(&self, j: usize, xcols: &mut Vec<&'a [u32]>, acc: &mut Acc, out: &mut DualElement) {
        if j == self.l {
            self.on_x(xcols, acc, out);
            return;
        }
        for cand in self.x_cands[j].iter() {
            let fits = cand.iter().enumerate().all(|(i, &v)| {
                v == 0
                    || (!(i >= 1 && acc.rows[i] + v > self.r1v.get(i).copied().unwrap_or(0))
                        && acc.or[i + j] & v == 0)
            });
            if !fits {
                continue;
            }
            debug_assert!(cand.len() + j <= NB, "anti-diagonal index out of range");
            for (i, &v) in cand.iter().enumerate() {
                acc.rows[i] += v;
                acc.or[i + j] ^= v;
                acc.sum[i + j] += v;
            }
            xcols.push(cand);
            self.enum_x(j + 1, xcols, acc, out);
            xcols.pop();
            for (i, &v) in cand.iter().enumerate() {
                acc.rows[i] -= v;
                acc.or[i + j] ^= v;
                acc.sum[i + j] -= v;
            }
        }
    }

    /// A complete `X` (already `b = 1`): form `S′`, the `Y` column targets, and the
    /// output P-part `T(X) = acc.sum`, then enumerate `Y`.
    fn on_x(&self, xcols: &[&'a [u32]], acc: &Acc, out: &mut DualElement) {
        // S′ = R₁ − S(X) on indices ≥ 1 (S′[0] = 0); τ-power = Σ(S′). This runs once per `X`
        // matrix, so the working vectors are stack arrays rather than allocations.
        let mut sprime = [0u32; NB];
        let (mut sigma2_sprime, mut sigma_sprime) = (0i64, 0u32);
        for (j, sp) in sprime.iter_mut().enumerate().take(self.l).skip(1) {
            let v = self.r1v[j] - acc.rows[j];
            *sp = v;
            sigma2_sprime += (v as i64) << j;
            sigma_sprime += v;
        }
        let sprime = &sprime[..self.l];

        // RY targets: column j ≥ 1 of Y has weighted sum R₂[j] − R(X)[j].
        let mut ry = [0i64; NB];
        for j in 1..self.l {
            let rxj: u32 = xcols[j].iter().enumerate().map(|(i, &v)| v << i).sum();
            ry[j] = self.r2v[j] as i64 - rxj as i64;
            if ry[j] < 0 {
                return;
            }
        }
        // Σ₂(S(Y)) = Σ₂(E₁) + Σ₂(S′) is forced and equals Σ_{j≥0} R(Y)[j], so column
        // 0 of Y has a determined weighted sum.
        let ry0 = (self.sigma2_e1 + sigma2_sprime) - ry[1..self.l].iter().sum::<i64>();
        if ry0 < 0 {
            return;
        }

        // The output P-part T(X). The anti-diagonal sums are paper-indexed, so entry `i` of the
        // packed exponent sequence is anti-diagonal `i + 1` (index 0 is the xi_0 slot).
        let mut out_r = PPart::zero();
        for i in 0..PPart::MAX_LEN {
            out_r.set(i, acc.sum[i + 1]);
        }

        let mut y_cands: Vec<Rc<Columns>> = Vec::with_capacity(self.l);
        y_cands.push(columns_eq_cached(ry0 as u32));
        for &t in &ry[1..self.l] {
            y_cands.push(columns_eq_cached(t as u32));
        }

        ClosedY {
            y_cands: &y_cands,
            e1_mask: self.e1_mask,
            e2_mask: self.e2_mask,
            sprime,
            sigma_sprime,
            out_r,
        }
        .enum_y(0, &mut Acc::zero(), out);
    }
}

/// Immutable context for the `Y` half of the recursion, the counterpart of [`Closed`]. `X` is
/// already fixed by the time this is built, so `sprime` and `out_r` are constants here.
struct ClosedY<'a> {
    y_cands: &'a [Rc<Columns>],
    e1_mask: u32,
    e2_mask: u32,
    sprime: &'a [u32],
    sigma_sprime: u32,
    out_r: PPart,
}

impl ClosedY<'_> {
    /// Enumerate the `Y` matrix column by column (each column an exact weighted sum),
    /// pruning on anti-diagonal collisions (`b(Y) = 0`) and accumulating each matching
    /// contribution.
    fn enum_y(&self, j: usize, acc: &mut Acc, out: &mut DualElement) {
        if j == self.y_cands.len() {
            self.on_y(acc, out);
            return;
        }
        for cand in self.y_cands[j].iter() {
            if !cand
                .iter()
                .enumerate()
                .all(|(i, &v)| acc.or[i + j] & v == 0)
            {
                continue;
            }
            for (i, &v) in cand.iter().enumerate() {
                acc.rows[i] += v;
                acc.or[i + j] ^= v;
                acc.sum[i + j] += v;
            }
            self.enum_y(j + 1, acc, out);
            for (i, &v) in cand.iter().enumerate() {
                acc.rows[i] -= v;
                acc.or[i + j] ^= v;
                acc.sum[i + j] -= v;
            }
        }
    }

    /// A complete `Y` (already `b = 1`): accept it if it contributes, and add its term.
    fn on_y(&self, acc: &Acc, out: &mut DualElement) {
        // The rewriting τ(S(Y)) contains the term with exterior part E₁ and ξ-part
        // S′ iff the ρ = 0 degree equation Σ(E₁) + 2Σ(S′) = Σ(S(Y)) holds (with
        // Σ(E₁) = popcount) and c(S(Y), S′) ≠ 0. (Σ₂(E₁) + Σ₂(S′) = Σ₂(S(Y)) is
        // forced by the Y-column construction.) That term contributes at τ^{Σ(S′)}.
        let sigma_sy: u32 = acc.rows.iter().sum();
        if self.e1_mask.count_ones() + 2 * self.sigma_sprime != sigma_sy {
            return;
        }
        let sy_len = (0..NB)
            .rev()
            .find(|&i| acc.rows[i] > 0)
            .map_or(0, |i| i + 1);
        if c_coeff(&acc.rows[..sy_len], self.sprime) == 0 {
            return;
        }
        // E_out = E₂ + T(Y) must be square-free (Seq₁). Q indices fit in a u32.
        let mut out_e_mask = 0u32;
        for i in 0..u32::BITS as usize {
            let val = ((self.e2_mask >> i) & 1) + acc.sum[i];
            if val > 1 {
                return;
            }
            if val == 1 {
                out_e_mask |= 1 << i;
            }
        }
        out.add_term(
            Monomial::new(out_e_mask, self.out_r),
            Tau::power(self.sigma_sprime),
        );
    }
}

/// The product `a · b` in `A_C` via Kong–Lin Theorem 5.1 (ρ = 0). Same contract
/// as [`multiply`] (the duality oracle it is validated against).
pub fn multiply_closed(a: Monomial, b: Monomial) -> DualElement {
    let (e1_mask, e2_mask) = (a.q_part, b.q_part);
    // The recursion below follows Kong–Lin's indexing, in which `R[0]` is the xi_0 slot, so the
    // exponents are unpacked into paper-indexed working vectors here.
    let l = a.p_part.len().max(b.p_part.len()) + 1;
    let paper = |m: Monomial| -> Vec<u32> {
        std::iter::once(0)
            .chain((0..l - 1).map(|i| m.p_part.get(i)))
            .collect()
    };
    let (r1v, r2v) = (paper(a), paper(b));

    // Per-column candidate columns for X: column 0 bounded by R₁ rows, columns
    // j ≥ 1 by R₂[j] weighted.
    let mut x_cands: Vec<Rc<Columns>> = Vec::with_capacity(l);
    x_cands.push(Rc::new(col0_x_options(&r1v)));
    for &bound in r2v.iter().skip(1) {
        x_cands.push(columns_le_cached(bound));
    }

    let ctx = Closed {
        r1v: &r1v,
        r2v: &r2v,
        l,
        e1_mask,
        e2_mask,
        sigma2_e1: e1_mask as i64, // Σ (bit i)·2ⁱ = e1_mask
        x_cands: &x_cands,
    };
    let mut out = DualElement::new();
    let mut xcols: Vec<&[u32]> = Vec::with_capacity(l);
    let mut acc = Acc::zero();
    ctx.enum_x(0, &mut xcols, &mut acc, &mut out);
    out
}

// ---------------------------------------------------------------------------
// The C-motivic Steenrod algebra A_C as a free 𝔽₂[τ]-module on the Milnor basis.
// ---------------------------------------------------------------------------

/// The C-motivic (prime 2) Steenrod algebra $A_C$, presented as a free
/// $\mathbb{F}_2[\tau]$-module on the Milnor basis $\{Q(E)P(R)\}$ with lazy
/// per-(topological-)degree basis indexing.
///
/// This is the $A_C$ product **engine** over $\mathbb{F}_2[\tau]$: the dual-based product is
/// exposed through [`product_indexed`](MotivicMilnorAlgebra::product_indexed), whose coefficients
/// are [`Tau`] scalars. It is deliberately not an [`Algebra`](crate::algebra::Algebra)
/// implementation, because that trait is over $\mathbb{F}_p$; the mod-$\tau$ reduction, which *is*
/// such an algebra, is a follow-up layer on top of this engine, and the honest
/// $\mathbb{F}_2[\tau]$ resolution is built by lifting against this engine (Phase 2).
///
/// Weight convention: [`bidegree`](MotivicMilnorAlgebra::bidegree) returns `(t, w)` where `t` is
/// the topological degree and `w` is the motivic weight in the presentation where $\tau$ has
/// weight $-1$ and products are weight-homogeneous (i.e. `w = -(dual monomial weight)`; see the
/// module-level note).
#[derive(Default)]
pub struct MotivicMilnorAlgebra {
    /// `basis[t]` is the $\mathbb{F}_2[\tau]$-basis in topological degree `t`, sorted for stable
    /// indexing.
    basis: OnceVec<Vec<Monomial>>,
    /// Memoized basis-element products, one dense [`ProductBlock`] per pair of
    /// topological degrees `(t1, t2)`. The duality product is expensive and a
    /// resolution asks for the same structure constants repeatedly, so we cache
    /// them (the role the classical Milnor algebra's `cache-multiplication` table
    /// plays). Blocking by degree pair — rather than a flat `(t1, idx1, t2, idx2)`
    /// map — makes the cache the natural *batch unit* (see [`ProductBlock`]): the
    /// per-degree-pair registry is small and read-mostly, and each block's entries
    /// fill through independent [`OnceLock`]s, so a concurrent resolution reads
    /// hits lock-free instead of serializing on one global product lock.
    blocks: RwLock<FxHashMap<(i32, i32), Arc<ProductBlock>>>,
}

/// A dense block of basis-element products for one pair of topological degrees
/// `(t1, t2)`. Entry `(idx1, idx2)`, at flat position `idx1 * dim2 + idx2`, is the
/// product of the `idx1`-th basis element in degree `t1` with the `idx2`-th in
/// degree `t2` — the `(Tau, index-in-degree-(t1+t2))` list [`multiply_closed`]
/// returns.
///
/// This is the natural **batch unit** for the product: every structure constant
/// the resolution needs for a given degree pair lives in one block. Entries fill
/// lazily (each an independent [`OnceLock`], so reads are lock-free and a
/// concurrent resolution never serializes on a global product lock), and
/// [`MotivicMilnorAlgebra::fill_block`] computes the whole block at once — the
/// shape a GPU kernel would fill in a single launch.
pub struct ProductBlock {
    /// The number of basis elements in degree `t2` (the block's row stride).
    dim2: usize,
    entries: Vec<OnceLock<Vec<(Tau, usize)>>>,
}

impl MotivicMilnorAlgebra {
    pub fn new() -> Self {
        Self {
            basis: OnceVec::new(),
            blocks: RwLock::new(FxHashMap::default()),
        }
    }

    /// Compute and cache the basis in every topological degree up to and including `degree`.
    /// Idempotent and cheap to re-call.
    pub fn compute_basis(&self, degree: i32) {
        for t in self.basis.len() as i32..=degree {
            let mut b = enum_basis(t);
            b.sort();
            self.basis.push(b);
        }
    }

    /// The $\mathbb{F}_2[\tau]$-rank of `A_C` in topological degree `degree`.
    pub fn dimension(&self, degree: i32) -> usize {
        if degree < 0 {
            return 0;
        }
        self.basis[degree as usize].len()
    }

    /// The `idx`-th basis monomial `(E, R)` in degree `degree`.
    pub fn basis_element(&self, degree: i32, idx: usize) -> &Monomial {
        &self.basis[degree as usize][idx]
    }

    /// The index of a basis monomial in its degree, if present.
    pub fn index_of(&self, degree: i32, elt: &Monomial) -> Option<usize> {
        self.basis[degree as usize].binary_search(elt).ok()
    }

    /// The `(topological degree, motivic weight)` of a basis element.
    pub fn bidegree(&self, degree: i32, idx: usize) -> (i32, i32) {
        let (t, w) = self.basis[degree as usize][idx].bidegree();
        (t, -w)
    }

    /// Parse a basis element written as `basis_element_to_string` prints it —
    /// `1`, `Q_i`, `P(r_0, r_1, …)`, or a space-separated product `Q_i … P(R)` —
    /// into its `(topological degree, index)`. The inverse of
    /// [`Self::basis_element_to_string`]; returns `None` on a malformed string or a
    /// monomial absent from the basis. This is what lets `.json` module descriptors
    /// be written over the motivic Steenrod algebra.
    pub fn basis_element_from_string(&self, elt: &str) -> Option<(i32, usize)> {
        let mut e_mask: u32 = 0;
        let mut r: Vec<u32> = Vec::new();
        // The `P(…)` block (if any) is last and may contain spaces after commas, so
        // split it off before whitespace-tokenizing the `Q_i` factors.
        let (q_part, p_part) = match elt.split_once("P(") {
            Some((q, rest)) => (q, Some(rest.strip_suffix(')')?)),
            None => (elt, None),
        };
        for token in q_part.split_whitespace() {
            if token == "1" {
                continue; // the unit factor
            }
            let i: u32 = token.strip_prefix("Q_")?.parse().ok()?;
            e_mask |= 1 << i;
        }
        if let Some(p) = p_part {
            r = p
                .split(',')
                .map(|x| x.trim().parse::<u32>())
                .collect::<Result<_, _>>()
                .ok()?;
        }

        let m = Monomial::from_paper(e_mask, &r)?;
        let degree = m.bidegree().0;
        if degree < 0 {
            return None;
        }
        self.compute_basis(degree);
        // The basis is stored trimmed, so trimming the parsed vector makes this an exact match.
        self.index_of(degree, &m).map(|idx| (degree, idx))
    }

    /// A display string for a basis element (`Q_i … P(R)`).
    pub fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        let m = self.basis_element(degree, idx);
        let mut parts = Vec::new();
        for i in fp::prime::iter::BitflagIterator::set_bit_iterator(m.q_part as u64) {
            parts.push(format!("Q_{i}"));
        }
        if !m.p_part.is_empty() {
            parts.push(format!("P({})", m.paper_p_part().iter().format(", ")));
        }
        if parts.is_empty() {
            "1".to_string()
        } else {
            parts.join(" ")
        }
    }

    /// The dense [`ProductBlock`] for the degree pair `(t1, t2)`, created (with all
    /// entries empty) on first request and shared thereafter. Creation is the only
    /// step that touches the block registry's write lock; entry computation happens
    /// lock-free through the block's [`OnceLock`]s.
    fn block(&self, t1: i32, t2: i32) -> Arc<ProductBlock> {
        if let Some(block) = self.blocks.read().unwrap().get(&(t1, t2)) {
            return Arc::clone(block);
        }
        // Ensure the operand and output bases exist before sizing/indexing.
        self.compute_basis(t1);
        self.compute_basis(t2);
        self.compute_basis(t1 + t2);
        let dim1 = self.dimension(t1);
        let dim2 = self.dimension(t2);
        let mut w = self.blocks.write().unwrap();
        Arc::clone(w.entry((t1, t2)).or_insert_with(|| {
            Arc::new(ProductBlock {
                dim2,
                entries: (0..dim1 * dim2).map(|_| OnceLock::new()).collect(),
            })
        }))
    }

    /// The closed-form product (Kong–Lin Theorem 5.1) of the two basis elements at
    /// `(t1, idx1)` and `(t2, idx2)`, mapped into the degree-`(t1+t2)` basis. The
    /// bases must already be computed (the caller through [`Self::block`] ensures
    /// this).
    fn compute_product(&self, t1: i32, idx1: usize, t2: i32, idx2: usize) -> Vec<(Tau, usize)> {
        let a = &self.basis[t1 as usize][idx1];
        let b = &self.basis[t2 as usize][idx2];
        let t = t1 + t2;
        multiply_closed(*a, *b)
            .into_iter()
            .map(|(z, c)| {
                (
                    c,
                    self.index_of(t, &z)
                        .expect("product landed outside the basis"),
                )
            })
            .collect()
    }

    /// The product of two basis elements, as an $\mathbb{F}_2[\tau]$-linear combination of basis
    /// elements in degree `t1 + t2`: a list of `(coefficient, index)` pairs. The product of two
    /// homogeneous basis elements is weight-homogeneous, so each coefficient is a single power of
    /// $\tau$ — a [`Tau`] scalar.
    pub fn product_indexed(&self, t1: i32, idx1: usize, t2: i32, idx2: usize) -> Vec<(Tau, usize)> {
        self.cached_product(&self.block(t1, t2), t1, idx1, t2, idx2)
            .to_vec()
    }

    /// One entry of `block`, computed on first request. `block` must be the block for
    /// `(t1, t2)`; it is passed in so [`Self::fill_block`] can look it up once for a whole
    /// block instead of per entry.
    fn cached_product<'a>(
        &self,
        block: &'a ProductBlock,
        t1: i32,
        idx1: usize,
        t2: i32,
        idx2: usize,
    ) -> &'a [(Tau, usize)] {
        block.entries[idx1 * block.dim2 + idx2]
            .get_or_init(|| self.compute_product(t1, idx1, t2, idx2))
    }

    /// Compute *every* entry of the `(t1, t2)` product block at once, filling it in
    /// parallel (under the `concurrent` feature) — the batch shape a GPU kernel
    /// fills in a single launch. Idempotent: entries already present are kept, and
    /// a second call is a cheap no-op. This is the batching boundary the GPU port
    /// hooks into; on CPU it lets a caller warm a whole degree pair up front
    /// instead of paying a lock-free-but-serial miss per structure constant.
    pub fn fill_block(&self, t1: i32, t2: i32) {
        let block = self.block(t1, t2);
        let dim2 = block.dim2;
        if dim2 == 0 || block.entries.is_empty() {
            return;
        }
        let dim1 = block.entries.len() / dim2;
        (0..dim1).into_maybe_par_iter().for_each(|idx1| {
            for idx2 in 0..dim2 {
                self.cached_product(&block, t1, idx1, t2, idx2);
            }
        });
    }
}

impl std::fmt::Display for MotivicMilnorAlgebra {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "MotivicMilnorAlgebra(C, p=2)")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    /// The classical mod-2 Milnor product of `P(a) * P(b)`, as a list of exponent sequences.
    ///
    /// The motivic layer works with the paper's `R = [r_0, r_1, ...]` indexing, where the
    /// leading slot belongs to ξ_0 = 1; the classical [`PPart`] starts at r_1. Callers convert
    /// with [`pp_to_paper`]/[`paper_to_pp`].
    fn classical_mul(a: &[u32], b: &[u32]) -> Vec<Vec<u32>> {
        use fp::{prime::TWO, vector::FpVector};

        use crate::algebra::{
            Algebra,
            milnor_algebra::{MilnorAlgebra, MilnorBasisElement, PPart},
        };

        // `MilnorAlgebra` is cheap to build but caches its basis, so keep one per process
        // rather than recomputing the tables for every product in a loop.
        thread_local! {
            static ALG: MilnorAlgebra = MilnorAlgebra::new(TWO, false);
        }
        ALG.with(|alg| {
            let mk = |p: &[u32]| {
                let mut m = MilnorBasisElement {
                    q_part: 0,
                    p_part: PPart::try_from_slice(p).unwrap(),
                    degree: 0,
                };
                m.compute_degree(TWO);
                m
            };
            let (m1, m2) = (mk(a), mk(b));
            let degree = m1.degree + m2.degree;
            alg.compute_basis(degree);
            let mut res = FpVector::new(TWO, alg.dimension(degree));
            alg.multiply(res.as_slice_mut(), 1, m1, m2);
            res.iter_nonzero()
                // `PPart::iter` stops at the last non-zero entry, so this is already trimmed.
                .map(|(idx, _)| {
                    alg.basis_element_from_index(degree, idx)
                        .p_part
                        .iter()
                        .collect()
                })
                .collect()
        })
    }

    /// A classical exponent sequence as the paper's `R`, with the ξ_0 slot prepended.
    fn pp_to_paper(pp: &[u32]) -> Vec<u32> {
        let mut r = vec![0];
        r.extend_from_slice(pp);
        trim(r)
    }

    /// A monomial from paper-indexed exponents, panicking on anything out of packing range.
    fn mon(q_part: u32, r: &[u32]) -> Monomial {
        Monomial::from_paper(q_part, r).unwrap()
    }

    /// The pure-ξ monomial with the given classical exponent sequence.
    fn paper(pp: &[u32]) -> Monomial {
        mon(0, &pp_to_paper(pp))
    }

    #[test]
    fn test_algebra_basis_and_multiply() {
        let alg = MotivicMilnorAlgebra::new();
        alg.compute_basis(8);

        // Degree 0 is the unit; degrees 1 and 2 are 1-dimensional (Q_0 and P(ξ_1)).
        assert_eq!(alg.dimension(0), 1);
        assert_eq!(alg.basis_element(0, 0), &mon(0u32, &[]));
        assert_eq!(alg.dimension(1), 1);
        assert_eq!(alg.basis_element(1, 0), &mon(0b1u32, &[])); // Q_0
        assert_eq!(alg.dimension(2), 1);
        assert_eq!(alg.basis_element(2, 0), &mon(0u32, &[0, 1])); // P(ξ_1)

        // bidegree: Q_0 is (1, 0), P(ξ_1) is (2, -1) in this presentation.
        assert_eq!(alg.bidegree(1, 0), (1, 0));
        assert_eq!(alg.bidegree(2, 0), (2, -1));

        // Q_0 · P(ξ_1) = Q_1 + Q_0 P(ξ_1), reconstructed from indices.
        let terms: DualElement = alg
            .product_indexed(1, 0, 2, 0)
            .into_iter()
            .map(|(c, idx)| (*alg.basis_element(3, idx), c))
            .collect();
        assert_eq!(
            terms,
            DualElement::from([
                (mon(0b10, &[]), Tau::one()),
                (mon(0b1, &[0, 1]), Tau::one())
            ])
        );
    }

    #[test]
    fn test_algebra_multiply_matches_raw_and_is_homogeneous() {
        let alg = MotivicMilnorAlgebra::new();
        alg.compute_basis(12);
        for t1 in 0..=6 {
            for idx1 in 0..alg.dimension(t1) {
                for t2 in 0..=6 {
                    for idx2 in 0..alg.dimension(t2) {
                        let a = *alg.basis_element(t1, idx1);
                        let b = *alg.basis_element(t2, idx2);
                        // Indexed product agrees with the raw monomial product.
                        let indexed: DualElement = alg
                            .product_indexed(t1, idx1, t2, idx2)
                            .into_iter()
                            .map(|(c, idx)| (*alg.basis_element(t1 + t2, idx), c))
                            .collect();
                        assert_eq!(indexed, multiply(a, b));

                        // Weight-homogeneous: w_out - (τ-power) = w_a + w_b.
                        let (wa, wb) = (alg.bidegree(t1, idx1).1, alg.bidegree(t2, idx2).1);
                        for (c, idx) in alg.product_indexed(t1, idx1, t2, idx2) {
                            let w = alg.bidegree(t1 + t2, idx).1;
                            assert_eq!(w - c.valuation().unwrap() as i32, wa + wb);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_dual_mul_relations() {
        // τ_0^2 = τ ξ_1 (the defining relation at ρ = 0).
        assert_eq!(
            dual_mul(&tau_gen(0), &tau_gen(0)),
            DualElement::from([(mon(0, &[0, 1]), Tau::power(1))]) // τ^1 · ξ_1
        );
        // ξ_1^2 is just the monomial ξ_1^2.
        assert_eq!(
            dual_mul(&xi_gen(1), &xi_gen(1)),
            DualElement::from([(mon(0, &[0, 2]), Tau::one())])
        );
        // Distinct τ's commute and stay square-free: τ_0 τ_1.
        assert_eq!(
            dual_mul(&tau_gen(0), &tau_gen(1)),
            DualElement::from([(mon(0b11, &[]), Tau::one())])
        );
        // τ_1^2 = τ ξ_2.
        assert_eq!(
            dual_mul(&tau_gen(1), &tau_gen(1)),
            DualElement::from([(mon(0, &[0, 0, 1]), Tau::power(1))])
        );
        // Multiplication by the unit is the identity.
        assert_eq!(dual_mul(&dual_one(), &tau_gen(2)), tau_gen(2));
    }

    #[test]
    fn test_pure_xi_matches_classical_milnor() {
        // Motivically the even part is NOT closed: P(R_1)·P(R_2) can leak into Q-terms
        // carrying τ (e.g. Sq^2·Sq^2 ∋ τ Q_0 Q_1). But the pure-ξ *outputs* (E = 0) are τ-free
        // and must agree with the ordinary mod-2 Milnor product. Compare those against the
        // codebase's `MilnorAlgebra` (paper R = [0, p_part...]; drop the ξ_0 slot to convert).
        let classical = |p1: &[u32], p2: &[u32]| -> Vec<Vec<u32>> {
            let mut out = classical_mul(p1, p2);
            out.sort();
            out
        };

        let motivic = |p1: &[u32], p2: &[u32]| -> Vec<Vec<u32>> {
            let mut out: Vec<Vec<u32>> = multiply(paper(p1), paper(p2))
                .into_iter()
                .filter(|(m, _)| m.q_part == 0) // keep the pure-ξ outputs
                .map(|(m, c)| {
                    assert_eq!(c, Tau::one(), "pure-ξ output carried a τ power");
                    m.p_part.iter().collect()
                })
                .collect();
            out.sort();
            out
        };

        // The paper uses *conjugate* generators, so P(paper R) differs from the codebase's
        // standard Milnor basis by the antipode χ for ξ_2 and higher (e.g. χ(ξ_2)=ξ_2+ξ_1^3).
        // They agree on the self-conjugate ξ_1-power sector (ξ_1 is primitive), so restrict the
        // comparison to outputs that are pure powers of ξ_1 (a `p_part` of length ≤ 1).
        let xi1_only = |mut v: Vec<Vec<u32>>| {
            v.retain(|pp| pp.len() <= 1);
            v
        };
        for (p1, p2) in [
            (vec![1u32], vec![1u32]),
            (vec![2], vec![1]),
            (vec![1], vec![2]),
            (vec![3], vec![1]),
            (vec![2, 1], vec![1]),
            (vec![0, 1], vec![0, 1]),
            (vec![2], vec![2]),
            (vec![3], vec![3]),
        ] {
            assert_eq!(
                xi1_only(motivic(&p1, &p2)),
                xi1_only(classical(&p1, &p2)),
                "ξ_1-sector mismatch for {p1:?} * {p2:?}"
            );
        }
    }

    #[test]
    fn test_full_xi_matches_classical_via_antipode() {
        // Full (all-ξ) cross-check against the codebase's classical Milnor product, using the
        // antipode χ to reconcile the paper's conjugate generators with the codebase's standard
        // ones. Identity checked (pure-ξ, τ-free, so over 𝔽₂): expressing both inputs and
        // outputs of the motivic conjugate product in the standard basis via χ reproduces the
        // codebase product. `A[μ][W] = coeff of conj-mon μ in std ξ(W) = [χ(mon_W)]_μ`.
        let mdeg = |pp: &[u32]| paper(pp).bidegree().0;
        let xi_pps = |t: i32| -> Vec<Vec<u32>> {
            enum_basis(t)
                .into_iter()
                .filter(|m| m.q_part == 0)
                .map(|m| m.p_part.iter().collect())
                .collect()
        };
        let a_coeff = |mu_pp: &[u32], w_pp: &[u32]| -> u32 {
            let ap = antipode(&DualElement::from([(paper(w_pp), Tau::one())]));
            // The antipode of a pure-ξ element is pure-ξ and τ-free, so a present monomial has
            // coefficient τ^0.
            ap.get(&paper(mu_pp))
                .map_or(0, |&c| u32::from(c == Tau::one()))
        };
        let cb_mul = |a_pp: &[u32], b_pp: &[u32], acc: &mut BTreeMap<Vec<u32>, u32>| {
            for pp in classical_mul(a_pp, b_pp) {
                *acc.entry(pp).or_insert(0) ^= 1;
            }
        };

        for (r1, r2) in [
            (vec![2u32], vec![1u32]),
            (vec![1], vec![2]),
            (vec![0, 1], vec![1]),
            (vec![2], vec![2]),
            (vec![3], vec![1]),
            (vec![1, 1], vec![1]),
            (vec![4], vec![1]),
            (vec![2, 1], vec![2]),
        ] {
            // LHS: motivic conjugate product (E=0), converted to std via χ.
            let conj: Vec<Vec<u32>> = multiply(paper(&r1), paper(&r2))
                .into_iter()
                .filter(|(m, _)| m.q_part == 0)
                .map(|(m, _)| m.p_part.iter().collect())
                .collect();
            let mut lhs: BTreeMap<Vec<u32>, u32> = BTreeMap::new();
            for w in xi_pps(mdeg(&r1) + mdeg(&r2)) {
                let s = conj.iter().fold(0u32, |acc, mu| acc ^ a_coeff(mu, &w));
                if s != 0 {
                    lhs.insert(w, s);
                }
            }

            // RHS: convert inputs to std via χ, multiply in the codebase.
            let in1: Vec<Vec<u32>> = xi_pps(mdeg(&r1))
                .into_iter()
                .filter(|w| a_coeff(&r1, w) == 1)
                .collect();
            let in2: Vec<Vec<u32>> = xi_pps(mdeg(&r2))
                .into_iter()
                .filter(|w| a_coeff(&r2, w) == 1)
                .collect();
            let mut rhs: BTreeMap<Vec<u32>, u32> = BTreeMap::new();
            for w1 in &in1 {
                for w2 in &in2 {
                    cb_mul(w1, w2, &mut rhs);
                }
            }
            rhs.retain(|_, v| *v != 0);

            assert_eq!(lhs, rhs, "full ξ cross-check failed for {r1:?} * {r2:?}");
        }
    }

    #[test]
    fn test_product_associative() {
        // Associativity (a·b)·c = a·(b·c) is convention-independent and a strong global check.
        // Extend the basis×basis product to element×basis, F_2[τ]-linearly (add_term handles the
        // coefficient sum + zero cancellation via Tau arithmetic).
        fn mul_elt_basis(x: &DualElement, c: &Monomial) -> DualElement {
            let mut out = DualElement::new();
            for &(z, cz) in x {
                for (w, cw) in multiply(z, *c) {
                    out.add_term(w, cz * cw);
                }
            }
            out
        }

        let basis: Vec<Monomial> = (0..=5).flat_map(enum_basis).collect();
        for a in &basis {
            for b in &basis {
                let ab = multiply(*a, *b);
                for c in &basis {
                    let lhs = mul_elt_basis(&ab, c);
                    let bc = multiply(*b, *c);
                    // a · (b·c): multiply basis `a` on the left of each term of bc.
                    let mut rhs = DualElement::new();
                    for &(z, cz) in &bc {
                        for (w, cw) in multiply(*a, z) {
                            rhs.add_term(w, cz * cw);
                        }
                    }
                    assert_eq!(lhs, rhs, "associativity failed at {a:?},{b:?},{c:?}");
                }
            }
        }
    }

    #[test]
    fn test_product_q0_p_xi1() {
        // The case my reading of Kong–Lin Theorem 5.1 got wrong. By duality:
        //   Q_0 · P(ξ_1) = Q_1 + Q_0 P(ξ_1)   (both with coefficient τ^0).
        let q0 = mon(0b1, &[]);
        let p_xi1 = mon(0, &[0, 1]);
        assert_eq!(
            multiply(q0, p_xi1),
            DualElement::from([
                (mon(0b10, &[]), Tau::one()),    // Q_1
                (mon(0b1, &[0, 1]), Tau::one()), // Q_0 P(ξ_1)
            ])
        );
    }

    #[test]
    fn test_product_unit_and_squares() {
        // 1 · x = x.
        let x = mon(0b101, &[0, 2]);
        assert_eq!(
            multiply(mon(0, &[]), x),
            DualElement::from([(x, Tau::one())])
        );
        // Q_i^2 = 0. (Q_0 = Sq^1 is the motivic Bockstein; P(ξ_1) = Sq^2 does NOT square to
        // zero — its square is τ Q_0 Q_1 + …, a genuine motivic feature.)
        for i in 0..3 {
            assert!(
                multiply(
                    Monomial::new(1 << i, PPart::zero()),
                    Monomial::new(1 << i, PPart::zero())
                )
                .is_empty(),
                "Q_{i}^2 ≠ 0"
            );
        }
    }

    #[test]
    fn test_product_weight_homogeneous_and_tau_appears() {
        // Every product is weight-homogeneous: term z occurs with a single power τ^k, and by
        // weight-preservation of ψ, k = (w_a + w_b) - w_z ≥ 0 in *dual*-monomial weights (the
        // algebra weight of a basis element is the negative of its dual monomial's weight, so
        // τ, of weight -1, absorbs the difference). Also confirm τ genuinely enters at least
        // one product (i.e. the τ_i^2 = τξ_{i+1} relation fires).
        let basis: Vec<Monomial> = (0..=6).flat_map(enum_basis).collect();
        let mut saw_tau = false;
        for a in &basis {
            for b in &basis {
                let (wa, wb) = (a.bidegree().1, b.bidegree().1);
                for (z, c) in multiply(*a, *b) {
                    // Each coefficient is a single τ power by construction (a homogeneous Tau).
                    let k = c.valuation().expect("stored coefficients are never zero") as i32;
                    let wz = z.bidegree().1;
                    assert_eq!(wa + wb - wz, k, "weight mismatch: {a:?}*{b:?} → z={z:?}");
                    assert!(k >= 0);
                    saw_tau |= k > 0;
                }
            }
        }
        assert!(
            saw_tau,
            "no product produced a τ coefficient — τ_i^2 relation never exercised"
        );
    }

    #[test]
    fn test_antipode() {
        // χ(ξ_1) = ξ_1 (primitive); χ(ξ_2) = ξ_2 + ξ_1^3; χ(τ_0) = τ_0.
        assert_eq!(antipode(&xi_gen(1)), xi_gen(1));
        assert_eq!(
            antipode(&xi_gen(2)),
            DualElement::from([
                (mon(0, &[0, 0, 1]), Tau::one()),
                (mon(0, &[0, 3]), Tau::one())
            ])
        );
        assert_eq!(antipode(&tau_gen(0)), tau_gen(0));

        // χ is an involution: χ(χ(x)) = x.
        for x in [
            xi_gen(1),
            xi_gen(2),
            xi_gen(3),
            tau_gen(0),
            tau_gen(1),
            tau_gen(2),
            dual_mul(&tau_gen(0), &xi_gen(2)),
        ] {
            assert_eq!(antipode(&antipode(&x)), x, "χ not an involution on {x:?}");
        }

        // Antipode axiom: m(χ ⊗ id)ψ(x) = η ε(x); for positive-degree x this is 0.
        let counit_via_antipode = |x: &DualElement| -> DualElement {
            let mut out = DualElement::new();
            for ((l, r), c) in coproduct(x) {
                let prod = dual_mul(
                    &antipode(&DualElement::from([(l, Tau::one())])),
                    &DualElement::from([(r, Tau::one())]),
                );
                for (mon, cc) in prod {
                    out.add_term(mon, c * cc);
                }
            }
            out
        };
        for x in [
            xi_gen(2),
            xi_gen(3),
            tau_gen(1),
            tau_gen(2),
            dual_mul(&tau_gen(0), &tau_gen(1)),
        ] {
            assert!(
                counit_via_antipode(&x).is_empty(),
                "antipode axiom failed on {x:?}"
            );
        }
    }

    #[test]
    fn test_coproduct_generators() {
        // ψ(τ_1) = 1⊗τ_1 + τ_0⊗ξ_1 + τ_1⊗1  (Kong–Lin §2.2).
        assert_eq!(
            coproduct(&tau_gen(1)),
            TensorElement::from([
                ((mon(0, &[]), mon(0b10, &[])), Tau::one()),    // 1 ⊗ τ_1
                ((mon(0b1, &[]), mon(0, &[0, 1])), Tau::one()), // τ_0 ⊗ ξ_1
                ((mon(0b10, &[]), mon(0, &[])), Tau::one()),    // τ_1 ⊗ 1
            ])
        );
        // ψ(ξ_2) = 1⊗ξ_2 + ξ_1⊗ξ_1^2 + ξ_2⊗1  (matches Milnor).
        assert_eq!(
            coproduct(&xi_gen(2)),
            TensorElement::from([
                ((mon(0, &[]), mon(0, &[0, 0, 1])), Tau::one()), // 1 ⊗ ξ_2
                ((mon(0, &[0, 1]), mon(0, &[0, 2])), Tau::one()), // ξ_1 ⊗ ξ_1^2
                ((mon(0, &[0, 0, 1]), mon(0, &[])), Tau::one()), // ξ_2 ⊗ 1
            ])
        );
    }

    #[test]
    fn test_coproduct_is_algebra_map() {
        // ψ(xy) = ψ(x)ψ(y); in particular this checks that the τ_i^2 = τξ_{i+1} reduction is
        // compatible with the (τ-free) coproduct.
        for (x, y) in [
            (tau_gen(0), tau_gen(0)),
            (tau_gen(1), tau_gen(1)),
            (tau_gen(0), tau_gen(1)),
            (xi_gen(1), xi_gen(1)),
            (tau_gen(0), xi_gen(1)),
            (tau_gen(1), xi_gen(2)),
        ] {
            let lhs = coproduct(&dual_mul(&x, &y));
            let rhs = tensor_mul(&coproduct(&x), &coproduct(&y));
            assert_eq!(lhs, rhs, "ψ not an algebra map on {x:?} * {y:?}");
        }
    }

    #[test]
    fn test_rewrite_tau_square_free_is_identity() {
        // A square-free τ(E) is already a basis element: τ_0 τ_2 → Q(E)P(0), τ^0.
        let terms = rewrite_tau(&[1, 0, 1]);
        assert_eq!(
            terms,
            vec![MotivicTerm {
                tau_pow: 0,
                e_mask: 0b101,
                r: vec![],
            }]
        );
    }

    #[test]
    fn test_rewrite_tau_example_3_1() {
        // Kong–Lin Example 3.1, specialized to ρ = 0 (C-motivic):
        //   τ_0^2 τ_1 = τ · τ_1 ξ_1
        // Encoded: S = (2, 1); expected single term τ^1 · Q({1}) P((0,1)).
        let terms = rewrite_tau(&[2, 1]);
        assert_eq!(
            terms,
            vec![MotivicTerm {
                tau_pow: 1,
                e_mask: 0b10,  // τ_1
                r: vec![0, 1], // ξ_1^1
            }],
            "τ_0^2 τ_1 = τ τ_1 ξ_1 (ρ=0)"
        );

        //   τ_0^4 = τ^2 ξ_1^2
        // Encoded: S = (4); expected single term τ^2 · Q(∅) P((0,2)).
        let terms = rewrite_tau(&[4]);
        assert_eq!(
            terms,
            vec![MotivicTerm {
                tau_pow: 2,
                e_mask: 0,
                r: vec![0, 2], // ξ_1^2
            }],
            "τ_0^4 = τ^2 ξ_1^2 (ρ=0)"
        );
    }

    #[test]
    fn test_closed_form_small_cases() {
        // The case a naive reading of Theorem 5.1 gets wrong (the ξ₀ = 1 index
        // absorption): Q_0 · P(ξ_1) = Q_1 + Q_0 P(ξ_1).
        let q0 = mon(0b1, &[]);
        let p_xi1 = mon(0, &[0, 1]);
        assert_eq!(multiply_closed(q0, p_xi1), multiply(q0, p_xi1));

        // The τ-generating case: P(ξ_1)² has a τ Q_0 Q_1 term.
        assert_eq!(multiply_closed(p_xi1, p_xi1), multiply(p_xi1, p_xi1));

        // Q_i² = 0.
        for i in 0..3 {
            assert_eq!(
                multiply_closed(
                    Monomial::new(1 << i, PPart::zero()),
                    Monomial::new(1 << i, PPart::zero())
                ),
                multiply(
                    Monomial::new(1 << i, PPart::zero()),
                    Monomial::new(1 << i, PPart::zero())
                )
            );
        }
    }

    #[test]
    fn test_closed_form_matches_duality_oracle() {
        // Exhaustive fuzz: the closed form (Theorem 5.1) agrees with the duality
        // oracle on every ordered pair of basis elements with deg(a)+deg(b) ≤ 18,
        // including the τ-carrying and multi-Q cases. (The bound is kept modest
        // because the *oracle* is slow; the closed form is not.)
        let basis: Vec<(i32, Monomial)> = (0..=14)
            .flat_map(|t| enum_basis(t).into_iter().map(move |m| (t, m)))
            .collect();
        for (ta, a) in &basis {
            for (tb, b) in &basis {
                if ta + tb > 18 {
                    continue;
                }
                assert_eq!(
                    multiply_closed(*a, *b),
                    multiply(*a, *b),
                    "closed form ≠ oracle for {a:?} · {b:?}"
                );
            }
        }
    }

    #[test]
    fn test_fill_block_matches_lazy_products() {
        // The batch path (`fill_block`, which a GPU kernel would drive) must agree
        // entry-for-entry with the lazy per-product path. Fill whole degree-pair
        // blocks up front on one algebra, compute the same products lazily on
        // another, and compare every structure constant.
        let batched = MotivicMilnorAlgebra::new();
        let lazy = MotivicMilnorAlgebra::new();
        batched.compute_basis(12);
        lazy.compute_basis(12);
        for t1 in 0..=6 {
            for t2 in 0..=6 {
                batched.fill_block(t1, t2);
                for idx1 in 0..batched.dimension(t1) {
                    for idx2 in 0..batched.dimension(t2) {
                        assert_eq!(
                            batched.product_indexed(t1, idx1, t2, idx2),
                            lazy.product_indexed(t1, idx1, t2, idx2),
                            "batch ≠ lazy at ({t1},{idx1})·({t2},{idx2})"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_rewrite_tau_unit() {
        assert_eq!(
            rewrite_tau(&[]),
            vec![MotivicTerm {
                tau_pow: 0,
                e_mask: 0,
                r: vec![],
            }]
        );
    }
}
