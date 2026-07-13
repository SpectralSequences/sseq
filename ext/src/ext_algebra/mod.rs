//! Ext as a bigraded algebra and its modules.
//!
//! This splits the two objects that a resolution computes:
//!
//! - [`ExtAlgebra`] is the **ring** $\Ext(k, k)$, backed by a resolution of the base field `k`. It
//!   owns the ring-product cache (multiplication maps among $\Ext(k, k)$ generators, `res(k) →
//!   res(k)`), and in particular is the single home for the multiply-by-a-class maps that Massey
//!   products need (see [`ExtAlgebra::class_product_map`]).
//! - [`ExtModule`] is a **module** $\Ext(M, k)$ over that ring, backed by a resolution of `M`. It
//!   holds a shared [`Arc`] to the [`ExtAlgebra`] (so every module over the same `k` shares one ring
//!   cache) and its own module-action cache (`M`'s Ext-generators acted on by ring elements, `res(M)
//!   → res(k)`). When `M == k` the module shares its resolution with the ring, so "a module over
//!   itself" is just an [`ExtModule`] whose resolution is `Arc`-equal to the ring's (see
//!   [`ExtModule::is_unit`]); there is no `is_unit` special-casing baked into the ring.
//!
//! [`ExtAlgebra`] implements the [`algebra::Algebra`] trait (as `Algebra<2>`, i.e. bigraded) and
//! [`ExtModule`] implements [`algebra::module::Module`] (as `Module<2>`) with `Algebra =
//! ExtAlgebra`. The product/action trait methods are *total* (they panic if the relevant bidegree
//! has not been resolved — call [`ExtAlgebra::compute_through_bidegree`] /
//! [`ExtModule::compute_through_bidegree`] first). The inherent `multiply_into` / `try_multiply`
//! helpers return [`Option`] instead, for the "maybe out of computed range" ergonomics the examples
//! rely on.
//!
//! # Conventions
//! A product is realised by a [`ResolutionHomomorphism`] built from a fixed multiplier class. For a
//! module product the multiplier lives in $\Ext(M, k)$ (source = resolution of `M`, target =
//! resolution of `k`); for a ring product it lives in $\Ext(k, k)$ (source = target = resolution of
//! `k`). That single chain map computes the products of the multiplier with *all* classes of
//! $\Ext(k, k)$. We cache one such map per *generator* (keyed by [`BidegreeGenerator`]); a product
//! by a general class is assembled at request time from the generator maps. Products are computed
//! up to a sign (as `y · x` where convenient), matching the existing example scripts.
//!
//! The secondary differential ($d_2$) and the $\Mod_{C\lambda^2}$ secondary product live in the
//! [`secondary`] submodule ([`SecondaryExtAlgebra`]).

pub mod massey;
pub mod secondary;

use std::sync::Arc;

use dashmap::DashMap;
use fp::{matrix::Matrix, prime::ValidPrime, vector::FpVector};
use sseq::coordinates::{Bidegree, BidegreeElement, BidegreeGenerator};

pub use self::secondary::{SecondaryExtAlgebra, SecondaryProduct};
use crate::{
    chain_complex::{AugmentedChainComplex, FreeChainComplex},
    resolution_homomorphism::ResolutionHomomorphism,
    utils::{QueryModuleResolution, get_unit},
};

/// The ring $\Ext(k, k)$, backed by a resolution of the base field `k`.
///
/// See the [module-level documentation](self) for how this relates to [`ExtModule`].
pub struct ExtAlgebra<CC: FreeChainComplex> {
    /// Resolution of the base field `k`. Ring products live here.
    resolution: Arc<CC>,
    /// One multiplication map per generator of $\Ext(k, k)$, `res(k) → res(k)`, built on demand.
    products: DashMap<BidegreeGenerator, Arc<ResolutionHomomorphism<CC, CC>>>,
}

