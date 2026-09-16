//! $A_C/\tau$: the mod-$\tau$ reduction of the C-motivic Steenrod algebra,
//! presented to the resolution engine as an ordinary $\mathbb{F}_2$-algebra.
//!
//! Setting $\tau = 0$ in $A_C$ collapses the defining relation $\tau_i^2 =
//! \tau\xi_{i+1}$ to $\tau_i^2 = 0$, so
//! $$A_C/\tau \;\cong\; \mathbb{F}_2[\xi_1, \xi_2, \dots] \otimes E(\tau_0, \tau_1, \dots).$$
//! This is a **connected, finite-type $\mathbb{F}_2$-algebra** — once $\tau$ is
//! gone every generator has positive stem — and it is *strictly bigger* than the
//! classical dual Steenrod algebra $\mathbb{F}_2[\xi_i]$ (it carries the extra
//! exterior generators $\tau_i$). Its cohomology $\mathrm{Ext}_{A_C/\tau}$ is the
//! **algebraic Novikov $E_2$**, equivalently the motivic Adams $E_2$ of $C\tau$
//! (Gheorghe–Wang–Xu).
//!
//! Because $A_C$ is a free $\mathbb{F}_2[\tau]$-module on the Milnor basis
//! $\{Q(E)P(R)\}$, the reduction $A_C/\tau = A_C \otimes_{\mathbb{F}_2[\tau]}
//! \mathbb{F}_2$ has *the same basis*, and its product is the $\tau^0$ part of the
//! $A_C$ product: reduce each structure constant $\tau^k$ mod $\tau$, keeping only
//! $k = 0$. So this type is a thin $\mathbb{F}_2$ view over the
//! [`MotivicMilnorAlgebra`] engine, and the ordinary ($\mathbb{F}_p$-only)
//! resolution engine can resolve over it unchanged.
//!
//! Each basis element additionally carries a **motivic weight** (via
//! [`CTauAlgebra::weight`]); the algebra is graded by the single topological
//! degree `t`, with weight a bounded per-basis-element label that the Phase 2
//! lift consumes.

use fp::{
    prime::{TWO, ValidPrime},
    vector::{FpSliceMut, FpVector},
};

use super::{
    MotivicMilnorAlgebra,
    milnor::{Bigraded, Dual, Monomial},
};
use crate::algebra::{
    Algebra, GeneratedAlgebra,
    milnor_algebra::{MilnorBasisElement, PPartAllocation, milnor_product},
};

/// $A_C/\tau$, the mod-$\tau$ C-motivic Steenrod algebra as an $\mathbb{F}_2$-algebra.
///
/// Wraps the [`MotivicMilnorAlgebra`] $A_C$ engine and presents its $\tau^0$
/// product to the resolution engine. Resolving the trivial module over this
/// algebra yields the algebraic Novikov $E_2$.
#[derive(Default)]
pub struct CTauAlgebra {
    ac: MotivicMilnorAlgebra,
}

impl CTauAlgebra {
    pub fn new() -> Self {
        Self {
            ac: MotivicMilnorAlgebra::new(),
        }
    }

    /// The underlying $A_C$ product engine (over $\mathbb{F}_2[\tau]$), shared so
    /// that the Phase 2 lift can recover the $\tau$-powers this $\mathbb{F}_2$ view
    /// discards.
    pub fn engine(&self) -> &MotivicMilnorAlgebra {
        &self.ac
    }

    /// The motivic weight of the `idx`-th basis element in topological degree `t`.
    ///
    /// $A_C/\tau$ is graded by the single degree `t`; the weight is an extra label
    /// carried per basis element (inherited from the $A_C$ bigrading, where
    /// reducing mod $\tau$ does not change weights).
    pub fn weight(&self, t: i32, idx: usize) -> i32 {
        self.ac.bidegree(t, idx).1
    }
}

impl std::fmt::Display for CTauAlgebra {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CTauAlgebra(A_C/τ, p=2)")
    }
}

impl Algebra for CTauAlgebra {
    fn prefix(&self) -> &str {
        "motivic_ctau"
    }

