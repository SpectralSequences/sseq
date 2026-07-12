//! Ext as a bigraded ring and its modules.

pub mod massey;
pub mod secondary;

use std::sync::Arc;

use dashmap::DashMap;
use fp::{
    matrix::{AugmentedMatrix, Matrix, Subquotient, Subspace},
    prime::ValidPrime,
    vector::FpVector,
};
use sseq::coordinates::{Bidegree, BidegreeElement, BidegreeGenerator};

pub use self::secondary::{SecondaryExtAlgebra, SecondaryProduct};
use crate::{
    chain_complex::{AugmentedChainComplex, FreeChainComplex},
    resolution_homomorphism::ResolutionHomomorphism,
    utils::{QueryModuleResolution, get_unit},
};

/// The differential of the Ext DGA: the coboundary on the cochain complex
/// $\Hom(P_\bullet, k) = k^{\text{gens}}$ whose cohomology is the "Ext part" —
/// the next page.
///
/// It shifts bidegree by a fixed [`shift`](ExtDifferential::shift) and, at each
/// bidegree, gives its matrix in the generator bases. Over a field with a
/// *minimal* resolution this differential is identically zero — $d_s$ lands in
/// $\bar A \cdot P_{s-1}$, which every $\varphi\colon P_{s-1} \to k$ kills — so
/// $\Ext$ is just the generators and taking cohomology is a no-op. A deformation
/// (the motivic lift's $\delta$) or a secondary operation (the Adams $d_2$) is
/// what makes it nonzero and the cohomology nontrivial.
pub trait ExtDifferential: Send + Sync {
    /// The fixed bidegree shift the differential applies: $\delta\colon \Ext_b \to
    /// \Ext_{b + \mathrm{shift}}$.
    fn shift(&self) -> Bidegree;

    /// The matrix of $\delta$ out of bidegree `b`: rows index the generators at
    /// `b`, columns the generators at `b + shift`. `None` if the differential out
    /// of `b` is out of the computed range; a computed-but-empty bidegree yields a
    /// valid zero-size matrix, not `None`.
    fn matrix(&self, b: Bidegree) -> Option<Matrix>;

    /// For a **graded** coefficient (e.g. $\mathbb{F}_2[\tau]$, graded by motivic
    /// weight), the number of cochain generators at `b` whose grade is `≤ cap`.
    /// The default — an ungraded (field) coefficient — returns `None`, meaning "no
    /// grading", and the capped cohomology falls back to the full dimension.
    fn graded_dimension(&self, b: Bidegree, cap: i32) -> Option<usize> {
        let _ = (b, cap);
        None
    }

    /// The differential [`matrix`](Self::matrix) restricted to generators of grade
    /// `≤ cap` at both ends (rows and columns compacted to the kept generators).
    /// The default ignores `cap` (ungraded), returning the full matrix.
    ///
    /// A graded implementor **must override this together with
    /// [`graded_dimension`](Self::graded_dimension)** and keep them consistent: the
    /// capped matrix's row count must equal `graded_dimension(b, cap)` (and its
    /// column count `graded_dimension(b + shift, cap)`). Otherwise
    /// [`cohomology_dimension_capped`](ExtAlgebra::cohomology_dimension_capped) mixes
    /// a capped generator count with an uncapped rank.
    fn matrix_capped(&self, b: Bidegree, cap: i32) -> Option<Matrix> {
        let _ = cap;
        self.matrix(b)
    }
}

