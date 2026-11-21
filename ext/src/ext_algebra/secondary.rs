//! The secondary ($d_2$) layer over [`ExtAlgebra`] and [`ExtModule`].

use std::sync::Arc;

use algebra::pair_algebra::PairAlgebra;
use dashmap::DashMap;
use fp::{
    matrix::{Matrix, Subquotient},
    prime::Prime,
    vector::FpVector,
};
use sseq::coordinates::{Bidegree, BidegreeElement};

use super::{ExtAlgebra, ExtDifferential, ExtModule};
use crate::{
    chain_complex::FreeChainComplex,
    resolution_homomorphism::ResolutionHomomorphism,
    secondary::{
        LAMBDA_BIDEGREE, SecondaryLift, SecondaryResolution, SecondaryResolutionHomomorphism,
    },
};

/// The Adams $d_2$ as an [`ExtDifferential`], with shift $(n, s) \mapsto (n-1, s+2)$.
///
/// Its matrix out of a bidegree is the $d_2$ recorded by the secondary resolution's homotopies, so
/// attaching it to an [`ExtModule`] makes [`ExtModule::cohomology_subquotient`] the $E_3$ page.
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

        // Off the first quadrant Ext vanishes, a known zero. Inside it, an unresolved end means the
        // page there is unknown, so there is no differential: treating an unresolved target as
        // zero would make every source generator look like a surviving cycle.
        let gens = |x: Bidegree| -> Option<usize> {
            if x.n() < 0 || x.s() < 0 {
                Some(0)
            } else if res.has_computed_bidegree(x) {
                Some(res.number_of_gens_in_bidegree(x))
            } else {
                None
            }
        };
        let rows = gens(b)?;
        let cols = gens(target)?;

        let mut mat = Matrix::new(p, rows, cols);
        // `m[i]` is the d2 of the i-th generator of `b`, as a vector at `target`.
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

/// `resolution`'s Ext with the $d_2$ of `lift` attached, so that its cohomology is the $E_3$ page.
fn with_d2<CC: FreeChainComplex + 'static>(
    resolution: &Arc<CC>,
    lift: &Arc<SecondaryResolution<CC>>,
) -> ExtModule<CC>
where
    CC::Algebra: PairAlgebra,
{
    ExtModule::intrinsic(Arc::clone(resolution)).with_differential(Arc::new(SecondaryCoboundary {
        res_lift: Arc::clone(lift),
    }))
}

/// A single secondary product `x · y` in $\Mod_{C\lambda^2}$, where `y` is an $E_3$-surviving
/// class. See [`SecondaryExtModule::secondary_multiply_into`].
pub struct SecondaryProduct {
    /// The multiplicand: an $E_3$-surviving generator of the unit at the queried bidegree `b`.
    pub source: BidegreeElement,
    /// The $\Ext$ part of the product, in bidegree `b + x.degree()`.
    pub ext_part: FpVector,
    /// The $\lambda$ part of the product, in bidegree `b + x.degree() + LAMBDA_BIDEGREE`, already
    /// reduced by the image of $d_2$.
    pub lambda_part: FpVector,
}

/// The secondary layer over the ring $\Ext(k, k)$: the secondary resolution of `k` and its $E_3$
/// page.
///
/// It is a separate type from [`ExtAlgebra`] because it requires `CC::Algebra: PairAlgebra`, a
/// bound the primary layer does not impose.
pub struct SecondaryExtAlgebra<CC: FreeChainComplex>
where
    CC::Algebra: PairAlgebra,
{
    algebra: Arc<ExtAlgebra<CC>>,
    lift: Arc<SecondaryResolution<CC>>,
    /// The ring with $d_2$ attached; its cohomology is the $E_3$ page.
    e3: ExtModule<CC>,
}

