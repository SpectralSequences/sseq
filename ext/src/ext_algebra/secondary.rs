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

use std::{
    fmt,
    sync::{Arc, Mutex},
};

use algebra::pair_algebra::PairAlgebra;
use dashmap::DashMap;
use fp::{matrix::Subquotient, prime::Prime, vector::FpVector};
pub use sseq::coordinates::BZE;
use sseq::coordinates::{
    Bidegree, BidegreeElement, BidegreeGenerator, MultiDegree, MultiDegreeElement,
};

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

/// A conical basis generator of $\pi(S/\lambda^2)$.
///
/// Each Ext generator at bidegree $(n, s)$ contributes to the conical basis at one or both weights,
/// determined by its Adams BZE classification (see
/// [`adams_classify`](SecondaryExtAlgebra::adams_classify)):
/// - **B**: weight 0 only (killed at weight 1 by $d_2$ boundaries).
/// - **Z**: both weights (permanent cycle).
/// - **E**: weight 1 only (killed at weight 0 by supporting $d_2$).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PiGenerator {
    bidegree: Bidegree,
    weight: Weight,
    bze: BZE,
    idx: usize,
}

impl PiGenerator {
    pub fn new(bidegree: Bidegree, weight: Weight, bze: BZE, idx: usize) -> Self {
        Self {
            bidegree,
            weight,
            bze,
            idx,
        }
    }

    pub fn bidegree(&self) -> Bidegree {
        self.bidegree
    }

    pub fn weight(&self) -> Weight {
        self.weight
    }

    pub fn bze(&self) -> BZE {
        self.bze
    }

    pub fn idx(&self) -> usize {
        self.idx
    }
}

impl fmt::Display for PiGenerator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let g = BidegreeGenerator::new(self.bidegree, self.idx);
        let w = self.weight.as_i32();
        write!(f, "{} x_{g}^{w}", self.bze)
    }
}

/// An element in the $E_3 = E_\infty$ page of $\pi(S/\lambda^2)$ at a specific weight.
///
/// The coordinates are in the subquotient basis of $E_3$ at the given bidegree and weight. Each
/// coordinate corresponds to a surviving generator in the ambient Ext space, identified by
/// [`basis_indices`](Self::basis_indices).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PiElement {
    bidegree: Bidegree,
    weight: Weight,
    coords: Vec<u32>,
    basis_indices: Vec<usize>,
}

impl PiElement {
    pub fn bidegree(&self) -> Bidegree {
        self.bidegree
    }

    pub fn weight(&self) -> Weight {
        self.weight
    }

    /// Coordinates in the $E_3$ subquotient basis. `coords()[i]` is the coefficient of the
    /// generator at ambient index [`basis_indices()`](Self::basis_indices)`[i]`.
    pub fn coords(&self) -> &[u32] {
        &self.coords
    }

    /// The ambient Ext generator indices forming the $E_3$ basis.
    pub fn basis_indices(&self) -> &[usize] {
        &self.basis_indices
    }

    pub fn is_zero(&self) -> bool {
        self.coords.iter().all(|&c| c == 0)
    }
}