/// The ring $\Ext(k, k)$, backed by a resolution of the base field `k`.
///
/// A product is realised by a [`ResolutionHomomorphism`] from a fixed multiplier class, which
/// computes the products of that class with all of $\Ext(k, k)$ at once. One such map is cached per
/// generator; a product by a general class is assembled from them on request (see
/// [`class_product_map`](Self::class_product_map)). Products are computed up to a sign.
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

    /// Attach a DGA differential, turning this into the Ext DGA whose cohomology
    /// is the next page (see [`ExtDifferential`] and [`Self::cohomology_dimension`]).
    /// Without one, the cohomology is the field/minimal case — just the generators.
    #[must_use]
    pub fn with_differential(mut self, differential: Arc<dyn ExtDifferential>) -> Self {
        self.differential = Some(differential);
        self
    }

    /// The differential this DGA carries, if any.
    pub fn differential(&self) -> Option<&Arc<dyn ExtDifferential>> {
        self.differential.as_ref()
    }

    /// The dimension of the DGA's cohomology at `b` — the "Ext part":
    /// $\dim H_b = \dim\ker(\delta \text{ out of } b) - \mathrm{rank}(\delta \text{ into } b)
    /// = \mathrm{gens}(b) - \mathrm{rank}\,\delta_{\text{out}}(b) - \mathrm{rank}\,\delta_{\text{in}}(b)$.
    ///
    /// With no differential (a field/minimal resolution, the zero coboundary) this
    /// is exactly the generator count — the cohomology *is* $\Ext$, and "taking
    /// cohomology" degenerates to reading generators. A nonzero differential (the
    /// motivic $\delta$, an Adams $d_2$) makes it a genuine kernel-mod-image.
    ///
    /// Returns `None` if the outgoing differential at `b` is out of the computed
    /// range; a missing incoming differential (no source bidegree, or empty) counts
    /// as rank $0$.
    pub fn cohomology_dimension(&self, b: Bidegree) -> Option<usize> {
        self.cohomology_dimension_capped(b, i32::MAX)
    }

    /// The dimension of the DGA's cohomology at `b` restricted to the coefficient's
    /// weight slice `≤ cap` — for a graded coefficient like $\mathbb{F}_2[\tau]$
    /// this is a slice of the Ext *module*, and sweeping `cap` exposes the
    /// $\tau$-torsion (dimension above the free/`cap = ∞` rank). For an ungraded
    /// (field) coefficient the differential reports no grading and this is just
    /// [`cohomology_dimension`](Self::cohomology_dimension) for every `cap`.
    pub fn cohomology_dimension_capped(&self, b: Bidegree, cap: i32) -> Option<usize> {
        let Some(d) = &self.differential else {
            return Some(self.dimension(b));
        };
        let gens = d
            .graded_dimension(b, cap)
            .unwrap_or_else(|| self.dimension(b));
        let shift = d.shift();
        let source = Bidegree::n_s(b.n() - shift.n(), b.s() - shift.s());
        // The capped matrix must line up with the capped generator count `gens`, or
        // an undersized matrix would understate a rank and overstate the cohomology
        // (matching the shape checks in `cohomology_subquotient`).
        let mut out = d.matrix_capped(b, cap)?;
        assert_eq!(
            out.rows(),
            gens,
            "ExtDifferential::matrix_capped({b:?}, {cap}) must have gens = {gens} rows, got {}",
            out.rows()
        );
        let rank_out = out.row_reduce();
        let rank_in = match d.matrix_capped(source, cap) {
            Some(mut incoming) => {
                assert_eq!(
                    incoming.columns(),
                    gens,
                    "ExtDifferential::matrix_capped({source:?}, {cap}) into {b:?} must have gens \
                     = {gens} columns, got {}",
                    incoming.columns()
                );
                incoming.row_reduce()
            }
            None => 0,
        };
        // ker ⊇ im requires d∘d = 0; a malformed differential could underflow here.
        debug_assert!(
            rank_out + rank_in <= gens,
            "ExtDifferential violates d∘d=0 at {b:?}: rank_out={rank_out}, rank_in={rank_in}, \
             gens={gens}"
        );
        Some(gens - rank_out - rank_in)
    }

    /// The DGA's cohomology at `b` as a [`Subquotient`] of the generators — the
    /// actual kernel-mod-image subspace, so callers get *representatives* of the
    /// surviving classes, not just the [dimension](Self::cohomology_dimension).
    /// The numerator is $\ker(\delta \text{ out of } b)$, the denominator is
    /// $\operatorname{im}(\delta \text{ into } b)$. With no differential attached
    /// every generator survives, so this is the full space.
    ///
    /// `None` if the outgoing differential at `b` is out of the computed range (as
    /// with [`cohomology_dimension`](Self::cohomology_dimension)).
    pub fn cohomology_subquotient(&self, b: Bidegree) -> Option<Subquotient> {
        let p = self.prime();
        let dim = self.dimension(b);
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

        // Denominator: im(δ into b) = row space of δ out of the source bidegree. A
        // well-shaped differential lands in the gens(b)-space (`dim` columns), so its
        // rows are vectors of the right ambient; a missing source is the zero image.
        let shift = d.shift();
        let source = Bidegree::n_s(b.n() - shift.n(), b.s() - shift.s());
        let denominator = match d.matrix(source) {
            Some(m) => {
                assert_eq!(
                    m.columns(),
                    dim,
                    "ExtDifferential::matrix({source:?}) into {b:?} must have gens(b) = {dim} \
                     columns, got {}",
                    m.columns()
                );
                Subspace::from_matrix(m)
            }
            None => Subspace::new(p, dim),
        };

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
    CC: FreeChainComplex + AugmentedChainComplex,
{
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
        // A differential shaped per the `matrix` contract: `gens(b)` rows,
        // `gens(b + shift)` columns (0 off the first quadrant), with the single
        // nonzero d2 entry d(h_0^2) = h_0 at (0,2) → (0,1). Sizing from the real
        // generator counts keeps it a valid reference for `cohomology_subquotient`,
        // not just the rank-only `cohomology_dimension`.
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

        // The shape-correct mock drives `cohomology_subquotient` too: same answers,
        // now with representatives.
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
    fn check_class_product_map<CC: FreeChainComplex + AugmentedChainComplex>(
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
}
