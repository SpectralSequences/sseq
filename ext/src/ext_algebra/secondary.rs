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
use fp::{
    matrix::{Matrix, Subquotient, Subspace},
    prime::Prime,
    vector::FpVector,
};
use itertools::Itertools;
use sseq::coordinates::{Bidegree, BidegreeElement};

use super::{ExtAlgebra, ExtModule};
use crate::{
    chain_complex::{ChainHomotopy, FreeChainComplex},
    resolution_homomorphism::ResolutionHomomorphism,
    secondary::{
        LAMBDA_BIDEGREE, SecondaryChainHomotopy, SecondaryLift, SecondaryResolution,
        SecondaryResolutionHomomorphism,
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

/// A class in $\Mod_{C\lambda^2}$: an $\Ext$ part at `degree` together with a $\lambda$ part at
/// `degree + LAMBDA_BIDEGREE`. Secondary Massey products need classes that are *not* standard lifts
/// of $\Ext$ classes, i.e. carry a chosen $\lambda$ part; see
/// [`SecondaryExtModule::secondary_massey`].
#[derive(Clone)]
pub struct SecondaryClass {
    degree: Bidegree,
    /// Coordinates of the $\Ext$ part, in the generator basis at `degree`.
    ext: FpVector,
    /// Coordinates of the $\lambda$ part, in the generator basis at `degree + LAMBDA_BIDEGREE`
    /// (empty when there is no $\lambda$ part).
    lambda: FpVector,
}

impl SecondaryClass {
    /// A class with the given $\Ext$ and $\lambda$ coordinates.
    pub fn new(degree: Bidegree, ext: FpVector, lambda: FpVector) -> Self {
        Self {
            degree,
            ext,
            lambda,
        }
    }

    /// A class with only an $\Ext$ part (no $\lambda$).
    pub fn ext_only(degree: Bidegree, ext: FpVector) -> Self {
        let lambda = FpVector::new(ext.prime(), 0);
        Self {
            degree,
            ext,
            lambda,
        }
    }

    /// The $\Ext$-part bidegree of the class.
    pub fn degree(&self) -> Bidegree {
        self.degree
    }

    /// The full coordinate vector: the $\Ext$ part followed by the $\lambda$ part.
    fn full(&self) -> FpVector {
        let mut v = FpVector::new(self.ext.prime(), self.ext.len() + self.lambda.len());
        v.slice_mut(0, self.ext.len()).assign(self.ext.as_slice());
        v.slice_mut(self.ext.len(), self.ext.len() + self.lambda.len())
            .assign(self.lambda.as_slice());
        v
    }
}

/// A single secondary Massey bracket `⟨-, b, a⟩` in $\Mod_{C\lambda^2}$, up to a sign. See
/// [`SecondaryExtModule::secondary_massey`].
pub struct SecondaryMasseyResult {
    /// The bidegree of the first factor `-` (the multiplicand).
    pub degree: Bidegree,
    /// The $\Ext$ part of the first factor `-`, in the generator basis at `degree`.
    pub multiplicand: FpVector,
    /// The $\lambda$ part of the first factor `-`, at `degree + LAMBDA_BIDEGREE`.
    pub multiplicand_lambda: FpVector,
    /// The $\Ext$ part of the bracket value (up to a sign).
    pub ext_part: FpVector,
    /// The $\lambda$ part of the bracket value (up to a sign).
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

    /// The $E_3$ page of `k`, or `None` before [`extend`](Self::extend) has run.
    pub(crate) fn sseq(&self) -> Option<Arc<sseq::Sseq<2, sseq::Adams>>> {
        self.sseq.lock().unwrap().clone()
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

    /// The secondary Massey products $\langle -, b, a\rangle$ in $\Mod_{C\lambda^2}$, computed up to
    /// a sign, for every valid first factor `-`.
    ///
    /// `a ∈ Ext(M, k)` (the module side, from `self`) and `b, - ∈ Ext(k, k)` (the ring side, from
    /// [`self.algebra()`](Self::algebra)); both `a` and `b` are [`SecondaryClass`]es, i.e. may carry
    /// a chosen $\lambda$ part. The caller must have run [`extend_all`](Self::extend_all) and
    /// resolved far enough, and is responsible for ensuring `a · b = 0` (it is not verified).
    ///
    /// When `job` is `Some(s)` this instead computes only the secondary chain-homotopy data for
    /// filtration `s` (sharding, see the `secondary` example docs) and returns an empty vector.
    ///
    /// This encapsulates the plumbing the `secondary_massey` example used to hand-roll; it is the
    /// $\Mod_{C\lambda^2}$ analogue of [`ExtModule::massey_iter_a`](super::ExtModule::massey_iter_a).
    pub fn secondary_massey(
        &self,
        a: &SecondaryClass,
        b: &SecondaryClass,
        job: Option<i32>,
    ) -> Vec<SecondaryMasseyResult> {
        let p = self.prime();
        let resolution = self.module.resolution();
        let unit = self.module.algebra().resolution();

        // `a ∈ Ext(M, k)`: source = `M`'s secondary resolution, target = `k`'s. `b ∈ Ext(k, k)`:
        // source = target = `k`'s secondary resolution (from the shared ring layer).
        let (a_lift, a_lambda) = build_secondary_hom(a, &self.lift, self.algebra.lift(), "a");
        let (b_lift, b_lambda) =
            build_secondary_hom(b, self.algebra.lift(), self.algebra.lift(), "b");
        let b_class = b.full();

        let shift = Bidegree::s_t(
            (a_lift.underlying().shift + b_lift.underlying().shift).s(),
            (a_lift.shift() + b_lift.shift()).t(),
        );

        a_lift.underlying().extend_all();
        a_lift.extend_all();
        b_lift.underlying().extend_all();
        b_lift.extend_all();
        if let Some(al) = &a_lambda {
            al.extend_all();
        }
        if let Some(bl) = &b_lambda {
            bl.extend_all();
        }

        let res_sseq = self.sseq().expect("call extend_all() first");
        let unit_sseq = self.algebra.sseq().expect("call extend_all() first");

        let b_shift = b_lift.underlying().shift;

        let chain_homotopy = Arc::new(ChainHomotopy::new(a_lift.underlying(), b_lift.underlying()));
        chain_homotopy.initialize_homotopies((b_shift + a_lift.underlying().shift).s());

        // The first homotopy of the composite `b ∘ a`.
        {
            let v = a_lift.product_nullhomotopy(
                a_lambda.as_deref(),
                &res_sseq,
                b_shift,
                b_class.as_slice(),
            );
            let homotopy = chain_homotopy.homotopy(b_shift.s() + a_lift.underlying().shift.s() - 1);
            let htpy_source = a_lift.shift() + b_shift;
            homotopy.extend_by_zero(htpy_source.t() - 1);
            homotopy.add_generators_from_rows(
                htpy_source.t(),
                v.into_iter()
                    .map(|x| FpVector::from_slice(p, &[x]))
                    .collect(),
            );
        }
        chain_homotopy.extend_all();

        let ch_lift = SecondaryChainHomotopy::new(
            Arc::clone(&a_lift),
            Arc::clone(&b_lift),
            a_lambda.clone(),
            b_lambda.clone(),
            Arc::clone(&chain_homotopy),
        );

        if let Some(s) = job {
            ch_lift.compute_partial(s);
            return Vec::new();
        }
        ch_lift.extend_all();

        let h_0 = ch_lift.algebra().p_tilde();
        let mut results = Vec::new();
        let mut scratch0: Vec<u32> = Vec::new();
        let mut scratch1 = FpVector::new(p, 0);

        // Iterate through the multiplicand `-`.
        for c in unit.iter_stem() {
            if !resolution.has_computed_bidegree(c + shift - Bidegree::s_t(2, 0))
                || !resolution.has_computed_bidegree(c + shift + Bidegree::s_t(0, 1))
            {
                continue;
            }

            let source = c + shift - Bidegree::s_t(1, 0);

            let source_num_gens = resolution.number_of_gens_in_bidegree(source);
            let source_lambda_num_gens =
                resolution.number_of_gens_in_bidegree(source + LAMBDA_BIDEGREE);

            if source_num_gens + source_lambda_num_gens == 0 {
                continue;
            }

            // We find the kernel of multiplication by `b`.
            let target_num_gens = unit.number_of_gens_in_bidegree(c);
            let target_lambda_num_gens = unit.number_of_gens_in_bidegree(c + LAMBDA_BIDEGREE);
            let target_all_gens = target_num_gens + target_lambda_num_gens;

            let prod_num_gens = unit.number_of_gens_in_bidegree(c + b_shift);
            let prod_lambda_num_gens =
                unit.number_of_gens_in_bidegree(c + b_shift + LAMBDA_BIDEGREE);
            let prod_all_gens = prod_num_gens + prod_lambda_num_gens;

            let e3_kernel = {
                let target_page_data = e3_page_data_at(&unit_sseq, c);
                let target_lambda_page_data = e3_page_data_at(&unit_sseq, c + LAMBDA_BIDEGREE);
                let product_lambda_page_data =
                    e3_page_data_at(&unit_sseq, c + b_shift + LAMBDA_BIDEGREE);

                // We first compute elements whose product vanishes mod lambda, and later see what
                // the possible lifts are. We do it this way to avoid Z/p^2 problems.
                let e2_kernel: Subspace = {
                    let mut product_matrix = Matrix::new(
                        p,
                        target_page_data.subspace_dimension(),
                        target_num_gens + prod_num_gens,
                    );

                    let m0 = Matrix::from_vec(
                        p,
                        &b_lift
                            .underlying()
                            .get_map(c.s() + b_lift.underlying().shift.s())
                            .hom_k(c.t()),
                    );
                    for (g, mut out) in target_page_data
                        .subspace_gens()
                        .zip_eq(product_matrix.iter_mut())
                    {
                        out.slice_mut(prod_num_gens, prod_num_gens + target_num_gens)
                            .add(g, 1);
                        for (i, v) in g.iter_nonzero() {
                            out.slice_mut(0, prod_num_gens).add(m0.row(i), v);
                        }
                    }
                    product_matrix.row_reduce();
                    product_matrix.compute_kernel(prod_num_gens)
                };

                // Now compute the E3 kernel.
                {
                    // First add the lifts from Ext.
                    let e2_ker_dim = e2_kernel.dimension();
                    let mut product_matrix = Matrix::new(
                        p,
                        e2_ker_dim + target_lambda_page_data.quotient_dimension(),
                        target_all_gens + prod_all_gens,
                    );

                    b_lift.hom_k_with(
                        b_lambda.as_deref(),
                        Some(&unit_sseq),
                        c,
                        e2_kernel.basis(),
                        product_matrix
                            .slice_mut(0, e2_ker_dim, 0, prod_all_gens)
                            .iter_mut(),
                    );
                    for (v, mut t) in e2_kernel.basis().zip(product_matrix.iter_mut()) {
                        t.slice_mut(prod_all_gens, prod_all_gens + target_num_gens)
                            .assign(v);
                    }

                    // Now add the lambda multiples.
                    let m = Matrix::from_vec(
                        p,
                        &b_lift
                            .underlying()
                            .get_map(b_shift.s() + c.s() + 1)
                            .hom_k(c.t() + 1),
                    );

                    let mut count = 0;
                    for (i, &v) in target_lambda_page_data.quotient_pivots().iter().enumerate() {
                        if v >= 0 {
                            continue;
                        }
                        let mut row = product_matrix.row_mut(e2_ker_dim + count as usize);
                        row.add_basis_element(prod_all_gens + target_num_gens + i, 1);
                        row.slice_mut(prod_num_gens, prod_all_gens).add(m.row(i), 1);
                        product_lambda_page_data
                            .reduce_by_quotient(row.slice_mut(prod_num_gens, prod_all_gens));
                        count += 1;
                    }

                    product_matrix.row_reduce();
                    product_matrix.compute_kernel(prod_all_gens)
                }
            };

            if e3_kernel.dimension() == 0 {
                continue;
            }

            let m0 = chain_homotopy.homotopy(source.s()).hom_k(c.t());
            let mt = Matrix::from_vec(p, &chain_homotopy.homotopy(source.s() + 1).hom_k(c.t() + 1));
            let m1 = Matrix::from_vec(
                p,
                &ch_lift.homotopies()[source.s() + 1].homotopies.hom_k(c.t()),
            );
            let mp = Matrix::from_vec(
                p,
                &resolution
                    .filtration_one_product(1, h_0, Bidegree::s_t(source.s(), c.t() + shift.t()))
                    .unwrap(),
            );
            let ma = a_lift
                .underlying()
                .get_map(source.s())
                .hom_k(c.t() + b_shift.t());
            let mb = b_lift
                .underlying()
                .get_map(c.s() + b_shift.s())
                .hom_k(c.t());

            for g in e3_kernel.iter() {
                scratch0.clear();
                scratch0.resize(source_num_gens, 0);
                scratch1.set_scratch_vector_size(source_lambda_num_gens);

                // First deal with the null-homotopy of `ab`.
                for (i, v) in g.restrict(0, target_num_gens).iter_nonzero() {
                    scratch0
                        .iter_mut()
                        .zip_eq(&m0[i])
                        .for_each(|(a, b)| *a += v * b);
                    scratch1.as_slice_mut().add(m1.row(i), v);
                }
                for (i, v) in g.restrict(target_num_gens, target_all_gens).iter_nonzero() {
                    scratch1.as_slice_mut().add(mt.row(i), v);
                }
                // Now do the -1 part of the null-homotopy of `bc`.
                {
                    let sign = p * p - 1;
                    let out = b_lift.product_nullhomotopy(b_lambda.as_deref(), &unit_sseq, c, g);
                    for (i, v) in out.iter_nonzero() {
                        scratch0
                            .iter_mut()
                            .zip_eq(&ma[i])
                            .for_each(|(a, b)| *a += v * b * sign);
                    }
                }
                for (i, v) in scratch0.iter().enumerate() {
                    let extra = *v / p;
                    scratch1.as_slice_mut().add(mp.row(i), extra % p);
                }

                // The Ext part of the bracket, before `scratch0` is reused below.
                let ext_part =
                    FpVector::from_slice(p, &scratch0.iter().map(|x| *x % p).collect::<Vec<_>>());
                let multiplicand = g.restrict(0, target_num_gens).to_owned();
                let multiplicand_lambda = g.restrict(target_num_gens, target_all_gens).to_owned();

                // Then deal with the rest of the null-homotopy of `bc`. This is just the
                // null-homotopy of 2.
                scratch0.clear();
                scratch0.resize(prod_num_gens, 0);

                for (i, v) in g.restrict(0, target_num_gens).iter_nonzero() {
                    scratch0
                        .iter_mut()
                        .zip_eq(&mb[i])
                        .for_each(|(a, b)| *a += v * b);
                }
                for (i, v) in scratch0.iter().enumerate() {
                    let extra = (*v / p) % p;
                    if extra == 0 {
                        continue;
                    }
                    for gen_idx in 0..source_lambda_num_gens {
                        let m = a_lift.underlying().get_map((source + LAMBDA_BIDEGREE).s());
                        let dx = m.output((source + LAMBDA_BIDEGREE).t(), gen_idx);
                        let idx = unit.module((c + shift).s()).operation_generator_to_index(
                            1,
                            h_0,
                            (c + shift).t(),
                            i,
                        );
                        scratch1.add_basis_element(gen_idx, dx.entry(idx));
                    }
                }

                results.push(SecondaryMasseyResult {
                    degree: c,
                    multiplicand,
                    multiplicand_lambda,
                    ext_part,
                    lambda_part: scratch1.clone(),
                });
            }
        }
        results
    }
}

/// The $E_3$-page subquotient a spectral sequence records at bidegree `b`.
fn e3_page_data(sseq: &sseq::Sseq<2, sseq::Adams>, b: Bidegree) -> &Subquotient {
    let d = sseq.page_data(b);
    &d[std::cmp::min(3, d.len() - 1)]
}

/// The $E_3$-page subquotient a spectral sequence records at bidegree `b`, owned copy for use where
/// a `&`-borrow of the locked page would not live long enough.
fn e3_page_data_at(sseq: &sseq::Sseq<2, sseq::Adams>, b: Bidegree) -> Subquotient {
    e3_page_data(sseq, b).clone()
}

/// Build the secondary lift of a $\Mod_{C\lambda^2}$ class from `source`'s resolution to `target`'s,
/// together with its optional $\lambda$-part chain map. This generalises the `secondary_massey`
/// example's `get_hom`: it takes explicit coordinates instead of querying the user.
fn build_secondary_hom<CC>(
    class: &SecondaryClass,
    source: &Arc<SecondaryResolution<CC>>,
    target: &Arc<SecondaryResolution<CC>>,
    name: &str,
) -> (
    Arc<SecondaryResolutionHomomorphism<CC, CC>>,
    Option<Arc<ResolutionHomomorphism<CC, CC>>>,
)
where
    CC: FreeChainComplex + crate::chain_complex::AugmentedChainComplex,
    CC::Algebra: PairAlgebra,
{
    let p = source.prime();
    let shift = class.degree;

    source
        .underlying()
        .compute_through_bidegree(shift + LAMBDA_BIDEGREE);

    let hom = Arc::new(ResolutionHomomorphism::new(
        name.to_owned(),
        source.underlying(),
        target.underlying(),
        shift,
    ));

    let num_gens = source.underlying().number_of_gens_in_bidegree(shift);
    let mut matrix = Matrix::new(p, num_gens, 1);
    for (i, x) in class.ext.iter().enumerate() {
        matrix.row_mut(i).set_entry(0, x);
    }
    hom.extend_step(shift, Some(&matrix));

    let hom_lift = Arc::new(SecondaryResolutionHomomorphism::new(
        Arc::clone(source),
        Arc::clone(target),
        hom,
    ));

    let lambda_part = if !class.lambda.is_zero() {
        let coords: Vec<u32> = class.lambda.iter().collect();
        Some(Arc::new(ResolutionHomomorphism::from_class(
            format!("λ{name}"),
            hom_lift.source(),
            hom_lift.target(),
            shift + LAMBDA_BIDEGREE,
            &coords,
        )))
    } else {
        None
    };

    (hom_lift, lambda_part)
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
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

    /// Format a bracket exactly as the `secondary_massey` example prints it, so the golden strings
    /// below double as documentation of the expected output.
    fn format_bracket(a_name: &str, b_name: &str, r: &SecondaryMasseyResult) -> String {
        let mut s = format!("<{a_name}, {b_name}, ");
        let has_ext = !r.multiplicand.is_zero();
        if has_ext {
            s += &format!(
                "[{}]",
                BidegreeElement::new(r.degree, r.multiplicand.clone()).to_basis_string()
            );
        }
        let num_lambda = r.multiplicand_lambda.iter_nonzero().count();
        if num_lambda > 0 {
            if has_ext {
                s += " + ";
            }
            s += "λ";
            let basis =
                BidegreeElement::new(r.degree + LAMBDA_BIDEGREE, r.multiplicand_lambda.clone())
                    .to_basis_string();
            s += &if num_lambda == 1 {
                basis
            } else {
                format!("({basis})")
            };
        }
        s += &format!(
            "> = ±[{}] + λ{}",
            r.ext_part.iter().format(", "),
            r.lambda_part
        );
        s
    }

    /// Regression: `<-, h_0, h_1>` on `S_2` (i.e. `a = h_1 ∈ Ext(M, k)`, `b = h_0 ∈ Ext(k, k)`,
    /// `M == k`) must reproduce the exact family the (pre-refactor) `secondary_massey` example
    /// printed. This pins the `secondary_massey` method — the delicate Z/p² + λ read-off — against a
    /// known-good baseline captured from the hand-rolled version.
    #[test]
    fn test_sphere_secondary_massey() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(12, 7));
        let module = Arc::new(ExtModule::intrinsic(res));
        let sec = SecondaryExtModule::from_module(Arc::clone(&module));
        sec.extend_all();

        let p = module.prime();
        // h_1 = (n=1, s=1), h_0 = (n=0, s=1); h_0 · h_1 = 0, so the bracket is defined.
        let a = SecondaryClass::ext_only(Bidegree::n_s(1, 1), FpVector::from_slice(p, &[1]));
        let b = SecondaryClass::ext_only(Bidegree::n_s(0, 1), FpVector::from_slice(p, &[1]));

        let results = sec.secondary_massey(&a, &b, None);
        let mut got: Vec<String> = results
            .iter()
            .map(|r| format_bracket("[h1]", "[h0]", r))
            .collect();
        got.sort();

        let mut expected: Vec<String> = [
            "<[h1], [h0], λx_(1, 1, 0)> = ±[0] + λ[1]",
            "<[h1], [h0], [x_(1, 1, 0)]> = ±[1] + λ[1]",
            "<[h1], [h0], λx_(6, 2, 0)> = ±[0] + λ[1]",
            "<[h1], [h0], [x_(6, 2, 0)]> = ±[1] + λ[]",
            "<[h1], [h0], λx_(7, 4, 0)> = ±[0] + λ[1]",
            "<[h1], [h0], [x_(7, 4, 0)]> = ±[1] + λ[]",
            "<[h1], [h0], [x_(9, 3, 0)]> = ±[] + λ[0]",
            "<[h1], [h0], λx_(9, 4, 0)> = ±[] + λ[0]",
            "<[h1], [h0], [x_(9, 4, 0)]> = ±[0] + λ[1]",
            "<[h1], [h0], λx_(9, 5, 0)> = ±[0] + λ[1]",
            "<[h1], [h0], [x_(9, 5, 0)]> = ±[1] + λ[1]",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        expected.sort();

        assert_eq!(got, expected);
    }
}
