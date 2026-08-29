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

use std::sync::{Arc, Mutex};

use algebra::pair_algebra::PairAlgebra;
use dashmap::DashMap;
use fp::{matrix::Subquotient, prime::Prime, vector::FpVector};
use sseq::coordinates::{Bidegree, BidegreeElement};

use super::ExtAlgebra;
use crate::{
    chain_complex::FreeChainComplex,
    resolution_homomorphism::ResolutionHomomorphism,
    secondary::{
        LAMBDA_BIDEGREE, SecondaryDegree, SecondaryElement, SecondaryGenerator, SecondaryLift,
        SecondaryResolution, SecondaryResolutionHomomorphism, Weight,
    },
};

/// A single secondary product `x · y` in $\Mod_{C\lambda^2}$, where `y` is an $E_3$-surviving
/// class. See [`SecondaryExtAlgebra::secondary_multiply_into`].
pub struct SecondaryProduct {
    /// The multiplicand: an $E_3$-surviving generator of the unit at the queried bidegree `b`.
    pub source: BidegreeElement,
    /// The product `x · source`, a class in the secondary ($\Mod_{C\lambda^2}$) homotopy with base
    /// bidegree `b + x.degree()`. Its $\lambda$ part is already reduced by the image of $d_2$.
    pub value: SecondaryElement,
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
    /// $E_3$ page of the resolution, filled by [`extend_all`](Self::extend_all).
    res_sseq: Mutex<Option<Arc<sseq::Sseq<2, sseq::Adams>>>>,
    /// $E_3$ page of the unit, filled by [`extend_all`](Self::extend_all).
    unit_sseq: Mutex<Option<Arc<sseq::Sseq<2, sseq::Adams>>>>,
    /// Secondary lift of the multiplication map, cached per multiplier class `(degree, coords)`.
    secondary_products: DashMap<BidegreeElement, Arc<SecondaryResolutionHomomorphism<CC, CC>>>,
}

