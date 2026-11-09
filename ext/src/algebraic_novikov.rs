//! The $C\tau$ Adams $E_2$ page from classical data, via Miller's square.

use std::{collections::BTreeMap, sync::Arc};

use algebra::{
    Algebra, MilnorAlgebra,
    milnor_algebra::{MilnorBasisElement, PPart},
    module::FDModule,
};
use bivec::BiVec;
use rustc_hash::{FxHashMap, FxHashSet};
use sseq::coordinates::Bidegree;

use crate::chain_complex::{ChainComplex, FreeChainComplex};

/// Which generators of $P_*$ appear in the coaction on $Q$.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Convention {
    /// $\psi(v_n) = \sum_i \xi_{n-i}^{2^i} \otimes v_i$.
    Xi,
    /// $\psi(v_n) = \sum_i \zeta_{n-i}^{2^i} \otimes v_i$ with $\zeta_j = \chi \xi_j$.
    Conjugate,
}

/// A monomial in the $\xi_j$, as exponents of $\xi_1, \xi_2, \dots$, trimmed.
type XiMonomial = Vec<u32>;

/// A monomial in the $v_i$, as exponents of $v_0, v_1, \dots$, trimmed.
pub type VMonomial = Vec<u32>;

/// A polynomial over $\mathbb{F}_2$ in the $\xi_j$: the set of monomials with coefficient 1.
type XiPoly = FxHashSet<XiMonomial>;

/// Drops trailing zeros, so exponent vectors compare and hash canonically.
fn trim(mut v: Vec<u32>) -> Vec<u32> {
    while v.last() == Some(&0) {
        v.pop();
    }
    v
}

/// The exponent vector of the product of two monomials.
fn add_exponents(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut r = vec![0; a.len().max(b.len())];
    for (i, &x) in a.iter().enumerate() {
        r[i] += x;
    }
    for (i, &x) in b.iter().enumerate() {
        r[i] += x;
    }
    r
}

/// Adds `x` to a set of terms with coefficient 1, over $\mathbb{F}_2$.
fn toggle<T: Eq + std::hash::Hash>(set: &mut FxHashSet<T>, x: T) {
    if !set.remove(&x) {
        set.insert(x);
    }
}

/// The product of two polynomials.
fn poly_mul(a: &XiPoly, b: &XiPoly) -> XiPoly {
    let mut r = XiPoly::default();
    for x in a {
        for y in b {
            toggle(&mut r, add_exponents(x, y));
        }
    }
    r
}

/// $\xi_j$ as a polynomial; $\xi_0 = 1$.
fn xi(j: usize) -> XiPoly {
    let mut m = vec![0; j];
    if j > 0 {
        m[j - 1] = 1;
    }
    std::iter::once(m).collect()
}

/// The Frobenius $f \mapsto f^{2^i}$.
fn frobenius(f: &XiPoly, i: u32) -> XiPoly {
    f.iter()
        .map(|m| m.iter().map(|&e| e << i).collect())
        .collect()
}

/// $\chi \xi_j$ for `j` in `0..=max`, from $\sum_{i=0}^{j} \xi_{j-i}^{2^i} \chi\xi_i = 0$.
fn conjugates(max: usize) -> Vec<XiPoly> {
    let mut chi: Vec<XiPoly> = vec![xi(0)];
    for j in 1..=max {
        let mut c = XiPoly::default();
        for (i, chi_i) in chi.iter().enumerate() {
            for m in poly_mul(&frobenius(&xi(j - i), i as u32), chi_i) {
                toggle(&mut c, m);
            }
        }
        chi.push(c);
    }
    chi
}

/// $|v_i| = 2^i - 1$ in halved degrees.
pub fn v_degree(i: usize) -> i32 {
    (1 << i) - 1
}

/// The degree of a monomial in the $v_i$.
pub fn monomial_degree(m: &[u32]) -> i32 {
    m.iter()
        .enumerate()
        .map(|(i, &e)| e as i32 * v_degree(i))
        .sum()
}

/// All monomials of length `k` and degree at most `max_degree`, bucketed by degree.
pub fn monomials(k: u32, max_degree: i32) -> Vec<Vec<VMonomial>> {
    // The largest i with |v_i| <= max_degree.
    let top = (0..)
        .take_while(|&i| v_degree(i) <= max_degree)
        .last()
        .unwrap_or(0);
    let mut out = vec![Vec::new(); max_degree as usize + 1];
    let mut current = vec![0u32; top + 1];
    /// Chooses the exponent of `v_i`, then recurses on `v_{i-1}`; `v_0` takes what is left.
    fn rec(
        i: usize,
        remaining: u32,
        degree: i32,
        max_degree: i32,
        current: &mut Vec<u32>,
        out: &mut Vec<Vec<VMonomial>>,
    ) {
        if i == 0 {
            current[0] = remaining;
            out[degree as usize].push(trim(current.clone()));
            current[0] = 0;
            return;
        }
        let d = v_degree(i);
        let mut e = 0;
        while e <= remaining && degree + e as i32 * d <= max_degree {
            current[i] = e;
            rec(
                i - 1,
                remaining - e,
                degree + e as i32 * d,
                max_degree,
                current,
                out,
            );
            e += 1;
        }
        current[i] = 0;
    }
    rec(top, k, 0, max_degree, &mut current, &mut out);
    for bucket in &mut out {
        bucket.sort();
    }
    out
}