impl fmt::Display for PiElement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let w = self.weight.as_i32();
        let mut first = true;
        for (&c, &idx) in self.coords.iter().zip(&self.basis_indices) {
            if c == 0 {
                continue;
            }
            if !first {
                write!(f, " + ")?;
            }
            first = false;
            let g = BidegreeGenerator::new(self.bidegree, idx);
            if c == 1 {
                write!(f, "x_{g}^{w}")?;
            } else {
                write!(f, "{c} x_{g}^{w}")?;
            }
        }
        if first {
            write!(f, "0")?;
        }
        Ok(())
    }
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
    /// Trigraded spectral sequence for $S/\lambda^2$. Coordinates are `(n, s, bock)` = (stem,
    /// Adams filtration, Bockstein degree). Filled by [`extend_all`](Self::extend_all).
    lambda2_sseq: Mutex<Option<Arc<sseq::Sseq<3, sseq::AdamsLambda2>>>>,
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
            lambda2_sseq: Mutex::new(None),
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

        *self.lambda2_sseq.lock().unwrap() = Some(Arc::new(self.build_lambda2_sseq()));
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

    /// Classify a generator at bidegree `b` as B, Z, or E in the $d_2$ decomposition.
    ///
    /// - **B** (boundary): in the image of $d_2$ from another bidegree.
    /// - **Z** (cycle mod boundary): a $d_2$-cycle that is not a boundary; survives to $E_3$.
    /// - **E** (supports $d_2$): $d_2(x) \neq 0$.
    ///
    /// At each bidegree, $\Ext = B \oplus Z \oplus E$ and $d_2$ restricts to an isomorphism
    /// $E_{(n,s)} \to B_{(n-1,s+2)}$.
    pub fn classify(&self, g: BidegreeGenerator) -> BZE {
        let [n, s] = g.degree().coords();
        self.lambda2_sseq()
            .classify(MultiDegree::new([n, s, 0]), 3, g.idx())
    }

    /// The full Adams BZE classification, combining both weights of the $\lambda^2$ spectral
    /// sequence.
    ///
    /// Unlike [`classify`](Self::classify) (which only inspects weight 0), this checks both:
    /// - **E**: supports $d_2$ at weight 0.
    /// - **B**: boundary of $d_2$ at weight 1.
    /// - **Z**: permanent cycle (neither E nor B).
    pub fn adams_classify(&self, g: BidegreeGenerator) -> BZE {
        let [n, s] = g.degree().coords();
        let l2 = self.lambda2_sseq();

        let td0 = MultiDegree::new([n, s, 0]);
        if l2.defined(td0) && l2.classify(td0, 3, g.idx()) == BZE::E {
            return BZE::E;
        }

        let td1 = MultiDegree::new([n, s, 1]);
        if l2.defined(td1) && l2.classify(td1, 3, g.idx()) == BZE::B {
            return BZE::B;
        }

        BZE::Z
    }

    /// The conical basis of $\pi(S/\lambda^2)$ at bidegree `b` (Condition 3.2 of the paper).
    ///
    /// Each Ext generator at `b` is classified by [`adams_classify`](Self::adams_classify), then
    /// placed at the weights it contributes to:
    /// - **B**: weight 0 only.
    /// - **Z**: both weights.
    /// - **E**: weight 1 only.
    pub fn pi_basis(&self, b: Bidegree) -> Vec<PiGenerator> {
        let dim = self.alg.dimension(b);
        let mut result = Vec::new();

        for i in 0..dim {
            let g = BidegreeGenerator::new(b, i);
            let bze = self.adams_classify(g);

            if bze != BZE::E {
                result.push(PiGenerator::new(b, Weight::Ext, bze, i));
            }
            if bze != BZE::B {
                result.push(PiGenerator::new(b, Weight::Lambda, bze, i));
            }
        }

        result
    }

    /// Project a [`SecondaryElement`] to the $E_3$ subquotient at each weight, giving a pair of
    /// [`PiElement`]s.
    ///
    /// The first element is the weight-0 projection (Ext part at `base`), the second is the
    /// weight-1 projection ($\lambda$ part at `base + LAMBDA_BIDEGREE`).
    pub fn to_pi(&self, elt: &SecondaryElement) -> (PiElement, PiElement) {
        let base = elt.base();
        let ext_pi = self.project_to_pi(base, Weight::Ext, elt.ext());
        let lambda_pi = self.project_to_pi(base + LAMBDA_BIDEGREE, Weight::Lambda, elt.lambda());
        (ext_pi, lambda_pi)
    }

    fn project_to_pi(
        &self,
        bidegree: Bidegree,
        weight: Weight,
        ambient_vec: fp::vector::FpSlice,
    ) -> PiElement {
        let [n, s] = bidegree.coords();
        let td = MultiDegree::new([n, s, weight.as_i32()]);

        if let Some(sq) = self.lambda2_page_data(td) {
            let mut v = ambient_vec.to_owned();
            let coords = sq.reduce(v.as_slice_mut());

            let complement: Vec<usize> = sq.complement_pivots().collect();
            let basis_indices: Vec<usize> = (0..sq.ambient_dimension())
                .filter(|&i| sq.zeros().pivots()[i] < 0 && !complement.contains(&i))
                .collect();

            PiElement {
                bidegree,
                weight,
                coords,
                basis_indices,
            }
        } else {
            PiElement {
                bidegree,
                weight,
                coords: vec![],
                basis_indices: vec![],
            }
        }
    }

    /// Build the trigraded spectral sequence for $S/\lambda^2$.
    ///
    /// E2 has two copies of Ext at each bidegree $(n, s)$: one at Bockstein degree 0 and one at
    /// Bockstein degree 1. The $d_2$ differential maps $(n, s, 0) \to (n-1, s+2, 1)$ using the
    /// same hom\_k data as the Adams $d_2$. The E3 page (which equals $E_\infty$) gives
    /// $\pi(S/\lambda^2)$.
    fn build_lambda2_sseq(&self) -> sseq::Sseq<3, sseq::AdamsLambda2> {
        let p = self.prime();
        let res = self.alg.resolution();
        let mut sseq = sseq::Sseq::new(p);

        for b in res.iter_stem() {
            let dim = res.number_of_gens_in_bidegree(b);
            let [n, s] = b.coords();
            sseq.set_dimension(MultiDegree::new([n, s, 0]), dim);
            sseq.set_dimension(MultiDegree::new([n, s, 1]), dim);
        }

        let mut source_vec = FpVector::new(p, 0);
        let mut target_vec = FpVector::new(p, 0);

        for b in res.iter_stem() {
            let target_bidegree = b + Bidegree::n_s(-1, 2);
            if b.t() > 0 && res.has_computed_bidegree(target_bidegree) {
                let m = self.res_lift.homotopy(b.s() + 2).homotopies.hom_k(b.t());
                if m.is_empty() || m[0].is_empty() {
                    continue;
                }

                let [n, s] = b.coords();
                source_vec.set_scratch_vector_size(m.len());
                target_vec.set_scratch_vector_size(m[0].len());

                for (i, row) in m.into_iter().enumerate() {
                    source_vec.set_to_zero();
                    source_vec.set_entry(i, 1);
                    target_vec.copy_from_slice(&row);

                    let source = MultiDegreeElement::new(MultiDegree::new([n, s, 0]), source_vec);
                    sseq.add_differential(2, &source, target_vec.as_slice());

                    source_vec = source.into_vec();
                }
            }
        }

        let invalid: Vec<_> = sseq.iter_degrees().filter(|&b| sseq.invalid(b)).collect();
        for b in invalid {
            sseq.update_degree(b);
        }

        sseq
    }

    /// The trigraded spectral sequence for $S/\lambda^2$.
    pub fn lambda2_sseq(&self) -> Arc<sseq::Sseq<3, sseq::AdamsLambda2>> {
        Arc::clone(
            self.lambda2_sseq
                .lock()
                .unwrap()
                .as_ref()
                .expect("call extend_all() first"),
        )
    }

    /// The $E_3$-page subquotient at a trigraded degree `(n, s, bock)` of $S/\lambda^2$.
    /// Returns `None` if the degree is not defined in the spectral sequence.
    pub fn lambda2_page_data(&self, b: MultiDegree<3>) -> Option<Subquotient> {
        let g = self.lambda2_sseq.lock().unwrap();
        let sseq = g.as_ref().expect("call extend_all() first");
        if !sseq.defined(b) {
            return None;
        }
        let d = sseq.page_data(b);
        Some(d[std::cmp::min(3, d.len() - 1)].clone())
    }

    /// The dimension of $E_3 = E_\infty$ of $S/\lambda^2$ at the trigraded degree `(n, s, bock)`.
    /// Returns 0 if the degree is not defined.
    pub fn lambda2_e3_dimension(&self, n: i32, s: i32, bock: i32) -> usize {
        self.lambda2_page_data(MultiDegree::new([n, s, bock]))
            .map_or(0, |sq| sq.dimension())
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
    use crate::{chain_complex::ChainComplex, utils::construct_standard};

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

    #[test]
    fn test_lambda2_sseq() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(16, 6));
        let e2 = Arc::new(ExtAlgebra::new(Arc::clone(&res), res));
        let sec_e2 = SecondaryExtAlgebra::new(Arc::clone(&e2));
        sec_e2.extend_all();

        let _l2 = sec_e2.lambda2_sseq();

        // Structural check: at each bidegree (n, s),
        //   E3(n, s, 0) = ker(d2 from (n,s))  — d2-cycles at bock=0
        //   E3(n, s, 1) = coker(d2 into (n,s)) — quotient by boundaries at bock=1
        //
        // d2 maps E_{(n,s)} isomorphically to B_{(n-1, s+2)}, so:
        //   dim(Ext(n,s)) - dim(E3(n,s,0)) = dim(Ext(n-1,s+2)) - dim(E3(n-1,s+2,1))
        for b in e2.resolution().iter_stem() {
            let ext_dim = e2.dimension(b);
            let [n, s] = b.coords();

            // dim(E) at (n, s) = dim(Ext) - dim(ker d2) = ext_dim - dim(E3(n,s,0))
            let e3_bock0 = sec_e2.lambda2_e3_dimension(n, s, 0);
            let e_dim = ext_dim - e3_bock0;

            // dim(B) at (n-1, s+2) = dim(im d2) = dim(Ext(n-1,s+2)) - dim(E3(n-1,s+2,1))
            let target = b + Bidegree::n_s(-1, 2);
            if e2.resolution().has_computed_bidegree(target) {
                let target_ext_dim = e2.dimension(target);
                let [tn, ts] = target.coords();
                let e3_target_bock1 = sec_e2.lambda2_e3_dimension(tn, ts, 1);
                let b_dim = target_ext_dim - e3_target_bock1;

                assert_eq!(
                    e_dim, b_dim,
                    "dim(E at ({n},{s})) should equal dim(B at ({tn},{ts})): d2: E → B is an \
                     isomorphism"
                );
            }

            // dim(ker d2) + dim(E) = dim(Ext) always holds.
            assert_eq!(
                e3_bock0 + e_dim,
                ext_dim,
                "dim(ker d2) + dim(E) = dim(Ext) at ({n}, {s})"
            );
        }

        // h4 at (15, 1) supports d2, so E3(15, 1, 0) should be 0.
        assert_eq!(
            sec_e2.lambda2_e3_dimension(15, 1, 0),
            0,
            "h4 should not survive in E3 at bock=0"
        );

        // d2(h4) = h0*h3^2 lands at (14, 3, 1), quotienting out one generator.
        let ext_dim_14_3 = e2.dimension(Bidegree::n_s(14, 3));
        assert_eq!(
            sec_e2.lambda2_e3_dimension(14, 3, 1),
            ext_dim_14_3 - 1,
            "d2 image should quotient out one generator at (14, 3, 1)"
        );

        // h_0 at (0, 1) is a permanent cycle.
        assert_eq!(sec_e2.lambda2_e3_dimension(0, 1, 0), 1);
        assert_eq!(sec_e2.lambda2_e3_dimension(0, 1, 1), 1);
    }

    #[test]
    fn test_pi_types() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(16, 6));
        let e2 = Arc::new(ExtAlgebra::new(Arc::clone(&res), res));
        let sec_e2 = SecondaryExtAlgebra::new(Arc::clone(&e2));
        sec_e2.extend_all();

        // h0 at (0, 1) is a permanent Z-cycle: appears at both weights.
        let h0_bze = sec_e2.adams_classify(BidegreeGenerator::new(Bidegree::n_s(0, 1), 0));
        assert_eq!(h0_bze, BZE::Z);
        let pi_01 = sec_e2.pi_basis(Bidegree::n_s(0, 1));
        assert_eq!(pi_01.len(), 2); // weight 0 + weight 1
        assert_eq!(pi_01[0].weight(), Weight::Ext);
        assert_eq!(pi_01[0].bze(), BZE::Z);
        assert_eq!(pi_01[1].weight(), Weight::Lambda);
        assert_eq!(pi_01[1].bze(), BZE::Z);

        // h4 at (15, 1) supports d2: classified as E, appears at weight 1 only.
        let h4_bze = sec_e2.adams_classify(BidegreeGenerator::new(Bidegree::n_s(15, 1), 0));
        assert_eq!(h4_bze, BZE::E);
        let pi_15_1 = sec_e2.pi_basis(Bidegree::n_s(15, 1));
        assert_eq!(pi_15_1.len(), 1);
        assert_eq!(pi_15_1[0].weight(), Weight::Lambda);
        assert_eq!(pi_15_1[0].bze(), BZE::E);

        // h0*h3^2 at (14, 3) is a d2-boundary: classified as B, appears at weight 0 only.
        let b14_3 = sec_e2.adams_classify(BidegreeGenerator::new(Bidegree::n_s(14, 3), 0));
        assert_eq!(b14_3, BZE::B);
        let pi_14_3 = sec_e2.pi_basis(Bidegree::n_s(14, 3));
        assert_eq!(pi_14_3.len(), 1);
        assert_eq!(pi_14_3[0].weight(), Weight::Ext);
        assert_eq!(pi_14_3[0].bze(), BZE::B);

        // to_pi: a zero secondary element projects to zero PiElements.
        let zero_elt = sec_e2.element(SecondaryDegree::new(Bidegree::n_s(0, 1)), &[0], &[0]);
        let (pi_ext, pi_lambda) = sec_e2.to_pi(&zero_elt);
        assert!(pi_ext.is_zero());
        assert!(pi_lambda.is_zero());

        // to_pi: h0 as a unit vector at weight 0 should project to a nonzero PiElement.
        let h0_elt = sec_e2.element(SecondaryDegree::new(Bidegree::n_s(0, 1)), &[1], &[0]);
        let (pi_ext, _pi_lambda) = sec_e2.to_pi(&h0_elt);
        assert!(!pi_ext.is_zero());
        assert_eq!(pi_ext.weight(), Weight::Ext);

        // Display: PiGenerator and PiElement produce meaningful output.
        assert!(!format!("{}", pi_01[0]).is_empty());
        assert!(!format!("{pi_ext}").is_empty());
    }
}