impl<CC: FreeChainComplex> SecondaryExtAlgebra<CC>
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
        Self {
            alg,
            res_lift,
            unit_lift,
            res_sseq: Mutex::new(None),
            unit_sseq: Mutex::new(None),
            secondary_products: DashMap::new(),
        }
    }

    /// Extend the secondary resolutions as far as the underlying resolutions allow, then compute
    /// the $E_3$ pages. Must be called before [`d2`](Self::d2), [`page_data`](Self::page_data) or
    /// [`secondary_multiply_into`](Self::secondary_multiply_into).
    pub fn extend_all(&self) {
        self.res_lift.extend_all();
        if !self.alg.is_unit() {
            self.unit_lift.extend_all();
        }

        *self.res_sseq.lock().unwrap() = Some(Arc::new(self.res_lift.e3_page()));
        let unit = if self.alg.is_unit() {
            Arc::clone(self.res_sseq.lock().unwrap().as_ref().unwrap())
        } else {
            Arc::new(self.unit_lift.e3_page())
        };
        *self.unit_sseq.lock().unwrap() = Some(unit);
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

    /// The $E_3$-page subquotient of $\Ext(M, k)$ at bidegree `b`.
    pub fn page_data(&self, b: Bidegree) -> Subquotient {
        let g = self.res_sseq.lock().unwrap();
        Self::e3_page_data(g.as_ref().expect("call extend_all() first"), b).clone()
    }

    /// The $E_3$-page subquotient of the unit $\Ext(k, k)$ at bidegree `b`.
    pub fn unit_page_data(&self, b: Bidegree) -> Subquotient {
        let g = self.unit_sseq.lock().unwrap();
        Self::e3_page_data(g.as_ref().expect("call extend_all() first"), b).clone()
    }

    // Indexing the Cλ²-module the way `Bidegree{,Generator,Element}` index `ExtAlgebra`. These read
    // the *ambient* generator counts (like `ExtAlgebra::dimension`), so the secondary `d_2`/$E_3$
    // structure stays a separate computation. `b` and `b + LAMBDA_BIDEGREE` must be computed;
    // otherwise `number_of_gens_in_bidegree` panics, matching `ExtAlgebra::dimension`.

    /// The dimension of the weight-`weight` part of the secondary homotopy of `M` at `deg`: the
    /// ambient number of generators of $\Ext(M, k)$ in that part's bidegree.
    pub fn weight_dimension(&self, deg: SecondaryDegree, weight: Weight) -> usize {
        self.alg
            .resolution()
            .number_of_gens_in_bidegree(deg.bidegree(weight))
    }

    /// The total dimension of the secondary homotopy of `M` at `deg` (weight 0 plus weight 1).
    pub fn dimension(&self, deg: SecondaryDegree) -> usize {
        self.weight_dimension(deg, Weight::Ext) + self.weight_dimension(deg, Weight::Lambda)
    }

    /// The basis generators of the secondary homotopy of `M` at `deg`: the weight-0 generators
    /// followed by the weight-1 (λ) generators.
    pub fn basis(&self, deg: SecondaryDegree) -> Vec<SecondaryGenerator> {
        let base = deg.base();
        [Weight::Ext, Weight::Lambda]
            .into_iter()
            .flat_map(|w| {
                (0..self.weight_dimension(deg, w)).map(move |i| SecondaryGenerator::new(base, w, i))
            })
            .collect()
    }

    /// A class in the secondary homotopy of `M` at `deg` from its coordinates in the weight-0 and
    /// weight-1 generator bases.
    pub fn element(
        &self,
        deg: SecondaryDegree,
        ext_coords: &[u32],
        lambda_coords: &[u32],
    ) -> SecondaryElement {
        assert_eq!(self.weight_dimension(deg, Weight::Ext), ext_coords.len());
        assert_eq!(
            self.weight_dimension(deg, Weight::Lambda),
            lambda_coords.len()
        );
        let p = self.prime();
        SecondaryElement::new(
            deg.base(),
            FpVector::from_slice(p, ext_coords),
            FpVector::from_slice(p, lambda_coords),
        )
    }

    /// A single generator of the secondary homotopy of `M` as a class.
    pub fn generator(&self, g: SecondaryGenerator) -> SecondaryElement {
        let deg = g.degree();
        assert!(self.weight_dimension(deg, g.weight()) > g.idx());
        g.into_element(
            self.prime(),
            self.weight_dimension(deg, Weight::Ext),
            self.weight_dimension(deg, Weight::Lambda),
        )
    }

    /// The dimension of the weight-`weight` part of the secondary homotopy of the unit `k` at
    /// `deg` (the multiplicand / "scalar" side, i.e. $C\lambda^2$ itself).
    pub fn unit_weight_dimension(&self, deg: SecondaryDegree, weight: Weight) -> usize {
        self.alg
            .unit()
            .number_of_gens_in_bidegree(deg.bidegree(weight))
    }

    /// The total dimension of the secondary homotopy of the unit `k` at `deg`.
    pub fn unit_dimension(&self, deg: SecondaryDegree) -> usize {
        self.unit_weight_dimension(deg, Weight::Ext)
            + self.unit_weight_dimension(deg, Weight::Lambda)
    }

    /// The basis generators of the secondary homotopy of the unit `k` at `deg`.
    pub fn unit_basis(&self, deg: SecondaryDegree) -> Vec<SecondaryGenerator> {
        let base = deg.base();
        [Weight::Ext, Weight::Lambda]
            .into_iter()
            .flat_map(|w| {
                (0..self.unit_weight_dimension(deg, w))
                    .map(move |i| SecondaryGenerator::new(base, w, i))
            })
            .collect()
    }

    /// A class in the secondary homotopy of the unit `k` at `deg`.
    pub fn unit_element(
        &self,
        deg: SecondaryDegree,
        ext_coords: &[u32],
        lambda_coords: &[u32],
    ) -> SecondaryElement {
        assert_eq!(
            self.unit_weight_dimension(deg, Weight::Ext),
            ext_coords.len()
        );
        assert_eq!(
            self.unit_weight_dimension(deg, Weight::Lambda),
            lambda_coords.len()
        );
        let p = self.prime();
        SecondaryElement::new(
            deg.base(),
            FpVector::from_slice(p, ext_coords),
            FpVector::from_slice(p, lambda_coords),
        )
    }

    /// A single generator of the secondary homotopy of the unit `k` as a class.
    pub fn unit_generator(&self, g: SecondaryGenerator) -> SecondaryElement {
        let deg = g.degree();
        assert!(self.unit_weight_dimension(deg, g.weight()) > g.idx());
        g.into_element(
            self.prime(),
            self.unit_weight_dimension(deg, Weight::Ext),
            self.unit_weight_dimension(deg, Weight::Lambda),
        )
    }

    fn e3_page_data(sseq: &sseq::Sseq<2, sseq::Adams>, b: Bidegree) -> &Subquotient {
        let d = sseq.page_data(b);
        &d[std::cmp::min(3, d.len() - 1)]
    }
}

