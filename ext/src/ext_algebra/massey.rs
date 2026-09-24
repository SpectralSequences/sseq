//! Primary Massey products in $\Ext$.
//!
//! [`ExtAlgebra::massey`] computes a single triple Massey product $\langle a, b, c\rangle$, while
//! [`ExtAlgebra::massey_iter_c`] and [`ExtAlgebra::massey_iter_a`] sweep a whole family at once:
//! the former fixes $a, b$ and ranges over every valid third factor $\langle a, b, -\rangle$, the
//! latter fixes $b, c$ and ranges over every valid first factor $\langle -, b, c\rangle$. The two
//! directions differ in whether the `b ∘ c` null-homotopy is rebuilt per `c` or reused for fixed
//! `b, c`.
//!
//! All three wrap [`ChainHomotopy`]: we lift the multiplication maps, build the null-homotopy of
//! the composite `b ∘ c`, and read off the bracket by pairing against the first factor. The valid
//! choices of `a` and `c` are the kernel of multiplication by `b`.
//!
//! The result is an [`AffineSubspace`]: a coset representative (the offset) together with the
//! indeterminacy $a \cdot \Ext + \Ext \cdot c$ (the linear part). Both terms of the indeterminacy
//! are the $\Ext(k, k)$-module action on $\Ext(M, k)$, so it is computed for any `M`. This matches
//! (and reuses the logic of) the `massey` example, which computes the products up to a sign.

use std::sync::Arc;

use fp::{
    matrix::{AffineSubspace, AugmentedMatrix, Matrix, Subspace},
    prime::Prime,
    vector::{FpSlice, FpVector},
};
use sseq::coordinates::{Bidegree, BidegreeElement, BidegreeGenerator};

use super::{Cochain, CochainCup, ExtAlgebra};
use crate::{
    chain_complex::{AugmentedChainComplex, ChainHomotopy, FreeChainComplex},
    resolution_homomorphism::ResolutionHomomorphism,
};

/// The result of a Massey product computation
pub struct MasseyResult {
    /// The bidegree of the bracket, `a.degree() + b.degree() + c.degree() - Bidegree::s_t(1, 0)`.
    pub degree: Bidegree,
    /// The value of the bracket as a coset.
    pub coset: AffineSubspace,
}

impl MasseyResult {
    /// Returns a representative element of the Massey product.
    pub fn representative(&self) -> BidegreeElement {
        BidegreeElement::new(self.degree, self.coset.offset().clone())
    }

    /// Whether the Massey product contains zero.
    pub fn contains_zero(&self) -> bool {
        self.coset.contains_zero()
    }
}

