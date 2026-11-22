//! Ext as a bigraded ring and its modules.

pub mod lin;
pub mod massey;
pub mod secondary;

use std::sync::{Arc, Mutex, MutexGuard};

use algebra::{Algebra, module::Module};
use dashmap::DashMap;
use fp::{
    matrix::{AugmentedMatrix, Matrix, Subquotient, Subspace},
    prime::ValidPrime,
    vector::FpVector,
};
use sseq::coordinates::{Bidegree, BidegreeElement, BidegreeGenerator};

pub use self::{
    lin::{Lin, LinRange},
    secondary::{SecondaryExtAlgebra, SecondaryProduct},
};
use crate::{
    chain_complex::{AugmentedChainComplex, ChainComplex, FreeChainComplex},
    lift::{LiftError, Liftable, MultiLift},
    resolution_homomorphism::ResolutionHomomorphism,
    utils::{QueryModuleResolution, get_unit},
};

/// The coboundary of the Ext cochain complex $\Hom(P_\bullet, k) = k^{\text{gens}}$, whose
/// cohomology is $\Ext$.
///
/// It shifts bidegree by a fixed [`shift`](ExtDifferential::shift) and, at each bidegree, gives its
/// matrix in the generator bases. For a **minimal** resolution this coboundary is identically zero.
/// For a **non-minimal** resolution it is the canonical dualised differential $\Hom(d, k)$.
///
/// This is also how a deformation encodes the connecting differential in this complex — the Adams
/// $d_2$ ([`secondary`]) or the motivic $\delta$ — as instances defined in their own modules.
pub trait ExtDifferential: Send + Sync {
    /// The fixed bidegree shift the differential applies: $\delta\colon \Ext_b \to
    /// \Ext_{b + \mathrm{shift}}$.
    fn shift(&self) -> Bidegree;

    /// The matrix of $\delta$ out of bidegree `b`: rows index the generators at `b`, columns the
    /// generators at `b + shift`. `None` if the differential out of `b` is out of the computed
    /// range.
    fn matrix(&self, b: Bidegree) -> Option<Matrix>;
}

/// The ring $\Ext(k, k)$, backed by a resolution of the base field `k`.
///
/// A product is realised by a [`ResolutionHomomorphism`] from a fixed multiplier class, which
/// computes the products of that class with all of $\Ext(k, k)$ at once. One such map is cached per
/// generator; a product by a general class is assembled from them on request (see
/// [`class_product_map`](Self::class_product_map)). Products are computed up to a sign.
///
/// [`multiply_into`](Self::multiply_into) does not use these maps by default (see
/// [`ProductMethod`]): it answers from the ring's multiplicative structure computed by W. Lin's
/// method ([`lin`]), which lifts one chain map per basis monomial, each only over a small corner,
/// and over the Milnor basis at `p = 2` reads products by `h_i` off the differentials.
pub struct ExtAlgebra<CC: FreeChainComplex> {
    /// Resolution of the base field `k`. Ring products live here.
    resolution: Arc<CC>,
    /// One multiplication map per generator of $\Ext(k, k)$, `res(k) → res(k)`, built on demand.
    products: DashMap<BidegreeGenerator, Arc<ResolutionHomomorphism<CC, CC>>>,
    /// The ring's products by Lin's method, through the range computed so far.
    lin: Mutex<Option<Lin<CC>>>,
    method: Mutex<ProductMethod>,
}

/// How [`ExtAlgebra::multiply_into`] and [`ExtModule::multiply_into`] compute products. Both give
/// the same products; the choice only affects the cost.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProductMethod {
    /// W. Lin's method ([`lin`]), falling back to [`ChainMaps`](Self::ChainMaps) outside the
    /// largest range in which the resolutions are computed through a stem, or if a lift fails.
    #[default]
    Lin,
    /// One chain map per generator of the multiplier's bidegree, lifted over the whole computed
    /// range.
    ChainMaps,
}

impl<CC: FreeChainComplex> ExtAlgebra<CC> {
    /// Build the ring $\Ext(k, k)$ from a resolution of `k`.
    pub fn new(resolution: Arc<CC>) -> Self {
        Self {
            resolution,
            products: DashMap::new(),
            lin: Mutex::new(None),
            method: Mutex::new(ProductMethod::default()),
        }
    }

