//! The C-motivic (over $\mathbb{C}$, prime 2) Steenrod algebra in the Milnor basis.
//!
//! This follows Kong–Lin, *Product formulas for motivic Milnor basis* (arXiv:2411.12890),
//! specialized to the C-motivic point where $\rho = 0$ (the motivic cohomology of a point
//! over $\mathbb{C}$ is $\mathbb{F}_2[\tau]$). We use the *conjugate* generators of that
//! paper, so the coproduct and Milnor-matrix product match Milnor's classical formulas on
//! the polynomial (`ξ`) part.
//!
//! The dual algebra is $A_{**} = \mathbb{F}_2[\tau][\xi_1, \xi_2, \dots] \otimes
//! E(\tau_0, \tau_1, \dots)$ with $\tau_i^2 = \tau \xi_{i+1}$ (the $\rho = 0$ reduction of
//! $\tau_i^2 = \tau\xi_{i+1} + \rho\tau_{i+1}$). Bidegrees: $|\xi_i| = (2(2^i-1), 2^i-1)$,
//! $|\tau_i| = (2^{i+1}-1, 2^i-1)$, $|\tau| = (0, -1)$; see
//! [`combinatorics::motivic_bidegree`](super::combinatorics::motivic_bidegree).
//!
//! Basis elements are $Q(E)P(R)$ with $E \in \mathrm{Seq}_1$ (entries in $\{0,1\}$, encoded
//! as a bitmask: bit $i$ is $\tau_i$) and $R \in \mathrm{Seq}$ (the exponent vector of the
//! $\xi_i$).
//!
//! The product in the Steenrod algebra $A$ is computed by **duality**: the dual algebra
//! $A_{**}$ is a commutative $\mathbb{F}_2[\tau]$-algebra whose multiplication ([`dual_mul`])
//! reduces $\tau_i^2 = \tau\xi_{i+1}$ via [`rewrite_tau`] (Kong–Lin Theorem 3.4), and its
//! coproduct $\psi$ ([`coproduct`], Kong–Lin §2.2) is a $\tau$-free algebra map. The product
//! $a \cdot b$ in $A$ is then read off from $\psi$: the coefficient of $z$ in $a \cdot b$ is
//! the coefficient of $\mathrm{mon}(a) \otimes \mathrm{mon}(b)$ in $\psi(\mathrm{mon}(z))$
//! ([`multiply`]). This is a first, correctness-oriented implementation; the closed-form
//! product (Kong–Lin Theorem 5.1) can replace it later for speed, validated against this one.
//!
//! Weight convention: the motivic weight of an algebra basis element is the *negative* of the
//! weight of the dual monomial it pairs with, so that products are weight-homogeneous with
//! $\tau$ (weight $-1$) absorbing the difference.

use std::collections::BTreeMap;