impl<CC> ExtAlgebra<CC>
where
    CC: FreeChainComplex + AugmentedChainComplex + Sync,
{
    /// The bidegree shift of $\langle a, b, -\rangle$: a class `c` produces a bracket in bidegree
    /// `c.degree() + a.degree() + b.degree() - (1, 0)`.
    fn massey_shift(a: &BidegreeElement, b: &BidegreeElement) -> Bidegree {
        a.degree() + b.degree() - Bidegree::s_t(1, 0)
    }

    /// The multiplication-by-`b` chain self-map of the unit, extended far enough for brackets
    /// landing at `shift`.
    fn massey_b_hom(
        &self,
        b: &BidegreeElement,
        shift: Bidegree,
    ) -> Arc<ResolutionHomomorphism<CC, CC>> {
        let b_coords: Vec<u32> = b.vec().iter().collect();
        let hom = Arc::new(ResolutionHomomorphism::from_class(
            String::new(),
            Arc::clone(self.unit()),
            Arc::clone(self.unit()),
            b.degree(),
            &b_coords,
        ));
        hom.extend_through_stem(shift);
        hom
    }

    /// The kernel of multiplication by `b` at bidegree `c_deg`: the valid third factors of
    /// $\langle a, b, -\rangle$, since the bracket is defined only when `b · c = 0`.
    ///
    /// Computed from the product maps alone (no null-homotopy), as `c · b` (equal to `b · c` up to
    /// sign, so the same kernel). Returns `None` when the product bidegree `c_deg + b.degree()` is
    /// uncomputed, so callers never mistake an uncomputed product for a zero one; a computed but
    /// empty product bidegree correctly yields the full space.
    pub(crate) fn massey_kernel(&self, b: &BidegreeElement, c_deg: Bidegree) -> Option<Subspace> {
        let p = self.prime();
        let resolution = self.resolution();

        let prod_deg = c_deg + b.degree();
        if !resolution.has_computed_bidegree(prod_deg) {
            return None;
        }
        // Work in the Ext (cohomology) basis: on the untwisted tensor resolution the complex is
        // non-minimal, so its generator count is *not* the Ext dimension.
        let num_gens = self.dimension(c_deg);
        let product_num_gens = self.dimension(prod_deg);

        let mut product = AugmentedMatrix::<2>::new(p, num_gens, [product_num_gens, num_gens]);
        product.segment(1, 1).add_identity();
        for i in 0..num_gens {
            let c_gen = self.generator(BidegreeGenerator::new(c_deg, i));
            let prod = self.try_multiply(&c_gen, b)?;
            product
                .row_mut(i)
                .slice_mut(0, product_num_gens)
                .add(prod.vec(), 1);
        }
        product.row_reduce();
        Some(product.compute_kernel())
    }

    /// The bracket $\langle a, b, c\rangle$ for a single third factor `c`, which the caller must
    /// have checked lies in the kernel of multiplication by `b` (so that `b · c = 0` and the
    /// null-homotopy exists); otherwise the lift in [`ChainHomotopy::extend`] cannot complete.
    ///
    /// Unlike the removed per-generator scheme, this realises the *actual* class `c` (a linear
    /// combination) via [`ResolutionHomomorphism::from_class`] and builds a single valid
    /// null-homotopy, matching the approach of [`massey_iter_a`](Self::massey_iter_a). Returns
    /// `None` only when the bracket bidegree `c.degree() + shift` is uncomputed; a computed but
    /// empty bidegree yields the (defined) zero bracket, not `None`.
    fn massey_bracket_of(
        &self,
        a: &BidegreeElement,
        b: &BidegreeElement,
        b_hom: Arc<ResolutionHomomorphism<CC, CC>>,
        shift: Bidegree,
        c: &BidegreeElement,
    ) -> Option<MasseyResult> {
        // On a non-minimal resolution (the untwisted tensor resolution attaches a cup product) the
        // chain-map bracket below degenerates; dispatch to the cochain-DGA bracket instead. The DGA
        // path reads its cup matrices off the shared per-generator self-map cache, so it needs `b`
        // itself, not `b_hom`.
        if let Some(cup) = self.cup() {
            return self.massey_bracket_dga(&Arc::clone(cup), a, b, shift, c);
        }

        let p = self.prime();
        let resolution = self.resolution();
        let unit = self.unit();

        let c_deg = c.degree();
        let tot = c_deg + shift;
        if !resolution.has_computed_bidegree(tot) {
            return None;
        }
        // The bracket is read in the cochain (resolution-generator) basis, then projected to Ext.
        let target_num_gens = resolution.number_of_gens_in_bidegree(tot);

        // When `tot` is computed but empty the bracket lands in the zero group, so it is the
        // (defined) zero element: skip the null-homotopy and use a zero representative. Otherwise
        // read the bracket by pairing the top homotopy against `a`, as the old per-generator scheme
        // did, but for the single realised class `c`.
        let representative = if target_num_gens == 0 {
            FpVector::new(p, 0)
        } else {
            // Where `a`'s generators sit in the homotopy output, so we can pair against them.
            let offset_a =
                unit.module(a.degree().s())
                    .generator_offset(a.degree().t(), a.degree().t(), 0);
            let a_coords: Vec<u32> = a.vec().iter().collect();
            // Realise `c` as a cocycle in the resolution's cochain basis (identity on a minimal
            // resolution); `f_c` needs its cochain coordinates, not the Ext ones.
            let c_coords: Vec<u32> = self.lift(c).vec().iter().collect();

            let f_c = Arc::new(ResolutionHomomorphism::from_class(
                String::new(),
                Arc::clone(resolution),
                Arc::clone(unit),
                c_deg,
                &c_coords,
            ));
            f_c.extend_through_stem(tot);

            let homotopy = ChainHomotopy::new(f_c, b_hom);
            homotopy.extend(tot);

            let last = homotopy.homotopy(tot.s());
            let mut representative = FpVector::new(p, target_num_gens);
            for i in 0..target_num_gens {
                let output = last.output(tot.t(), i);
                for (k, &val) in a_coords.iter().enumerate() {
                    if val != 0 {
                        representative.add_basis_element(i, val * output.entry(offset_a + k));
                    }
                }
            }
            representative
        };

        // Project the cochain representative to its Ext class, so it lives in the same basis as the
        // indeterminacy (identity on a minimal resolution).
        let representative = self.project(&Cochain::new(tot, representative)).into_vec();
        let indeterminacy = self.massey_indeterminacy(a, c, tot);
        Some(MasseyResult {
            degree: tot,
            coset: AffineSubspace::new(representative, indeterminacy),
        })
    }

    /// The bracket $\langle a, b, c\rangle$ via the **cochain DGA**, for a non-minimal resolution
    /// carrying a [`CochainCup`]. Because the unit is minimal ($a\cdot b = 0$ at the cochain level,
    /// so the $a\cup b$ null-homotopy $u = 0$), the bracket is $[\,a \cup v\,]$ with $\delta_Q v =
    /// b \cup c$. Both cups ($b\cup c$ and $a\cup v$) are read via
    /// [`cup_class_matrix`](Self::cup_class_matrix), so they draw on the *same* per-generator unit
    /// self-map cache as the product path and the family sweep — no bracket-specific self-map is
    /// ever built. $\delta_Q$ is the attached [`differential`](Self::differential). Returns `None`
    /// if `b·c ≠ 0` or the range is uncomputed.
    fn massey_bracket_dga(
        &self,
        cup: &Arc<dyn CochainCup<CC>>,
        a: &BidegreeElement,
        b: &BidegreeElement,
        shift: Bidegree,
        c: &BidegreeElement,
    ) -> Option<MasseyResult> {
        let p = self.prime();
        let res = self.resolution();

        let c_deg = c.degree();
        let tot = c_deg + shift;
        let bc_deg = b.degree() + c_deg;
        let v_deg = bc_deg + Bidegree::n_s(1, -1);
        // Every cochain read (b∪c at `bc_deg`, its δ-preimage `v`, the bracket `tot`) must be in
        // the computed box. `has_computed_bidegree` is panic-safe unlike
        // `cohomology`/`cochain_dimension`.
        for d in [bc_deg, v_deg, tot] {
            if d.s() < 0 || d.t() < 0 || !res.has_computed_bidegree(d) {
                return None;
            }
        }
        self.cohomology(tot)?;

        // b∪c as a cochain: cup by b applied to the cocycle rep of c, through the shared cup cache.
        let bc_mat = self.cup_class_matrix(cup, b, c_deg);
        let lc = self.lift(c);
        let mut bc = FpVector::new(p, bc_mat.columns());
        bc_mat.apply(bc.as_slice_mut(), 1, lc.vec());

        // v with δ_Q v = b∪c; `None` iff b·c ≠ 0 (b∪c is not a coboundary).
        let v = self.delta_preimage(v_deg, bc.as_slice())?;

        // a∪v, then project to an Ext class — again through the shared cup cache.
        let av_mat = self.cup_class_matrix(cup, a, v_deg);
        let mut av = FpVector::new(p, av_mat.columns());
        av_mat.apply(av.as_slice_mut(), 1, v.as_slice());

        let representative = self.project(&Cochain::new(tot, av)).into_vec();
        let indeterminacy = self.massey_indeterminacy(a, c, tot);
        Some(MasseyResult {
            degree: tot,
            coset: AffineSubspace::new(representative, indeterminacy),
        })
    }

    /// A cochain `v` at `v_deg` with `δ_Q v = z` (the attached differential's matrix out of `v_deg`
    /// lands at `z`'s bidegree), or `None` if `z ∉ im δ_Q` (so the bracket is undefined). Solved by
    /// a quasi-inverse of `[δ_Q | I]`, with a `δ_Q v = z` post-check (the quasi-inverse silently
    /// drops any component of `z` outside the image).
    fn delta_preimage(&self, v_deg: Bidegree, z: FpSlice) -> Option<FpVector> {
        let p = self.prime();
        let d = self.differential()?.matrix(v_deg)?;
        let (r, c) = (d.rows(), d.columns());
        // A shape mismatch is a caller bug, never a mathematical answer — returning `None` here
        // would be indistinguishable from "`b∪c` is not a coboundary" and would silently drop the
        // bracket from the family.
        assert_eq!(
            z.len(),
            c,
            "delta_preimage at {v_deg:?}: cocycle has length {} but δ has {c} columns",
            z.len()
        );
        // The quasi-inverse of `δ_Q` out of `v_deg` is intrinsic — shared across every bracket that
        // lands here — so it is row-reduced once and cached.
        let qi = self.delta_quasi_inverse(v_deg, &d, r, c);

        let mut v = FpVector::new(p, r);
        qi.apply(v.as_slice_mut(), 1, z);

        let mut back = FpVector::new(p, c);
        d.apply(back.as_slice_mut(), 1, v.as_slice());
        back.as_slice_mut().add(z, p.as_u32() - 1);
        if !back.is_zero() {
            return None;
        }
        Some(v)
    }

    /// Compute a representative of a Massey product evaluated at `row` from the per-generator
    /// bracket matrix `answers`. Used by [`massey_iter_a`](Self::massey_iter_a), which builds one
    /// null-homotopy for fixed `b, c` and reads a whole family of first factors off `answers`.
    fn massey_representative(&self, answers: &Matrix, row: FpSlice) -> FpVector {
        let mut v = FpVector::new(self.prime(), answers.columns());
        answers.apply(v.as_slice_mut(), 1, row);
        v
    }

    /// Assemble a [`MasseyResult`] at the bracket bidegree `tot` from the per-generator bracket
    /// values `answers` and the third factor `c` (with coordinates `row`).
    fn massey_result(
        &self,
        a: &BidegreeElement,
        c: &BidegreeElement,
        answers: &Matrix,
        row: FpSlice,
        tot: Bidegree,
    ) -> MasseyResult {
        // `answers` is in the cochain basis; project the bracket representative to its Ext class so
        // it matches the (cohomology-basis) indeterminacy. Identity on a minimal resolution.
        let representative = self.massey_representative(answers, row);
        let representative = self.project(&Cochain::new(tot, representative)).into_vec();
        let indeterminacy = self.massey_indeterminacy(a, c, tot);
        MasseyResult {
            degree: tot,
            coset: AffineSubspace::new(representative, indeterminacy),
        }
    }

    /// The indeterminacy $a \cdot \Ext^{|b| + |c| - (1,0)} + \Ext^{|a| + |b| - (1,0)} \cdot c$ at
    /// the bracket bidegree `tot`, as a subspace of $\Ext(M, k)$ at `tot`.
    pub(crate) fn massey_indeterminacy(
        &self,
        a: &BidegreeElement,
        c: &BidegreeElement,
        tot: Bidegree,
    ) -> Subspace {
        let mut sub = self.massey_indeterminacy_a(a, tot);
        self.massey_indeterminacy_c(c, tot, &mut sub);
        sub
    }

    /// The $a \cdot \Ext^{|b| + |c| - (1,0)}$ half of the Massey indeterminacy at `tot`. This
    /// depends only on `a` and `tot`, so a family sweep over the third factor computes it once per
    /// `c_deg`.
    fn massey_indeterminacy_a(&self, a: &BidegreeElement, tot: Bidegree) -> Subspace {
        let mut sub = Subspace::new(self.prime(), self.dimension(tot));
        // a · Ext(M, k)^{tot - a.degree()}, computed as y · a (equal up to sign).
        for y in self.basis(tot - a.degree()) {
            if let Some(prod) = self.try_multiply(&self.generator(y), a) {
                sub.add_vector(prod.vec());
            }
        }
        sub
    }

    /// Add the $\Ext^{|a| + |b| - (1,0)} \cdot c$ half of the Massey indeterminacy at `tot` into an
    /// existing subspace (typically one already carrying the `a`-half). This half depends on the
    /// specific third factor `c`, so it is the per-row part of a family sweep.
    fn massey_indeterminacy_c(&self, c: &BidegreeElement, tot: Bidegree, sub: &mut Subspace) {
        // Ext(k, k)^{tot - c.degree()} · c, computed as c · x (equal up to sign).
        for x in self.unit_basis(tot - c.degree()) {
            if let Some(prod) = self.try_multiply(c, &self.unit_generator(x)) {
                sub.add_vector(prod.vec());
            }
        }
    }

    /// Compute the family of Massey products $\langle a, b, -\rangle$ for fixed `a` and `b` and
    /// every valid third factor `c` across all computed bidegrees.
    ///
    /// `a` and `b` are taken in $\Ext(k, k)$; the third factor ranges over $\Ext(M, k)$. The caller
    /// must have resolved `M` and the unit far enough. This assumes `a · b = 0` so that the bracket
    /// is defined; it is not verified.
    ///
    /// Brackets that contain `0` are omitted.
    ///
    /// This iterates over the third factor, building a fresh null-homotopy of `b ∘ c` per `c`. To
    /// vary the *first* factor with `b, c` fixed instead, use
    /// [`massey_iter_a`](Self::massey_iter_a).
    pub fn massey_iter_c(
        &self,
        a: &BidegreeElement,
        b: &BidegreeElement,
    ) -> Vec<(BidegreeElement, MasseyResult)> {
        let shift = Self::massey_shift(a, b);

        // On the untwisted tensor resolution the bracket is the cochain-DGA formula, whose
        // per-`c_deg` cup matrices and δ_Q quasi-inverse are independent of the specific kernel
        // row; hoist them out of the row loop rather than rebuilding them per bracket via
        // `massey_bracket_of`. The cup path reads its cup matrices off `cup_class_matrix`, so the
        // chain-map `b_hom` is never needed.
        if let Some(cup) = self.cup() {
            return self.massey_iter_c_cup(&Arc::clone(cup), a, b, shift);
        }

        let b_hom = self.massey_b_hom(b, shift);
        let mut results = Vec::new();
        // The third factor ranges over bidegrees where $\Ext(M, k)$ is nonzero. On a minimal
        // resolution those are the resolution's generators (`iter_nonzero_stem`); on the untwisted
        // tensor resolution the resolution is the small $P_\bullet$, so the Ext bidegrees are read
        // off `dimension` (the cohomology of $\delta_Q$) instead — hence sweep `iter_stem` and
        // filter.
        for c_deg in self.resolution().iter_stem() {
            if self.dimension(c_deg) == 0 {
                continue;
            }
            let Some(kernel) = self.massey_kernel(b, c_deg) else {
                continue;
            };
            for row in kernel.iter() {
                let c = BidegreeElement::new(c_deg, row.to_owned());
                let Some(result) = self.massey_bracket_of(a, b, Arc::clone(&b_hom), shift, &c)
                else {
                    continue;
                };
                if result.contains_zero() {
                    continue;
                }
                results.push((c, result));
            }
        }
        results
    }

    /// The cochain-DGA specialisation of [`massey_iter_c`](Self::massey_iter_c): computes the
    /// family $\langle a, b, -\rangle$ for an untwisted tensor resolution carrying a
    /// [`CochainCup`]. For each third-factor bidegree `c_deg` it builds the cup-by-`b` matrix, the
    /// cup-by-`a` matrix, the δ_Q quasi-inverse at `v_deg`, and the `a·Ext` half of the
    /// indeterminacy **once**, then applies them to every kernel row — the only per-row work is
    /// `lift(c)`, two matrix applications, the δ-preimage solve (against the shared quasi-inverse),
    /// the projection, and the `Ext·c` half of the indeterminacy. This is the hoisted equivalent of
    /// looping [`massey_bracket_dga`](Self::massey_bracket_dga) per row.
    fn massey_iter_c_cup(
        &self,
        cup: &Arc<dyn CochainCup<CC>>,
        a: &BidegreeElement,
        b: &BidegreeElement,
        shift: Bidegree,
    ) -> Vec<(BidegreeElement, MasseyResult)> {
        let p = self.prime();
        let res = self.resolution();
        let b_deg = b.degree();

        let mut results = Vec::new();
        for c_deg in res.iter_stem() {
            if self.dimension(c_deg) == 0 {
                continue;
            }
            let Some(kernel) = self.massey_kernel(b, c_deg) else {
                continue;
            };
            if kernel.dimension() == 0 {
                continue;
            }

            let tot = c_deg + shift;
            let bc_deg = b_deg + c_deg;
            let v_deg = bc_deg + Bidegree::n_s(1, -1);
            // Every cochain read must lie in the computed box (mirrors `massey_bracket_dga`).
            if [bc_deg, v_deg, tot]
                .iter()
                .any(|d| d.s() < 0 || d.t() < 0 || !res.has_computed_bidegree(*d))
            {
                continue;
            }
            if self.cohomology(tot).is_none() {
                continue;
            }

            // Per-`c_deg` matrices, independent of the specific kernel row: cup-by-`b` (source
            // `c_deg`), cup-by-`a` (source `v_deg`), and the `a·Ext` half of the indeterminacy.
            // Both cup matrices go through `cup_class_matrix`, so their (expensive) per-generator
            // builds are memoised in `cup_cache` and shared with the product path; the δ_Q
            // quasi-inverse at `v_deg` is likewise memoised inside `delta_preimage`.
            let bc_mat = self.cup_class_matrix(cup, b, c_deg);
            let av_mat = self.cup_class_matrix(cup, a, v_deg);
            let a_indet = self.massey_indeterminacy_a(a, tot);

            for row in kernel.iter() {
                let c = BidegreeElement::new(c_deg, row.to_owned());

                // b∪c as a cochain, then its δ_Q-preimage `v` (`None` iff b·c ≠ 0).
                let lc = self.lift(&c);
                let mut bc = FpVector::new(p, bc_mat.columns());
                bc_mat.apply(bc.as_slice_mut(), 1, lc.vec());
                let Some(v) = self.delta_preimage(v_deg, bc.as_slice()) else {
                    continue;
                };

                // a∪v, projected to an Ext class.
                let mut av = FpVector::new(p, av_mat.columns());
                av_mat.apply(av.as_slice_mut(), 1, v.as_slice());
                let representative = self.project(&Cochain::new(tot, av)).into_vec();

                let mut indeterminacy = a_indet.clone();
                self.massey_indeterminacy_c(&c, tot, &mut indeterminacy);
                let result = MasseyResult {
                    degree: tot,
                    coset: AffineSubspace::new(representative, indeterminacy),
                };
                if result.contains_zero() {
                    continue;
                }
                results.push((c, result));
            }
        }
        results
    }

    /// Compute the family of Massey products $\langle -, b, c\rangle$ for fixed `b` and `c` and
    /// every valid first factor `a` across all computed bidegrees.
    ///
    /// Note the homotopy is read at the *first factor's* filtration, so the cost grows with how far
    /// out the first factor ranges (and `f_b` must be extended over that range). For the dual
    /// pattern — fixed small `a, b`, sweeping a large third factor —
    /// [`massey_iter_c`](Self::massey_iter_c) is faster, since it reads at the small fixed `a.s`.
    ///
    /// `b` is taken in $\Ext(k, k)$ and `c` in $\Ext(M, k)$; the first factor ranges over $\Ext(k,
    /// k)$. The caller must have resolved `M` and the unit far enough. This assumes `b · c = 0` so
    /// that the bracket is defined; it is not verified. Brackets that contain `0` are omitted.
    pub fn massey_iter_a(
        &self,
        b: &BidegreeElement,
        c: &BidegreeElement,
    ) -> Vec<(BidegreeElement, MasseyResult)> {
        let p = self.prime();
        let resolution = self.resolution();
        let unit = self.unit();

        // The bracket of a first factor `a` lands at `tot = a.degree() + bc_shift`.
        let bc_shift = b.degree() + c.degree() - Bidegree::s_t(1, 0);

        // `f_c` realises `c` (resolution of `M` → unit); `f_b` is multiplication by `b` (in the
        // unit). The single null-homotopy `s_bc` of `b ∘ c` is reused for every first factor. `c`'s
        // cochain (cocycle) coordinates realise the chain map — the identity of its Ext coordinates
        // on a minimal resolution, a genuine lift on the untwisted tensor resolution.
        let c_coords: Vec<u32> = self.lift(c).vec().iter().collect();
        let f_c = Arc::new(ResolutionHomomorphism::from_class(
            String::new(),
            Arc::clone(resolution),
            Arc::clone(unit),
            c.degree(),
            &c_coords,
        ));
        let b_coords: Vec<u32> = b.vec().iter().collect();
        let f_b = Arc::new(ResolutionHomomorphism::from_class(
            String::new(),
            Arc::clone(unit),
            Arc::clone(unit),
            b.degree(),
            &b_coords,
        ));
        let s_bc = ChainHomotopy::new(Arc::clone(&f_c), Arc::clone(&f_b));

        let mut results = Vec::new();
        for a_deg in unit.iter_nonzero_stem() {
            let tot = a_deg + bc_shift;
            if !resolution.has_computed_bidegree(tot) {
                continue;
            }
            let target_num_gens = resolution.number_of_gens_in_bidegree(tot);
            let unit_dim = unit.number_of_gens_in_bidegree(a_deg);
            if target_num_gens == 0 || unit_dim == 0 {
                continue;
            }

            // Extend the maps and the (single) homotopy far enough to read off this bracket.
            f_c.extend_through_stem(tot);
            f_b.extend_through_stem(tot);
            s_bc.extend(tot);

            // `answers[j][i]` is the bracket of the `j`th first-factor generator, read off the
            // homotopy at filtration `a.s` (where the first factor lives).
            let offset_a = unit
                .module(a_deg.s())
                .generator_offset(a_deg.t(), a_deg.t(), 0);
            let last = s_bc.homotopy(tot.s());
            let mut answers = Matrix::new(p, unit_dim, target_num_gens);
            for i in 0..target_num_gens {
                let output = last.output(tot.t(), i);
                for j in 0..unit_dim {
                    answers.row_mut(j).set_entry(i, output.entry(offset_a + j));
                }
            }

            // Valid first factors are the kernel of `(· b)` on $\Ext(k, k)$, computed as `b · a`
            // (equal up to sign, so the same kernel) via the fixed `f_b`.
            let product_num_gens = unit.number_of_gens_in_bidegree(a_deg + b.degree());
            let mut product = AugmentedMatrix::<2>::new(p, unit_dim, [product_num_gens, unit_dim]);
            product.segment(1, 1).add_identity();
            for j in 0..unit_dim {
                let g = BidegreeGenerator::new(a_deg, j);
                f_b.act(product.row_mut(j).slice_mut(0, product_num_gens), 1, g);
            }
            product.row_reduce();
            let kernel = product.compute_kernel();

            for row in kernel.iter() {
                let a = BidegreeElement::new(a_deg, row.to_owned());
                let result = self.massey_result(&a, c, &answers, row, tot);
                if result.contains_zero() {
                    continue;
                }
                results.push((a, result));
            }
        }
        results
    }

    /// Compute the triple Massey product $\langle a, b, c\rangle$.
    ///
    /// `a` and `b` are taken in $\Ext(k, k)$ and `c` in $\Ext(M, k)$. Returns `None` if `a · b !=
    /// 0` or `b · c != 0`.
    pub fn massey(
        &self,
        a: &BidegreeElement,
        b: &BidegreeElement,
        c: &BidegreeElement,
    ) -> Option<MasseyResult> {
        let shift = Self::massey_shift(a, b);
        let b_hom = self.massey_b_hom(b, shift);

        // The bracket is defined only when `a · b = 0`. Compute `b · a` (equal to `a · b` up to
        // sign, so the same vanishing condition) via the multiplication-by-`b` self-map of the
        // unit, `b_hom`. The product lands at `a.degree() + b.degree()`, one filtration above
        // `shift`, so `b_hom` must be extended one step further than `massey_b_hom` built it.
        let ab_deg = a.degree() + b.degree();
        b_hom.extend_through_stem(ab_deg);
        let mut ab = FpVector::new(self.prime(), self.unit().number_of_gens_in_bidegree(ab_deg));
        for (j, coef) in a.vec().iter_nonzero() {
            b_hom.act(
                ab.as_slice_mut(),
                coef,
                BidegreeGenerator::new(a.degree(), j),
            );
        }
        if !ab.is_zero() {
            return None;
        }

        // The bracket is also defined only when `b · c = 0`. Check this via `c · b` (equal up to
        // sign) *before* building any null-homotopy: an invalid `c` has no null-homotopy and would
        // otherwise fail to lift.
        match self.try_multiply(c, b) {
            Some(prod) if prod.vec().is_zero() => {}
            _ => return None,
        }

        self.massey_bracket_of(a, b, b_hom, shift, c)
    }
}