    fn magic(&self) -> u32 {
        // Distinct from the classical Milnor/Adem magics so motivic save files can
        // never be cross-loaded.
        0x004D_0003
    }

    fn prime(&self) -> ValidPrime {
        TWO
    }

    fn compute_basis(&self, degree: i32) {
        self.ac.compute_basis(degree);
    }

    fn dimension(&self, degree: i32) -> usize {
        self.ac.dimension(degree)
    }

    fn multiply_basis_elements(
        &self,
        mut result: FpSliceMut,
        coeff: u32,
        r_degree: i32,
        r_idx: usize,
        s_degree: i32,
        s_idx: usize,
    ) {
        let t = r_degree + s_degree;
        let mbe = |d: i32, idx: usize| {
            let Dual(m) = *self.ac.basis_element(d, idx);
            MilnorBasisElement {
                q_part: m.q_part,
                p_part: m.p_part,
                degree: d,
            }
        };
        // $A_C/\tau \cong \mathbb{F}_2[\xi_i] \otimes E(\tau_i)$ is the odd-primary dual's
        // shape with $2^i$ for $p^i$, so the classical product *is* this product at `p = 2`: the
        // exterior commutation shifts by $2^k$ and the signs collapse over $\mathbb{F}_2$.
        // `milnor_product` takes its *left* factor first, commuting the right factor's exterior
        // part leftwards past it; the engine's `product_indexed` orders its arguments the other
        // way. Passing `s` then `r` keeps this product equal to the engine's τ^0 part, which the
        // cross-check below pins down.
        PPartAllocation::with_local(|allocation| {
            milnor_product(
                TWO,
                mbe(s_degree, s_idx),
                mbe(r_degree, r_idx),
                allocation,
                |c, res| {
                    let elt = Dual(Monomial::new(res.q_part, res.p_part));
                    // `enum_basis` holds every monomial of degree `t`, so a miss is a broken
                    // product invariant rather than a term to drop.
                    let idx = self
                        .ac
                        .index_of(t, &elt)
                        .expect("A_C/τ product left the degree-t basis");
                    result.add_basis_element(idx, coeff * c);
                },
            )
        });
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        self.ac.basis_element(degree, idx).to_string()
    }

    /// Parse `1`, `Q_i`, `P(R)`, or a product `Q_i … P(R)`, inverting the [`Dual<Monomial>`]
    /// display so `.json` module descriptors can be written over $A_C/\tau$.
    fn basis_element_from_string(&self, elt: &str) -> Option<(i32, usize)> {
        let elt = elt.trim();
        let (q_str, p_str) = match elt.find("P(") {
            Some(i) => (&elt[..i], Some(&elt[i..])),
            None => (elt, None),
        };
        let mut q_part = 0;
        for tok in q_str.split_whitespace() {
            if tok == "1" {
                continue;
            }
            // The exterior part is a 32-bit mask, so an out-of-range index is malformed
            // input, not an out-of-range shift.
            q_part |= 1u32.checked_shl(tok.strip_prefix("Q_")?.parse::<u32>().ok()?)?;
        }
        let r = match p_str {
            Some(s) => {
                let inner = s.strip_prefix("P(")?.strip_suffix(')')?.trim();
                if inner.is_empty() {
                    Vec::new()
                } else {
                    inner
                        .split(',')
                        .map(|x| x.trim().parse::<u32>().ok())
                        .collect::<Option<Vec<_>>>()?
                }
            }
            None => Vec::new(),
        };
        let elt = Dual(Monomial::from_paper(q_part, &r)?);
        let degree = elt.bidegree().0;
        self.ac.compute_basis(degree);
        Some((degree, self.ac.index_of(degree, &elt)?))
    }
}

/// Reduce `v` (with companion coefficient vector `c`) against the echelon `rows`,
/// clearing every leading entry that a row already pivots. On return `v` is the
/// residual and `c` records which candidate columns were added — see
/// [`CTauAlgebra::decompose_basis_element`].
fn reduce_row(v: &mut FpVector, c: &mut FpVector, rows: &[(usize, FpVector, FpVector)]) {
    while let Some((pivot, _)) = v.first_nonzero() {
        match rows.iter().find(|(p, _, _)| *p == pivot) {
            Some((_, row_v, row_c)) => {
                v.add(row_v, 1);
                c.add(row_c, 1);
            }
            None => break,
        }
    }
}

