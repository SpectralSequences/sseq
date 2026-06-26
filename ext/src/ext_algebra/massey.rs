//! Primary Massey products in $\Ext$.
//!
//! [`ExtAlgebra::massey`] computes a single triple Massey product $\langle a, b, c\rangle$, while
//! [`ExtAlgebra::massey_iter_c`] and [`ExtAlgebra::massey_iter_a`] sweep a whole family at once: the
//! former fixes $a, b$ and ranges over every valid third factor $\langle a, b, -\rangle$, the latter
//! fixes $b, c$ and ranges over every valid first factor $\langle -, b, c\rangle$. The two
//! directions differ in which null-homotopy is reused — see their docs for when to prefer each.
//!
//! All three wrap [`ChainHomotopy`]: we lift the multiplication maps, build the null-homotopy of
//! the composite (`b ∘ c` or `a ∘ b`), and read off the bracket by pairing against the remaining
//! factor. The valid factors (those whose product with `b` vanishes) are exactly the kernel of
//! multiplication by `b`.
//!
//! The result is an [`AffineSubspace`]: a coset representative (the offset) together with the
//! indeterminacy $a \cdot \Ext + \Ext \cdot c$ (the linear part). Both terms of the indeterminacy
//! are the $\Ext(k, k)$-module action on $\Ext(M, k)$, so it is computed for any `M`. This matches
//! (and reuses the logic of) the `massey` example, which computes the products up to a sign.

use std::sync::Arc;

use fp::{
    matrix::{AffineSubspace, AugmentedMatrix, Matrix, Subspace},
    vector::{FpSlice, FpVector},
};
use sseq::coordinates::{Bidegree, BidegreeElement, BidegreeGenerator};

use super::ExtAlgebra;
use crate::{
    chain_complex::{AugmentedChainComplex, ChainHomotopy, FreeChainComplex},
    resolution_homomorphism::ResolutionHomomorphism,
};

/// The result of a Massey product computation: a coset, given by a representative together with
/// the indeterminacy subspace.
#[derive(Debug, Clone)]
pub struct MasseyResult {
    /// The bidegree of the bracket, `a.degree() + b.degree() + c.degree() - (1, 0)`.
    pub degree: Bidegree,
    /// The bracket as a coset: a representative (the offset) modulo the indeterminacy
    /// $a \cdot \Ext + \Ext \cdot c$ (the linear part).
    pub coset: AffineSubspace,
}

impl MasseyResult {
    /// A representative of the Massey product, as an element of the bracket's bidegree.
    pub fn representative(&self) -> BidegreeElement {
        BidegreeElement::new(self.degree, self.coset.offset().clone())
    }

    /// Whether `0` lies in the Massey product, i.e. the representative lies in the indeterminacy.
    /// Such brackets carry no information and are typically omitted from output.
    pub fn contains_zero(&self) -> bool {
        self.coset.contains_zero()
    }
}

