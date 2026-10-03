//! The secondary ($d_2$) layer of [`ExtAlgebra`].
//!
//! [`SecondaryExtAlgebra`] composes an [`ExtAlgebra`] with the secondary resolutions of `M` and
//! the unit `k`, and exposes:
//! - the secondary differential [`d2`](SecondaryExtAlgebra::d2) (and the survival check
//!   [`survives`](SecondaryExtAlgebra::survives)),
//! - the $E_3$-page data [`page_data`](SecondaryExtAlgebra::page_data), and
//! - the $\Mod_{C\lambda^2}$ secondary product
//!   [`secondary_multiply_into`](SecondaryExtAlgebra::secondary_multiply_into).
//!
//! These wrap [`SecondaryResolution`] and [`SecondaryResolutionHomomorphism`]; no new linear
//! algebra is implemented here. The layer is split out from [`ExtAlgebra`] because the secondary
//! machinery requires `CC::Algebra: PairAlgebra`, a bound the primary layer does not impose.

use std::sync::Arc;

use algebra::pair_algebra::PairAlgebra;
use dashmap::DashMap;
use fp::{
    matrix::{Matrix, Subquotient},
    prime::Prime,
    vector::FpVector,
};
use sseq::coordinates::{Bidegree, BidegreeElement};

use super::{ExtAlgebra, ExtDifferential};
use crate::{
    chain_complex::FreeChainComplex,
    resolution_homomorphism::ResolutionHomomorphism,
    secondary::{
        LAMBDA_BIDEGREE, SecondaryLift, SecondaryResolution, SecondaryResolutionHomomorphism,
    },
};

/// The Adams $d_2$ presented as an [`ExtDifferential`] on the primary
/// [`ExtAlgebra`]: the same coboundary shape as the motivic $\delta$, only with the
/// Adams shift $(n, s) \mapsto (n-1, s+2)$. Its matrix out of a bidegree is exactly
/// the $d_2$ the secondary resolution's homotopies record. Attaching it makes
/// [`ExtAlgebra::cohomology_subquotient`] compute the $E_3$ page on the shared
/// kernel-mod-image path — the same object the spectral-sequence bookkeeping gives.
pub(crate) struct SecondaryCoboundary<CC: FreeChainComplex>
where
    CC::Algebra: PairAlgebra,
{
    res_lift: Arc<SecondaryResolution<CC>>,
}

impl<CC: FreeChainComplex> ExtDifferential for SecondaryCoboundary<CC>
where
    CC::Algebra: PairAlgebra,
{
    fn shift(&self) -> Bidegree {
        Bidegree::n_s(-1, 2)
    }

    fn matrix(&self, b: Bidegree) -> Option<Matrix> {
        let res = self.res_lift.underlying();
        let p = res.prime();
        let target = b + self.shift();

        // The shape must be exactly `gens(b) × gens(target)`: an `a × 0` (empty
        // target — the whole source is a d2-cycle) and a `0 × b` (off-axis source,
        // in-quadrant target — contributes to the ambient-`b` image) are genuinely
        // different and both matter to `cohomology_subquotient`, so size each end at
        // its own bidegree. Off the first quadrant Ext vanishes (0 generators, a
        // *known* zero).
        //
        // The source and target ends differ when the bidegree is in the first
        // quadrant but unresolved:
        //   * Source `b` (rows): the page at `b` is then unknown, so the whole
        //     differential is unavailable — `None`.
        //   * Target (cols): the secondary resolution simply records no d2 landing
        //     there yet. The E3-page convention (`SecondaryResolution::e3_page`)
        //     treats an uncomputed outgoing differential as zero, so we give a
        //     `rows × 0` matrix — the whole source is a *provisional* d2-cycle —
        //     rather than `None`. (This `rows × 0` matrix is never consumed as an
        //     incoming differential: the target bidegree's own subquotient short-
        //     circuits to `None` at its numerator, since its rows are unresolved.)
        //
        // `gens` gives the generator count at an end, or `None` when the bidegree is
        // in the first quadrant but unresolved (a known zero off the quadrant).
        let gens = |x: Bidegree| -> Option<usize> {
            if x.n() < 0 || x.s() < 0 {
                Some(0)
            } else if res.has_computed_bidegree(x) {
                Some(res.number_of_gens_in_bidegree(x))
            } else {
                None
            }
        };
        // Source unresolved ⇒ unknown page ⇒ no differential; target unresolved ⇒ no
        // d2 recorded yet ⇒ provisional cycle (`rows × 0`).
        let rows = gens(b)?;
        let cols = gens(target).unwrap_or(0);

        let mut mat = Matrix::new(p, rows, cols);
        // Fill only when both ends carry generators; `m[i]` is the d2 of the i-th
        // generator of `b`, as a vector at `target` — the same matrix
        // `SecondaryResolution::e3_page` reads to install d2.
        if rows > 0 && cols > 0 {
            let m = self.res_lift.homotopy(b.s() + 2).homotopies.hom_k(b.t());
            if !m.is_empty() && !m[0].is_empty() {
                for (i, row) in m.iter().enumerate() {
                    for (k, &v) in row.iter().enumerate() {
                        if v != 0 {
                            mat.row_mut(i).set_entry(k, v);
                        }
                    }
                }
            }
        }
        Some(mat)
    }
}

