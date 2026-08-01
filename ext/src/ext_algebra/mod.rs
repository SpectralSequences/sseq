//! A bigraded-algebra view of a resolution.
//!
//! [`ExtAlgebra`] wraps a resolution of a module `M` together with the resolution of the base
//! field `k` (the "unit"), and presents $\Ext(M, k)$ as a bigraded module over the bigraded
//! algebra $\Ext(k, k)$. When `M == k` this is the algebra $\Ext(k, k)$ itself.
//!
//! The goal is ergonomics: computing a product of Ext classes is a single [`ExtAlgebra::multiply`]
//! call instead of the manual [`ResolutionHomomorphism`] + `extend` + `hom_k` plumbing that the
//! examples currently re-derive.
//!
//! # Conventions
//! A product is realised by a [`ResolutionHomomorphism`] built from a fixed multiplier class living
//! in $\Ext(M, k)$ (source = resolution of `M`, target = resolution of `k`). That single chain map
//! computes the products of the multiplier with *all* classes of $\Ext(k, k)$. We cache one such
//! map per *generator* of $\Ext(M, k)$ (keyed by [`BidegreeGenerator`]); a product by a general
//! class is assembled at request time as the corresponding linear combination of generator maps.
//!
//! # Cohomology
//! $\Ext(M, k)$ is the cohomology of the cochain complex $\Hom(P_\bullet, k)$. For a **minimal**
//! resolution its coboundary is identically zero, so $\Ext$ is just the generators and taking
//! cohomology is a no-op; for a **non-minimal** resolution the coboundary is the canonical dualised
//! differential $\Hom(d, k)$. Either way this is classical homological algebra — see
//! [`ExtDifferential`] and [`ExtAlgebra::cohomology_subquotient`].
//!
//! The same cochain complex can also host the connecting differential of a *deformation* — the
//! Adams $d_2$ or the motivic $\delta$ — but that story is deliberately kept out of [`ExtAlgebra`]'s
//! own interface: the differentials are supplied by their own modules. The $d_2$ and the
//! $\Mod_{C\lambda^2}$ secondary product live in the [`secondary`] submodule
//! ([`SecondaryExtAlgebra`]).

pub mod massey;
pub mod secondary;
pub mod tensor_resolution;

use std::sync::Arc;

use dashmap::DashMap;
use fp::{
    matrix::{AugmentedMatrix, Matrix, Subquotient, Subspace},
    prime::ValidPrime,
    vector::{FpSlice, FpVector},
};
use sseq::coordinates::{Bidegree, BidegreeElement, BidegreeGenerator};

pub use self::{
    secondary::{SecondaryExtAlgebra, SecondaryProduct},
    tensor_resolution::{
        Antipode, TensorResolutionDifferential, tensor_resolution_ext,
        tensor_resolution_ext_with_save_dir, tensor_resolution_products,
    },
};
use crate::{
    chain_complex::{AugmentedChainComplex, FreeChainComplex},
    resolution_homomorphism::ResolutionHomomorphism,
    utils::{QueryModuleResolution, get_unit},
};

/// The coboundary of the Ext cochain complex $\Hom(P_\bullet, k) = k^{\text{gens}}$,
/// whose cohomology is $\Ext$.
///
/// It shifts bidegree by a fixed [`shift`](ExtDifferential::shift) and, at each
/// bidegree, gives its matrix in the generator bases. For a **minimal** resolution
/// this coboundary is identically zero — $d_s$ lands in $\bar A \cdot P_{s-1}$,
/// which every $\varphi\colon P_{s-1} \to k$ kills — so $\Ext$ is just the
/// generators and taking cohomology is a no-op (no differential is attached). For a
/// **non-minimal** resolution it is the canonical dualised differential
/// $\Hom(d, k)$. Both are classical homological algebra.
///
/// The same trait is *also* how a **deformation** hangs its connecting differential
/// on this complex — the Adams $d_2$ ([`secondary`]) or the motivic $\delta$ — as
/// instances defined in their own modules; those are what
/// [`ExtAlgebra::cohomology_subquotient`] reads to compute the next page of a
/// deformation spectral sequence. That story is kept out of [`ExtAlgebra`]'s own
/// interface on purpose: here the differential is just a pluggable coboundary,
/// deformation or not.
pub trait ExtDifferential: Send + Sync {
    /// The fixed bidegree shift the differential applies: $\delta\colon \Ext_b \to
    /// \Ext_{b + \mathrm{shift}}$.
    fn shift(&self) -> Bidegree;

    /// The matrix of $\delta$ out of bidegree `b`: rows index the generators at
    /// `b`, columns the generators at `b + shift`. `None` if the differential out
    /// of `b` is out of the computed range; a computed-but-empty bidegree yields a
    /// valid zero-size matrix, not `None`.
    fn matrix(&self, b: Bidegree) -> Option<Matrix>;