/// The coaction on $Q$, memoised per monomial.
pub struct Coaction {
    /// `factors[n]` is $\psi(v_n)$ as a list of $(\text{coefficient}, i)$ meaning
    /// $\text{coefficient} \otimes v_i$.
    factors: Vec<Vec<(XiPoly, usize)>>,
}

/// $\psi(m)$ as the set of $(R, m')$ with $\xi^R \otimes m'$ having coefficient 1.
pub type CoactionValue = FxHashSet<(XiMonomial, VMonomial)>;

impl Coaction {
    /// The coaction on $v_0, \dots, v_{\text{max\_n}}$ and their products.
    pub fn new(max_n: usize, convention: Convention) -> Self {
        let gens: Vec<XiPoly> = match convention {
            Convention::Xi => (0..=max_n).map(xi).collect(),
            Convention::Conjugate => conjugates(max_n),
        };
        let factors = (0..=max_n)
            .map(|n| {
                (0..=n)
                    .map(|i| (frobenius(&gens[n - i], i as u32), i))
                    .collect()
            })
            .collect();
        Self { factors }
    }

    /// $\psi(m)$ for a monomial $m$.
    pub fn apply(&self, m: &[u32]) -> CoactionValue {
        let mut acc: CoactionValue = std::iter::once((Vec::new(), Vec::new())).collect();
        for (n, &e) in m.iter().enumerate() {
            for _ in 0..e {
                let mut next = CoactionValue::default();
                for (r, mm) in &acc {
                    for (poly, i) in &self.factors[n] {
                        let mut mm2 = mm.clone();
                        if mm2.len() <= *i {
                            mm2.resize(i + 1, 0);
                        }
                        mm2[*i] += 1;
                        for s in poly {
                            toggle(&mut next, (trim(add_exponents(r, s)), mm2.clone()));
                        }
                    }
                }
                acc = next;
            }
        }
        acc
    }
}

/// The $A$-module $Q_k^\vee$ truncated to degrees `<= max_degree`, in halved degrees.
///
/// $\Sigma^{-k} Q_k$ is a $P_*$-comodule in even degrees, and $P_*$ is $A_*$ with every degree
/// doubled, so halving degrees makes it an $A_*$-comodule with $|v_i| = 2^i - 1$. Its dual is
/// infinite, but the part above `max_degree` is a submodule, so truncating leaves
/// $\mathrm{Ext}^{a, u}$ unchanged for $u \leq$ `max_degree`.
///
/// The basis in each degree is the dual basis to [`monomials`]; $P(R)$ sends $m'^*$ to the sum
/// of the $m^*$ for which $\xi^R \otimes m'$ appears in $\psi(m)$.
pub fn q_dual_module(
    algebra: Arc<MilnorAlgebra>,
    k: u32,
    max_degree: i32,
    convention: Convention,
) -> FDModule<MilnorAlgebra> {
    let basis = monomials(k, max_degree);
    let top = basis
        .iter()
        .rposition(|b| !b.is_empty())
        .map_or(0, |d| d as i32);
    algebra.compute_basis(top);

    let mut graded_dimension = BiVec::new(0);
    for d in 0..=top {
        graded_dimension.push(basis[d as usize].len());
    }
    let mut module = FDModule::new(
        Arc::clone(&algebra),
        format!("Q_{k}^dual"),
        graded_dimension,
    );
    for (d, bucket) in basis.iter().enumerate().take(top as usize + 1) {
        for (idx, m) in bucket.iter().enumerate() {
            module.set_basis_element_name(d as i32, idx, format!("{m:?}*"));
        }
    }

    let max_n = basis.iter().flatten().map(|m| m.len()).max().unwrap_or(1);
    let coaction = Coaction::new(max_n, convention);

    let index: FxHashMap<&VMonomial, usize> = basis
        .iter()
        .flat_map(|b| b.iter().enumerate().map(|(i, m)| (m, i)))
        .collect();

    // For each output m, read off ψ(m) and record its contribution to P(R)·m'*.
    for (out_deg, bucket) in basis.iter().enumerate().take(top as usize + 1) {
        let out_deg = out_deg as i32;
        for (out_idx, m) in bucket.iter().enumerate() {
            for (r, m_in) in coaction.apply(m) {
                let in_deg = monomial_degree(&m_in);
                let op_deg = out_deg - in_deg;
                if op_deg == 0 {
                    continue;
                }
                let elt = MilnorBasisElement {
                    q_part: 0,
                    p_part: PPart::from_slice(&r),
                    degree: op_deg,
                };
                let op_idx = algebra.basis_element_to_index(&elt);
                let in_idx = index[&m_in];
                let v = module.action_mut(op_deg, op_idx, in_deg, in_idx);
                v.add_basis_element(out_idx, 1);
            }
        }
    }
    module
}