    /// How [`multiply_into`](ExtAlgebra::multiply_into) computes products.
    pub fn product_method(&self) -> ProductMethod {
        *self.method.lock().unwrap()
    }

    /// Choose how [`multiply_into`](ExtAlgebra::multiply_into) computes products.
    pub fn set_product_method(&self, method: ProductMethod) {
        *self.method.lock().unwrap() = method;
    }

    /// The resolution of `k` backing this ring.
    pub fn resolution(&self) -> &Arc<CC> {
        &self.resolution
    }

    /// The prime of the underlying resolution.
    pub fn prime(&self) -> ValidPrime {
        self.resolution.prime()
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
    CC: FreeChainComplex + AugmentedChainComplex + 'static,
{
    /// The ring's [`Lin`] products, decided at least through `need` (and through every stem the
    /// resolution is computed through in those filtrations). `None` if the resolution is not
    /// computed through `need`'s stem in every filtration up to `need`'s, or if Lin's method failed
    /// (after which [`multiply_into`](Self::multiply_into) uses chain maps).
    fn lin_through(&self, need: Bidegree) -> Option<MutexGuard<'_, Option<Lin<CC>>>> {
        if self.product_method() != ProductMethod::Lin {
            return None;
        }
        let mut guard = self.lin.lock().unwrap();
        let old = guard.as_ref().and_then(Lin::range);
        if old.is_some_and(|r| r.contains(need)) {
            return Some(guard);
        }
        let max_s = need.s().max(old.map_or(0, |r| r.max_s));
        let max_n = computed_stem(&*self.resolution, max_s)?;
        if max_n < need.n() {
            return None;
        }
        let lin = guard.get_or_insert_with(|| Lin::ring(Arc::clone(&self.resolution)));
        let range = LinRange {
            min_n: 0,
            max_n,
            max_s,
        };
        if let Err(e) = lin.extend(range, None) {
            tracing::warn!("Lin's method failed, using chain maps from now on: {e:#}");
            *guard = None;
            self.set_product_method(ProductMethod::ChainMaps);
            return None;
        }
        Some(guard)
    }

    /// The range through which Lin's method has decided the ring so far, without computing more.
    pub fn lin_range(&self) -> Option<LinRange> {
        self.lin.lock().unwrap().as_ref().and_then(Lin::range)
    }

    /// The indecomposables of $\Ext(k, k)$ through `b` (every filtration up to `b.s()`, every
    /// stem up to `b.n()`), in the order Lin's method found them: a generator at each, together
    /// with the products of earlier ones, spans its bidegree. `None` if the resolution is not
    /// computed that far.
    pub fn indecomposables(&self, b: Bidegree) -> Option<Vec<BidegreeGenerator>> {
        let guard = self.lin_through(b)?;
        let lin = guard.as_ref()?;
        Some(
            lin.gens
                .iter()
                .copied()
                .filter(|g| g.s() <= b.s() && g.n() <= b.n())
                .collect(),
        )
    }

    /// Run `f` on the ring's [`Lin`] products, decided through `b` as in
    /// [`indecomposables`](Self::indecomposables).
    pub fn with_lin<R>(&self, b: Bidegree, f: impl FnOnce(&mut Lin<CC>) -> R) -> Option<R> {
        let mut guard = self.lin_through(b)?;
        guard.as_mut().map(f)
    }

    /// [`multiply_into`](Self::multiply_into) by Lin's method, if it applies.
    fn lin_multiply_into(&self, x: &BidegreeElement, b: Bidegree) -> Option<Matrix> {
        let target = x.degree() + b;
        let mut guard = self.lin_through(target)?;
        let lin = guard.as_mut()?;
        let cell = lin.ring_cell(b)?;
        lin.right_products(x.degree(), &x.vec().to_owned(), b, &cell)
    }

    /// `x · g_j` for the generators `g_j` at `b` when `x` is `h_k` (see [`h_product`]).
    fn h_multiply_into(&self, x: &BidegreeElement, b: Bidegree) -> Option<Matrix> {
        let a = x.degree();
        if a.s() != 1
            || !(a.t() as u32).is_power_of_two()
            || self.dimension(a) != 1
            || x.vec().is_zero()
        {
            return None;
        }
        h_product(&*self.resolution, a.t().trailing_zeros(), b)
    }

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
    /// A generator with coefficient one returns the cached
    /// [`generator_product_map`](Self::generator_product_map) itself; any other nonzero class adds
    /// the cached generator maps via [`ResolutionHomomorphism::linear_combination`] (no
    /// quasi-inverse lift).
    pub fn class_product_map(
        &self,
        x: &BidegreeElement,
        max: Bidegree,
    ) -> Arc<ResolutionHomomorphism<CC, CC>> {
        let summands: Vec<(u32, Arc<ResolutionHomomorphism<CC, CC>>)> = x
            .vec()
            .iter_nonzero()
            .map(|(idx, c)| {
                let map = self.generator_product_map(BidegreeGenerator::new(x.degree(), idx));
                map.extend_through_stem(max);
                (c, map)
            })
            .collect();
        match summands.as_slice() {
            [(1, map)] => Arc::clone(map),
            [] => {
                // With no initial images, extending lifts zero everywhere.
                let hom = Arc::new(ResolutionHomomorphism::new(
                    String::new(),
                    Arc::clone(&self.resolution),
                    Arc::clone(&self.resolution),
                    x.degree(),
                ));
                hom.extend_through_stem(max);
                hom
            }
            _ => Arc::new(ResolutionHomomorphism::linear_combination(
                String::new(),
                &summands,
                max,
            )),
        }
    }

    /// Left-multiplication by `x ∈ Ext(k, k)`, applied to every basis generator of $\Ext(k, k)$ at
    /// bidegree `b`. See [`ExtModule::multiply_into`] for the return convention.
    ///
    /// Products by `h_i` over the Milnor basis at `p = 2` are read off the differentials; the rest
    /// come from Lin's method or chain maps, as chosen by [`ProductMethod`].
    pub fn multiply_into(&self, x: &BidegreeElement, b: Bidegree) -> Option<Matrix> {
        if !self.resolution.has_computed_bidegree(b)
            || !self.resolution.has_computed_bidegree(x.degree() + b)
        {
            return None;
        }
        if self.product_method() == ProductMethod::Lin
            && let Some(m) = self
                .h_multiply_into(x, b)
                .or_else(|| self.lin_multiply_into(x, b))
        {
            return Some(m);
        }
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
        Some(combine_product(
            &matrix,
            y,
            x.degree() + y.degree(),
            self.prime(),
        ))
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
/// Products follow the [`ExtAlgebra`] conventions, with the multiplier in $\Ext(M, k)$ and the
/// chain maps running `res(M) → res(k)`. When `M == k` the module and ring share one resolution
/// (see [`is_unit`](Self::is_unit)).
pub struct ExtModule<CC: FreeChainComplex> {
    /// Resolution of `M`; the module's classes and the module-action products land in its Ext.
    resolution: Arc<CC>,
    /// Shared handle to the ring $\Ext(k, k)$. `Arc`-shared so all modules over the same `k` reuse
    /// one ring cache.
    algebra: Arc<ExtAlgebra<CC>>,
    /// One multiplication map per generator of $\Ext(M, k)$, `res(M) → res(k)`, built on demand.
    /// Read through [`product_cache`](Self::product_cache).
    products: DashMap<BidegreeGenerator, Arc<ResolutionHomomorphism<CC, CC>>>,
    /// The DGA differential, if any. `None` is the field/minimal case (zero
    /// coboundary), where the cohomology is just the generators.
    differential: Option<Arc<dyn ExtDifferential>>,
    /// The module's products by Lin's method when `M != k` (otherwise the ring's are used).
    lin: Mutex<Option<Lin<CC>>>,
    /// Set if Lin's method failed for this module, which then uses chain maps.
    lin_failed: std::sync::atomic::AtomicBool,
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
            differential: None,
            lin: Mutex::new(None),
            lin_failed: std::sync::atomic::AtomicBool::new(false),
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

    /// Attach a differential, so that the cohomology queries compute kernel mod image under it.
    ///
    /// Without one the coboundary is zero and the cohomology is the generators.
    #[must_use]
    pub fn with_differential(mut self, differential: Arc<dyn ExtDifferential>) -> Self {
        self.differential = Some(differential);
        self
    }

    /// The differential this DGA carries, if any.
    pub fn differential(&self) -> Option<&Arc<dyn ExtDifferential>> {
        self.differential.as_ref()
    }

    /// The dimension of the cohomology at `b`:
    /// $\mathrm{gens}(b) - \mathrm{rank}\,\delta_{\text{out}}(b) - \mathrm{rank}\,\delta_{\text{in}}(b)$.
    ///
    /// `None` if either differential at `b` is out of the computed range. An unknown incoming rank
    /// is not read as zero: that would overstate the cohomology at the edge of the computed region.
    pub fn cohomology_dimension(&self, b: Bidegree) -> Option<usize> {
        let Some(d) = &self.differential else {
            return Some(self.dimension(b));
        };
        let gens = self.dimension(b);
        // Nothing survives in an empty bidegree, so neither differential has to be known.
        if gens == 0 {
            return Some(0);
        }
        let shift = d.shift();
        let source = Bidegree::n_s(b.n() - shift.n(), b.s() - shift.s());
        // An undersized matrix would understate a rank and overstate the cohomology.
        let mut out = d.matrix(b)?;
        assert_eq!(
            out.rows(),
            gens,
            "ExtDifferential::matrix({b:?}) must have gens = {gens} rows, got {}",
            out.rows()
        );
        let rank_out = out.row_reduce();
        let mut incoming = d.matrix(source)?;
        assert_eq!(
            incoming.columns(),
            gens,
            "ExtDifferential::matrix({source:?}) into {b:?} must have gens = {gens} columns, got \
             {}",
            incoming.columns()
        );
        let rank_in = incoming.row_reduce();
        // ker ⊇ im requires d∘d = 0; a malformed differential could underflow here.
        debug_assert!(
            rank_out + rank_in <= gens,
            "ExtDifferential violates d∘d=0 at {b:?}: rank_out={rank_out}, rank_in={rank_in}, \
             gens={gens}"
        );
        Some(gens - rank_out - rank_in)
    }

    /// The cohomology at `b` as a [`Subquotient`] of the generators, giving representatives of the
    /// surviving classes.
    ///
    /// The numerator is $\ker(\delta \text{ out of } b)$ and the denominator is
    /// $\operatorname{im}(\delta \text{ into } b)$.
    ///
    /// `None` if either differential at `b` is out of the computed range, as with
    /// [`cohomology_dimension`](Self::cohomology_dimension).
    pub fn cohomology_subquotient(&self, b: Bidegree) -> Option<Subquotient> {
        let p = self.prime();
        let dim = self.dimension(b);
        if dim == 0 {
            return Some(Subquotient::new_full(p, 0));
        }
        let Some(d) = &self.differential else {
            return Some(Subquotient::new_full(p, dim));
        };

        // Numerator: ker(δ out of b), via the standard augmented-identity kernel.
        let out = d.matrix(b)?;
        assert_eq!(
            out.rows(),
            dim,
            "ExtDifferential::matrix({b:?}) must have gens(b) = {dim} rows, got {}",
            out.rows()
        );
        let target_dim = out.columns();
        let mut aug = AugmentedMatrix::<2>::new(p, dim, [target_dim, dim]);
        aug.segment(1, 1).add_identity();
        for i in 0..dim {
            aug.row_mut(i).slice_mut(0, target_dim).add(out.row(i), 1);
        }
        aug.row_reduce();
        let numerator = aug.compute_kernel();

        // Denominator: im(δ into b) = row space of δ out of the source bidegree.
        let shift = d.shift();
        let source = Bidegree::n_s(b.n() - shift.n(), b.s() - shift.s());
        let m = d.matrix(source)?;
        assert_eq!(
            m.columns(),
            dim,
            "ExtDifferential::matrix({source:?}) into {b:?} must have gens(b) = {dim} columns, \
             got {}",
            m.columns()
        );
        let denominator = Subspace::from_matrix(m);

        Some(Subquotient::from_parts(numerator, denominator))
    }

    /// The resolution of `M` backing this module.
    pub fn resolution(&self) -> &Arc<CC> {
        &self.resolution
    }

    /// The ring $\Ext(k, k)$ this is a module over.
    pub fn algebra(&self) -> &Arc<ExtAlgebra<CC>> {
        &self.algebra
    }

    /// Whether `M == k`, i.e. the module shares its resolution with its ring.
    pub fn is_unit(&self) -> bool {
        Arc::ptr_eq(&self.resolution, self.algebra.resolution())
    }

    /// The prime of the underlying resolution.
    pub fn prime(&self) -> ValidPrime {
        self.resolution.prime()
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
    CC: FreeChainComplex + AugmentedChainComplex + 'static,
{
    /// The range through which Lin's method has decided the module's products so far (the ring's,
    /// when `M == k`), without computing more.
    pub fn lin_range(&self) -> Option<LinRange> {
        if self.is_unit() {
            return self.algebra.lin_range();
        }
        self.lin.lock().unwrap().as_ref().and_then(Lin::range)
    }

    /// [`multiply_into`](Self::multiply_into) by Lin's method, if it applies: the module's products
    /// are decided through the largest stem both resolutions are computed through in the
    /// filtrations up to the product's, and the ring's through the degrees of their factors.
    fn lin_multiply_into(&self, x: &BidegreeElement, c: Bidegree) -> Option<Matrix> {
        use std::sync::atomic::Ordering;
        if self.is_unit() {
            return self.algebra.lin_multiply_into(x, c);
        }
        if self.algebra.product_method() != ProductMethod::Lin
            || self.lin_failed.load(Ordering::Relaxed)
        {
            return None;
        }
        let target = x.degree() + c;
        let mut guard = self.lin.lock().unwrap();
        let old = guard.as_ref().and_then(Lin::range);
        let min_n = self.resolution.min_degree();
        if !old.is_some_and(|r| r.contains(target) && r.contains(x.degree())) {
            let max_s = target.s().max(old.map_or(0, |r| r.max_s));
            let max_n = computed_stem(&*self.resolution, max_s)?
                .min(computed_stem(&**self.algebra.resolution(), max_s)? + min_n);
            if max_n < target.n() || x.degree().n() < min_n {
                return None;
            }
            // The factors of the module's monomials have stems up to `max_n - min_n`.
            let ring = self
                .algebra
                .lin_through(Bidegree::n_s(max_n - min_n, max_s))?;
            let lin = guard.get_or_insert_with(|| {
                Lin::module(
                    Arc::clone(&self.resolution),
                    Arc::clone(self.algebra.resolution()),
                )
            });
            let range = LinRange {
                min_n,
                max_n,
                max_s,
            };
            if let Err(e) = lin.extend(range, ring.as_ref()) {
                tracing::warn!("Lin's method failed for a module, using chain maps: {e:#}");
                *guard = None;
                self.lin_failed.store(true, Ordering::Relaxed);
                return None;
            }
        }
        let cell = self.algebra.lin_through(c)?.as_ref()?.ring_cell(c)?;
        guard
            .as_mut()?
            .right_products(x.degree(), &x.vec().to_owned(), c, &cell)
    }

    /// The per-generator product maps for this module.
    ///
    /// When `M == k` these are the ring's maps, so the module and its ring build each one once.
    fn product_cache(&self) -> &DashMap<BidegreeGenerator, Arc<ResolutionHomomorphism<CC, CC>>> {
        if self.is_unit() {
            &self.algebra.products
        } else {
            &self.products
        }
    }

    /// The multiplication map for a single generator `g` of $\Ext(M, k)$ (`res(M) → res(k)`), built
    /// and cached on first use. The returned map is *not* guaranteed to be extended;
    /// [`multiply_into`](Self::multiply_into) extends it as needed.
    pub fn generator_product_map(
        &self,
        g: BidegreeGenerator,
    ) -> Arc<ResolutionHomomorphism<CC, CC>> {
        cached_generator_product_map(
            self.product_cache(),
            &self.resolution,
            self.algebra.resolution(),
            g,
        )
    }

    /// Build the product map for every generator of $\Ext(M, k)$ in the computed range and extend
    /// them all together, in bidegree-major order, via [`MultiLift`].
    ///
    /// All the product maps share the unit resolution as target, so lifting them together means the
    /// unit's quasi-inverse at each output bidegree is solved once and reused across every product,
    /// rather than once per product.
    ///
    /// # Errors
    ///
    /// If the unit resolution cannot lift at some bidegree; see [`LiftError`].
    pub fn extend_all_products(&self) -> Result<(), LiftError>
    where
        CC: Sync + 'static,
    {
        let liftables: Vec<Arc<dyn Liftable>> = self
            .resolution
            .iter_stem()
            .flat_map(|b| {
                (0..self.resolution.number_of_gens_in_bidegree(b))
                    .map(move |i| BidegreeGenerator::new(b, i))
            })
            .map(|g| self.generator_product_map(g) as Arc<dyn Liftable>)
            .collect();
        if liftables.is_empty() {
            return Ok(());
        }
        MultiLift::new(Arc::clone(self.algebra.resolution()), liftables).extend_all()
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
    ///
    /// The products come from Lin's method or chain maps, as chosen by the ring's
    /// [`ProductMethod`].
    pub fn multiply_into(&self, x: &BidegreeElement, b: Bidegree) -> Option<Matrix> {
        if self.is_unit() {
            return self.algebra.multiply_into(x, b);
        }
        if self.algebra.resolution().has_computed_bidegree(b)
            && self.resolution.has_computed_bidegree(x.degree() + b)
            && let Some(m) = self.lin_multiply_into(x, b)
        {
            return Some(m);
        }
        products_into(
            &self.resolution,
            self.algebra.resolution(),
            self.product_cache(),
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
        Some(combine_product(
            &matrix,
            y,
            x.degree() + y.degree(),
            self.prime(),
        ))
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
/// The largest stem through which `res` is computed in every filtration `0..=max_s`, if it is
/// computed in filtration `max_s` at all.
fn computed_stem<CC: ChainComplex>(res: &CC, max_s: i32) -> Option<i32> {
    if max_s < 0 || max_s >= res.next_homological_degree() {
        return None;
    }
    (0..=max_s)
        .map(|s| res.module(s).max_computed_degree() - s)
        .min()
}

/// Whether [`h_product`] reads the products by `h_i` off the differentials of `res`: over the Milnor
/// basis at `p = 2`.
pub(crate) fn reads_h_products<CC: ChainComplex>(res: &CC) -> bool {
    res.prime() == 2 && res.algebra().prefix() == "milnor"
}

/// Multiplication by `h_k` out of bidegree `b`, read off the differentials of a minimal resolution
/// `F` over the Milnor basis at `p = 2`: row `j` is `x_(b, j) · h_k` in the generators at
/// `b + (1, 2^k)`. No chain map is lifted.
///
/// `h_k` is dual to the generator of the resolution of `F_2` in filtration one with
/// `d = Sq(2^k) ι`, and for `x ∈ Ext^{s,t}` dual to the generator `g_x` of `F_s`,
///
/// ```text
///     x · h_k = Σ_j <Sq(2^k) g_x, d g_j> g_j^*      (g_j the generators of F_{s+1} in degree t + 2^k)
/// ```
///
/// i.e. the coefficient of `Sq(2^k)` on `g_x` in `d g_j`, which is
/// [`FreeChainComplex::try_filtration_one_product`]. (`Sq(2^k)` is the only Milnor basis element of
/// degree `2^k` pairing non-trivially with `ξ_1^{2^k}`, the cobar representative of `h_k`; in the
/// admissible basis this fails.) `None` off the Milnor basis at `p = 2`, or when `b` or the target
/// has not been computed.
pub fn h_product<CC: FreeChainComplex>(res: &CC, k: u32, b: Bidegree) -> Option<Matrix> {
    if !reads_h_products(res) {
        return None;
    }
    let deg = 1 << k;
    let target = b + Bidegree::s_t(1, deg);
    if !res.has_computed_bidegree(b) || !res.has_computed_bidegree(target) {
        return None;
    }
    let (_, op_idx) = res
        .algebra()
        .basis_element_from_string(&format!("Sq{deg}"))?;
    let rows = res.try_filtration_one_product(deg, op_idx, b).ok()?;
    let mut matrix = Matrix::new(
        res.prime(),
        res.number_of_gens_in_bidegree(b),
        res.number_of_gens_in_bidegree(target),
    );
    for (j, row) in rows.iter().enumerate() {
        for (i, &v) in row.iter().enumerate() {
            matrix.row_mut(j).set_entry(i, v);
        }
    }
    Some(matrix)
}

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
        let map = cached_generator_product_map(
            products,
            source,
            target,
            BidegreeGenerator::new(shift, i),
        );
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

#[cfg(test)]
mod tests {
    use fp::prime::TWO;

    use super::*;
    use crate::{chain_complex::ChainComplex, utils::construct_standard};

    /// A module over itself reads the ring's product cache, so the two share each generator map.
    #[test]
    fn test_unit_module_shares_ring_cache() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(4, 4));
        let module = ExtModule::intrinsic(res);

        let h0 = BidegreeGenerator::new(Bidegree::n_s(0, 1), 0);
        assert!(Arc::ptr_eq(
            &module.generator_product_map(h0),
            &module.algebra().generator_product_map(h0),
        ));
    }

    #[test]
    fn test_zero_differential_cohomology_is_generators() {
        // The field/minimal case: with no differential the DGA cohomology is just
        // the generators — "taking the Ext" is a no-op.
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(8, 8));
        let alg = ExtModule::intrinsic(res);
        for s in 0..=8 {
            for n in 0..=8 {
                let b = Bidegree::n_s(n, s);
                assert_eq!(alg.cohomology_dimension(b), Some(alg.dimension(b)));
            }
        }
    }

    #[test]
    fn test_differential_cohomology_kills_kernel_and_image() {
        // A synthetic rank-1 differential (0,2) -> (0,1) must kill both ends in
        // cohomology: h_0^2 by the outgoing rank, h_0 by the incoming rank. An
        // untouched bidegree (h_1) is unchanged.
        // Shaped per the `matrix` contract, with the single nonzero entry d(h_0^2) = h_0.
        struct MockDiff {
            dims: Arc<dyn Fn(Bidegree) -> usize + Send + Sync>,
        }
        impl ExtDifferential for MockDiff {
            fn shift(&self) -> Bidegree {
                Bidegree::n_s(0, -1) // lowers filtration: (0,2) -> (0,1)
            }

            fn matrix(&self, b: Bidegree) -> Option<Matrix> {
                let rows = (self.dims)(b);
                let cols = (self.dims)(b + self.shift());
                let mut m = Matrix::new(TWO, rows, cols);
                if b == Bidegree::n_s(0, 2) && rows == 1 && cols == 1 {
                    m.row_mut(0).set_entry(0, 1);
                }
                Some(m)
            }
        }

        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(8, 8));
        let dims: Arc<dyn Fn(Bidegree) -> usize + Send + Sync> = {
            let res = Arc::clone(&res);
            Arc::new(move |b: Bidegree| {
                if b.n() >= 0 && b.s() >= 0 && res.has_computed_bidegree(b) {
                    res.number_of_gens_in_bidegree(b)
                } else {
                    0
                }
            })
        };
        let alg =
            ExtModule::intrinsic(Arc::clone(&res)).with_differential(Arc::new(MockDiff { dims }));

        // Sanity: all three source bidegrees are 1-dimensional on the E-page.
        assert_eq!(alg.dimension(Bidegree::n_s(0, 1)), 1); // h_0
        assert_eq!(alg.dimension(Bidegree::n_s(0, 2)), 1); // h_0^2
        assert_eq!(alg.dimension(Bidegree::n_s(1, 1)), 1); // h_1

        assert_eq!(alg.cohomology_dimension(Bidegree::n_s(0, 2)), Some(0)); // outgoing rank 1
        assert_eq!(alg.cohomology_dimension(Bidegree::n_s(0, 1)), Some(0)); // incoming rank 1
        assert_eq!(alg.cohomology_dimension(Bidegree::n_s(1, 1)), Some(1)); // untouched

        assert_eq!(
            alg.cohomology_subquotient(Bidegree::n_s(0, 2))
                .unwrap()
                .dimension(),
            0
        );
        assert_eq!(
            alg.cohomology_subquotient(Bidegree::n_s(0, 1))
                .unwrap()
                .dimension(),
            0
        );
        assert_eq!(
            alg.cohomology_subquotient(Bidegree::n_s(1, 1))
                .unwrap()
                .dimension(),
            1
        );
    }

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

    /// Products of `Ext(M, k)` classes with `M != k`, where the chain maps run `res(M) → res(k)`.
    /// The unit `1 ∈ Ext^{0,0}(k, k)` acts trivially, so `x · 1 = x` for any `x ∈ Ext(M, k)`.
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
        let one = module
            .algebra()
            .generator(BidegreeGenerator::new(unit_deg, 0));

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

    /// Check that `class_product_map(x)` induces the same products as [`ExtModule::multiply_into`],
    /// which sums the per-generator maps at the `hom_k` level rather than the chain level.
    fn check_class_product_map<CC: FreeChainComplex + AugmentedChainComplex + 'static>(
        module: &ExtModule<CC>,
        x: &BidegreeElement,
        max: Bidegree,
    ) {
        let algebra = module.algebra();
        let map = algebra.class_product_map(x, max);
        map.extend_all();

        let mut compared = 0;
        for b in algebra.resolution().iter_nonzero_stem() {
            // `multiply_into` returns `None` once `b + x.degree()` is out of the computed range.
            let Some(reference) = module.multiply_into(x, b) else {
                continue;
            };
            let target = b + x.degree();
            let hom_k = map.get_map(target.s()).hom_k(b.t());
            assert_eq!(reference.rows(), hom_k.len());
            for (j, row) in hom_k.iter().enumerate() {
                let via_ref: Vec<u32> = reference.row(j).iter().collect();
                assert_eq!(&via_ref, row, "product mismatch at multiplicand {b}");
                compared += 1;
            }
        }
        assert!(compared > 0, "expected at least one product comparison");
    }

    /// A multi-generator class takes the chain-level
    /// [`ResolutionHomomorphism::linear_combination`] path; agreeing with
    /// [`ExtModule::multiply_into`] pins `linear_combination`.
    #[test]
    fn test_class_product_map_multi_generator() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        let max = Bidegree::n_s(20, 9);
        res.compute_through_stem(max);
        let module = ExtModule::intrinsic(res);

        // (n = 15, s = 5) is the first bidegree of Ext(F_2, F_2) with two generators.
        let x_deg = Bidegree::n_s(15, 5);
        assert_eq!(module.algebra().dimension(x_deg), 2);
        let x = module.algebra().element(x_deg, &[1, 1]);
        check_class_product_map(&module, &x, max);
    }

    /// A single generator with a coefficient other than one is a one-summand
    /// [`ResolutionHomomorphism::linear_combination`].
    #[test]
    fn test_class_product_map_scaled_generator() {
        let res = Arc::new(construct_standard::<false, _, _>("S_3", None).unwrap());
        let max = Bidegree::n_s(20, 6);
        res.compute_through_stem(max);
        let module = ExtModule::intrinsic(res);

        let a0 = Bidegree::n_s(0, 1);
        assert_eq!(module.algebra().dimension(a0), 1);
        let x = module.algebra().element(a0, &[2]);
        check_class_product_map(&module, &x, max);
    }

    /// The zero class has no summands and yields the zero map.
    #[test]
    fn test_class_product_map_zero() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        let max = Bidegree::n_s(20, 9);
        res.compute_through_stem(max);
        let module = ExtModule::intrinsic(res);

        let x = module.algebra().element(Bidegree::n_s(15, 5), &[0, 0]);
        check_class_product_map(&module, &x, max);
    }

    /// Every product of generators, as `(x, y degree, rows of x · basis(y degree))`.
    fn product_table<CC>(
        module: &ExtModule<CC>,
    ) -> Vec<(BidegreeGenerator, Bidegree, Vec<Vec<u32>>)>
    where
        CC: FreeChainComplex + AugmentedChainComplex + 'static,
    {
        let mut table = Vec::new();
        for x_deg in module.resolution().iter_stem() {
            for g in module.basis(x_deg) {
                let x = module.generator(g);
                for b in module.algebra().resolution().iter_nonzero_stem() {
                    if let Some(matrix) = module.multiply_into(&x, b) {
                        let rows = matrix.iter().map(|row| row.iter().collect()).collect();
                        table.push((g, b, rows));
                    }
                }
            }
        }
        table
    }

    /// Without a save directory, Nassau's resolution recomputes each quasi-inverse on demand, so
    /// [`ExtModule::extend_all_products`] works and gives exactly the products read from saved
    /// quasi-inverses.
    #[test]
    fn test_nassau_products_without_save_dir() {
        let max = Bidegree::n_s(14, 6);
        let products = |save_dir: Option<std::path::PathBuf>| {
            let res = Arc::new(crate::utils::construct_nassau("S_2", save_dir).unwrap());
            res.compute_through_stem(max);
            let module = ExtModule::intrinsic(res);
            module.extend_all_products().unwrap();
            product_table(&module)
        };
        let tempdir = tempfile::TempDir::new().unwrap();
        let unsaved = products(None);
        assert!(
            unsaved
                .iter()
                .any(|(_, _, rows)| rows.iter().flatten().any(|&c| c != 0))
        );
        assert_eq!(unsaved, products(Some(tempdir.path().to_owned())));
    }
}