impl<CC: FreeChainComplex> ExtAlgebra<CC> {
    /// Build the ring $\Ext(k, k)$ from a resolution of `k`.
    pub fn new(resolution: Arc<CC>) -> Self {
        Self {
            resolution,
            products: DashMap::new(),
        }
    }

    /// The resolution of `k` backing this ring.
    pub fn resolution(&self) -> &Arc<CC> {
        &self.resolution
    }

    pub fn prime(&self) -> ValidPrime {
        self.resolution.prime()
    }

    /// Ensure the resolution is computed through the given bidegree.
    pub fn compute_through_bidegree(&self, b: Bidegree) {
        self.resolution.compute_through_bidegree(b);
    }

    /// The dimension of $\Ext^{s,t}(k, k)$ at the given bidegree.
    pub fn dimension(&self, b: Bidegree) -> usize {
        self.resolution.number_of_gens_in_bidegree(b)
    }

    /// The basis generators of $\Ext(k, k)$ at the given bidegree.
    pub fn basis(&self, b: Bidegree) -> Vec<BidegreeGenerator> {
        (0..self.dimension(b))
            .map(|i| BidegreeGenerator::new(b, i))
            .collect()
    }

    /// A class in $\Ext(k, k)$ from its coordinates in the generator basis at bidegree `b`.
    pub fn element(&self, b: Bidegree, coords: &[u32]) -> BidegreeElement {
        assert_eq!(self.dimension(b), coords.len());
        BidegreeElement::new(b, FpVector::from_slice(self.prime(), coords))
    }

    /// A single generator of $\Ext(k, k)$ as a class.
    pub fn generator(&self, g: BidegreeGenerator) -> BidegreeElement {
        let ambient = self.dimension(g.degree());
        assert!(ambient > g.idx());
        g.into_element(self.prime(), ambient)
    }
}

impl<CC> ExtAlgebra<CC>
where
    CC: FreeChainComplex + AugmentedChainComplex,
{
    /// The multiplication map for a single generator `g` of $\Ext(k, k)$ (`res(k) → res(k)`), built
    /// and cached on first use. The returned map is *not* guaranteed to be extended.
    pub fn generator_product_map(
        &self,
        g: BidegreeGenerator,
    ) -> Arc<ResolutionHomomorphism<CC, CC>> {
        cached_generator_product_map(&self.products, &self.resolution, &self.resolution, g)
    }

    /// The multiply-by-`x` chain self-map of `res(k)` (`res(k) → res(k)`), extended through `max`.
    ///
    /// This is the single home for the ring-side multiplication maps that Massey products need
    /// (`massey_b_hom`). For a single generator it returns the cached
    /// [`generator_product_map`](Self::generator_product_map); for a general class it realises the
    /// class directly via [`ResolutionHomomorphism::from_class`].
    pub fn class_product_map(
        &self,
        x: &BidegreeElement,
        max: Bidegree,
    ) -> Arc<ResolutionHomomorphism<CC, CC>> {
        let nonzero: Vec<(usize, u32)> = x.vec().iter_nonzero().collect();
        if let [(idx, 1)] = nonzero[..] {
            let map = self.generator_product_map(BidegreeGenerator::new(x.degree(), idx));
            map.extend_through_stem(max);
            return map;
        }
        let coords: Vec<u32> = x.vec().iter().collect();
        let hom = Arc::new(ResolutionHomomorphism::from_class(
            String::new(),
            Arc::clone(&self.resolution),
            Arc::clone(&self.resolution),
            x.degree(),
            &coords,
        ));
        hom.extend_through_stem(max);
        hom
    }

    /// Left-multiplication by `x ∈ Ext(k, k)`, applied to every basis generator of $\Ext(k, k)$ at
    /// bidegree `b`. See [`ExtModule::multiply_into`] for the return convention.
    pub fn multiply_into(&self, x: &BidegreeElement, b: Bidegree) -> Option<Matrix> {
        products_into(
            &self.resolution,
            &self.resolution,
            &self.products,
            self.prime(),
            x,
            b,
        )
    }

    /// The ring product `x · y` (both in $\Ext(k, k)$) if it lies in the computed range, else
    /// `None`. The result lies in bidegree `x.degree() + y.degree()`.
    pub fn try_multiply(
        &self,
        x: &BidegreeElement,
        y: &BidegreeElement,
    ) -> Option<BidegreeElement> {
        let matrix = self.multiply_into(x, y.degree())?;
        Some(combine_product(&matrix, y, x.degree() + y.degree(), self.prime()))
    }

    /// The ring product `x · y`, both in $\Ext(k, k)$. Panics if out of the computed range; use
    /// [`try_multiply`](Self::try_multiply) to handle that case.
    pub fn multiply(&self, x: &BidegreeElement, y: &BidegreeElement) -> BidegreeElement {
        self.try_multiply(x, y).expect(
            "multiply: product is out of the computed range; compute further or use try_multiply",
        )
    }
}