/// A single secondary product `x · y` in $\Mod_{C\lambda^2}$, where `y` is an $E_3$-surviving
/// class. See [`SecondaryExtAlgebra::secondary_multiply_into`].
pub struct SecondaryProduct {
    /// The multiplicand: an $E_3$-surviving generator of the unit at the queried bidegree `b`.
    pub source: BidegreeElement,
    /// The $\Ext$ part of the product, in bidegree `b + x.degree()`.
    pub ext_part: FpVector,
    /// The $\lambda$ part of the product, in bidegree `b + x.degree() + LAMBDA_BIDEGREE`, already
    /// reduced by the image of $d_2$.
    pub lambda_part: FpVector,
}

/// The secondary layer over an [`ExtAlgebra`]: the $d_2$ differential and the $\Mod_{C\lambda^2}$
/// product. See the [module documentation](self).
pub struct SecondaryExtAlgebra<CC: FreeChainComplex>
where
    CC::Algebra: PairAlgebra,
{
    alg: Arc<ExtAlgebra<CC>>,
    res_lift: Arc<SecondaryResolution<CC>>,
    /// `Arc`-shared with `res_lift` when `M == k`.
    unit_lift: Arc<SecondaryResolution<CC>>,
    /// The primary Ext with the Adams $d_2$ ([`SecondaryCoboundary`]) attached: its
    /// [`cohomology_subquotient`](ExtAlgebra::cohomology_subquotient) is the $E_3$
    /// page of $\Ext(M, k)$. The page is computed on demand from the extended
    /// secondary homotopies — no separate spectral-sequence object.
    alg_d2: ExtAlgebra<CC>,
    /// The unit Ext with $d_2$ attached: the $E_3$ page of $\Ext(k, k)$.
    unit_d2: ExtAlgebra<CC>,
    /// Secondary lift of the multiplication map, cached per multiplier class `(degree, coords)`.
    secondary_products: DashMap<BidegreeElement, Arc<SecondaryResolutionHomomorphism<CC, CC>>>,
}