    /// For a coefficient **graded by a deformation base** $R$ (e.g.
    /// $\mathbb{F}_2[\tau]$ graded by motivic weight), the number of cochain
    /// generators at `b` whose grade is `≤ cap`; sweeping `cap` walks up the
    /// $R$-adic tower and exposes the $R$-torsion of the Ext module. The default —
    /// an ungraded (tensor) coefficient — returns `None`, meaning "no grading", and
    /// the capped cohomology falls back to the full dimension.
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

    /// The number of cochain generators at `b`, when the differential's cochain complex is *not*
    /// the one read off the backing resolution's generators.
    ///
    /// The default `None` means "use the backing resolution's generator count"
    /// ([`ExtAlgebra::dimension`]) — the right thing for the minimal/secondary case, where the
    /// cochain generators *are* the resolution's generators. A differential whose complex lives
    /// elsewhere overrides this: the untwisted tensor resolution coboundary
    /// ([`tensor_resolution::TensorResolutionDifferential`]) resolves a *different* module `M` by
    /// tensoring the backing $k$-resolution with `M`, so its cochain generators at
    /// $(n, s)$ number $\sum_i \dim M_{t - d_i}$ (the indecomposables of $P_s \otimes M$), not the
    /// $k$-resolution's `number_of_gens_in_bidegree`.
    fn dimension(&self, b: Bidegree) -> Option<usize> {
        let _ = b;
        None
    }
}

/// A cochain-level **cup product** $x \cup -$ by a class $x \in \Ext(k, k)$, on the cochain complex
/// $\Hom_A(Q_\bullet, k)$.
///
/// This is what lets [`ExtAlgebra::massey`] compute Massey products on a **non-minimal**
/// resolution: the chain-map/null-homotopy bracket degenerates there (the lifted product map can be
/// forced to zero), but the dual cochain-DGA formula $\langle a,b,c\rangle = [a \cup v]$, $\delta_Q
/// v = b\cup c$, stays correct precisely because $\delta_Q \neq 0$. It is attached by the untwisted
/// tensor resolution ([`tensor_resolution_products`]); a minimal resolution attaches none and
/// [`ExtAlgebra::massey`] falls back to the chain-map construction. Like [`ExtDifferential`], the
/// cup is a pluggable structure supplied by whoever builds the [`ExtAlgebra`].
pub trait CochainCup<CC: FreeChainComplex>: Send + Sync {
    /// The matrix of $x \cup -\colon \Hom_A(Q_s, k)_t \to \Hom_A(Q_{s + \mathrm{shift}.s}, k)_{t +
    /// \mathrm{shift}.t}$: rows index the source cochain basis at `(src_s, src_t)`, columns the
    /// target at `(src_s + shift.s(), src_t + shift.t())`. `f_x` is the chain self-map of the unit
    /// realising `x` (its [`shift`](ResolutionHomomorphism) is `x`'s bidegree), extended through
    /// the target filtration.
    fn cup_matrix(
        &self,
        f_x: &ResolutionHomomorphism<CC, CC>,
        src_s: i32,
        src_t: i32,
        shift: Bidegree,
    ) -> Matrix;
}

/// A cochain of the Ext cochain complex $\Hom_A(P_\bullet, k)$ at a bidegree: a vector over the
/// **cochain generators** (dual to the resolution's free generators).
///
/// This is the native "cochain" world, distinct from [`BidegreeElement`], which `Ext*` interprets
/// as a cohomology **class**. The two coincide only when the coboundary vanishes (a minimal
/// resolution); in general convert with [`ExtAlgebra::lift`] (class → cocycle representative) and
/// [`ExtAlgebra::project`] (cochain → class).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cochain {
    degree: Bidegree,
    vec: FpVector,
}

impl Cochain {
    pub fn new(degree: Bidegree, vec: FpVector) -> Self {
        Self { degree, vec }
    }

    pub fn degree(&self) -> Bidegree {
        self.degree
    }

    pub fn vec(&self) -> FpSlice<'_> {
        self.vec.as_slice()
    }

    pub fn into_vec(self) -> FpVector {
        self.vec
    }
}