#[cfg(test)]
mod tests {
    use sseq::coordinates::BidegreeGenerator;

    use super::*;
    use crate::utils::construct_standard;

    #[test]
    fn test_sphere_massey() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(6, 5));
        let alg = ExtAlgebra::new(Arc::clone(&res), res);

        let h0 = alg.generator(BidegreeGenerator::new(Bidegree::n_s(0, 1), 0));
        let h1 = alg.generator(BidegreeGenerator::new(Bidegree::n_s(1, 1), 0));

        // The classic relation <h0, h1, h0> = h1^2, the generator of Ext^{2,4} at (2, 2).
        let bracket = alg
            .massey(&h0, &h1, &h0)
            .expect("<h0, h1, h0> should be defined");
        assert_eq!(bracket.degree, Bidegree::n_s(2, 2));
        assert_eq!(alg.dimension(Bidegree::n_s(2, 2)), 1);
        assert!(
            !bracket.coset.offset().is_zero(),
            "<h0, h1, h0> = h1^2 should be nonzero"
        );

        let h1_sq = alg.multiply(&h1, &h1);
        assert_eq!(
            bracket.coset.offset().iter().collect::<Vec<_>>(),
            h1_sq.vec().iter().collect::<Vec<_>>(),
            "<h0, h1, h0> should equal h1^2"
        );
        // The indeterminacy a·Ext + Ext·c vanishes here, so the bracket is a single class.
        assert_eq!(bracket.coset.linear_part().dimension(), 0);

        // <h0, h1, h1> is undefined: h1 · h1 = h1^2 != 0, so h1 is not a valid third factor.
        assert!(
            alg.massey(&h0, &h1, &h1).is_none(),
            "<h0, h1, h1> should be undefined since h1^2 != 0"
        );

        // <h0, h0, h1> is undefined: h0 · h0 = h0^2 != 0, so the bracket's a · b = 0 fails.
        assert!(
            alg.massey(&h0, &h0, &h1).is_none(),
            "<h0, h0, h1> should be undefined since h0^2 != 0"
        );

        // Regression (issue #116): a first factor with `s >= 2` engages the homotopy-lift
        // obstruction. <h1^2, h0, h0> is undefined (b · c = h0 · h0 = h0^2 != 0), and `a · b =
        // h1^2 · h0 = 0` passes, so the third-factor path is exercised. The old per-generator
        // scheme built the null-homotopy for the non-kernel generator h0 and panicked ("Failed to
        // lift"); the fix rejects `c` up front and returns `None` without lifting.
        assert!(
            alg.massey(&h1_sq, &h0, &h0).is_none(),
            "<h1^2, h0, h0> should be undefined since h0^2 != 0, and must not panic"
        );
    }

    /// For `M == k`, iterating the first factor must agree with iterating the third, via the
    /// symmetry `<h0, h1, x> = ±<x, h1, h0>`. At `p = 2` the sign is trivial, so the cosets match
    /// exactly. This pins that the single-homotopy `massey_iter_a` path is correct.
    #[test]
    fn test_iter_a_matches_iter_c() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(6, 5));
        let alg = ExtAlgebra::new(Arc::clone(&res), res);

        let h0 = alg.generator(BidegreeGenerator::new(Bidegree::n_s(0, 1), 0));
        let h1 = alg.generator(BidegreeGenerator::new(Bidegree::n_s(1, 1), 0));

        // `<h0, h1, ->` over the third factor vs `<-, h1, h0>` over the first factor.
        let by_c = alg.massey_iter_c(&h0, &h1);
        let by_a = alg.massey_iter_a(&h1, &h0);
        assert!(!by_c.is_empty(), "expected some defined brackets");

        let normalize = |family: Vec<(BidegreeElement, MasseyResult)>| {
            let mut keyed: Vec<(String, AffineSubspace)> = family
                .into_iter()
                .map(|(x, result)| (format!("{x}"), result.coset))
                .collect();
            keyed.sort_by(|l, r| l.0.cmp(&r.0));
            keyed
        };
        assert_eq!(normalize(by_c), normalize(by_a));
    }

    /// Regression (issue #116) with a first factor of filtration `s = 2`. The old `massey_iter_c`
    /// built a null-homotopy per generator of each third-factor bidegree and panicked ("Failed to
    /// lift") whenever some generator was not killed by `b` — e.g. `h0` at `c_deg = (0, 1)`, since
    /// `h0 · h0 = h0^2 != 0`. This only surfaced for `a.s() >= 2` (see the homotopy top step). The
    /// fix realises the actual kernel class, so `massey_iter_c(h1^2, h0)` no longer panics and must
    /// agree with the reference `massey_iter_a(h0, h1^2)` via `<h1^2, h0, x> = ±<x, h0, h1^2>`
    /// (sign trivial at `p = 2`). The mere fact that `massey_iter_c` runs to completion here is the
    /// regression guarantee; the equality additionally pins that the fixed `iter_c` agrees with the
    /// independent `iter_a` path.
    #[test]
    fn test_iter_c_proper_kernel() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(6, 5));
        let alg = ExtAlgebra::new(Arc::clone(&res), res);

        let h0 = alg.generator(BidegreeGenerator::new(Bidegree::n_s(0, 1), 0));
        let h1 = alg.generator(BidegreeGenerator::new(Bidegree::n_s(1, 1), 0));
        let h1_sq = alg.multiply(&h1, &h1); // (n = 2, s = 2), so the fixed first factor has s = 2.

        // Old code panicked ("Failed to lift") building the per-generator homotopy for a non-kernel
        // generator (e.g. h0 at c_deg = (0, 1), since h0^2 != 0); the fix realises the actual
        // kernel class and completes.
        let by_c = alg.massey_iter_c(&h1_sq, &h0);
        let by_a = alg.massey_iter_a(&h0, &h1_sq);

        let normalize = |family: Vec<(BidegreeElement, MasseyResult)>| {
            let mut keyed: Vec<(String, AffineSubspace)> = family
                .into_iter()
                .map(|(x, result)| (format!("{x}"), result.coset))
                .collect();
            keyed.sort_by(|l, r| l.0.cmp(&r.0));
            keyed
        };
        assert_eq!(normalize(by_c), normalize(by_a));
    }
}
