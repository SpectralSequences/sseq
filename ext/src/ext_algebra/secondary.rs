//! The secondary ($d_2$ / $\Mod_{C\lambda^2}$) layer, split into a ring and a module.
//!
//! This mirrors the primary [`ExtAlgebra`] / [`ExtModule`] split one level up:
//!
//! - [`SecondaryExtAlgebra`] is the **ring** secondary layer over `k`: it wraps an
//!   [`ExtAlgebra`] and owns `k`'s secondary resolution and its $E_3$ page.
//! - [`SecondaryExtModule`] is the **module** secondary layer over `M`: it wraps an
//!   [`ExtModule`], holds a shared [`Arc`] to the [`SecondaryExtAlgebra`], and
//!   owns `M`'s secondary resolution, its $E_3$ page, the secondary differential
//!   [`d2`](SecondaryExtModule::d2) (and [`survives`](SecondaryExtModule::survives)), and the
//!   $\Mod_{C\lambda^2}$ secondary product
//!   [`secondary_multiply_into`](SecondaryExtModule::secondary_multiply_into).
//!
//! When `M == k` the module shares its secondary resolution and $E_3$ page with the ring (see
//! [`SecondaryExtModule::is_unit`]), so there is no `is_unit` special-casing baked into the ring.
//!
//! These wrap [`SecondaryResolution`] and [`SecondaryResolutionHomomorphism`]; no new linear
//! algebra is implemented here. The layer is split out from the primary types because the secondary
//! machinery requires `CC::Algebra: PairAlgebra`, a bound the primary layer does not impose.

use std::sync::{Arc, Mutex};

use algebra::pair_algebra::PairAlgebra;
use dashmap::DashMap;
use fp::{matrix::Subquotient, prime::Prime, vector::FpVector};
use sseq::coordinates::{Bidegree, BidegreeElement};

use super::{ExtAlgebra, ExtModule};
use crate::{
    chain_complex::FreeChainComplex,
    resolution_homomorphism::ResolutionHomomorphism,
    secondary::{
        LAMBDA_BIDEGREE, SecondaryLift, SecondaryResolution, SecondaryResolutionHomomorphism,
    },
};

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

/// The **ring** secondary layer over `k`: the secondary resolution of `k` and its $E_3$ page. See
/// the [module documentation](self).
pub struct SecondaryExtAlgebra<CC: FreeChainComplex>
where
    CC::Algebra: PairAlgebra,
{
    algebra: Arc<ExtAlgebra<CC>>,
    /// Secondary resolution of `k`.
    lift: Arc<SecondaryResolution<CC>>,
    /// $E_3$ page of `k`, filled by [`extend`](Self::extend).
    sseq: Mutex<Option<Arc<sseq::Sseq<2, sseq::Adams>>>>,
}

impl<CC: FreeChainComplex> SecondaryExtAlgebra<CC>
where
    CC::Algebra: PairAlgebra,
{
    /// Build the ring secondary layer over the ring `algebra`. Construction is cheap; call
    /// [`extend`](Self::extend) to compute the secondary resolution and $E_3$ page.
    pub fn new(algebra: Arc<ExtAlgebra<CC>>) -> Self {
        let lift = Arc::new(SecondaryResolution::new(Arc::clone(algebra.resolution())));
        Self {
            algebra,
            lift,
            sseq: Mutex::new(None),
        }
    }

    /// The primary [`ExtAlgebra`] this is built on.
    pub fn algebra(&self) -> &Arc<ExtAlgebra<CC>> {
        &self.algebra
    }

    /// The secondary resolution of `k`.
    pub fn lift(&self) -> &Arc<SecondaryResolution<CC>> {
        &self.lift
    }

    /// Extend the secondary resolution of `k` as far as the underlying resolution allows, then
    /// compute its $E_3$ page.
    pub fn extend(&self) {
        self.lift.extend_all();
        *self.sseq.lock().unwrap() = Some(Arc::new(self.lift.e3_page()));
    }

    /// Install an already-computed $E_3$ page. Used by [`SecondaryExtModule::extend_all`] in the
    /// `M == k` case, where the module and ring share one secondary resolution and hence one page.
    fn install_sseq(&self, sseq: Arc<sseq::Sseq<2, sseq::Adams>>) {
        *self.sseq.lock().unwrap() = Some(sseq);
    }

    /// Sharding entry point: compute only the secondary resolution data for filtration `s`. Mirrors
    /// [`SecondaryLift::compute_partial`]. Returns before any $E_3$ page is built.
    pub fn compute_partial(&self, s: i32) {
        self.lift.compute_partial(s);
    }

    /// The $E_3$-page subquotient of the ring $\Ext(k, k)$ at bidegree `b`.
    pub fn page_data(&self, b: Bidegree) -> Subquotient {
        let g = self.sseq.lock().unwrap();
        e3_page_data(g.as_ref().expect("call extend() first"), b).clone()
    }
}