/// The module $\Ext(M, k)$ over the ring [`ExtAlgebra`] $\Ext(k, k)$, backed by a resolution of
/// `M`.
///
/// See the [module-level documentation](self) for conventions.
pub struct ExtModule<CC: FreeChainComplex> {
    /// Resolution of `M`; the module's classes and the module-action products land in its Ext.
    resolution: Arc<CC>,
    /// Shared handle to the ring $\Ext(k, k)$. `Arc`-shared so all modules over the same `k` reuse
    /// one ring cache.
    algebra: Arc<ExtAlgebra<CC>>,
    /// One multiplication map per generator of $\Ext(M, k)$, `res(M) → res(k)`, built on demand.
    products: DashMap<BidegreeGenerator, Arc<ResolutionHomomorphism<CC, CC>>>,
}

impl ExtAlgebra<QueryModuleResolution> {
    /// Ensure the resolution of `k` is computed through the given stem.
    pub fn compute_through_stem(&self, max: Bidegree) {
        self.resolution.compute_through_stem(max);
    }
}

impl ExtModule<QueryModuleResolution> {
    /// Build an [`ExtModule`] from a resolution of `M`, deriving the unit `k` via [`get_unit`].
    ///
    /// This may prompt for the unit's save directory when `M != k` (see [`get_unit`]); for a fully
    /// non-interactive setup, use [`ExtModule::new`] with an explicit ring instead.
    pub fn from_resolution(resolution: Arc<QueryModuleResolution>) -> anyhow::Result<Self> {
        let (_, unit) = get_unit(Arc::clone(&resolution))?;
        Ok(Self::new(resolution, Arc::new(ExtAlgebra::new(unit))))
    }

    /// Ensure both the module's resolution and the ring's resolution are computed through the given
    /// stem.
    pub fn compute_through_stem(&self, max: Bidegree) {
        self.algebra.compute_through_stem(max);
        if !self.is_unit() {
            self.resolution.compute_through_stem(max);
        }
    }
}

impl<CC: FreeChainComplex> ExtModule<CC> {
    /// Build $\Ext(M, k)$ from a resolution of `M` and the ring $\Ext(k, k)$.
    pub fn new(resolution: Arc<CC>, algebra: Arc<ExtAlgebra<CC>>) -> Self {
        assert_eq!(resolution.prime(), algebra.prime());
        Self {
            resolution,
            algebra,
            products: DashMap::new(),
        }
    }

    /// Build the module `M == k`, i.e. $\Ext(k, k)$ as a module over itself, sharing one resolution
    /// (and hence one ring cache) between the module and its ring.
    pub fn over_unit(algebra: Arc<ExtAlgebra<CC>>) -> Self {
        let resolution = Arc::clone(algebra.resolution());
        Self::new(resolution, algebra)
    }