/// The trigraded dimensions of $\mathrm{Ext}_{A/\tau}(\mathbb{F}_2)$, keyed by (stem, s, weight).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TrigradedDims(pub BTreeMap<(i32, i32, i32), usize>);

impl TrigradedDims {
    /// Sum over weights, keyed by (stem, s).
    pub fn bigraded(&self) -> BTreeMap<(i32, i32), usize> {
        let mut out = BTreeMap::new();
        for (&(n, s, _), &d) in &self.0 {
            *out.entry((n, s)).or_default() += d;
        }
        out
    }
}

/// $\mathrm{Ext}_{A/\tau}(\mathbb{F}_2)$ in stems `0..=max_n` and filtrations `0..=max_s`,
/// computed as $\bigoplus_k \mathrm{Ext}_P(Q_k)$ with one classical resolution per $k$.
///
/// The extension $E(Q_0, Q_1, \dots) \to A/\tau \to P$ gives a Cartan–Eilenberg spectral
/// sequence $\mathrm{Ext}_P(\mathbb{F}_2, Q) \Rightarrow \mathrm{Ext}_{A/\tau}(\mathbb{F}_2)$ with
/// $Q = \mathbb{F}_2[v_0, v_1, \dots]$, which collapses for weight reasons (Gheorghe–Wang–Xu).
/// A class of $\mathrm{Ext}_A^{a, u}(Q_k^\vee)$ (see [`q_dual_module`]) has stem $2u - a$,
/// filtration $a + k$ and motivic weight $u$.
///
/// `resolve` resolves a finite module through the given classical (stem, s) and returns the
/// number of generators at a classical $(s, t)$.
pub fn ctau_e2_with<F>(
    max_n: i32,
    max_s: i32,
    convention: Convention,
    mut resolve: F,
) -> TrigradedDims
where
    F: FnMut(Arc<FDModule<MilnorAlgebra>>, Bidegree) -> Box<dyn Fn(Bidegree) -> usize>,
{
    let algebra = Arc::new(MilnorAlgebra::new(fp::prime::TWO, false));
    // n = 2u - a <= max_n and a <= max_s give u <= (max_n + max_s) / 2.
    let max_u = (max_n + max_s) / 2;
    let mut dims = TrigradedDims::default();
    for k in 0..=max_s {
        let max_a = max_s - k;
        let module = Arc::new(q_dual_module(
            Arc::clone(&algebra),
            k as u32,
            max_u,
            convention,
        ));
        let gens = resolve(module, Bidegree::n_s(max_u, max_a));
        for a in 0..=max_a {
            for u in a..=max_u {
                let n = 2 * u - a;
                if n > max_n {
                    continue;
                }
                let d = gens(Bidegree::s_t(a, u));
                if d > 0 {
                    dims.0.insert((n, a + k, u), d);
                }
            }
        }
    }
    dims
}

/// [`ctau_e2_with`] using Nassau's algorithm for each $Q_k^\vee$.
pub fn ctau_e2(max_n: i32, max_s: i32, convention: Convention) -> TrigradedDims {
    ctau_e2_with(max_n, max_s, convention, |module, through| {
        let res = crate::nassau::Resolution::new(module);
        res.compute_through_stem(through);
        Box::new(move |b| res.number_of_gens_in_bidegree(b))
    })
}