use fp::prime::Binomial;

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
        let mut r_trimmed = r.to_vec();
        while let Some(&0) = r_trimmed.last() {
            r_trimmed.pop();
        }
        out.push(MotivicTerm {
            tau_pow: sigma(r),
            e_mask,
            r: r_trimmed,
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
// The dual algebra A_** (used to compute the product in A by duality).
// ---------------------------------------------------------------------------

/// An element of $\mathbb{F}_2[\tau]$, encoded as a bitmask of exponents: bit $e$ set means
/// $\tau^e$ appears (coefficients mod 2). Addition is `XOR`; multiplication by $\tau^j$ is a
/// left shift; general multiplication is carry-less (polynomial) multiplication.
type TauPoly = u64;

/// An element of the dual algebra $A_{**}$: basis monomials $\tau(E)\xi(R)$ (`E` a square-free
/// bitmask, `R` a trimmed exponent vector) mapped to their $\mathbb{F}_2[\tau]$ coefficients.
/// Zero coefficients are never stored, so equality is canonical.
pub type DualElement = BTreeMap<(u32, Vec<u32>), TauPoly>;

/// Carry-less ($\mathbb{F}_2[\tau]$) product of two coefficients.
fn tau_mul(a: TauPoly, b: TauPoly) -> TauPoly {
    let mut res = 0;
    for k in 0..u64::BITS {
        if (a >> k) & 1 != 0 {
            res ^= b << k;
        }
    }
    res
}

/// Elementwise sum of two exponent vectors, trimmed of trailing zeros.
fn vec_add(a: &[u32], b: &[u32]) -> Vec<u32> {
    let n = a.len().max(b.len());
    let mut r: Vec<u32> = (0..n)
        .map(|i| a.get(i).copied().unwrap_or(0) + b.get(i).copied().unwrap_or(0))
        .collect();
    while let Some(&0) = r.last() {
        r.pop();
    }
    r
}

/// Add `coeff * mon` into `acc`, dropping the entry if it cancels to zero.
fn dual_add(acc: &mut DualElement, mon: (u32, Vec<u32>), coeff: TauPoly) {
    use std::collections::btree_map::Entry;
    if coeff == 0 {
        return;
    }
    match acc.entry(mon) {
        Entry::Occupied(mut o) => {
            *o.get_mut() ^= coeff;
            if *o.get() == 0 {
                o.remove();
            }
        }
        Entry::Vacant(v) => {
            v.insert(coeff);
        }
    }
}

/// Multiply two basis monomials of $A_{**}$ and accumulate `coeff * (m1 * m2)` into `acc`.
///
/// $A_{**}$ is commutative, so $\tau(E_1)\xi(R_1)\cdot\tau(E_2)\xi(R_2)$ is obtained by adding
/// exponents: the exterior parts form $S = E_1 + E_2$ (entries in $\{0,1,2\}$), which is
/// rewritten into the square-free basis via [`rewrite_tau`] ($\tau_i^2 = \tau\xi_{i+1}$), and
/// the resulting $\xi$ exponents are added to $R_1 + R_2$.
fn mul_monomials(
    m1: &(u32, Vec<u32>),
    m2: &(u32, Vec<u32>),
    coeff: TauPoly,
    acc: &mut DualElement,
) {
    let (e1, e2) = (m1.0, m2.0);
    let bits = u32::BITS - (e1 | e2).leading_zeros();
    let s: Vec<u32> = (0..bits)
        .map(|i| ((e1 >> i) & 1) + ((e2 >> i) & 1))
        .collect();
    let r12 = vec_add(&m1.1, &m2.1);
    for term in rewrite_tau(&s) {
        let r = vec_add(&term.r, &r12);
        dual_add(acc, (term.e_mask, r), tau_mul(coeff, 1 << term.tau_pow));
    }
}

/// The product of two elements of $A_{**}$.
pub fn dual_mul(a: &DualElement, b: &DualElement) -> DualElement {
    let mut out = DualElement::new();
    for (m1, &c1) in a {
        for (m2, &c2) in b {
            mul_monomials(m1, m2, tau_mul(c1, c2), &mut out);
        }
    }
    out
}

/// The generator $\tau_i \in A_{**}$.
pub fn tau_gen(i: usize) -> DualElement {
    DualElement::from([((1u32 << i, vec![]), 1)])
}

/// The generator $\xi_i \in A_{**}$ (for $i \ge 1$).
pub fn xi_gen(i: usize) -> DualElement {
    let mut r = vec![0u32; i + 1];
    r[i] = 1;
    DualElement::from([((0, r), 1)])
}

/// The unit $1 \in A_{**}$.
pub fn dual_one() -> DualElement {
    DualElement::from([((0, vec![]), 1)])
}

// ---------------------------------------------------------------------------
// The coproduct ψ: A_** → A_** ⊗ A_**, an algebra map (Kong–Lin §2.2).
// ---------------------------------------------------------------------------

/// An element of $A_{**} \otimes A_{**}$: pairs of basis monomials → $\mathbb{F}_2[\tau]$
/// coefficients. Zero coefficients are never stored.
pub type TensorElement = BTreeMap<((u32, Vec<u32>), (u32, Vec<u32>)), TauPoly>;

fn tensor_add(acc: &mut TensorElement, key: ((u32, Vec<u32>), (u32, Vec<u32>)), coeff: TauPoly) {
    use std::collections::btree_map::Entry;
    if coeff == 0 {
        return;
    }
    match acc.entry(key) {
        Entry::Occupied(mut o) => {
            *o.get_mut() ^= coeff;
            if *o.get() == 0 {
                o.remove();
            }
        }
        Entry::Vacant(v) => {
            v.insert(coeff);
        }
    }
}

/// The unit $1 \otimes 1$.
fn tensor_one() -> TensorElement {
    TensorElement::from([(((0, vec![]), (0, vec![])), 1)])
}

/// Multiply in $A_{**} \otimes A_{**}$: $(a_L \otimes a_R)(b_L \otimes b_R) = (a_L b_L) \otimes (a_R b_R)$.
fn tensor_mul(t1: &TensorElement, t2: &TensorElement) -> TensorElement {
    let mut out = TensorElement::new();
    for ((al, ar), &c1) in t1 {
        for ((bl, br), &c2) in t2 {
            let mut left = DualElement::new();
            mul_monomials(al, bl, 1, &mut left);
            let mut right = DualElement::new();
            mul_monomials(ar, br, 1, &mut right);
            let c = tau_mul(c1, c2);
            for (ml, &cl) in &left {
                for (mr, &cr) in &right {
                    tensor_add(
                        &mut out,
                        (ml.clone(), mr.clone()),
                        tau_mul(c, tau_mul(cl, cr)),
                    );
                }
            }
        }
    }
    out
}

/// The monomial $\xi_j^p$ (with $\xi_0 = 1$).
fn xi_pow_mon(j: usize, p: u32) -> (u32, Vec<u32>) {
    if j == 0 || p == 0 {
        (0, vec![])
    } else {
        let mut r = vec![0u32; j + 1];
        r[j] = p;
        (0, r)
    }
}

/// $\psi(\tau_k) = 1 \otimes \tau_k + \sum_{i=0}^{k} \tau_i \otimes \xi_{k-i}^{2^i}$.
fn coprod_tau(k: usize) -> TensorElement {
    let mut out = TensorElement::new();
    tensor_add(&mut out, ((0, vec![]), (1 << k, vec![])), 1);
    for i in 0..=k {
        tensor_add(&mut out, ((1 << i, vec![]), xi_pow_mon(k - i, 1 << i)), 1);
    }
    out
}

/// $\psi(\xi_k) = \sum_{i=0}^{k} \xi_i \otimes \xi_{k-i}^{2^i}$ (with $\xi_0 = 1$).
fn coprod_xi(k: usize) -> TensorElement {
    let mut out = TensorElement::new();
    for i in 0..=k {
        tensor_add(&mut out, (xi_pow_mon(i, 1), xi_pow_mon(k - i, 1 << i)), 1);
    }
    out
}

/// The coproduct of a basis monomial $\tau(E)\xi(R)$, computed as an algebra map: the product
/// of the coproducts of its generators.
fn coproduct_monomial(e_mask: u32, r: &[u32]) -> TensorElement {
    let mut acc = tensor_one();
    for i in 0..u32::BITS {
        if (e_mask >> i) & 1 != 0 {
            acc = tensor_mul(&acc, &coprod_tau(i as usize));
        }
    }
    for (j, &rj) in r.iter().enumerate() {
        for _ in 0..rj {
            acc = tensor_mul(&acc, &coprod_xi(j));
        }
    }
    acc
}

/// The coproduct of an arbitrary element of $A_{**}$ (extended $\mathbb{F}_2[\tau]$-linearly).
pub fn coproduct(elt: &DualElement) -> TensorElement {
    let mut out = TensorElement::new();
    for ((e, r), &c) in elt {
        for (key, cc) in coproduct_monomial(*e, r) {
            tensor_add(&mut out, key, tau_mul(c, cc));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The antipode χ: A_** → A_** (conjugate generators ⟷ standard Milnor generators).
// ---------------------------------------------------------------------------

/// $\xi_j^p \in A_{**}$ (with $\xi_0 = 1$).
fn xi_pow_elt(j: usize, p: u32) -> DualElement {
    DualElement::from([(xi_pow_mon(j, p), 1)])
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
            dual_add(&mut acc, mon, c);
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
            dual_add(&mut acc, mon, c);
        }
    }
    acc
}

/// The antipode $\chi$ of $A_{**}$ — an algebra map (since $A_{**}$ is commutative) extended
/// from [`chi_tau`]/[`chi_xi`] on the generators. It converts the paper's conjugate
/// generators to the standard Milnor/Voevodsky generators (`χ(ξ_2) = ξ_2 + ξ_1^3`, etc.) and
/// is an involution.
pub fn antipode(elt: &DualElement) -> DualElement {
    let mut out = DualElement::new();
    for ((e, r), &c) in elt {
        let mut m = dual_one();
        for i in 0..u32::BITS {
            if (e >> i) & 1 != 0 {
                m = dual_mul(&m, &chi_tau(i as usize));
            }
        }
        for (j, &rj) in r.iter().enumerate() {
            for _ in 0..rj {
                m = dual_mul(&m, &chi_xi(j));
            }
        }
        for (mon, cc) in m {
            dual_add(&mut out, mon, tau_mul(c, cc));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The product in the Steenrod algebra A, computed by dualizing ψ.
// ---------------------------------------------------------------------------

/// The C-motivic (prime 2) bidegree `(t, w)` of the basis monomial `τ(E)ξ(R)`, in the
/// *paper's* R-convention (`r[j]` is the exponent of `ξ_j`, with `ξ_0 = 1` so `r[0]` is
/// ignored). `|τ_i| = (2^{i+1}-1, 2^i-1)`, `|ξ_j| = (2^{j+1}-2, 2^j-1)`.
fn paper_bidegree(e_mask: u32, r: &[u32]) -> (i32, i32) {
    let (mut t, mut w) = (0i32, 0i32);
    for i in 0..u32::BITS {
        if (e_mask >> i) & 1 != 0 {
            t += (1 << (i + 1)) - 1;
            w += (1 << i) - 1;
        }
    }
    for (j, &rj) in r.iter().enumerate().skip(1) {
        t += rj as i32 * ((1 << (j + 1)) - 2);
        w += rj as i32 * ((1 << j) - 1);
    }
    (t, w)
}

/// All basis monomials `τ(E)ξ(R)` of a given topological degree `target`.
fn enum_basis(target: i32) -> Vec<(u32, Vec<u32>)> {
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
    out: &mut Vec<(u32, Vec<u32>)>,
) {
    if idx == gens.len() {
        if rem == 0 {
            let mut r = r.to_vec();
            while let Some(&0) = r.last() {
                r.pop();
            }
            out.push((*e, r));
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

/// The product `a · b` in the C-motivic Steenrod algebra `A`, where `a`, `b` are Milnor basis
/// elements `Q(E)P(R)` given as the monomials `(E, R)` they are dual to. The result is a map
/// from basis monomials to their `𝔽₂[τ]` coefficients.
///
/// Computed by duality: the coefficient of `z` in `a · b` is the coefficient of
/// `mon(a) ⊗ mon(b)` in `ψ(mon(z))`, summed over the basis `z` of the appropriate
/// topological degree.
pub fn multiply(a: &(u32, Vec<u32>), b: &(u32, Vec<u32>)) -> BTreeMap<(u32, Vec<u32>), TauPoly> {
    let key = (a.clone(), b.clone());
    let t = paper_bidegree(a.0, &a.1).0 + paper_bidegree(b.0, &b.1).0;
    let mut out = BTreeMap::new();
    for z in enum_basis(t) {
        if let Some(&c) = coproduct_monomial(z.0, &z.1).get(&key)
            && c != 0
        {
            out.insert(z, c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tau_mul() {
        assert_eq!(tau_mul(0b1, 0b1), 0b1); // 1 * 1 = 1
        assert_eq!(tau_mul(0b10, 0b10), 0b100); // τ * τ = τ^2
        assert_eq!(tau_mul(0b11, 0b1), 0b11); // (1+τ) * 1
        assert_eq!(tau_mul(0b11, 0b11), 0b101); // (1+τ)^2 = 1 + τ^2 (mod 2)
    }

    #[test]
    fn test_dual_mul_relations() {
        // τ_0^2 = τ ξ_1 (the defining relation at ρ = 0).
        assert_eq!(
            dual_mul(&tau_gen(0), &tau_gen(0)),
            DualElement::from([((0, vec![0, 1]), 0b10)]) // τ^1 · ξ_1
        );
        // ξ_1^2 is just the monomial ξ_1^2.
        assert_eq!(
            dual_mul(&xi_gen(1), &xi_gen(1)),
            DualElement::from([((0, vec![0, 2]), 1)])
        );
        // Distinct τ's commute and stay square-free: τ_0 τ_1.
        assert_eq!(
            dual_mul(&tau_gen(0), &tau_gen(1)),
            DualElement::from([((0b11, vec![]), 1)])
        );
        // τ_1^2 = τ ξ_2.
        assert_eq!(
            dual_mul(&tau_gen(1), &tau_gen(1)),
            DualElement::from([((0, vec![0, 0, 1]), 0b10)])
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
        use fp::{prime::TWO, vector::FpVector};

        use crate::algebra::{
            Algebra,
            milnor_algebra::{MilnorAlgebra, MilnorBasisElement},
        };

        let alg = MilnorAlgebra::new(TWO, false);
        let trim = |mut v: Vec<u32>| {
            while let Some(&0) = v.last() {
                v.pop();
            }
            v
        };

        let classical = |p1: &[u32], p2: &[u32]| -> Vec<Vec<u32>> {
            let mk = |p: &[u32]| {
                let mut m = MilnorBasisElement {
                    q_part: 0,
                    p_part: p.to_vec(),
                    degree: 0,
                };
                m.compute_degree(TWO);
                m
            };
            let (m1, m2) = (mk(p1), mk(p2));
            let deg = m1.degree + m2.degree;
            alg.compute_basis(deg);
            let mut res = FpVector::new(TWO, alg.dimension(deg));
            alg.multiply(res.as_slice_mut(), 1, &m1, &m2);
            let mut out: Vec<Vec<u32>> = res
                .iter_nonzero()
                .map(|(idx, _)| trim(alg.basis_element_from_index(deg, idx).p_part.clone()))
                .collect();
            out.sort();
            out
        };

        let motivic = |p1: &[u32], p2: &[u32]| -> Vec<Vec<u32>> {
            let to_paper = |pp: &[u32]| {
                let mut r = vec![0u32];
                r.extend_from_slice(pp);
                trim(r)
            };
            let mut out: Vec<Vec<u32>> = multiply(&(0, to_paper(p1)), &(0, to_paper(p2)))
                .into_iter()
                .filter(|((e, _), _)| *e == 0) // keep the pure-ξ outputs
                .map(|((_, r), c)| {
                    assert_eq!(c, 1, "pure-ξ output carried a τ power");
                    trim(r.get(1..).map(<[u32]>::to_vec).unwrap_or_default())
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
        use fp::{prime::TWO, vector::FpVector};

        use crate::algebra::{
            Algebra,
            milnor_algebra::{MilnorAlgebra, MilnorBasisElement},
        };

        let alg = MilnorAlgebra::new(TWO, false);
        let trim = |mut v: Vec<u32>| {
            while let Some(&0) = v.last() {
                v.pop();
            }
            v
        };
        let pp_to_paper = |pp: &[u32]| {
            let mut r = vec![0u32];
            r.extend_from_slice(pp);
            trim(r)
        };
        let paper_to_pp = |r: &[u32]| trim(r.get(1..).map(<[u32]>::to_vec).unwrap_or_default());
        let mdeg = |pp: &[u32]| paper_bidegree(0, &pp_to_paper(pp)).0;
        let xi_pps = |t: i32| -> Vec<Vec<u32>> {
            enum_basis(t)
                .into_iter()
                .filter(|(e, _)| *e == 0)
                .map(|(_, r)| paper_to_pp(&r))
                .collect()
        };
        let a_coeff = |mu_pp: &[u32], w_pp: &[u32]| -> u32 {
            let ap = antipode(&DualElement::from([((0u32, pp_to_paper(w_pp)), 1u64)]));
            (ap.get(&(0u32, pp_to_paper(mu_pp))).copied().unwrap_or(0) & 1) as u32
        };
        let cb_mul = |a_pp: &[u32], b_pp: &[u32], acc: &mut BTreeMap<Vec<u32>, u32>| {
            let mk = |p: &[u32]| {
                let mut m = MilnorBasisElement {
                    q_part: 0,
                    p_part: p.to_vec(),
                    degree: 0,
                };
                m.compute_degree(TWO);
                m
            };
            let (m1, m2) = (mk(a_pp), mk(b_pp));
            let deg = m1.degree + m2.degree;
            alg.compute_basis(deg);
            let mut res = FpVector::new(TWO, alg.dimension(deg));
            alg.multiply(res.as_slice_mut(), 1, &m1, &m2);
            for (idx, _) in res.iter_nonzero() {
                let pp = trim(alg.basis_element_from_index(deg, idx).p_part.clone());
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
            let conj: Vec<Vec<u32>> = multiply(&(0, pp_to_paper(&r1)), &(0, pp_to_paper(&r2)))
                .into_iter()
                .filter(|((e, _), _)| *e == 0)
                .map(|((_, r), _)| paper_to_pp(&r))
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
        // Extend the basis×basis product to element×basis.
        fn mul_elt_basis(
            x: &BTreeMap<(u32, Vec<u32>), TauPoly>,
            c: &(u32, Vec<u32>),
        ) -> BTreeMap<(u32, Vec<u32>), TauPoly> {
            let mut out: BTreeMap<(u32, Vec<u32>), TauPoly> = BTreeMap::new();
            for (z, &cz) in x {
                for (w, cw) in multiply(z, c) {
                    *out.entry(w).or_insert(0) ^= tau_mul(cz, cw);
                }
            }
            out.retain(|_, v| *v != 0);
            out
        }

        let basis: Vec<(u32, Vec<u32>)> = (0..=5).flat_map(enum_basis).collect();
        for a in &basis {
            for b in &basis {
                let ab = multiply(a, b);
                for c in &basis {
                    let lhs = mul_elt_basis(&ab, c);
                    let bc = multiply(b, c);
                    // a · (b·c): multiply basis `a` on the left of each term of bc.
                    let mut rhs = BTreeMap::new();
                    for (z, &cz) in &bc {
                        for (w, cw) in multiply(a, z) {
                            let e = rhs.entry(w).or_insert(0);
                            *e ^= tau_mul(cz, cw);
                        }
                    }
                    rhs.retain(|_, v| *v != 0);
                    assert_eq!(lhs, rhs, "associativity failed at {a:?},{b:?},{c:?}");
                }
            }
        }
    }

    #[test]
    fn test_product_q0_p_xi1() {
        // The case my reading of Kong–Lin Theorem 5.1 got wrong. By duality:
        //   Q_0 · P(ξ_1) = Q_1 + Q_0 P(ξ_1)   (both with coefficient τ^0).
        let q0 = (0b1, vec![]);
        let p_xi1 = (0, vec![0, 1]);
        assert_eq!(
            multiply(&q0, &p_xi1),
            BTreeMap::from([
                ((0b10, vec![]), 1),    // Q_1
                ((0b1, vec![0, 1]), 1), // Q_0 P(ξ_1)
            ])
        );
    }

    #[test]
    fn test_product_unit_and_squares() {
        // 1 · x = x.
        let x = (0b101, vec![0, 2]);
        assert_eq!(multiply(&(0, vec![]), &x), BTreeMap::from([(x.clone(), 1)]));
        // Q_i^2 = 0. (Q_0 = Sq^1 is the motivic Bockstein; P(ξ_1) = Sq^2 does NOT square to
        // zero — its square is τ Q_0 Q_1 + …, a genuine motivic feature.)
        for i in 0..3 {
            assert!(
                multiply(&(1 << i, vec![]), &(1 << i, vec![])).is_empty(),
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
        let basis: Vec<(u32, Vec<u32>)> = (0..=6).flat_map(enum_basis).collect();
        let mut saw_tau = false;
        for a in &basis {
            for b in &basis {
                let (wa, wb) = (paper_bidegree(a.0, &a.1).1, paper_bidegree(b.0, &b.1).1);
                for (z, c) in multiply(a, b) {
                    assert!(
                        c.is_power_of_two(),
                        "coeff {c:b} not a single τ power for {a:?}*{b:?}"
                    );
                    let k = c.trailing_zeros() as i32;
                    let wz = paper_bidegree(z.0, &z.1).1;
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
            DualElement::from([((0, vec![0, 0, 1]), 1), ((0, vec![0, 3]), 1)])
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
                    &antipode(&DualElement::from([(l, 1)])),
                    &DualElement::from([(r, 1)]),
                );
                for (mon, cc) in prod {
                    dual_add(&mut out, mon, tau_mul(c, cc));
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
                (((0, vec![]), (0b10, vec![])), 1),    // 1 ⊗ τ_1
                (((0b1, vec![]), (0, vec![0, 1])), 1), // τ_0 ⊗ ξ_1
                (((0b10, vec![]), (0, vec![])), 1),    // τ_1 ⊗ 1
            ])
        );
        // ψ(ξ_2) = 1⊗ξ_2 + ξ_1⊗ξ_1^2 + ξ_2⊗1  (matches Milnor).
        assert_eq!(
            coproduct(&xi_gen(2)),
            TensorElement::from([
                (((0, vec![]), (0, vec![0, 0, 1])), 1),  // 1 ⊗ ξ_2
                (((0, vec![0, 1]), (0, vec![0, 2])), 1), // ξ_1 ⊗ ξ_1^2
                (((0, vec![0, 0, 1]), (0, vec![])), 1),  // ξ_2 ⊗ 1
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