impl<CC: FreeChainComplex + 'static> SecondaryExtAlgebra<CC>
where
    CC::Algebra: PairAlgebra,
{
    /// Build the secondary layer over the ring `algebra`.
    ///
    /// Construction is cheap; call [`extend_all`](Self::extend_all) to compute the secondary
    /// resolution.
    pub fn new(algebra: Arc<ExtAlgebra<CC>>) -> Self {
        let lift = Arc::new(SecondaryResolution::new(Arc::clone(algebra.resolution())));
        let e3 = with_d2(algebra.resolution(), &lift);
        Self { algebra, lift, e3 }
    }

    /// The primary ring this is built on.
    pub fn algebra(&self) -> &Arc<ExtAlgebra<CC>> {
        &self.algebra
    }

    /// The secondary resolution of `k`.
    pub fn lift(&self) -> &Arc<SecondaryResolution<CC>> {
        &self.lift
    }

    /// Extend the secondary resolution of `k` as far as its resolution allows; the $E_3$ page is
    /// then computed on demand.
    pub fn extend_all(&self) {
        self.lift.extend_all();
    }

    /// Sharding entry point: compute only the secondary resolution data for filtration `s`.
    /// Mirrors [`SecondaryLift::compute_partial`].
    pub fn compute_partial(&self, s: i32) {
        self.lift.compute_partial(s);
    }

    /// The $E_3$-page subquotient of $\Ext(k, k)$ at bidegree `b`.
    pub fn page_data(&self, b: Bidegree) -> Subquotient {
        self.e3
            .cohomology_subquotient(b)
            .expect("call extend_all() first (and query a computed bidegree)")
    }
}

/// The secondary layer over a module $\Ext(M, k)$: the secondary resolution of `M`, its $E_3$ page,
/// the $d_2$ differential and the $\Mod_{C\lambda^2}$ product.
pub struct SecondaryExtModule<CC: FreeChainComplex>
where
    CC::Algebra: PairAlgebra,
{
    module: Arc<ExtModule<CC>>,
    algebra: Arc<SecondaryExtAlgebra<CC>>,
    /// The same `Arc` as `algebra.lift()` when `M == k`.
    lift: Arc<SecondaryResolution<CC>>,
    /// The module with $d_2$ attached; its cohomology is the $E_3$ page.
    e3: ExtModule<CC>,
    /// Secondary lift of the multiplication map, cached per multiplier class `(degree, coords)`.
    secondary_products: DashMap<BidegreeElement, Arc<SecondaryResolutionHomomorphism<CC, CC>>>,
}