impl<CC: FreeChainComplex + crate::chain_complex::AugmentedChainComplex> SecondaryExtAlgebra<CC>
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
        let res_sseq = Arc::clone(
            self.res_sseq
                .lock()
                .unwrap()
                .as_ref()
                .expect("call extend_all() first"),
        );

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
            Some(&res_sseq),
            b,
            page.subspace_gens(),
            outputs.iter_mut().map(FpVector::as_slice_mut),
        );

        page.subspace_gens()
            .zip(outputs)
            .map(|(g, out)| SecondaryProduct {
                source: BidegreeElement::new(b, g.to_owned()),
                value: SecondaryElement::from_concatenated(b + shift, out.as_slice(), ext_dim),
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
    fn test_secondary_indexing() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(8, 8));
        let e2 = Arc::new(ExtAlgebra::new(Arc::clone(&res), res));
        let sec = SecondaryExtAlgebra::new(Arc::clone(&e2));

        // (0,0): bottom class + λh0; (0,1): h0 + λh0²; (1,1): h1 + λ-part.
        for (n, s) in [(0, 0), (0, 1), (1, 1)] {
            let base = Bidegree::n_s(n, s);
            let deg = SecondaryDegree::new(base);

            let ext_dim = e2.resolution().number_of_gens_in_bidegree(base);
            let lambda_dim = e2
                .resolution()
                .number_of_gens_in_bidegree(base + LAMBDA_BIDEGREE);

            assert_eq!(sec.weight_dimension(deg, Weight::Ext), ext_dim);
            assert_eq!(sec.weight_dimension(deg, Weight::Lambda), lambda_dim);
            assert_eq!(sec.dimension(deg), ext_dim + lambda_dim);

            let basis = sec.basis(deg);
            assert_eq!(basis.len(), sec.dimension(deg));

            for (i, g) in basis.iter().enumerate() {
                // Weight-0 generators come first, then weight-1.
                let (expected_weight, expected_idx) = if i < ext_dim {
                    (Weight::Ext, i)
                } else {
                    (Weight::Lambda, i - ext_dim)
                };
                assert_eq!(g.weight(), expected_weight);
                assert_eq!(g.idx(), expected_idx);
                assert_eq!(g.base(), base);

                // generator(g) round-trips to element(...) with a single 1 in the right part.
                let elt = sec.generator(*g);
                let mut ext_coords = vec![0u32; ext_dim];
                let mut lambda_coords = vec![0u32; lambda_dim];
                match g.weight() {
                    Weight::Ext => ext_coords[g.idx()] = 1,
                    Weight::Lambda => lambda_coords[g.idx()] = 1,
                }
                assert_eq!(elt, sec.element(deg, &ext_coords, &lambda_coords));
                assert_eq!(
                    elt.ext().iter_nonzero().count() + elt.lambda().iter_nonzero().count(),
                    1
                );
            }
        }
    }
}