impl<CC> ExtAlgebra<CC>
where
    CC: FreeChainComplex + AugmentedChainComplex,
{
    /// The bidegree shift of $\langle a, b, -\rangle$: a class `c` produces a bracket in bidegree
    /// `c.degree() + a.degree() + b.degree() - (1, 0)`.
    fn massey_shift(a: &BidegreeElement, b: &BidegreeElement) -> Bidegree {
        a.degree() + b.degree() - Bidegree::s_t(1, 0)
    }

    /// The multiplication-by-`b` chain map (in the unit), extended far enough for brackets landing
    /// at `shift`.
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

    /// Compute, for a single multiplicand bidegree `c_deg`, the per-generator bracket values and
    /// the kernel of multiplication by `b` (the valid third factors). The bracket values form a
    /// `num_gens × target_num_gens` matrix whose row `gen` is the bracket of the `gen`th generator
    /// of `c_deg`. Returns `None` if the bracket bidegree is empty or uncomputed.
    fn massey_at(
        &self,
        a: &BidegreeElement,
        b: &BidegreeElement,
        b_hom: &Arc<ResolutionHomomorphism<CC, CC>>,
        shift: Bidegree,
        offset_a: usize,
        c_deg: Bidegree,
    ) -> Option<(Matrix, Subspace, Bidegree)> {
        let p = self.prime();
        let resolution = self.resolution();
        let unit = self.unit();

        if !resolution.has_computed_bidegree(c_deg + shift) {
            return None;
        }
        let tot = c_deg + shift;

        let num_gens = resolution.number_of_gens_in_bidegree(c_deg);
        let product_num_gens = resolution.number_of_gens_in_bidegree(b.degree() + c_deg);
        let target_num_gens = resolution.number_of_gens_in_bidegree(tot);
        if target_num_gens == 0 {
            return None;
        }

        let a_coords: Vec<u32> = a.vec().iter().collect();
        let b_coords: Vec<u32> = b.vec().iter().collect();

        let mut answers = Matrix::new(p, num_gens, target_num_gens);
        let mut product = AugmentedMatrix::<2>::new(p, num_gens, [product_num_gens, num_gens]);
        product.segment(1, 1).add_identity();

        let mut matrix = Matrix::new(p, num_gens, 1);
        for idx in 0..num_gens {
            let hom = Arc::new(ResolutionHomomorphism::new(
                String::new(),
                Arc::clone(resolution),
                Arc::clone(unit),
                c_deg,
            ));

            matrix.row_mut(idx).set_entry(0, 1);
            hom.extend_step(c_deg, Some(&matrix));
            matrix.row_mut(idx).set_entry(0, 0);

            hom.extend_through_stem(tot);

            let homotopy = ChainHomotopy::new(Arc::clone(&hom), Arc::clone(b_hom));
            homotopy.extend(tot);

            let last = homotopy.homotopy(tot.s());
            let mut answer_row = answers.row_mut(idx);
            for i in 0..target_num_gens {
                let output = last.output(tot.t(), i);
                for (k, &val) in a_coords.iter().enumerate() {
                    if val != 0 {
                        answer_row.add_basis_element(i, val * output.entry(offset_a + k));
                    }
                }
            }

            for (k, &val) in b_coords.iter().enumerate() {
                if val != 0 {
                    let g = BidegreeGenerator::new(b.degree(), k);
                    hom.act(product.row_mut(idx).slice_mut(0, product_num_gens), val, g);
                }
            }
        }
        product.row_reduce();
        let kernel = product.compute_kernel();

        Some((answers, kernel, tot))
    }

    /// The bracket representative coordinates for a third factor with coordinates `row`: the
    /// per-generator bracket values `answers` contracted against `row` (a matrix-vector product).
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
        let representative = self.massey_representative(answers, row);
        let indeterminacy = self.massey_indeterminacy(a, c, tot);
        MasseyResult {
            degree: tot,
            coset: AffineSubspace::new(representative, indeterminacy),
        }
    }

    /// The indeterminacy $a \cdot \Ext^{|b| + |c| - (1,0)} + \Ext^{|a| + |b| - (1,0)} \cdot c$ at
    /// the bracket bidegree `tot`, as a subspace of $\Ext(M, k)$ at `tot`.
    ///
    /// Both terms are the $\Ext(k, k)$-module action on $\Ext(M, k)$, so this is valid for any `M`,
    /// not just `M == k`. The first term ranges `a` over $\Ext(M, k)$ in the complementary degree;
    /// the second ranges over $\Ext(k, k)$. Products are computed up to sign (as elsewhere), which
    /// does not affect the spanned subspace.
    fn massey_indeterminacy(
        &self,
        a: &BidegreeElement,
        c: &BidegreeElement,
        tot: Bidegree,
    ) -> Subspace {
        let mut sub = Subspace::new(self.prime(), self.dimension(tot));

        // a · Ext(M, k)^{tot - a.degree()}, computed as y · a (equal up to sign).
        for y in self.basis(tot - a.degree()) {
            if let Some(prod) = self.try_multiply(&self.generator(y), a) {
                sub.add_vector(prod.vec());
            }
        }
        // Ext(k, k)^{tot - c.degree()} · c, computed as c · x (equal up to sign).
        for x in self.unit_basis(tot - c.degree()) {
            if let Some(prod) = self.try_multiply(c, &self.unit_generator(x)) {
                sub.add_vector(prod.vec());
            }
        }
        sub
    }

    /// Compute the family of Massey products $\langle a, b, -\rangle$ for fixed `a` and `b` and
    /// every valid third factor `c` (those with `b · c = 0`), across all computed bidegrees.
    ///
    /// `a` and `b` are taken in $\Ext(k, k)$; the third factor ranges over $\Ext(M, k)$. The
    /// caller must have resolved `M` and the unit far enough. This assumes `a · b = 0` (so that the
    /// bracket is defined); it is not verified.
    ///
    /// Brackets that contain `0` (the representative lies in the indeterminacy) carry no
    /// information and are omitted.
    ///
    /// This iterates over the third factor, building a fresh null-homotopy of `b ∘ c` per `c`. Each
    /// is cheap when `a` is small, since it is only read at filtration `a.s`. To vary the *first*
    /// factor with `b, c` fixed instead, use [`massey_iter_a`](Self::massey_iter_a).
    pub fn massey_iter_c(
        &self,
        a: &BidegreeElement,
        b: &BidegreeElement,
    ) -> Vec<(BidegreeElement, MasseyResult)> {
        let shift = Self::massey_shift(a, b);
        let offset_a =
            self.unit()
                .module(a.degree().s())
                .generator_offset(a.degree().t(), a.degree().t(), 0);
        let b_hom = self.massey_b_hom(b, shift);

        let mut results = Vec::new();
        for c_deg in self.resolution().iter_nonzero_stem() {
            let Some((answers, kernel, tot)) = self.massey_at(a, b, &b_hom, shift, offset_a, c_deg)
            else {
                continue;
            };
            for row in kernel.iter() {
                let c = BidegreeElement::new(c_deg, row.to_owned());
                let result = self.massey_result(a, &c, &answers, row, tot);
                if result.contains_zero() {
                    continue;
                }
                results.push((c, result));
            }
        }
        results
    }

    /// Compute the family of Massey products $\langle -, b, c\rangle$ for fixed `b` and `c` and
    /// every valid first factor `a` (those with `a · b = 0`), across all computed bidegrees.
    ///
    /// The null-homotopy of `b ∘ c` depends only on `b` and `c`, so it is built **once** and
    /// re-read at each first factor's filtration. This is the right direction when you want to vary
    /// the first factor: it avoids rebuilding a homotopy per factor (which is what looping
    /// [`massey_iter_c`](Self::massey_iter_c) over `a` would do). It works for any `M`, unlike the
    /// symmetric "fix the `a ∘ b` homotopy" idea, which would land on the conventionally-zero bottom
    /// homotopy.
    ///
    /// Note the homotopy is read at the *first factor's* filtration, so the cost grows with how far
    /// out the first factor ranges (and `f_b` must be extended over that range). For the dual
    /// pattern — fixed small `a, b`, sweeping a large third factor —
    /// [`massey_iter_c`](Self::massey_iter_c) is faster, since it reads at the small fixed `a.s`.
    ///
    /// `b` is taken in $\Ext(k, k)$ and `c` in $\Ext(M, k)$; the first factor ranges over
    /// $\Ext(k, k)$. The caller must have resolved `M` and the unit far enough. This assumes
    /// `b · c = 0` (so that the bracket is defined); it is not verified. Brackets that contain `0`
    /// are omitted.
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
        // unit). The single null-homotopy `s_bc` of `b ∘ c` is reused for every first factor.
        let c_coords: Vec<u32> = c.vec().iter().collect();
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
    /// `a` and `b` are taken in $\Ext(k, k)$ and `c` in $\Ext(M, k)$. Returns `None` if
    /// `b · c != 0` (so the bracket is undefined). This assumes `a · b = 0`; it is not verified.
    pub fn massey(
        &self,
        a: &BidegreeElement,
        b: &BidegreeElement,
        c: &BidegreeElement,
    ) -> Option<MasseyResult> {
        let shift = Self::massey_shift(a, b);
        let offset_a =
            self.unit()
                .module(a.degree().s())
                .generator_offset(a.degree().t(), a.degree().t(), 0);
        let b_hom = self.massey_b_hom(b, shift);

        let (answers, kernel, tot) = self.massey_at(a, b, &b_hom, shift, offset_a, c.degree())?;

        // The bracket is defined exactly when b · c = 0, i.e. c lies in the kernel of (· b).
        let mut reduced = c.vec().to_owned();
        kernel.reduce(reduced.as_slice_mut());
        if !reduced.is_zero() {
            return None;
        }

        Some(self.massey_result(a, c, &answers, c.vec(), tot))
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
}