impl<CC: FreeChainComplex + 'static> SecondaryExtModule<CC>
where
    CC::Algebra: PairAlgebra,
{
    /// Build the secondary layer over `module`, together with a secondary layer over its ring.
    ///
    /// Construction is cheap; call [`extend_all`](Self::extend_all) to compute the secondary
    /// resolutions.
    pub fn from_module(module: Arc<ExtModule<CC>>) -> Self {
        let algebra = Arc::new(SecondaryExtAlgebra::new(Arc::clone(module.algebra())));
        Self::new(module, algebra)
    }

    /// Build the secondary layer over `module`, sharing an existing secondary layer over its ring.
    ///
    /// Panics if `algebra` is not built on `module`'s ring.
    pub fn new(module: Arc<ExtModule<CC>>, algebra: Arc<SecondaryExtAlgebra<CC>>) -> Self {
        assert!(
            Arc::ptr_eq(module.algebra(), algebra.algebra()),
            "the secondary ring layer must be built on the module's ring"
        );
        let lift = if module.is_unit() {
            Arc::clone(algebra.lift())
        } else {
            Arc::new(SecondaryResolution::new(Arc::clone(module.resolution())))
        };
        let e3 = with_d2(module.resolution(), &lift);
        Self {
            module,
            algebra,
            lift,
            e3,
            secondary_products: DashMap::new(),
        }
    }

    /// The primary module this is built on.
    pub fn module(&self) -> &Arc<ExtModule<CC>> {
        &self.module
    }

    /// The secondary layer over the ring.
    pub fn algebra(&self) -> &Arc<SecondaryExtAlgebra<CC>> {
        &self.algebra
    }

    /// The secondary resolution of `M`.
    pub fn lift(&self) -> &Arc<SecondaryResolution<CC>> {
        &self.lift
    }

    /// Whether `M == k`.
    pub fn is_unit(&self) -> bool {
        self.module.is_unit()
    }

    /// The prime of the underlying resolution.
    fn prime(&self) -> fp::prime::ValidPrime {
        self.module.prime()
    }

    /// Extend the secondary resolutions of `M` and `k` as far as their resolutions allow.
    ///
    /// Must be called before [`d2`](Self::d2), [`page_data`](Self::page_data) or
    /// [`secondary_multiply_into`](Self::secondary_multiply_into); the $E_3$ pages are then
    /// computed on demand.
    pub fn extend_all(&self) {
        if self.is_unit() {
            self.lift.extend_all();
        } else {
            maybe_rayon::join(|| self.lift.extend_all(), || self.algebra.extend_all());
        }
    }

    /// Sharding entry point: compute only the secondary resolution data for filtration `s`,
    /// distributed across machines sharing a save directory (see the `secondary` example docs).
    /// Mirrors [`SecondaryLift::compute_partial`].
    pub fn compute_partial(&self, s: i32) {
        self.lift.compute_partial(s);
        if !self.is_unit() {
            self.algebra.compute_partial(s);
        }
    }

    /// The secondary differential $d_2(x)$, a class in bidegree `(n - 1, s + 2)`.
    ///
    /// Returns `None` if the target bidegree has not been computed (so $d_2$ is unknown). A
    /// computed-but-zero differential is `Some` of a zero class.
    pub fn d2(&self, x: &BidegreeElement) -> Option<BidegreeElement> {
        let b = x.degree();
        let target = b + Bidegree::n_s(-1, 2);
        let res = self.lift.underlying();
        if !(b.t() > 0 && res.has_computed_bidegree(target)) {
            return None;
        }

        let target_dim = res.number_of_gens_in_bidegree(target);
        let mut out = FpVector::new(self.prime(), target_dim);

        // `m[i]` is the d2 of the i-th generator of `b`, as a vector at `target`.
        let m = self.lift.homotopy(b.s() + 2).homotopies.hom_k(b.t());
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

    /// The $E_3$-page subquotient of $\Ext(M, k)$ at bidegree `b`.
    pub fn page_data(&self, b: Bidegree) -> Subquotient {
        self.e3
            .cohomology_subquotient(b)
            .expect("call extend_all() first (and query a computed bidegree)")
    }
}

impl<CC: FreeChainComplex + crate::chain_complex::AugmentedChainComplex + 'static>
    SecondaryExtModule<CC>
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
            Arc::clone(self.module.resolution()),
            Arc::clone(self.module.algebra().resolution()),
            x.degree(),
            &x.vec().iter().collect::<Vec<_>>(),
        ));
        let lift = Arc::new(SecondaryResolutionHomomorphism::new(
            Arc::clone(&self.lift),
            Arc::clone(self.algebra.lift()),
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
        // `hom_k` queries the page at the λ-part's source, which only it knows.
        let lambda_page = |bd: Bidegree| self.e3.cohomology_subquotient(bd);

        let ext_dim = self
            .module
            .resolution()
            .number_of_gens_in_bidegree(b + shift);
        let lambda_dim = self
            .module
            .resolution()
            .number_of_gens_in_bidegree(b + shift + LAMBDA_BIDEGREE);

        let page = self.algebra.page_data(b);
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
    use crate::{chain_complex::ChainComplex, utils::construct_standard};

    #[test]
    fn test_sphere_d2() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        // Far enough to reach the first Adams differential d2(h4) = h0 h3^2 at (14, 3).
        res.compute_through_stem(Bidegree::n_s(16, 6));
        let e2 = Arc::new(ExtModule::intrinsic(res));
        let sec_e2 = SecondaryExtModule::from_module(Arc::clone(&e2));
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
    fn a_module_over_its_own_ring_shares_the_secondary_resolution() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(16, 6));
        let sec = SecondaryExtModule::from_module(Arc::new(ExtModule::intrinsic(res)));
        assert!(sec.is_unit());
        assert!(Arc::ptr_eq(sec.lift(), sec.algebra().lift()));
        sec.extend_all();

        for (n, s) in [(0, 1), (14, 3), (15, 1)] {
            let b = Bidegree::n_s(n, s);
            assert_eq!(
                sec.page_data(b).dimension(),
                sec.algebra().page_data(b).dimension(),
                "module and ring E3 pages differ at (n={n}, s={s})"
            );
        }
    }

    #[test]
    #[should_panic(expected = "built on the module's ring")]
    fn a_secondary_ring_over_another_ring_is_rejected() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(4, 4));
        let module = Arc::new(ExtModule::intrinsic(Arc::clone(&res)));
        // Same resolution, but a different `ExtAlgebra` handle than the module's ring.
        let other = Arc::new(SecondaryExtAlgebra::new(Arc::new(ExtAlgebra::new(res))));
        let _ = SecondaryExtModule::new(module, other);
    }

    #[test]
    fn d2_as_ext_differential_reproduces_the_e3_page() {
        // `cohomology_subquotient` under `SecondaryCoboundary` must match `page_data`: same
        // dimension at every bidegree and the same d2-image quotient.
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(16, 6));
        let e2 = Arc::new(ExtModule::intrinsic(Arc::clone(&res)));
        let sec = SecondaryExtModule::from_module(Arc::clone(&e2));
        sec.extend_all();

        let coboundary = Arc::new(SecondaryCoboundary {
            res_lift: Arc::clone(&sec.lift),
        });
        let e2_d2 = ExtModule::intrinsic(Arc::clone(&res)).with_differential(coboundary);

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

    /// At the top of the computed region the incoming $d_2$ is unknown, so no page is claimed.
    ///
    /// $d_2$ shifts $(n, s) \mapsto (n-1, s+2)$, so what lands on `b` comes from $(n+1, s-2)$ —
    /// past the last computed stem here. Reading that as rank zero would report the $E_2$
    /// dimension as though nothing could hit `b`, at exactly the bidegree where something might.
    #[test]
    fn an_unknown_incoming_d2_is_not_read_as_zero() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(8, 4));
        let e2 = Arc::new(ExtModule::intrinsic(Arc::clone(&res)));
        let sec = SecondaryExtModule::from_module(Arc::clone(&e2));
        sec.extend_all();
        let e2_d2 = ExtModule::intrinsic(Arc::clone(&res)).with_differential(Arc::new(
            SecondaryCoboundary {
                res_lift: Arc::clone(&sec.lift),
            },
        ));

        let b = Bidegree::n_s(8, 2);
        assert!(res.has_computed_bidegree(b), "b itself must be computed");
        assert!(
            !res.has_computed_bidegree(Bidegree::n_s(9, 0)),
            "the incoming source must be unresolved for this to be the edge case"
        );
        assert_eq!(e2_d2.cohomology_dimension(b), None);
        assert!(e2_d2.cohomology_subquotient(b).is_none());

        // An *empty* bidegree is a different matter: nothing can survive there, so the unknown
        // incoming differential does not make the answer unknown.
        let empty = Bidegree::n_s(8, 4);
        assert_eq!(e2.dimension(empty), 0);
        assert!(!res.has_computed_bidegree(Bidegree::n_s(9, 2)));
        assert_eq!(e2_d2.cohomology_dimension(empty), Some(0));
        assert_eq!(
            e2_d2.cohomology_subquotient(empty).map(|q| q.dimension()),
            Some(0)
        );
    }

    #[test]
    fn secondary_product_runs_and_ext_part_is_the_primary_product() {
        // Every product's Ext part equals the primary Ext product x · source.
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(10, 8));
        let e2 = Arc::new(ExtModule::intrinsic(Arc::clone(&res)));
        let sec = SecondaryExtModule::from_module(Arc::clone(&e2));
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