/// $\mathrm{Ext}_{A/\tau}(\mathbb{F}_2)$ by a direct minimal resolution over $A/\tau$, keyed by
/// (stem, s). This is the reference [`ctau_e2`] is checked against.
pub fn ctau_e2_direct(max_n: i32, max_s: i32) -> BTreeMap<(i32, i32), usize> {
    use algebra::{SteenrodAlgebra, milnor_algebra::MilnorProfile, module::steenrod_module};

    use crate::{chain_complex::FiniteChainComplex, resolution::Resolution};

    let algebra = Arc::new(SteenrodAlgebra::MilnorAlgebra(
        MilnorAlgebra::motivic_mod_tau(MilnorProfile::default(), false),
    ));
    let mut graded = BiVec::new(0);
    graded.push(1);
    let module: FDModule<SteenrodAlgebra> = FDModule::new(algebra, "F2".into(), graded);
    let cc: crate::CCC = FiniteChainComplex::ccdz(Arc::new(steenrod_module::erase(module)));
    let res = Resolution::new(Arc::new(cc));
    res.compute_through_stem(Bidegree::n_s(max_n, max_s));
    let mut out = BTreeMap::new();
    for b in res.iter_stem() {
        let d = res.number_of_gens_in_bidegree(b);
        if d > 0 && b.n() <= max_n && b.s() <= max_s {
            out.insert((b.n(), b.s()), d);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use algebra::module::Module;

    use super::*;

    /// $\chi(\xi_2) = \xi_2 + \xi_1^3$.
    #[test]
    fn conjugate_xi_2() {
        let c = conjugates(2);
        let expected: XiPoly = [vec![0, 1], vec![3]].into_iter().collect();
        assert_eq!(c[2], expected);
    }

    /// Length 2, degree at most 3: $v_0^2$, $v_0 v_1$, $v_1^2$, $v_0 v_2$.
    #[test]
    fn monomial_counts() {
        let m = monomials(2, 3);
        assert_eq!(m.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1, 1, 1]);
    }

    /// P(R)(P(S) x) = (P(R)P(S)) x on every basis element.
    fn check_associative(module: &FDModule<MilnorAlgebra>) {
        let algebra = module.algebra();
        let p = algebra.prime();
        let max = module.max_degree().unwrap();
        for in_deg in 0..=max {
            for d1 in 1..=max - in_deg {
                for d2 in 1..=max - in_deg - d1 {
                    let out_deg = in_deg + d1 + d2;
                    for i1 in 0..algebra.dimension(d1) {
                        for i2 in 0..algebra.dimension(d2) {
                            let mut prod = fp::vector::FpVector::new(p, algebra.dimension(d1 + d2));
                            algebra.multiply_basis_elements(prod.as_slice_mut(), 1, d1, i1, d2, i2);
                            for x in 0..module.dimension(in_deg) {
                                let mut lhs =
                                    fp::vector::FpVector::new(p, module.dimension(out_deg));
                                let mut mid =
                                    fp::vector::FpVector::new(p, module.dimension(in_deg + d2));
                                module.act_on_basis(mid.as_slice_mut(), 1, d2, i2, in_deg, x);
                                module.act(
                                    lhs.as_slice_mut(),
                                    1,
                                    d1,
                                    i1,
                                    in_deg + d2,
                                    mid.as_slice(),
                                );
                                let mut rhs =
                                    fp::vector::FpVector::new(p, module.dimension(out_deg));
                                module.act_by_element_on_basis(
                                    rhs.as_slice_mut(),
                                    1,
                                    d1 + d2,
                                    prod.as_slice(),
                                    in_deg,
                                    x,
                                );
                                assert_eq!(
                                    lhs, rhs,
                                    "not a module: ops ({d1},{i1})({d2},{i2}) on ({in_deg},{x})"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Agrees with a minimal resolution over $A/\tau$ in every (stem, s).
    #[test]
    fn matches_direct_resolution() {
        let ours = ctau_e2(30, 15, Convention::Xi).bigraded();
        assert_eq!(ours, ctau_e2_direct(30, 15));
    }

    /// With conjugates in the coaction the dual is not even a module, so the convention matters.
    #[test]
    #[should_panic(expected = "not a module")]
    fn conjugate_convention_is_not_a_module() {
        let algebra = Arc::new(MilnorAlgebra::new(fp::prime::TWO, false));
        check_associative(&q_dual_module(algebra, 1, 12, Convention::Conjugate));
    }

    /// Classes whose motivic weight is known land in the right weight.
    #[test]
    fn weights_of_known_classes() {
        let dims = ctau_e2(20, 5, Convention::Xi);
        // h_0, h_1, h_2, h_3, c_0, Ph_1, d_0, e_0 and τg, each alone in its (stem, s).
        for (n, s, w) in [
            (0, 1, 0),
            (1, 1, 1),
            (3, 1, 2),
            (7, 1, 4),
            (8, 3, 5),
            (9, 5, 5),
            (14, 4, 8),
            (17, 4, 10),
            (20, 4, 11),
        ] {
            assert_eq!(dims.0.get(&(n, s, w)), Some(&1), "({n}, {s}, {w})");
        }
    }

    /// The $\xi$ convention gives a module for small `k`.
    #[test]
    fn q_dual_is_a_module() {
        let algebra = Arc::new(MilnorAlgebra::new(fp::prime::TWO, false));
        for k in 1..=3 {
            let m = q_dual_module(Arc::clone(&algebra), k, 12, Convention::Xi);
            check_associative(&m);
        }
    }
}