impl<CC: FreeChainComplex + 'static> SecondaryExtAlgebra<CC>
where
    CC::Algebra: PairAlgebra,
{
    /// Build the secondary layer over `alg`. Construction is cheap; call [`extend_all`](Self::extend_all)
    /// to actually compute the secondary resolutions and $E_3$ pages.
    pub fn new(alg: Arc<ExtAlgebra<CC>>) -> Self {
        let res_lift = Arc::new(SecondaryResolution::new(Arc::clone(alg.resolution())));
        let unit_lift = if alg.is_unit() {
            Arc::clone(&res_lift)
        } else {
            Arc::new(SecondaryResolution::new(Arc::clone(alg.unit())))
        };
        // Ext-with-d2 objects whose `cohomology_subquotient` is the E3 page. The
        // coboundary reads the secondary homotopies lazily, so building these before
        // `extend_all` is cheap.
        let alg_d2 = ExtAlgebra::new(Arc::clone(alg.resolution()), Arc::clone(alg.unit()))
            .with_differential(Arc::new(SecondaryCoboundary {
                res_lift: Arc::clone(&res_lift),
            }));
        let unit_d2 = ExtAlgebra::new(Arc::clone(alg.unit()), Arc::clone(alg.unit()))
            .with_differential(Arc::new(SecondaryCoboundary {
                res_lift: Arc::clone(&unit_lift),
            }));
        Self {
            alg,
            res_lift,
            unit_lift,
            alg_d2,
            unit_d2,
            secondary_products: DashMap::new(),
        }
    }

    /// Extend the secondary resolutions as far as the underlying resolutions allow.
    /// Must be called before [`d2`](Self::d2), [`page_data`](Self::page_data) or
    /// [`secondary_multiply_into`](Self::secondary_multiply_into); the $E_3$ pages are
    /// then computed on demand from the extended homotopies.
    pub fn extend_all(&self) {
        self.res_lift.extend_all();
        if !self.alg.is_unit() {
            self.unit_lift.extend_all();
        }
    }

    /// Sharding entry point: compute only the secondary resolution data for filtration `s`,
    /// distributed across machines sharing a save directory (see the `secondary` example docs).
    /// Mirrors [`SecondaryLift::compute_partial`]. Returns before any $E_3$ page is built.
    pub fn compute_partial(&self, s: i32) {
        self.res_lift.compute_partial(s);
        if !self.alg.is_unit() {
            self.unit_lift.compute_partial(s);
        }
    }

    /// The primary [`ExtAlgebra`] this is built on.
    pub fn ext_algebra(&self) -> &Arc<ExtAlgebra<CC>> {
        &self.alg
    }

    fn prime(&self) -> fp::prime::ValidPrime {
        self.alg.prime()
    }

    /// The secondary differential $d_2(x)$, a class in bidegree `(n - 1, s + 2)`.
    ///
    /// Returns `None` if the target bidegree has not been computed (so $d_2$ is unknown). A
    /// computed-but-zero differential is `Some` of a zero class.
    pub fn d2(&self, x: &BidegreeElement) -> Option<BidegreeElement> {
        let b = x.degree();
        let target = b + Bidegree::n_s(-1, 2);
        let res = self.res_lift.underlying();
        if !(b.t() > 0 && res.has_computed_bidegree(target)) {
            return None;
        }

        let target_dim = res.number_of_gens_in_bidegree(target);
        let mut out = FpVector::new(self.prime(), target_dim);

        // `m[i]` is the d2 of the i-th generator of `b`, as a vector at `target`. This is exactly
        // the matrix `SecondaryResolution::e3_page` reads to install d2 differentials.
        let m = self.res_lift.homotopy(b.s() + 2).homotopies.hom_k(b.t());
        if !m.is_empty() && !m[0].is_empty() {
            let p = self.prime().as_u32();
            for (i, c) in x.vec().iter_nonzero() {
                for (k, &v) in m[i].iter().enumerate() {
                    out.add_basis_element(k, (c * v) % p);
                }
            }
        }
        Some(BidegreeElement::new(target, out))
    }

    /// Whether `x` is a $d_2$-cycle (a permanent class through $E_3$).
    pub fn survives(&self, x: &BidegreeElement) -> Option<bool> {
        self.d2(x).map(|d| d.vec().is_zero())
    }

    /// The $E_3$-page subquotient of $\Ext(M, k)$ at bidegree `b` — the cohomology of
    /// the primary Ext with the Adams $d_2$ attached, on the shared
    /// [`cohomology_subquotient`](ExtAlgebra::cohomology_subquotient) path.
    pub fn page_data(&self, b: Bidegree) -> Subquotient {
        self.alg_d2
            .cohomology_subquotient(b)
            .expect("call extend_all() first (and query a computed bidegree)")
    }

    /// The $E_3$-page subquotient of the unit $\Ext(k, k)$ at bidegree `b`.
    pub fn unit_page_data(&self, b: Bidegree) -> Subquotient {
        self.unit_d2
            .cohomology_subquotient(b)
            .expect("call extend_all() first (and query a computed bidegree)")
    }
}

impl<CC: FreeChainComplex + crate::chain_complex::AugmentedChainComplex + 'static>
    SecondaryExtAlgebra<CC>