impl GeneratedAlgebra for CTauAlgebra {
    /// The algebra generators of $A_C/\tau$ in `degree`.
    ///
    /// The generators of a Hopf algebra are dual to the *primitives* of its dual.
    /// The dual $A_{**}/\tau = \mathbb{F}_2[\xi_1, \xi_2, \dots] \otimes E(\tau_0,
    /// \tau_1, \dots)$ has primitives exactly $\xi_1^{2^k}$ (from the polynomial
    /// part) and $\tau_0$ (the only primitive $\tau$). Dually, $A_C/\tau$ is
    /// generated by
    /// - $Q_0$ (dual $\tau_0$), the Bockstein, in degree $1$; and
    /// - $P(\xi_1^{2^k})$ (dual $\xi_1^{2^k}$), the motivic $\mathrm{Sq}^{2^{k+1}}$,
    ///   in degree $2^{k+1}$ (since $|\xi_1| = 2$) — i.e. degrees $2, 4, 8, 16,
    ///   \dots$.
    ///
    /// So each degree carries at most one generator. Every other basis element is
    /// decomposable (see [`Self::decompose_basis_element`]).
    fn generators(&self, degree: i32) -> Vec<usize> {
        if degree < 1 {
            return vec![];
        }
        self.ac.compute_basis(degree);
        if degree == 1 {
            // Q_0 = Q(E)P(R) with E = {0}, R empty.
            // Q_0 = Q(E)P(R) with E = {0}, R empty.
            let q0 = Dual(Monomial::from_paper(1, &[]).expect("Q_0 is a valid monomial"));
            return self.ac.index_of(1, &q0).into_iter().collect();
        }
        if (degree as u32).is_power_of_two() {
            // degree = 2^{k+1}: the generator P(ξ_1^{2^k}), R = [0, 2^k].
            let k = degree.trailing_zeros() - 1;
            let elt = Dual(
                Monomial::from_paper(0, &[0, 1u32 << k]).expect("P(ξ_1^{2^k}) is a valid monomial"),
            );
            return self.ac.index_of(degree, &elt).into_iter().collect();
        }
        vec![]
    }

    /// Decompose a non-generator basis element into products of generators.
    ///
    /// Every basis element that is not a generator is *decomposable* — a sum of
    /// products of positive-degree elements — and expanding each factor into
    /// generators shows it lies in the span of $\{g \cdot m\}$ where $g$ is a
    /// generator of strictly smaller degree and $m$ is a basis element filling the
    /// rest. So we enumerate those products (via the verified $A_C/\tau$ product),
    /// and solve the resulting $\mathbb{F}_2$-linear system for the target by
    /// augmented Gaussian elimination. This reuses the product engine as the source
    /// of truth rather than re-deriving the Milnor-matrix decomposition, and is
    /// validated by an exhaustive round-trip test (`decompose` then multiply gives
    /// back the element).
    fn decompose_basis_element(
        &self,
        degree: i32,
        idx: usize,
    ) -> Vec<(u32, (i32, usize), (i32, usize))> {
        self.ac.compute_basis(degree);
        let dim = self.dimension(degree);

        // Candidate factorizations `g · m` and their product vectors in `degree`.
        let mut cands: Vec<((i32, usize), (i32, usize))> = Vec::new();
        let mut prods: Vec<FpVector> = Vec::new();
        for g_deg in 1..degree {
            let gens = self.generators(g_deg);
            if gens.is_empty() {
                continue;
            }
            let m_deg = degree - g_deg;
            for &g_idx in &gens {
                for m_idx in 0..self.dimension(m_deg) {
                    let mut v = FpVector::new(TWO, dim);
                    self.multiply_basis_elements(v.as_slice_mut(), 1, g_deg, g_idx, m_deg, m_idx);
                    if v.is_zero() {
                        continue;
                    }
                    cands.push(((g_deg, g_idx), (m_deg, m_idx)));
                    prods.push(v);
                }
            }
        }

        // Augmented Gaussian elimination over F₂: reduce each product vector,
        // tracking (in a length-`n` companion vector) which candidates combine to
        // it, then reduce the target basis element and read off the combination.
        let n = prods.len();
        let mut rows: Vec<(usize, FpVector, FpVector)> = Vec::new();
        for (i, prod) in prods.iter().enumerate() {
            let mut v = prod.clone();
            let mut c = FpVector::new(TWO, n);
            c.add_basis_element(i, 1);
            reduce_row(&mut v, &mut c, &rows);
            if let Some((pivot, _)) = v.first_nonzero() {
                rows.push((pivot, v, c));
            }
        }

        let mut target = FpVector::new(TWO, dim);
        target.add_basis_element(idx, 1);
        let mut coeffs = FpVector::new(TWO, n);
        reduce_row(&mut target, &mut coeffs, &rows);
        assert!(
            target.is_zero(),
            "A_C/τ basis element ({degree}, {idx}) is not a generator but did not decompose"
        );

        coeffs
            .iter_nonzero()
            .map(|(i, _)| {
                let (g, m) = cands[i];
                (1, g, m)
            })
            .collect()
    }

