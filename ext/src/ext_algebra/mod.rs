//! A bigraded-algebra view of a resolution.
//!
//! [`ExtAlgebra`] wraps a resolution of a module `M` together with the resolution of the base
//! field `k` (the "unit"), and presents $\Ext(M, k)$ as a bigraded module over the bigraded
//! algebra $\Ext(k, k)$. When `M == k` this is the algebra $\Ext(k, k)$ itself.
//!
//! The goal is ergonomics: computing a product of Ext classes is a single [`ExtAlgebra::multiply`]
//! call instead of the manual [`ResolutionHomomorphism`] + `extend` + `hom_k` plumbing that the
//! examples currently re-derive. This is the foundational layer; the secondary differential ($d_2$)
//! and Massey products are planned follow-ups.
//!
//! # Conventions
//! A product is realised by a [`ResolutionHomomorphism`] built from a fixed multiplier class living
//! in $\Ext(M, k)$ (source = resolution of `M`, target = resolution of `k`). That single chain map
//! computes the products of the multiplier with *all* classes of $\Ext(k, k)$. We cache one such
//! map per *generator* of $\Ext(M, k)$ (keyed by [`BidegreeGenerator`]); a product by a general
//! class is assembled at request time as the corresponding linear combination of generator maps.
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
    fn matrix_capped(&self, b: Bidegree, cap: i32) -> Option<Matrix> {
        let _ = cap;
        self.matrix(b)
    }
}

/// $\Ext(M, k)$ as a bigraded module over the bigraded algebra $\Ext(k, k)$, backed by a
/// resolution. See the [module-level documentation](self) for conventions.
pub struct ExtAlgebra<CC: FreeChainComplex> {
    /// Resolution of `M`; products land in its Ext.
    resolution: Arc<CC>,
    /// Resolution of the base field `k`. `Arc`-shared with `resolution` when `M == k`.
    unit: Arc<CC>,
    is_unit: bool,
    /// One multiplication map per generator of $\Ext(M, k)$, built and extended on demand.
    products: DashMap<BidegreeGenerator, Arc<ResolutionHomomorphism<CC, CC>>>,
    /// The DGA differential, if any. `None` is the field/minimal case (zero
    /// coboundary), where the cohomology is just the generators.
    differential: Option<Arc<dyn ExtDifferential>>,
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
    /// Build an [`ExtAlgebra`] from an explicit `(resolution, unit)` pair.
    pub fn new(resolution: Arc<CC>, unit: Arc<CC>) -> Self {
        assert_eq!(resolution.prime(), unit.prime());
        Self {
            is_unit: Arc::ptr_eq(&resolution, &unit),
            resolution,
            unit,
            products: DashMap::new(),
            differential: None,
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
        let rank_out = d.matrix_capped(b, cap)?.row_reduce();
        let rank_in = d
            .matrix_capped(source, cap)
            .map_or(0, |mut m| m.row_reduce());
        Some(gens - rank_out - rank_in)
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
        g.into_element(self.prime(), self.dimension(g.degree()))
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

        let unit_dim = self.unit.number_of_gens_in_bidegree(b);
        let res_dim = self.resolution.number_of_gens_in_bidegree(target);
        let mut matrix = Matrix::new(self.prime(), unit_dim, res_dim);

        for (i, c) in x.vec().iter_nonzero() {
            let map = self.generator_product_map(BidegreeGenerator::new(shift, i));
            map.extend_all();

            // `hom_k(b.t())[j][k]`: `j` indexes the multiplicand generator of the unit at `b`, `k`
            // indexes the result generator of the resolution at `target`.
            let hom_k = map.get_map(target.s()).hom_k(b.t());
            for (j, row) in hom_k.iter().enumerate() {
                for (k, &v) in row.iter().enumerate() {
                    matrix.row_mut(j).add_basis_element(k, c * v);
                }
            }
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
    use crate::utils::construct_standard;

    #[test]
    fn test_zero_differential_cohomology_is_generators() {
        // The field/minimal case: with no differential the DGA cohomology is just
        // the generators — "taking the Ext" is a no-op.
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(8, 8));
        let alg = ExtAlgebra::new(Arc::clone(&res), res);
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
        struct MockDiff;
        impl ExtDifferential for MockDiff {
            fn shift(&self) -> Bidegree {
                Bidegree::n_s(0, -1) // lowers filtration: (0,2) -> (0,1)
            }
            fn matrix(&self, b: Bidegree) -> Option<Matrix> {
                if b == Bidegree::n_s(0, 2) {
                    let mut m = Matrix::new(TWO, 1, 1);
                    m.row_mut(0).set_entry(0, 1);
                    Some(m)
                } else {
                    Some(Matrix::new(TWO, 0, 0)) // rank 0 elsewhere
                }
            }
        }

        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(8, 8));
        let alg = ExtAlgebra::new(Arc::clone(&res), res).with_differential(Arc::new(MockDiff));

        // Sanity: all three source bidegrees are 1-dimensional on the E-page.
        assert_eq!(alg.dimension(Bidegree::n_s(0, 1)), 1); // h_0
        assert_eq!(alg.dimension(Bidegree::n_s(0, 2)), 1); // h_0^2
        assert_eq!(alg.dimension(Bidegree::n_s(1, 1)), 1); // h_1

        assert_eq!(alg.cohomology_dimension(Bidegree::n_s(0, 2)), Some(0)); // outgoing rank 1
        assert_eq!(alg.cohomology_dimension(Bidegree::n_s(0, 1)), Some(0)); // incoming rank 1
        assert_eq!(alg.cohomology_dimension(Bidegree::n_s(1, 1)), Some(1)); // untouched
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