/// $\Ext(M, k)$ as a bigraded module over the bigraded algebra $\Ext(k, k)$, backed by a
/// resolution. See the [module-level documentation](self) for conventions.
///
/// The unit resolution is held separately from the resolution of `M`, but both have the same type:
/// products and Massey maps always *target* the unit, so it must be an [`AugmentedChainComplex`],
/// while the resolution is only ever a *source*. The untwisted tensor resolution never needs them
/// to differ — its products go through the closed-form [`CochainCup`], which builds no chain map
/// out of the non-minimal $Q_ullet$ at all.
pub struct ExtAlgebra<CC: FreeChainComplex> {
    /// Resolution of `M`; products land in its Ext.
    resolution: Arc<CC>,
    /// Resolution of the base field `k`. `Arc`-shared with `resolution` when `M == k` (only
    /// possible when the two are the same `Arc`).
    unit: Arc<CC>,
    is_unit: bool,
    /// One multiplication map per generator of $\Ext(M, k)$, built and extended on demand. Used by
    /// the chain-map product path (minimal resolutions).
    products: DashMap<BidegreeGenerator, Arc<ResolutionHomomorphism<CC, CC>>>,
    /// One chain self-map of the unit per generator of $\Ext(k, k)$, built on demand. This is the
    /// single self-map cache: the **cup** product path (the closed-form untwisted tensor
    /// resolution) and *every* Massey bracket read their cup matrices off these maps (via
    /// [`cup_class_matrix`](Self::cup_class_matrix)), realising a general class as a linear
    /// combination of the per-generator maps rather than building a separate map per class. Because
    /// they all share one source keyed by the generator, they are also unified on disk (saved once,
    /// under the same `prod_{n}_{s}_{idx}` name as the unit's own product maps).
    unit_maps: DashMap<BidegreeGenerator, Arc<ResolutionHomomorphism<CC, CC>>>,
    /// Cup matrices `{y_j ∪ -}` keyed by `(shift, b)` (the multiplier and multiplicand bidegrees).
    /// They depend only on those two bidegrees, not on the multiplier's coordinates, so caching
    /// them collapses the per-generator recomputation a Massey kernel would otherwise do.
    cup_cache: DashMap<(Bidegree, Bidegree), Arc<Vec<Matrix>>>,
    /// Quasi-inverse of the coboundary `δ_Q` out of each bidegree, memoised. The Massey
    /// `δ`-preimage is intrinsic to `δ_Q` (not to the bracket factors), so this row-reduction is
    /// shared across every third factor rather than repeated per bracket.
    delta_qi_cache: DashMap<Bidegree, Arc<fp::matrix::QuasiInverse>>,
    /// The DGA differential, if any. `None` is the minimal case (zero
    /// coboundary), where the cohomology is just the generators.
    differential: Option<Arc<dyn ExtDifferential>>,
    /// The cochain cup product $x \cup -$, if any. Attached for a **non-minimal** resolution (the
    /// untwisted tensor resolution) so [`massey`](Self::massey) uses the cochain-DGA bracket;
    /// `None` for a minimal resolution, where [`massey`](Self::massey) uses the
    /// chain-map/null-homotopy bracket.
    cup: Option<Arc<dyn CochainCup<CC>>>,
    /// Memoised cohomology space (of the attached [`differential`](Self::differential)) at each
    /// bidegree — the vector space whose basis `Ext*` exposes. See [`Self::cohomology`].
    cohomology_cache: DashMap<Bidegree, Arc<Subquotient>>,
}

impl ExtAlgebra<QueryModuleResolution> {
    /// Build an [`ExtAlgebra`] from a resolution, deriving the unit via [`get_unit`].
    ///
    /// This may prompt for the unit's save directory when `M != k` (see [`get_unit`]); for a fully
    /// non-interactive setup, use [`ExtAlgebra::new`] with an explicit unit instead.
    pub fn from_resolution(resolution: Arc<QueryModuleResolution>) -> anyhow::Result<Self> {
        let (_, unit) = get_unit(Arc::clone(&resolution))?;
        Ok(Self::new(resolution, unit))
    }

    /// Ensure both the resolution and the unit are computed through the given stem.
    pub fn compute_through_stem(&self, max: Bidegree) {
        self.unit.compute_through_stem(max);
        if !self.is_unit {
            self.resolution.compute_through_stem(max);
        }
    }
}

impl<CC: FreeChainComplex> ExtAlgebra<CC> {
    /// Build an [`ExtAlgebra`] from an explicit `(resolution, unit)` pair of the **same** type.
    pub fn new(resolution: Arc<CC>, unit: Arc<CC>) -> Self {
        assert_eq!(resolution.prime(), unit.prime());
        Self {
            is_unit: Arc::ptr_eq(&resolution, &unit),
            resolution,
            unit,
            products: DashMap::new(),
            unit_maps: DashMap::new(),
            cup_cache: DashMap::new(),
            delta_qi_cache: DashMap::new(),
            differential: None,
            cup: None,
            cohomology_cache: DashMap::new(),
        }
    }

    /// Build an [`ExtAlgebra`] for resolution-*intrinsic* operations that do not involve products
    /// (notably the secondary `d2` differential), using the resolution itself in place of a unit.
    ///
    /// This avoids the unit-resolution setup (and any associated prompt) that
    /// [`from_resolution`](Self::from_resolution) performs. The product methods
    /// ([`multiply`](Self::multiply) etc.) and the unit-side queries are only meaningful here when
    /// `M == k`; for products with `M != k`, build with [`from_resolution`](Self::from_resolution)
    /// or [`new`](Self::new) instead.
    pub fn without_unit(resolution: Arc<CC>) -> Self {
        Self::new(Arc::clone(&resolution), resolution)
    }
}