where
    CC::Algebra: PairAlgebra,
{
    /// The secondary lift of multiplication by `x`, built and cached per multiplier class. The
    /// returned lift is *not* extended; [`secondary_multiply_into`](Self::secondary_multiply_into)
    /// extends it as needed. Exposed so callers can drive sharded computation
    /// (`lift.underlying().extend_all()` then `lift.compute_partial(s)`).
    pub fn secondary_product_lift(
        &self,
        x: &BidegreeElement,
    ) -> Arc<SecondaryResolutionHomomorphism<CC, CC>> {
        if let Some(map) = self.secondary_products.get(x) {
            return Arc::clone(&map);
        }

        let name = format!("prod_{x}",);
        let underlying = Arc::new(ResolutionHomomorphism::from_class(
            name,
            Arc::clone(self.alg.resolution()),
            Arc::clone(self.alg.unit()),
            x.degree(),
            &x.vec().iter().collect::<Vec<_>>(),
        ));
        let lift = Arc::new(SecondaryResolutionHomomorphism::new(
            Arc::clone(&self.res_lift),
            Arc::clone(&self.unit_lift),
            underlying,
        ));

        Arc::clone(
            self.secondary_products
                .entry(x.clone())
                .or_insert(lift)
                .value(),
        )
    }

    /// The secondary product of `x` with every $E_3$-surviving class of the unit at bidegree `b`,
    /// computed in $\Mod_{C\lambda^2}$.
    ///
    /// Returns one [`SecondaryProduct`] per surviving generator at `b`; the $\lambda$ part is
    /// already reduced by the image of $d_2$. The caller must have run [`extend_all`](Self::extend_all)
    /// and computed both resolutions far enough.
    pub fn secondary_multiply_into(
        &self,
        x: &BidegreeElement,
        b: Bidegree,
    ) -> Vec<SecondaryProduct> {
        let p = self.prime();
        let shift = x.degree();
        // `hom_k` reduces the λ-part of the product by the image of d2 at the λ-part's
        // source. Rather than reconstruct that bidegree here, hand it the E3 page as a
        // function of bidegree (from the shared cohomology path on the primary Ext,
        // where the product lands) and let `hom_k` query it at the right place.
        let lambda_page = |bd: Bidegree| self.alg_d2.cohomology_subquotient(bd);

        let ext_dim = self.alg.resolution().number_of_gens_in_bidegree(b + shift);
        let lambda_dim = self
            .alg
            .resolution()
            .number_of_gens_in_bidegree(b + shift + LAMBDA_BIDEGREE);

        let page = self.unit_page_data(b);
        let n = page.subspace_dimension();
        if n == 0 {
            return Vec::new();
        }

        let lift = self.secondary_product_lift(x);
        lift.underlying().extend_all();
        lift.extend_all();

        let mut outputs = vec![FpVector::new(p, ext_dim + lambda_dim); n];
        lift.hom_k(
            Some(&lambda_page),
            b,
            page.subspace_gens(),
            outputs.iter_mut().map(FpVector::as_slice_mut),
        );

        page.subspace_gens()
            .zip(outputs)
            .map(|(g, out)| SecondaryProduct {
                source: BidegreeElement::new(b, g.to_owned()),
                ext_part: out.slice(0, ext_dim).to_owned(),
                lambda_part: out.slice(ext_dim, ext_dim + lambda_dim).to_owned(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use sseq::coordinates::BidegreeGenerator;

    use super::*;
    use crate::utils::construct_standard;

    #[test]
    fn test_sphere_d2() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        // Far enough to reach the first Adams differential d2(h4) = h0 h3^2 at (14, 3).
        res.compute_through_stem(Bidegree::n_s(16, 6));
        let e2 = Arc::new(ExtAlgebra::new(Arc::clone(&res), res));
        let sec_e2 = SecondaryExtAlgebra::new(Arc::clone(&e2));
        sec_e2.extend_all();

        // h_0, h_1, h_2 are permanent cycles.
        for (n, s) in [(0, 1), (1, 1), (3, 1)] {
            let h = e2.generator(BidegreeGenerator::new(Bidegree::n_s(n, s), 0));
            let h_survives = sec_e2
                .survives(&h)
                .unwrap_or_else(|| panic!("h at (n={n}, s={s}) should have a computed d2"));
            assert!(h_survives, "h at (n={n}, s={s}) should survive d2");
            let h_d2 = sec_e2
                .d2(&h)
                .unwrap_or_else(|| panic!("h at (n={n}, s={s}) should have a computed d2"));
            assert!(
                h_d2.vec().is_zero(),
                "d2 of a permanent class should vanish"
            );
        }

        // The first Adams differential: d2(h4) = h0 h3^2, the generator of Ext^{3,17} at (14, 3).
        let h4 = e2.generator(BidegreeGenerator::new(Bidegree::n_s(15, 1), 0));
        let d = sec_e2.d2(&h4).expect("d2(h4) target should be computed");
        assert_eq!(d.degree(), Bidegree::n_s(14, 3));
        assert_eq!(e2.dimension(Bidegree::n_s(14, 3)), 1);
        assert!(!d.vec().is_zero(), "d2(h4) = h0 h3^2 should be nonzero");
        let h4_survives = sec_e2.survives(&h4).expect("h4 should have a computed d2");
        assert!(!h4_survives, "h4 should not survive d2");
    }

    #[test]
    fn d2_as_ext_differential_reproduces_the_e3_page() {
        // The Adams d2 is an `ExtDifferential`: attaching `SecondaryCoboundary` to the
        // primary ExtAlgebra makes the shared `cohomology_subquotient` path compute the
        // exact E3 page the spectral-sequence bookkeeping (`page_data`) gives — same
        // dimension at every bidegree, and the same d2-image quotient.
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(16, 6));
        let e2 = Arc::new(ExtAlgebra::new(Arc::clone(&res), Arc::clone(&res)));
        let sec = SecondaryExtAlgebra::new(Arc::clone(&e2));
        sec.extend_all();

        let coboundary = Arc::new(SecondaryCoboundary {
            res_lift: Arc::clone(&sec.res_lift),
        });
        let e2_d2 =
            ExtAlgebra::new(Arc::clone(&res), Arc::clone(&res)).with_differential(coboundary);

        let mut saw_nontrivial = false;
        for n in 0..=15 {
            for s in 1..=5 {
                let b = Bidegree::n_s(n, s);
                let Some(dim) = e2_d2.cohomology_dimension(b) else {
                    continue;
                };
                let page = sec.page_data(b);
                assert_eq!(
                    dim,
                    page.dimension(),
                    "E3 dimension mismatch at (n={n}, s={s})"
                );
                // The subquotient's denominator is the d2-image: reducing any E2 vector
                // by it must agree with the spectral sequence's page quotient.
                let sq = e2_d2.cohomology_subquotient(b).unwrap();
                assert_eq!(
                    sq.dimension(),
                    page.dimension(),
                    "E3 subquotient dimension mismatch at (n={n}, s={s})"
                );
                if page.dimension() != e2.dimension(b) {
                    saw_nontrivial = true; // d2 actually killed something here
                }
            }
        }
        assert!(
            saw_nontrivial,
            "expected d2 to be nontrivial somewhere in range (e.g. h4 at (15,1) → (14,3))"
        );
    }

    #[test]
    fn secondary_product_runs_and_ext_part_is_the_primary_product() {
        // End-to-end check of the product path after routing the E3 page onto the
        // shared `cohomology_subquotient` (the λ-part reduce now reads it, not a
        // separate Sseq): `secondary_multiply_into` runs, and every product's Ext part
        // equals the primary Ext product x · source.
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(10, 8));
        let e2 = Arc::new(ExtAlgebra::new(Arc::clone(&res), Arc::clone(&res)));
        let sec = SecondaryExtAlgebra::new(Arc::clone(&e2));
        sec.extend_all();

        // Multiply h0 into the classes at (0,1); the lone survivor is h0, so the Ext
        // part must be the primary product h0 · h0 = h0².
        let h0 = e2.generator(BidegreeGenerator::new(Bidegree::n_s(0, 1), 0));
        let products = sec.secondary_multiply_into(&h0, Bidegree::n_s(0, 1));
        assert!(
            !products.is_empty(),
            "expected a secondary product at (0,1)"
        );
        for prod in &products {
            let primary = e2.multiply(&h0, &prod.source);
            assert_eq!(
                prod.ext_part,
                primary.vec().to_owned(),
                "secondary product Ext part must equal the primary product"
            );
        }
    }
}