    /// Build a module for resolution-*intrinsic* operations that do not involve the unit (notably
    /// the secondary `d2` differential), using the resolution itself as its own `k`.
    ///
    /// This avoids the unit-resolution setup (and any associated prompt) that
    /// [`from_resolution`](Self::from_resolution) performs. The product/action methods and the
    /// ring-side queries are only meaningful here when `M == k`; for products with `M != k`, build
    /// with [`from_resolution`](Self::from_resolution) or [`new`](Self::new) instead.
    pub fn intrinsic(resolution: Arc<CC>) -> Self {
        let algebra = Arc::new(ExtAlgebra::new(Arc::clone(&resolution)));
        Self::new(resolution, algebra)
    }

    /// The resolution of `M` backing this module.
    pub fn resolution(&self) -> &Arc<CC> {
        &self.resolution
    }

    /// The ring $\Ext(k, k)$ this is a module over.
    pub fn algebra(&self) -> &Arc<ExtAlgebra<CC>> {
        &self.algebra
    }

    /// Whether `M == k`, i.e. the module shares its resolution with its ring. This is the structural
    /// replacement for the old `is_unit` flag.
    pub fn is_unit(&self) -> bool {
        Arc::ptr_eq(&self.resolution, self.algebra.resolution())
    }

    pub fn prime(&self) -> ValidPrime {
        self.resolution.prime()
    }

    /// Ensure both the module's resolution and the ring's resolution are computed through the given
    /// bidegree.
    pub fn compute_through_bidegree(&self, b: Bidegree) {
        self.algebra.compute_through_bidegree(b);
        if !self.is_unit() {
            self.resolution.compute_through_bidegree(b);
        }
    }

    /// The dimension of $\Ext^{s,t}(M, k)$ at the given bidegree.
    pub fn dimension(&self, b: Bidegree) -> usize {
        self.resolution.number_of_gens_in_bidegree(b)
    }

    /// The basis generators of $\Ext(M, k)$ at the given bidegree.
    pub fn basis(&self, b: Bidegree) -> Vec<BidegreeGenerator> {
        (0..self.dimension(b))
            .map(|i| BidegreeGenerator::new(b, i))
            .collect()
    }

    /// A class in $\Ext(M, k)$ from its coordinates in the generator basis at bidegree `b`.
    pub fn element(&self, b: Bidegree, coords: &[u32]) -> BidegreeElement {
        assert_eq!(self.dimension(b), coords.len());
        BidegreeElement::new(b, FpVector::from_slice(self.prime(), coords))
    }

    /// A single generator of $\Ext(M, k)$ as a class.
    pub fn generator(&self, g: BidegreeGenerator) -> BidegreeElement {
        let ambient = self.dimension(g.degree());
        assert!(ambient > g.idx());
        g.into_element(self.prime(), ambient)
    }
}

impl<CC> ExtModule<CC>
where
    CC: FreeChainComplex + AugmentedChainComplex,
{
    /// The multiplication map for a single generator `g` of $\Ext(M, k)$ (`res(M) → res(k)`), built
    /// and cached on first use. The returned map is *not* guaranteed to be extended;
    /// [`multiply_into`](Self::multiply_into) extends it as needed.
    pub fn generator_product_map(
        &self,
        g: BidegreeGenerator,
    ) -> Arc<ResolutionHomomorphism<CC, CC>> {
        cached_generator_product_map(
            &self.products,
            &self.resolution,
            self.algebra.resolution(),
            g,
        )
    }

    /// Left-multiplication by the class `x` (in $\Ext(M, k)$), applied to every basis generator of
    /// $\Ext(k, k)$ at bidegree `b`.
    ///
    /// Returns `None` when the product is out of the computed range — that is, when `b` or
    /// `b + x.degree()` has not been resolved — so callers never mistake an uncomputed product for a
    /// zero one. Otherwise returns a matrix with one row per generator of $\Ext(k, k)$ at `b`; row
    /// `j` is the product `x · g_j` expressed in the generator basis of $\Ext(M, k)$ at bidegree
    /// `b + x.degree()`. A computed-but-empty bidegree yields a valid zero-dimension matrix, not
    /// `None`.
    pub fn multiply_into(&self, x: &BidegreeElement, b: Bidegree) -> Option<Matrix> {
        products_into(
            &self.resolution,
            self.algebra.resolution(),
            &self.products,
            self.prime(),
            x,
            b,
        )
    }

    /// The product `x · y` if it lies in the computed range, else `None`. See
    /// [`multiply_into`](Self::multiply_into) for the operand conventions (`x ∈ Ext(M, k)`, `y ∈
    /// Ext(k, k)`). The result lies in bidegree `x.degree() + y.degree()`.
    pub fn try_multiply(
        &self,
        x: &BidegreeElement,
        y: &BidegreeElement,
    ) -> Option<BidegreeElement> {
        let matrix = self.multiply_into(x, y.degree())?;
        Some(combine_product(&matrix, y, x.degree() + y.degree(), self.prime()))
    }

    /// The product `x · y`, where `x ∈ Ext(M, k)` and `y ∈ Ext(k, k)`. When `M == k` both operands
    /// live in the same algebra $\Ext(k, k)$. The result lies in bidegree `x.degree() + y.degree()`.
    ///
    /// Panics if the product is out of the computed range; use
    /// [`try_multiply`](Self::try_multiply) to handle that case.
    pub fn multiply(&self, x: &BidegreeElement, y: &BidegreeElement) -> BidegreeElement {
        self.try_multiply(x, y).expect(
            "multiply: product is out of the computed range; compute further or use try_multiply",
        )
    }
}