    /// Relations among the generators, checked to validate module descriptors.
    ///
    /// For each pair of generators $g_1, g_2$ with $|g_1| + |g_2| = \text{degree}$,
    /// the product $g_1 g_2$ (computed by the engine) expands in the basis as
    /// $\sum_k b_k$, giving the relation $g_1 g_2 + \sum_k b_k = 0$ (over
    /// $\mathbb{F}_2$). These are the motivic analogue of the Adem relations
    /// (products of two generators re-expressed), and generate the relation ideal;
    /// `FiniteDimensionalModule::check_validity` applies them to reject descriptors
    /// whose generator actions are inconsistent (e.g. $Q_0^2 \ne 0$).
    fn generating_relations(&self, degree: i32) -> Vec<Vec<(u32, (i32, usize), (i32, usize))>> {
        self.ac.compute_basis(degree);
        let dim = self.dimension(degree);
        let mut out = Vec::new();
        for d1 in 1..degree {
            let g1s = self.generators(d1);
            if g1s.is_empty() {
                continue;
            }
            let d2 = degree - d1;
            let g2s = self.generators(d2);
            for &g1 in &g1s {
                for &g2 in &g2s {
                    let mut prod = FpVector::new(TWO, dim);
                    self.multiply_basis_elements(prod.as_slice_mut(), 1, d1, g1, d2, g2);
                    let mut relation = vec![(1, (d1, g1), (d2, g2))];
                    for (bk, _) in prod.iter_nonzero() {
                        relation.push((1, (degree, bk), (0, 0)));
                    }
                    out.push(relation);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use fp::vector::FpVector;

    use super::*;

    #[test]
    fn test_ctau_dimensions_match_ac_basis() {
        // A_C/τ has the same basis as A_C (same Milnor monomials per degree), so its
        // F_2-dimensions are the A_C F_2[τ]-ranks. Spot-check low degrees:
        //   t=0: 1 (unit); t=1: Q_0; t=2: P(ξ_1); t=3: Q_1, Q_0 P(ξ_1) → dim 2.
        let alg = CTauAlgebra::new();
        alg.compute_basis(6);
        assert_eq!(alg.dimension(0), 1);
        assert_eq!(alg.dimension(1), 1);
        assert_eq!(alg.dimension(2), 1);
        assert_eq!(alg.dimension(3), 2);
    }

    #[test]
    fn test_ctau_weight_labels() {
        // The weight label is the A_C bigrading's weight coordinate, carried through the
        // mod-τ reduction unchanged. In this presentation the algebra weight is the
        // *negative* of the dual monomial's weight (so products stay homogeneous with τ,
        // of weight −1): Q_0 = Sq^1 has weight 0; the ξ_1-dual (t = 2) has weight −1.
        // (The standard-sign motivic weight is recovered by a single global negation when
        // the Phase 1 chart is compared to published data.)
        let alg = CTauAlgebra::new();
        alg.compute_basis(2);
        assert_eq!(alg.weight(1, 0), alg.engine().bidegree(1, 0).1);
        assert_eq!(alg.weight(1, 0), 0);
        assert_eq!(alg.weight(2, 0), -1);
    }

    #[test]
    fn test_ctau_basis_element_from_string_round_trips() {
        // `basis_element_from_string` inverts `basis_element_to_string` on every basis
        // element in range — so `.json` module descriptors can name motivic operations.
        let alg = CTauAlgebra::new();
        alg.compute_basis(12);
        for degree in 0..=12 {
            for idx in 0..alg.dimension(degree) {
                let s = alg.basis_element_to_string(degree, idx);
                assert_eq!(
                    alg.basis_element_from_string(&s),
                    Some((degree, idx)),
                    "round-trip failed for {s:?} at ({degree}, {idx})"
                );
            }
        }
        // Spot-check known names, including the P(…) block with spaces.
        assert_eq!(alg.basis_element_from_string("1"), Some((0, 0)));
        assert_eq!(alg.basis_element_from_string("Q_0"), Some((1, 0)));
        assert_eq!(alg.basis_element_from_string("P(0, 1)"), Some((2, 0)));
        // Malformed input is rejected, not panicked on.
        assert_eq!(alg.basis_element_from_string("Sq1"), None);
        assert_eq!(alg.basis_element_from_string("Q_"), None);
        // The exterior mask is 32 bits wide, so `Q_32` is out of range: rejected rather
        // than shifted out of bounds.
        assert_eq!(alg.basis_element_from_string("Q_32"), None);
        assert_eq!(alg.basis_element_from_string("Q_99"), None);
    }

    #[test]
    fn test_ctau_product_is_tau0_part_of_ac() {
        // The defining contract, and a cross-check of two independent products: this algebra
        // multiplies through the classical `milnor_product`, while the engine computes the same
        // structure constants from the Kong–Lin closed form. The A_C/τ product must be exactly
        // the τ^0 part of the engine's A_C product. Also confirm some product genuinely drops a
        // τ-divisible term, so the reduction is doing real work.
        use std::collections::BTreeSet;

        let alg = CTauAlgebra::new();
        alg.compute_basis(10);
        let engine = alg.engine();
        let mut saw_drop = false;
        for t1 in 0..=5 {
            for idx1 in 0..alg.dimension(t1) {
                for t2 in 0..=(5 - t1) {
                    for idx2 in 0..alg.dimension(t2) {
                        let t = t1 + t2;
                        let raw = engine.product_indexed(t1, idx1, t2, idx2);
                        let tau_of = |i: usize| engine.tau_exponent(t1, idx1, t2, idx2, i);
                        let expected: BTreeSet<usize> =
                            raw.iter().copied().filter(|&i| tau_of(i) == 0).collect();
                        saw_drop |= raw.iter().any(|&i| tau_of(i) != 0);

                        let mut result = FpVector::new(TWO, alg.dimension(t));
                        alg.multiply_basis_elements(result.as_slice_mut(), 1, t1, idx1, t2, idx2);
                        let got: BTreeSet<usize> = result.iter_nonzero().map(|(i, _)| i).collect();
                        assert_eq!(
                            got, expected,
                            "mod-τ product ≠ τ^0 part at ({t1},{idx1})·({t2},{idx2})"
                        );
                    }
                }
            }
        }
        assert!(saw_drop, "no product in range dropped a τ-divisible term");
    }

    #[test]
    fn test_ctau_generators_are_q0_and_p_of_xi1_powers() {
        // The generators are Q_0 (degree 1) and P(ξ_1^{2^k}) (degrees 2, 4, 8, 16),
        // dual to the primitives ξ_1^{2^k} and τ_0 of A_**/τ. Every other degree has
        // none — in particular Q_1 (degree 3) and ξ_2-dual P(0, 1) (degree 6) are
        // decomposable, not generators.
        let alg = CTauAlgebra::new();
        alg.compute_basis(16);

        let name_of = |deg: i32, gens: &[usize]| -> Vec<String> {
            gens.iter()
                .map(|&i| alg.basis_element_to_string(deg, i))
                .collect()
        };
        assert_eq!(name_of(1, &alg.generators(1)), vec!["Q_0"]);
        assert_eq!(name_of(2, &alg.generators(2)), vec!["P(0, 1)"]);
        assert_eq!(name_of(4, &alg.generators(4)), vec!["P(0, 2)"]);
        assert_eq!(name_of(8, &alg.generators(8)), vec!["P(0, 4)"]);
        assert_eq!(name_of(16, &alg.generators(16)), vec!["P(0, 8)"]);

        for degree in [3, 5, 6, 7, 9, 10, 12] {
            assert!(
                alg.generators(degree).is_empty(),
                "degree {degree} should have no generators"
            );
        }
    }

    #[test]
    fn test_ctau_decompose_round_trips() {
        // The contract for `decompose_basis_element`: every non-generator basis
        // element, multiplied back out from its decomposition, returns itself — and
        // every factor has strictly smaller degree. This validates the generic
        // Gaussian-solve decomposition against the independently-verified product
        // engine, over the whole basis in a range.
        let alg = CTauAlgebra::new();
        alg.compute_basis(16);
        for degree in 1..=16 {
            let gens = alg.generators(degree);
            for idx in 0..alg.dimension(degree) {
                if gens.contains(&idx) {
                    continue;
                }
                let decomp = alg.decompose_basis_element(degree, idx);
                assert!(
                    !decomp.is_empty(),
                    "no decomposition for ({degree}, {idx}) = {}",
                    alg.basis_element_to_string(degree, idx)
                );
                let mut sum = FpVector::new(TWO, alg.dimension(degree));
                for (coef, (d1, i1), (d2, i2)) in decomp {
                    assert!(
                        d1 > 0 && d1 < degree && d2 > 0 && d2 < degree,
                        "factor of ({degree}, {idx}) is not strictly smaller: \
                         ({d1},{i1})·({d2},{i2})"
                    );
                    alg.multiply_basis_elements(sum.as_slice_mut(), coef, d1, i1, d2, i2);
                }
                let mut want = FpVector::new(TWO, alg.dimension(degree));
                want.add_basis_element(idx, 1);
                assert_eq!(
                    sum,
                    want,
                    "decomposition of ({degree}, {idx}) = {} does not reassemble",
                    alg.basis_element_to_string(degree, idx)
                );
            }
        }
    }

    #[test]
    fn test_ctau_generating_relations_encode_products() {
        // Each relation is `g1·g2 + Σ b_k = 0`, a true algebra identity: multiplying
        // it back out is zero. The degree-2 relation is just Q_0² = 0 (empty tail).
        let alg = CTauAlgebra::new();
        alg.compute_basis(8);

        let q0 = alg.basis_element_from_string("Q_0").unwrap();
        let rels2 = alg.generating_relations(2);
        assert!(
            rels2.iter().any(|r| r.len() == 1 && r[0] == (1, q0, q0)),
            "expected the Q_0^2 = 0 relation, got {rels2:?}"
        );

        for degree in 2..=8 {
            for relation in alg.generating_relations(degree) {
                let mut sum = FpVector::new(TWO, alg.dimension(degree));
                for (coef, (d1, i1), (d2, i2)) in relation {
                    alg.multiply_basis_elements(sum.as_slice_mut(), coef, d1, i1, d2, i2);
                }
                assert!(
                    sum.is_zero(),
                    "generating relation in degree {degree} is not zero in the algebra"
                );
            }
        }
    }

    #[test]
    fn test_ctau_unit_acts_trivially() {
        // 1 · x = x for the unit in degree 0.
        let alg = CTauAlgebra::new();
        alg.compute_basis(3);
        for t in 0..=3 {
            for idx in 0..alg.dimension(t) {
                let mut result = FpVector::new(TWO, alg.dimension(t));
                alg.multiply_basis_elements(result.as_slice_mut(), 1, 0, 0, t, idx);
                let mut expected = FpVector::new(TWO, alg.dimension(t));
                expected.add_basis_element(idx, 1);
                assert_eq!(
                    result, expected,
                    "unit did not act as identity on ({t}, {idx})"
                );
            }
        }
    }
}