/// The **module** secondary layer over `M`: the secondary resolution of `M`, its $E_3$ page, the
/// secondary differential, and the $\Mod_{C\lambda^2}$ product. See the [module documentation](self).
pub struct SecondaryExtModule<CC: FreeChainComplex>
where
    CC::Algebra: PairAlgebra,
{
    module: Arc<ExtModule<CC>>,
    /// Shared handle to the ring secondary layer over `k`.
    algebra: Arc<SecondaryExtAlgebra<CC>>,
    /// Secondary resolution of `M`. `Arc`-shared with `algebra.lift()` when `M == k`.
    lift: Arc<SecondaryResolution<CC>>,
    /// $E_3$ page of `M`, filled by [`extend_all`](Self::extend_all).
    sseq: Mutex<Option<Arc<sseq::Sseq<2, sseq::Adams>>>>,
    /// Secondary lift of the module-action map, cached per multiplier class `(degree, coords)`.
    secondary_products: DashMap<BidegreeElement, Arc<SecondaryResolutionHomomorphism<CC, CC>>>,
}

impl<CC: FreeChainComplex> SecondaryExtModule<CC>
where
    CC::Algebra: PairAlgebra,
{
    /// Build the module secondary layer over `module`, deriving the ring secondary layer over its
    /// unit `k`. Construction is cheap; call [`extend_all`](Self::extend_all) to compute the
    /// secondary resolutions and $E_3$ pages.
    pub fn from_module(module: Arc<ExtModule<CC>>) -> Self {
        let algebra = Arc::new(SecondaryExtAlgebra::new(Arc::clone(module.algebra())));
        Self::new(module, algebra)
    }

    /// Build the module secondary layer over `module` and an explicit ring secondary layer. The
    /// secondary resolution of `M` is `Arc`-shared with the ring's when `M == k`.
    pub fn new(module: Arc<ExtModule<CC>>, algebra: Arc<SecondaryExtAlgebra<CC>>) -> Self {
        let lift = if module.is_unit() {
            Arc::clone(algebra.lift())
        } else {
            Arc::new(SecondaryResolution::new(Arc::clone(module.resolution())))
        };
        Self {
            module,
            algebra,
            lift,
            sseq: Mutex::new(None),
            secondary_products: DashMap::new(),
        }
    }

    /// The primary [`ExtModule`] this is built on.
    pub fn module(&self) -> &Arc<ExtModule<CC>> {
        &self.module
    }

    /// The ring secondary layer over `k`.
    pub fn algebra(&self) -> &Arc<SecondaryExtAlgebra<CC>> {
        &self.algebra
    }

    /// The secondary resolution of `M`.
    pub fn lift(&self) -> &Arc<SecondaryResolution<CC>> {
        &self.lift
    }

    /// Whether `M == k`, i.e. the module shares its secondary resolution with the ring.
    pub fn is_unit(&self) -> bool {
        self.module.is_unit()
    }

    fn prime(&self) -> fp::prime::ValidPrime {
        self.module.prime()
    }

    /// Extend the secondary resolutions as far as the underlying resolutions allow, then compute
    /// the $E_3$ pages. Must be called before [`d2`](Self::d2), [`page_data`](Self::page_data) or
    /// [`secondary_multiply_into`](Self::secondary_multiply_into).
    pub fn extend_all(&self) {
        self.lift.extend_all();
        let sseq = Arc::new(self.lift.e3_page());
        *self.sseq.lock().unwrap() = Some(Arc::clone(&sseq));

        if self.is_unit() {
            // `self.lift` and `self.algebra.lift()` are the same `Arc`, so the ring's $E_3$ page is
            // the one we just computed — install it instead of recomputing.
            self.algebra.install_sseq(sseq);
        } else {
            self.algebra.extend();
        }
    }

    /// Sharding entry point: compute only the secondary resolution data for filtration `s`,
    /// distributed across machines sharing a save directory (see the `secondary` example docs).
    /// Mirrors [`SecondaryLift::compute_partial`]. Returns before any $E_3$ page is built.
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

        // `m[i]` is the d2 of the i-th generator of `b`, as a vector at `target`. This is exactly
        // the matrix `SecondaryResolution::e3_page` reads to install d2 differentials.
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
        let g = self.sseq.lock().unwrap();
        e3_page_data(g.as_ref().expect("call extend_all() first"), b).clone()
    }

    /// The $E_3$ page of `M`, or `None` before [`extend_all`](Self::extend_all) has run.
    pub(crate) fn sseq(&self) -> Option<Arc<sseq::Sseq<2, sseq::Adams>>> {
        self.sseq.lock().unwrap().clone()
    }
}

impl<CC: FreeChainComplex + crate::chain_complex::AugmentedChainComplex> SecondaryExtModule<CC>
where
    CC::Algebra: PairAlgebra,
{
    /// The secondary lift of module-action by `x`, built and cached per multiplier class. The
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
        let res_sseq = self.sseq().expect("call extend_all() first");

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
            Some(&res_sseq),
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

/// The $E_3$-page subquotient a spectral sequence records at bidegree `b`.
fn e3_page_data(sseq: &sseq::Sseq<2, sseq::Adams>, b: Bidegree) -> &Subquotient {
    let d = sseq.page_data(b);
    &d[std::cmp::min(3, d.len() - 1)]
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
}