/// Build/cache the per-generator product map `res(source) → res(target)` for generator `g`.
fn cached_generator_product_map<CC>(
    products: &DashMap<BidegreeGenerator, Arc<ResolutionHomomorphism<CC, CC>>>,
    source: &Arc<CC>,
    target: &Arc<CC>,
    g: BidegreeGenerator,
) -> Arc<ResolutionHomomorphism<CC, CC>>
where
    CC: FreeChainComplex + AugmentedChainComplex,
{
    if let Some(map) = products.get(&g) {
        return Arc::clone(&map);
    }

    let dim = source.number_of_gens_in_bidegree(g.degree());
    let mut class = vec![0u32; dim];
    class[g.idx()] = 1;

    let name = format!("prod_{}_{}_{}", g.n(), g.s(), g.idx());
    let hom = Arc::new(ResolutionHomomorphism::from_class(
        name,
        Arc::clone(source),
        Arc::clone(target),
        g.degree(),
        &class,
    ));

    Arc::clone(products.entry(g).or_insert(hom).value())
}

/// The shared body of `multiply_into`: left-multiplication by `x` (a class in `Ext(source, k)`)
/// applied to every generator of `Ext(target, k)` at bidegree `b`. Products land in `Ext(source,
/// k)` at `b + x.degree()`. Returns `None` when out of the computed range.
fn products_into<CC>(
    source: &Arc<CC>,
    target: &Arc<CC>,
    products: &DashMap<BidegreeGenerator, Arc<ResolutionHomomorphism<CC, CC>>>,
    prime: ValidPrime,
    x: &BidegreeElement,
    b: Bidegree,
) -> Option<Matrix>
where
    CC: FreeChainComplex + AugmentedChainComplex,
{
    let shift = x.degree();
    let result_deg = b + shift;

    if !target.has_computed_bidegree(b) || !source.has_computed_bidegree(result_deg) {
        return None;
    }

    let mult_dim = target.number_of_gens_in_bidegree(b);
    let res_dim = source.number_of_gens_in_bidegree(result_deg);
    let mut matrix = Matrix::new(prime, mult_dim, res_dim);

    for (i, c) in x.vec().iter_nonzero() {
        let map = cached_generator_product_map(products, source, target, BidegreeGenerator::new(shift, i));
        map.extend_all();

        // `hom_k(b.t())[j][k]`: `j` indexes the multiplicand generator of `Ext(k, k)` at `b`, `k`
        // indexes the result generator of `Ext(source, k)` at `result_deg`.
        let hom_k = map.get_map(result_deg.s()).hom_k(b.t());
        for (j, row) in hom_k.iter().enumerate() {
            for (k, &v) in row.iter().enumerate() {
                matrix.row_mut(j).add_basis_element(k, c * v);
            }
        }
    }
    Some(matrix)
}

