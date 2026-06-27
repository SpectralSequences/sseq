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
pub use crate::secondary::{SecondaryDegree, SecondaryElement, SecondaryGenerator, Weight};
use crate::{
    chain_complex::{AugmentedChainComplex, FreeChainComplex},
    resolution_homomorphism::ResolutionHomomorphism,
    utils::{QueryModuleResolution, get_unit},
};

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
    use super::*;
    use crate::utils::construct_standard;

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