impl<CC> ExtAlgebra<CC>
where
    CC: FreeChainComplex,
{
    /// Attach a cochain cup product, so [`massey`](Self::massey) uses the cochain-DGA bracket
    /// (correct on a non-minimal resolution) instead of the chain-map/null-homotopy one. See
    /// [`CochainCup`].
    #[must_use]
    pub fn with_cup(mut self, cup: Arc<dyn CochainCup<CC>>) -> Self {
        self.cup = Some(cup);
        self.cup_cache.clear();
        self
    }

    /// The cochain cup product this carries, if any.
    pub fn cup(&self) -> Option<&Arc<dyn CochainCup<CC>>> {
        self.cup.as_ref()
    }

    /// Attach a DGA differential, turning this into the Ext DGA whose cohomology
    /// is the next page (see [`ExtDifferential`] and [`Self::cohomology_dimension`]).
    /// Without one, the cohomology is the minimal case — just the generators.
    #[must_use]
    pub fn with_differential(mut self, differential: Arc<dyn ExtDifferential>) -> Self {
        self.differential = Some(differential);
        // Every cache below is derived from the differential: the cohomology subquotients, and the
        // memoised quasi-inverses of `δ` used to solve `δv = b∪c` (keyed by degree alone).
        self.cohomology_cache.clear();
        self.delta_qi_cache.clear();
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
    /// With no differential (a minimal resolution, the zero coboundary) this
    /// is exactly the generator count — the cohomology *is* $\Ext$, and "taking
    /// cohomology" degenerates to reading generators. A nonzero differential (a
    /// non-minimal resolution's $\Hom(d, k)$, or a deformation's connecting map)
    /// makes it a genuine kernel-mod-image.
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
    /// (tensor) coefficient the differential reports no grading and this is just
    /// [`cohomology_dimension`](Self::cohomology_dimension) for every `cap`.
    pub fn cohomology_dimension_capped(&self, b: Bidegree, cap: i32) -> Option<usize> {
        let Some(d) = &self.differential else {
            return Some(self.cochain_dimension(b));
        };
        let gens = d
            .graded_dimension(b, cap)
            .or_else(|| d.dimension(b))
            .unwrap_or_else(|| self.cochain_dimension(b));
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
        // ker ⊇ im requires d∘d = 0. `ExtDifferential` is a public trait, so a malformed
        // implementor can violate it; assert rather than wrap around to a ~2^64 "dimension".
        Some(gens.checked_sub(rank_out + rank_in).unwrap_or_else(|| {
            panic!(
                "ExtDifferential violates d∘d=0 at {b:?}: rank_out={rank_out}, rank_in={rank_in}, \
                 gens={gens}"
            )
        }))
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
        self.cohomology(b).map(|h| (*h).clone())
    }

    /// The cohomology space at `b` (of the attached [`differential`](Self::differential)) — the
    /// vector space whose basis `Ext*` exposes as `Ext`, memoised and `Arc`-shared. This is the
    /// canonical object behind [`dimension`](Self::dimension), [`basis`](Self::basis),
    /// [`lift`](Self::lift) and [`project`](Self::project): its `dimension()` is the Ext dimension
    /// and its `gens()` are cocycle representatives of the basis classes.
    ///
    /// `None` when the differential out of `b` is out of the computed range.
    pub fn cohomology(&self, b: Bidegree) -> Option<Arc<Subquotient>> {
        if let Some(h) = self.cohomology_cache.get(&b) {
            return Some(Arc::clone(&h));
        }
        let h = Arc::new(self.compute_cohomology(b)?);
        Some(Arc::clone(
            self.cohomology_cache.entry(b).or_insert(h).value(),
        ))
    }

    fn compute_cohomology(&self, b: Bidegree) -> Option<Subquotient> {
        let p = self.prime();
        let Some(d) = &self.differential else {
            return Some(Subquotient::new_full(p, self.cochain_dimension(b)));
        };
        let dim = self.cochain_dimension(b);

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

    pub fn resolution(&self) -> &Arc<CC> {
        &self.resolution
    }

    pub fn unit(&self) -> &Arc<CC> {
        &self.unit
    }

    pub fn is_unit(&self) -> bool {
        self.is_unit
    }

    pub fn prime(&self) -> ValidPrime {
        self.resolution.prime()
    }

    /// Ensure both the resolution and the unit are computed through the given bidegree.
    pub fn compute_through_bidegree(&self, b: Bidegree) {
        self.unit.compute_through_bidegree(b);
        if !self.is_unit {
            self.resolution.compute_through_bidegree(b);
        }
    }

    /// The dimension of $\Ext^{s,t}(M, k)$ at the given bidegree — the dimension of the
    /// **cohomology** of the attached [`differential`](Self::differential) (its Ext part). With no
    /// differential (a minimal resolution) this is the generator count, since every cochain is a
    /// cocycle; with one (the untwisted tensor resolution, or a $d_2$ page) it is a genuine
    /// kernel-mod-image. Returns `0` when out of the computed range.
    pub fn dimension(&self, b: Bidegree) -> usize {
        self.cohomology_dimension(b).unwrap_or(0)
    }

    /// The number of **cochain generators** at `b` — the ambient dimension of the cochain group
    /// $\Hom_A(P_\bullet, k)_b$ that the cohomology is a subquotient of. This is the "generator"
    /// world (as opposed to the cohomology-class world of [`dimension`](Self::dimension)); the two
    /// coincide exactly when the coboundary is zero (a minimal resolution).
    pub fn cochain_dimension(&self, b: Bidegree) -> usize {
        self.differential
            .as_ref()
            .and_then(|d| d.dimension(b))
            .unwrap_or_else(|| {
                // Panic-safe: an uncomputed bidegree has no cochains (rather than indexing past the
                // resolution). This lets callers sweep `iter_stem` without a prior range check.
                if b.s() >= 0 && self.resolution.has_computed_bidegree(b) {
                    self.resolution.number_of_gens_in_bidegree(b)
                } else {
                    0
                }
            })
    }

    /// The basis classes of $\Ext(M, k)$ at the given bidegree — one [`BidegreeGenerator`] per
    /// basis element of the cohomology.
    pub fn basis(&self, b: Bidegree) -> Vec<BidegreeGenerator> {
        (0..self.dimension(b))
            .map(|i| BidegreeGenerator::new(b, i))
            .collect()
    }

    /// A class in $\Ext(M, k)$ from its coordinates in the **cohomology** basis at bidegree `b`.
    pub fn element(&self, b: Bidegree, coords: &[u32]) -> BidegreeElement {
        assert_eq!(self.dimension(b), coords.len());
        BidegreeElement::new(b, FpVector::from_slice(self.prime(), coords))
    }

    /// A single basis class of $\Ext(M, k)$.
    pub fn generator(&self, g: BidegreeGenerator) -> BidegreeElement {
        let ambient = self.dimension(g.degree());
        assert!(ambient > g.idx());
        g.into_element(self.prime(), ambient)
    }

    /// Lift a cohomology **class** to a **cocycle representative** — a [`Cochain`] whose class is
    /// `x`. Concretely $\sum_i x_i \cdot g_i$ over the cocycle representatives
    /// `cohomology(b).gens()`. Inverse to [`project`](Self::project) up to a coboundary; on a
    /// minimal resolution (zero coboundary) it is the identity.
    pub fn lift(&self, x: &BidegreeElement) -> Cochain {
        let b = x.degree();
        let h = self
            .cohomology(b)
            .expect("lift: bidegree out of computed range");
        let mut vec = FpVector::new(self.prime(), h.ambient_dimension());
        for (i, c) in x.vec().iter_nonzero() {
            vec.as_slice_mut()
                .add(h.gens().nth(i).expect("class coordinate out of range"), c);
        }
        Cochain::new(b, vec)
    }

    /// Project a **cochain** to its cohomology **class** by reducing modulo coboundaries and
    /// reading coordinates in the cohomology basis. On a minimal resolution it is the identity. (A
    /// cochain with a non-cocycle part is reduced regardless; pass a cocycle for a meaningful
    /// class.)
    pub fn project(&self, c: &Cochain) -> BidegreeElement {
        let b = c.degree();
        let h = self
            .cohomology(b)
            .expect("project: bidegree out of computed range");
        let mut v = FpVector::new(self.prime(), h.ambient_dimension());
        v.as_slice_mut().add(c.vec(), 1);
        let coords = h.reduce(v.as_slice_mut());
        BidegreeElement::new(b, FpVector::from_slice(self.prime(), &coords))
    }

    /// The quasi-inverse of the coboundary `δ_Q` out of `v_deg` (given its matrix `d`, `r × c`),
    /// memoised. Used by the Massey `δ`-preimage; `δ_Q` is intrinsic, so the row reduction is done
    /// once and reused across every bracket landing at `v_deg`.
    pub(crate) fn delta_quasi_inverse(
        &self,
        v_deg: Bidegree,
        d: &Matrix,
        r: usize,
        c: usize,
    ) -> Arc<fp::matrix::QuasiInverse> {
        if let Some(q) = self.delta_qi_cache.get(&v_deg) {
            return Arc::clone(&q);
        }
        let mut aug = AugmentedMatrix::<2>::new(self.prime(), r, [c, r]);
        for i in 0..r {
            aug.row_mut(i).slice_mut(0, c).add(d.row(i), 1);
        }
        aug.segment(1, 1).add_identity();
        aug.row_reduce();
        let qi = Arc::new(aug.compute_quasi_inverse());
        Arc::clone(self.delta_qi_cache.entry(v_deg).or_insert(qi).value())
    }

    /// A [`Cochain`] from its coordinates in the cochain-generator basis at bidegree `b`.
    pub fn cochain_element(&self, b: Bidegree, coords: &[u32]) -> Cochain {
        assert_eq!(self.cochain_dimension(b), coords.len());
        Cochain::new(b, FpVector::from_slice(self.prime(), coords))
    }

    /// A single cochain generator (a basis element of the cochain group), as a [`Cochain`].
    pub fn cochain_generator(&self, g: BidegreeGenerator) -> Cochain {
        let ambient = self.cochain_dimension(g.degree());
        assert!(ambient > g.idx());
        Cochain::new(g.degree(), g.into_element(self.prime(), ambient).into_vec())
    }

    /// The dimension of $\Ext(k, k)$ at the given bidegree (the multiplicand/"scalar" side).
    pub fn unit_dimension(&self, b: Bidegree) -> usize {
        self.unit.number_of_gens_in_bidegree(b)
    }

    /// The basis generators of $\Ext(k, k)$ at the given bidegree.
    pub fn unit_basis(&self, b: Bidegree) -> Vec<BidegreeGenerator> {
        (0..self.unit_dimension(b))
            .map(|i| BidegreeGenerator::new(b, i))
            .collect()
    }

    /// A class in $\Ext(k, k)$ from its coordinates in the generator basis at bidegree `b`.
    pub fn unit_element(&self, b: Bidegree, coords: &[u32]) -> BidegreeElement {
        assert_eq!(self.unit_dimension(b), coords.len());
        BidegreeElement::new(b, FpVector::from_slice(self.prime(), coords))
    }

    /// A single generator of $\Ext(k, k)$ as a class.
    pub fn unit_generator(&self, g: BidegreeGenerator) -> BidegreeElement {
        let ambient = self.unit_dimension(g.degree());
        assert!(ambient > g.idx());
        g.into_element(self.prime(), ambient)
    }
}

impl<CC> ExtAlgebra<CC>
where
    CC: FreeChainComplex + AugmentedChainComplex,
{
    /// The multiplication map for a single generator `g` of $\Ext(M, k)$, built and cached on
    /// first use. The returned map is *not* guaranteed to be extended; [`ExtAlgebra::multiply_into`]
    /// extends it as needed.
    ///
    /// The map's source is the (possibly non-minimal) resolution of `M` and its target is the unit
    /// unit resolution; only the latter must be an [`AugmentedChainComplex`]. When
    pub fn generator_product_map(
        &self,
        g: BidegreeGenerator,
    ) -> Arc<ResolutionHomomorphism<CC, CC>> {
        if let Some(map) = self.products.get(&g) {
            return Arc::clone(&map);
        }

        let dim = self.resolution.number_of_gens_in_bidegree(g.degree());
        let mut class = vec![0u32; dim];
        class[g.idx()] = 1;

        let name = format!("prod_{}_{}_{}", g.n(), g.s(), g.idx());
        let hom = Arc::new(ResolutionHomomorphism::from_class(
            name,
            Arc::clone(&self.resolution),
            Arc::clone(&self.unit),
            g.degree(),
            &class,
        ));

        Arc::clone(self.products.entry(g).or_insert(hom).value())
    }

    /// The chain self-map $f_y\colon P_\bullet \to P_\bullet$ of the unit realising a generator `g`
    /// of $\Ext(k, k)$, built and cached on first use. Used by the closed-form cup product path.
    ///
    /// This is *the same map* the minimal product path builds for the unit's own generators via
    /// [`generator_product_map`](Self::generator_product_map): both are `from_class` on the unit
    /// realising `g`, so they must share the on-disk save name (`prod_{n}_{s}_{idx}`). That sharing
    /// is what lets a module resolved by the untwisted tensor resolution reuse the unit's
    /// already-computed self-maps: once the unit's product ring has been saved to disk, every
    /// module's cup path *reloads* the lifts instead of recomputing them (the expensive,
    /// `M`-independent part), leaving only the closed-form cup untwist per module.
    pub(crate) fn unit_self_map(
        &self,
        g: BidegreeGenerator,
    ) -> Arc<ResolutionHomomorphism<CC, CC>> {
        if let Some(map) = self.unit_maps.get(&g) {
            return Arc::clone(&map);
        }
        let dim = self.unit.number_of_gens_in_bidegree(g.degree());
        let mut class = vec![0u32; dim];
        class[g.idx()] = 1;
        let name = format!("prod_{}_{}_{}", g.n(), g.s(), g.idx());
        let hom = Arc::new(ResolutionHomomorphism::from_class(
            name,
            Arc::clone(&self.unit),
            Arc::clone(&self.unit),
            g.degree(),
            &class,
        ));
        Arc::clone(self.unit_maps.entry(g).or_insert(hom).value())
    }

    /// The cup matrices `{y_j ∪ -}` for each generator `y_j` of $\Ext(k, k)$ at `b`, mapping
    /// cochains at `shift` to cochains at `target = b + shift`. Cached by `(shift, b)` since they
    /// are independent of the multiplier's coordinates.
    fn cup_matrices(
        &self,
        cup: &Arc<dyn CochainCup<CC>>,
        src: Bidegree,
        mult_deg: Bidegree,
        target: Bidegree,
        unit_dim: usize,
    ) -> Arc<Vec<Matrix>> {
        if let Some(m) = self.cup_cache.get(&(src, mult_deg)) {
            return Arc::clone(&m);
        }
        let mats: Vec<Matrix> = (0..unit_dim)
            .map(|j| {
                let f_y = self.unit_self_map(BidegreeGenerator::new(mult_deg, j));
                f_y.extend_through_stem(target);
                cup.cup_matrix(&f_y, src.s(), src.t(), mult_deg)
            })
            .collect();
        Arc::clone(
            self.cup_cache
                .entry((src, mult_deg))
                .or_insert_with(|| Arc::new(mats))
                .value(),
        )
    }

    /// The cup matrix `x ∪ -` for a general class `x` of $\Ext(k, k)$, mapping cochains at `src` to
    /// cochains at `src + x.degree()`. Built by combining the cached per-generator cup matrices
    /// (`x ∪ - = Σ x_j (y_j ∪ -)`), so the expensive cup-matrix builds are memoised in `cup_cache`
    /// and shared with the product path (and, once persisted, with disk); only the linear combine
    /// is per call. Used by the closed-form Massey sweep for both the `b ∪ c` and `a ∪ v` cochains.
    pub(crate) fn cup_class_matrix(
        &self,
        cup: &Arc<dyn CochainCup<CC>>,
        x: &BidegreeElement,
        src: Bidegree,
    ) -> Matrix {
        let x_deg = x.degree();
        let target = src + x_deg;
        let unit_dim = self.unit.number_of_gens_in_bidegree(x_deg);
        let mats = self.cup_matrices(cup, src, x_deg, target, unit_dim);
        let rows = mats.first().map_or(0, Matrix::rows);
        let cols = mats.first().map_or(0, Matrix::columns);
        let mut out = Matrix::new(self.prime(), rows, cols);
        for (j, coef) in x.vec().iter_nonzero() {
            for i in 0..rows {
                out.row_mut(i).add(mats[j].row(i), coef);
            }
        }
        out
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
        let shift = x.degree();
        let target = b + shift;

        if !self.unit.has_computed_bidegree(b) || !self.resolution.has_computed_bidegree(target) {
            return None;
        }
        // Both ends of the transport must have a computable cohomology (they coincide with the
        // cochain space, so are always available, on a minimal resolution).
        self.cohomology(shift)?;
        self.cohomology(target)?;

        // Realise `x` as a cocycle in the cochain basis; on a minimal resolution `lift` is the
        // identity.
        let x_cochain = self.lift(x);
        let unit_dim = self.unit.number_of_gens_in_bidegree(b);

        // Closed-form untwisted tensor resolution: read each product `x·y_j = project(y_j ∪ x)` off
        // the cup, with no chain-map lift on a materialised `Q•` — cochain-sized work only. The cup
        // matrices depend only on `(shift, b)`, so they are cached and shared across every
        // multiplier at `shift`.
        if let Some(cup) = &self.cup {
            let cochain_cols = self.cochain_dimension(target);
            let cohomology_cols = self.dimension(target);
            let cup_mats = self.cup_matrices(cup, shift, b, target, unit_dim);
            let mut matrix = Matrix::new(self.prime(), unit_dim, cohomology_cols);
            for (j, cup_mat) in cup_mats.iter().enumerate() {
                let mut out = FpVector::new(self.prime(), cochain_cols);
                cup_mat.apply(out.as_slice_mut(), 1, x_cochain.vec());
                let class = self.project(&Cochain::new(target, out));
                matrix.row_mut(j).add(class.vec(), 1);
            }
            return Some(matrix);
        }

        let cochain_cols = self.cochain_dimension(target);
        // The products in the cochain (resolution-generator) basis of `target`.
        let mut raw = Matrix::new(self.prime(), unit_dim, cochain_cols);

        for (i, c) in x_cochain.vec().iter_nonzero() {
            let map = self.generator_product_map(BidegreeGenerator::new(shift, i));
            // Extend only as far as this product needs (`target`), not to the resolved edge. A
            // Nassau substrate keeps its quasi-inverse one filtration *behind* the edge (the QI at
            // filtration `s` inverts `d: s+1 → s`, which does not exist at the top), so
            // `extend_all` would request a quasi-inverse the substrate never saved. `target` stays
            // within the resolved-with-margin range, matching the closed-form cup path above.
            map.extend_through_stem(target);

            // `hom_k(b.t())[j][k]`: `j` indexes the multiplicand generator of the unit at `b`, `k`
            // indexes the result cochain generator of the resolution at `target`.
            let hom_k = map.get_map(target.s()).hom_k(b.t());
            for (j, row) in hom_k.iter().enumerate() {
                for (k, &v) in row.iter().enumerate() {
                    raw.row_mut(j).add_basis_element(k, c * v);
                }
            }
        }

        // Project each product cochain to its cohomology class, so the returned matrix is in the
        // Ext (cohomology) basis of `target`.
        let cohomology_cols = self.dimension(target);
        let mut matrix = Matrix::new(self.prime(), unit_dim, cohomology_cols);
        for j in 0..unit_dim {
            let class = self.project(&Cochain::new(target, raw.row(j).to_owned()));
            matrix.row_mut(j).add(class.vec(), 1);
        }
        Some(matrix)
    }

    /// The product `x · y` if it lies in the computed range, else `None`. See
    /// [`multiply_into`](Self::multiply_into) for the operand conventions. The result lies in
    /// bidegree `x.degree() + y.degree()`.
    pub fn try_multiply(
        &self,
        x: &BidegreeElement,
        y: &BidegreeElement,
    ) -> Option<BidegreeElement> {
        let target = x.degree() + y.degree();
        let matrix = self.multiply_into(x, y.degree())?;
        let mut out = FpVector::new(self.prime(), matrix.columns());
        for (j, c) in y.vec().iter_nonzero() {
            out.as_slice_mut().add(matrix.row(j), c);
        }
        Some(BidegreeElement::new(target, out))
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

#[cfg(test)]
mod tests {
    use fp::prime::TWO;

    use super::*;
    use crate::{chain_complex::ChainComplex, utils::construct_standard};

    #[test]
    fn test_zero_differential_cohomology_is_generators() {
        // The minimal case: with no differential the DGA cohomology is just
        // the generators — "taking the Ext" is a no-op.
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(8, 8));
        let alg = ExtAlgebra::new(Arc::clone(&res), res);
        for s in 0..=8 {
            for n in 0..=8 {
                let b = Bidegree::n_s(n, s);
                // No differential: cohomology = cochain generators, so the class world and the
                // cochain world coincide.
                assert_eq!(alg.cohomology_dimension(b), Some(alg.cochain_dimension(b)));
                assert_eq!(alg.dimension(b), alg.cochain_dimension(b));
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
        let alg = ExtAlgebra::new(Arc::clone(&res), Arc::clone(&res))
            .with_differential(Arc::new(MockDiff { dims }));

        // Sanity: all three source bidegrees are 1-dimensional on the E-page (the cochain level;
        // `dimension` itself is now the cohomology, computed below).
        assert_eq!(alg.cochain_dimension(Bidegree::n_s(0, 1)), 1); // h_0
        assert_eq!(alg.cochain_dimension(Bidegree::n_s(0, 2)), 1); // h_0^2
        assert_eq!(alg.cochain_dimension(Bidegree::n_s(1, 1)), 1); // h_1

        // `dimension` is the cohomology: the two killed classes drop to 0, h_1 survives.
        assert_eq!(alg.dimension(Bidegree::n_s(0, 2)), 0);
        assert_eq!(alg.dimension(Bidegree::n_s(0, 1)), 0);
        assert_eq!(alg.dimension(Bidegree::n_s(1, 1)), 1);

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
        let alg = ExtAlgebra::new(Arc::clone(&res), res);

        // h_i live in Ext^{1, *}: h_0 = (n=0, s=1), h_1 = (n=1, s=1), h_2 = (n=3, s=1).
        let h0 = alg.generator(BidegreeGenerator::new(Bidegree::n_s(0, 1), 0));
        let h1 = alg.generator(BidegreeGenerator::new(Bidegree::n_s(1, 1), 0));

        // h_0^2 is the nonzero generator of Ext^{2,2} = (n=0, s=2).
        let h0_sq = alg.multiply(&h0, &h0);
        assert_eq!(h0_sq.degree(), Bidegree::n_s(0, 2));
        assert_eq!(alg.dimension(Bidegree::n_s(0, 2)), 1);
        assert!(!h0_sq.vec().is_zero(), "h_0^2 should be nonzero");

        // The Adams relations h_0 h_1 = 0 = h_1 h_0.
        assert!(
            alg.multiply(&h0, &h1).vec().is_zero(),
            "h_0 h_1 should vanish"
        );
        assert!(
            alg.multiply(&h1, &h0).vec().is_zero(),
            "h_1 h_0 should vanish"
        );

        // Cross-check `multiply` against a direct `hom_k` read for h_0 · h_1.
        let rows = alg
            .multiply_into(&h0, h1.degree())
            .expect("h_0 · h_1 is in range");
        let direct: u32 = rows.row(0).iter().sum();
        assert_eq!(direct, 0);
    }
}