/// Combine the per-generator product `matrix` (rows indexed by generators of `y`'s bidegree) with
/// the coordinates of `y` into the class `x · y` at bidegree `target`.
fn combine_product(
    matrix: &Matrix,
    y: &BidegreeElement,
    target: Bidegree,
    prime: ValidPrime,
) -> BidegreeElement {
    let mut out = FpVector::new(prime, matrix.columns());
    for (j, c) in y.vec().iter_nonzero() {
        out.as_slice_mut().add(matrix.row(j), c);
    }
    BidegreeElement::new(target, out)
}

impl<CC: FreeChainComplex> std::fmt::Display for ExtAlgebra<CC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ext(k, k)")
    }
}

impl<CC: FreeChainComplex> std::fmt::Display for ExtModule<CC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ext(M, k)")
    }
}

impl<CC> algebra::Algebra<2> for ExtAlgebra<CC>
where
    CC: FreeChainComplex + AugmentedChainComplex + 'static,
{
    fn prime(&self) -> ValidPrime {
        self.resolution.prime()
    }

    fn compute_basis(&self, degree: impl Into<Bidegree>) {
        self.compute_through_bidegree(degree.into());
    }

    fn dimension(&self, degree: impl Into<Bidegree>) -> usize {
        self.resolution.number_of_gens_in_bidegree(degree.into())
    }

    /// The ring product of two generators, **computed up to the Koszul sign** (see the
    /// [module-level docs](self)): the underlying [`multiply`](ExtAlgebra::multiply) is sign-exact
    /// only at `p = 2`. At odd primes the result may differ from the true product by
    /// $(-1)^{|r||s|}$, so do not wire this trait method into odd-prime machinery that depends on
    /// the exact sign.
    fn multiply_basis_elements(
        &self,
        mut result: fp::vector::FpSliceMut,
        coeff: u32,
        r_degree: impl Into<Bidegree>,
        r_idx: usize,
        s_degree: impl Into<Bidegree>,
        s_idx: usize,
    ) {
        let r = self.generator(BidegreeGenerator::new(r_degree.into(), r_idx));
        let s = self.generator(BidegreeGenerator::new(s_degree.into(), s_idx));
        let prod = self.multiply(&r, &s);
        result.add(prod.vec(), coeff);
    }

    fn basis_element_to_string(&self, degree: impl Into<Bidegree>, idx: usize) -> String {
        let degree = degree.into();
        format!("x_{{{},{}}}^{}", degree.n(), degree.s(), idx)
    }

    fn basis_element_from_string(&self, _elt: &str) -> Option<(i32, usize)> {
        // A single `i32` degree cannot encode a bidegree, so string parsing is unsupported for Ext.
        None
    }
}

impl<CC> algebra::module::Module<2> for ExtModule<CC>
where
    CC: FreeChainComplex + AugmentedChainComplex + 'static,
{
    type Algebra = ExtAlgebra<CC>;

    fn algebra(&self) -> Arc<Self::Algebra> {
        Arc::clone(&self.algebra)
    }

    /// The unit module (`M == k`) in the sense of the `Ext(k, k)`-module category — the module that
    /// shares its resolution with the ring. This delegates to the inherent
    /// [`ExtModule::is_unit`](ExtModule::is_unit) so the trait and inherent methods agree (the
    /// generic `Module::is_unit` default, which inspects `min_degree`/`max_degree`, is meaningless
    /// for a bigraded `Ext`).
    fn is_unit(&self) -> bool {
        ExtModule::is_unit(self)
    }

    /// `Ext` is bigraded, so there is no single meaningful `t`-bound. Per the crate's convention
    /// (degree-*returning* trait methods report the filtration `s` axis), this returns the `s`
    /// lower bound `0`, **not** a `t`-bound as the [`Module`](algebra::module::Module) trait's prose
    /// suggests. The generic `try_act_*`/`total_dimension` defaults that consume it are not used on
    /// `Ext`.
    fn min_degree(&self) -> i32 {
        0
    }

    /// The maximum filtration `s` for which the resolution of `M` is defined (an `s`-bound, not the
    /// `t`-bound the [`Module`](algebra::module::Module) trait's prose describes — see
    /// [`min_degree`](Self::min_degree) for the bigraded convention).
    fn max_computed_degree(&self) -> i32 {
        self.resolution.next_homological_degree() - 1
    }

    fn compute_basis_multi(&self, degree: Bidegree) {
        self.compute_through_bidegree(degree);
    }

    fn dimension_multi(&self, degree: Bidegree) -> usize {
        self.dimension(degree)
    }

    /// The action of a ring element `op ∈ Ext(k, k)` on a module element `mod ∈ Ext(M, k)`,
    /// **computed up to the Koszul sign**. The trait models a *left* action `op · mod`, but this is
    /// realised as `mod · op` (the only direction [`ExtModule::multiply`] supports), which equals
    /// `op · mod` up to $(-1)^{|op||mod|}$ — **exact only at `p = 2`**. Do not consume this trait
    /// action in odd-prime machinery that depends on the exact sign; use it at `p = 2` or where the
    /// sign is irrelevant.
    fn act_on_basis_multi(
        &self,
        mut result: fp::vector::FpSliceMut,
        coeff: u32,
        op_degree: Bidegree,
        op_index: usize,
        mod_degree: Bidegree,
        mod_index: usize,
    ) {
        let op = self.algebra.generator(BidegreeGenerator::new(op_degree, op_index));
        let mod_elt = self.generator(BidegreeGenerator::new(mod_degree, mod_index));
        let prod = self.multiply(&mod_elt, &op);
        result.add(prod.vec(), coeff);
    }

    fn basis_element_to_string_multi(&self, degree: Bidegree, idx: usize) -> String {
        format!("x_{{{},{}}}^{}", degree.n(), degree.s(), idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::construct_standard;

    #[test]
    fn test_sphere_products() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(8, 8));
        let module = ExtModule::intrinsic(res);

        // h_i live in Ext^{1, *}: h_0 = (n=0, s=1), h_1 = (n=1, s=1), h_2 = (n=3, s=1).
        let h0 = module.generator(BidegreeGenerator::new(Bidegree::n_s(0, 1), 0));
        let h1 = module.generator(BidegreeGenerator::new(Bidegree::n_s(1, 1), 0));

        // h_0^2 is the nonzero generator of Ext^{2,2} = (n=0, s=2).
        let h0_sq = module.multiply(&h0, &h0);
        assert_eq!(h0_sq.degree(), Bidegree::n_s(0, 2));
        assert_eq!(module.dimension(Bidegree::n_s(0, 2)), 1);
        assert!(!h0_sq.vec().is_zero(), "h_0^2 should be nonzero");

        // The Adams relations h_0 h_1 = 0 = h_1 h_0.
        assert!(
            module.multiply(&h0, &h1).vec().is_zero(),
            "h_0 h_1 should vanish"
        );
        assert!(
            module.multiply(&h1, &h0).vec().is_zero(),
            "h_1 h_0 should vanish"
        );

        // Cross-check `multiply` against a direct `hom_k` read for h_0 · h_1.
        let rows = module
            .multiply_into(&h0, h1.degree())
            .expect("h_0 · h_1 is in range");
        let direct: u32 = rows.row(0).iter().sum();
        assert_eq!(direct, 0);
    }

    /// The bigraded `Algebra<2>` / `Module<2>` trait methods must agree with the inherent product.
    /// At `p = 2` there is no sign ambiguity, so `Algebra::multiply_basis_elements`,
    /// `Module::act_on_basis_multi`, and the inherent `multiply` all coincide (`M == k` here, so the
    /// ring generator and the module generator are the same class).
    #[test]
    fn test_trait_surface() {
        use algebra::{Algebra as _, module::Module as _};

        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(4, 4));
        let module = ExtModule::intrinsic(res);
        let algebra = module.algebra();

        let h0 = Bidegree::n_s(0, 1);
        let target = Bidegree::n_s(0, 2);
        let p = algebra.prime();

        // Inherent product h_0 · h_0 (the reference).
        let g = algebra.generator(BidegreeGenerator::new(h0, 0));
        let inherent: Vec<u32> = algebra.multiply(&g, &g).vec().iter().collect();
        assert_eq!(inherent.len(), algebra.dimension(target));

        // `Algebra::<2>::multiply_basis_elements`.
        let mut ring = FpVector::new(p, algebra.dimension(target));
        algebra.multiply_basis_elements(ring.as_slice_mut(), 1, h0, 0, h0, 0);
        assert_eq!(ring.iter().collect::<Vec<_>>(), inherent);

        // `Module::<2>::act_on_basis_multi` (op = h_0 in the ring acting on mod = h_0 in the module).
        let mut act = FpVector::new(p, module.dimension(target));
        module.act_on_basis_multi(act.as_slice_mut(), 1, h0, 0, h0, 0);
        assert_eq!(act.iter().collect::<Vec<_>>(), inherent);
    }

    /// Exercise the `M != k` path (the whole point of the split): products of `Ext(M, k)` classes
    /// use `source = res(M)`, `target = res(k)`, a distinction the `M == k` tests never hit. The
    /// unit `1 ∈ Ext^{0,0}(k, k)` acts trivially, so `x · 1 = x` for any `x ∈ Ext(M, k)`.
    #[test]
    fn test_non_unit_products() {
        let max = Bidegree::n_s(8, 8);
        let unit = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        let m = Arc::new(construct_standard::<false, _, _>("C2", None).unwrap());
        unit.compute_through_stem(max);
        m.compute_through_stem(max);
        let module = ExtModule::new(m, Arc::new(ExtAlgebra::new(unit)));
        assert!(!module.is_unit(), "C2 is not the sphere, so M != k");

        // The unit class 1 ∈ Ext^{0,0}(k, k).
        let unit_deg = Bidegree::n_s(0, 0);
        assert_eq!(module.algebra().dimension(unit_deg), 1);
        let one = module.algebra().generator(BidegreeGenerator::new(unit_deg, 0));

        // The bottom class of Ext(C2, k) at (0, 0); x · 1 = x.
        assert_eq!(module.dimension(Bidegree::n_s(0, 0)), 1);
        let x = module.generator(BidegreeGenerator::new(Bidegree::n_s(0, 0), 0));
        let prod = module.multiply(&x, &one);
        assert_eq!(prod.degree(), x.degree());
        assert_eq!(
            prod.vec().iter().collect::<Vec<_>>(),
            x.vec().iter().collect::<Vec<_>>(),
            "x · 1 = x"
        );
    }

    /// Exercise an odd prime (`p = 3`), the regime where the product API's up-to-Koszul-sign caveat
    /// bites. We assert only the sign-robust fact that `a_0^2 != 0` (the `a_0`-Bockstein tower).
    #[test]
    fn test_odd_prime_products() {
        let res = Arc::new(construct_standard::<false, _, _>("S_3", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(4, 4));
        let module = ExtModule::intrinsic(res);

        // a_0 ∈ Ext^{1,1}(F_3, F_3) at (n = 0, s = 1); a_0^2 ∈ Ext^{2,2} at (0, 2) is nonzero.
        let a0_deg = Bidegree::n_s(0, 1);
        assert_eq!(module.dimension(a0_deg), 1);
        let a0 = module.generator(BidegreeGenerator::new(a0_deg, 0));
        let a0_sq = module.multiply(&a0, &a0);
        assert_eq!(a0_sq.degree(), Bidegree::n_s(0, 2));
        assert!(
            !a0_sq.vec().is_zero(),
            "a_0^2 should be nonzero at p = 3 (up to sign)"
        );
    }
}
