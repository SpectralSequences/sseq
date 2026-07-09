pub(crate) mod chain_homotopy;
mod finite_chain_complex;

use std::sync::Arc;

use algebra::{
    Algebra, MuAlgebra,
    module::{
        Module, MuFreeModule,
        homomorphism::{ModuleHomomorphism, MuFreeModuleHomomorphism},
    },
};
// pub use hom_complex::HomComplex;
pub use chain_homotopy::ChainHomotopy;
pub use finite_chain_complex::{FiniteAugmentedChainComplex, FiniteChainComplex};
use fp::{
    matrix::Matrix,
    prime::{Prime, ValidPrime},
    vector::{FpSlice, FpSliceMut, FpVector},
};
use itertools::Itertools;
use sseq::coordinates::{Bidegree, BidegreeGenerator};

use crate::{save::SaveDirectory, utils::unicode_num};

pub enum ChainComplexGrading {
    Homological,
    Cohomological,
}

/// A tiny FNV-1a 64-bit accumulator used by [`ChainComplex::fingerprint`].
///
/// `std`'s `DefaultHasher` is not guaranteed stable across toolchain versions, which would
/// spuriously invalidate an on-disk save directory after a compiler upgrade. FNV-1a is a fixed,
/// specified algorithm, so the fingerprint is reproducible across runs and builds.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn write_u64(&mut self, x: u64) {
        for b in x.to_le_bytes() {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn write_i32(&mut self, x: i32) {
        self.write_u64(u64::from(x as u32));
    }

    fn write_usize(&mut self, x: usize) {
        self.write_u64(x as u64);
    }

    fn finish(self) -> u64 {
        self.0
    }
}

pub trait FreeChainComplex<const U: bool = false>:
    ChainComplex<
        Module = MuFreeModule<U, <Self as ChainComplex>::Algebra>,
        Homomorphism = MuFreeModuleHomomorphism<
            U,
            MuFreeModule<U, <Self as ChainComplex>::Algebra>,
        >,
    >
where
    <Self as ChainComplex>::Algebra: MuAlgebra<U>,
{
    fn graded_dimension_string(&self) -> String {
        let mut result = String::new();
        let min_degree = self.min_degree();
        for s in (0..self.next_homological_degree()).rev() {
            let module = self.module(s);

            for t in min_degree + s..=module.max_computed_degree() {
                result.push(unicode_num(module.number_of_gens_in_degree(t)));
                result.push(' ');
            }
            result.push('\n');
            // If it is empty so far, don't print anything
            if result.trim_start().is_empty() {
                result.clear()
            }
        }
        result
    }

    fn to_sseq(&self) -> sseq::Sseq<2, sseq::Adams> {
        let p = self.prime();
        let mut sseq = sseq::Sseq::new(p);
        for b in self.iter_stem() {
            sseq.set_dimension(b, self.number_of_gens_in_bidegree(b));
        }
        sseq
    }

    fn filtration_one_products(&self, op_deg: i32, op_idx: usize) -> sseq::Product<2> {
        let p = self.prime();
        let matrices = once::MultiIndexed::new();
        for x in self.min_degree()..self.module(0).max_computed_degree() - op_deg + 2 {
            let mut b = Bidegree::n_s(x, 0);
            while self.has_computed_bidegree(b + Bidegree::s_t(1, op_deg)) {
                if let Some(m) = self.filtration_one_product(op_deg, op_idx, b) {
                    matrices.insert(b, Matrix::from_vec(p, &m));
                }
                b = b + Bidegree::n_s(0, 1);
            }
        }

        sseq::Product {
            b: Bidegree::x_y(op_deg - 1, 1),
            left: true,
            matrices,
        }
    }

    /// Computes the filtration one product.
    ///
    /// # Returns
    /// `Some` when the product is defined (the target bidegree is computed and `op_idx` is in
    /// range), and `None` otherwise. This is
    /// [`try_filtration_one_product`](Self::try_filtration_one_product) with the error discarded;
    /// use that variant to learn why the product is unavailable.
    fn filtration_one_product(
        &self,
        op_deg: i32,
        op_idx: usize,
        source: Bidegree,
    ) -> Option<Vec<Vec<u32>>> {
        self.try_filtration_one_product(op_deg, op_idx, source).ok()
    }

    /// Computes the filtration one product, returning an error explaining why the product is
    /// unavailable instead of swallowing it as `None`.
    ///
    /// # Returns
    /// - `Err(..)` if the target bidegree has not been computed, or if `op_idx` is out of range for
    ///   the operation degree (in the unstable case this means the product is genuinely not
    ///   defined; in the stable case it is a caller error);
    /// - `Ok(products)` with the computed products otherwise.
    fn try_filtration_one_product(
        &self,
        op_deg: i32,
        op_idx: usize,
        source: Bidegree,
    ) -> anyhow::Result<Vec<Vec<u32>>> {
        anyhow::ensure!(
            op_deg >= 0,
            "filtration one product unavailable: op_deg {op_deg} is negative"
        );
        anyhow::ensure!(
            source.s() >= 0 && source.t() >= 0,
            "filtration one product unavailable: source bidegree {source} has negative coordinates"
        );

        let target = source + Bidegree::s_t(1, op_deg);
        anyhow::ensure!(
            self.has_computed_bidegree(target),
            "filtration one product unavailable: target bidegree {target} has not been computed"
        );

        let source_mod = self.module(target.s() - 1);
        let target_mod = self.module(target.s());

        let dim = self.algebra().dimension_unstable(op_deg, source.t());
        anyhow::ensure!(
            op_idx < dim,
            "op_idx {op_idx} out of range for operation degree {op_deg} (algebra dimension {dim})"
        );

        let source_dim = source_mod.number_of_gens_in_degree(source.t());
        let target_dim = target_mod.number_of_gens_in_degree(target.t());

        let d = self.differential(target.s());

        let mut products = vec![Vec::with_capacity(target_dim); source_dim];
        for i in 0..target_dim {
            let dx = d.output(target.t(), i);

            for (j, row) in products.iter_mut().enumerate() {
                let idx = source_mod.operation_generator_to_index(op_deg, op_idx, source.t(), j);
                row.push(dx.entry(idx));
            }
        }

        Ok(products)
    }

    fn number_of_gens_in_bidegree(&self, b: Bidegree) -> usize {
        self.module(b.s()).number_of_gens_in_degree(b.t())
    }

    /// Iterate through all nonzero bidegrees in increasing order of stem.
    fn iter_nonzero_stem(&self) -> impl Iterator<Item = Bidegree> + '_ {
        self.iter_stem()
            .filter(move |&b| self.number_of_gens_in_bidegree(b) > 0)
    }

    /// Get a string representation of d(gen), where d is the differential of the resolution.
    fn boundary_string(&self, g: BidegreeGenerator) -> String {
        let d = self.differential(g.s());
        let target = d.target();
        let result_vector = d.output(g.t(), g.idx());

        target.element_to_string(g.t(), result_vector.as_slice())
    }
}

impl<const U: bool, CC> FreeChainComplex<U> for CC
where
    CC: ChainComplex<
            Module = MuFreeModule<U, Self::Algebra>,
            Homomorphism = MuFreeModuleHomomorphism<U, MuFreeModule<U, Self::Algebra>>,
        >,
    Self::Algebra: MuAlgebra<U>,
{
}

/// A chain complex is defined to start in degree 0. The min_degree is the min_degree of the
/// modules in the chain complex, all of which must be the same.
pub trait ChainComplex: Send + Sync {
    type Algebra: Algebra;
    type Module: Module<Algebra = Self::Algebra>;
    type Homomorphism: ModuleHomomorphism<Source = Self::Module, Target = Self::Module>;

    fn prime(&self) -> ValidPrime {
        self.algebra().prime()
    }

    fn algebra(&self) -> Arc<Self::Algebra>;
    fn min_degree(&self) -> i32;
    fn zero_module(&self) -> Arc<Self::Module>;
    fn module(&self, homological_degree: i32) -> Arc<Self::Module>;

    /// This returns the differential starting from the sth module.
    fn differential(&self, s: i32) -> Arc<Self::Homomorphism>;

    /// If the complex has been computed at bidegree (s, t). This means the module has been
    /// computed at (s, t), and so has the differential at (s, t). In the case of a free module,
    /// the target of the differential, namely the bidegree (s - 1, t), need not be computed, as
    /// long as all the generators hit by the differential have already been computed.
    fn has_computed_bidegree(&self, b: Bidegree) -> bool;

    /// Ensure all bidegrees less than or equal to (s, t) have been computed
    fn compute_through_bidegree(&self, b: Bidegree);

    /// The first s such that `self.module(s)` is not defined.
    fn next_homological_degree(&self) -> i32;

    /// A stable content hash of this complex's structure.
    ///
    /// This is used to bind a save directory to the *specific complex* being resolved, not just
    /// to its algebra. Reusing a save directory for a different complex over the same algebra
    /// would otherwise silently load structurally-valid but wrong cached data. Two complexes that
    /// yield the same resolution hash equal; two that differ hash differently with overwhelming
    /// probability.
    ///
    /// The hash covers, over the complex's modules, each module's per-degree dimensions and the
    /// algebra action on every basis element, plus each differential's action — so it is
    /// basis-sensitive, exactly matching what the resolution algorithm consumes.
    ///
    /// It only makes sense for the *bounded* augmentation complexes that actually get resolved.
    /// Since [`next_homological_degree`](ChainComplex::next_homological_degree) is unbounded for a
    /// [`FiniteChainComplex`] (it returns `i32::MAX`), we walk `s` until [`module`](ChainComplex::module)
    /// returns the complex's cached zero module (the trailing padding of a bounded complex),
    /// bounded by a generous safety cap. A module with no `max_degree` (i.e. unbounded) is not a
    /// complex we can meaningfully fingerprint, so its structure is skipped rather than risking a
    /// runaway loop. The accumulator is a fixed FNV-1a so the value is stable across runs and
    /// toolchains (unlike `std`'s `DefaultHasher`).
    fn fingerprint(&self) -> u64 {
        /// Safety cap on the homological length so a pathological complex can never hang the
        /// hash. Real augmentation complexes have a handful of modules.
        const MAX_S: i32 = 1 << 20;

        let p = self.prime();
        let mut h = Fnv::new();
        h.write_u64(u64::from(p.as_u32()));
        h.write_i32(self.min_degree());

        let zero = self.zero_module();
        let mut num_modules = 0;
        for s in 0..MAX_S {
            let module = self.module(s);
            // Trailing zero modules mark the end of a bounded complex.
            if Arc::ptr_eq(&module, &zero) {
                break;
            }
            num_modules = s + 1;
            let lo = module.min_degree();
            h.write_i32(s);
            h.write_i32(lo);

            let Some(hi) = module.max_degree() else {
                // Unbounded module: not a resolvable complex, don't try to hash its structure.
                h.write_i32(i32::MIN);
                continue;
            };
            h.write_i32(hi);
            module.compute_basis(hi);
            let algebra = module.algebra();

            for deg in lo..=hi {
                let dim = module.dimension(deg);
                h.write_i32(deg);
                h.write_usize(dim);
                if dim == 0 {
                    continue;
                }
                // The action of every algebra basis element on every module basis element pins
                // down the module structure in the chosen basis.
                for op_deg in 0..=(hi - deg) {
                    algebra.compute_basis(op_deg);
                    let op_dim = algebra.dimension(op_deg);
                    let out_dim = module.dimension(deg + op_deg);
                    for op_idx in 0..op_dim {
                        for mod_idx in 0..dim {
                            let mut out = FpVector::new(p, out_dim);
                            module.act_on_basis(
                                out.as_slice_mut(),
                                1,
                                op_deg,
                                op_idx,
                                deg,
                                mod_idx,
                            );
                            for x in out.iter() {
                                h.write_u64(u64::from(x));
                            }
                        }
                    }
                }
            }

            // The differential out of module(s). Zero for the `ccdz` complexes usually resolved,
            // but non-trivial complexes (e.g. cofibers) must not collide with each other.
            let d = self.differential(s);
            let shift = d.degree_shift();
            let target = d.target();
            h.write_i32(shift);
            for deg in lo..=hi {
                let out_deg = deg - shift;
                target.compute_basis(out_deg);
                let out_dim = target.dimension(out_deg);
                for idx in 0..module.dimension(deg) {
                    let mut out = FpVector::new(p, out_dim);
                    d.apply_to_basis_element(out.as_slice_mut(), 1, deg, idx);
                    for x in out.iter() {
                        h.write_u64(u64::from(x));
                    }
                }
            }
        }
        h.write_i32(num_modules);

        h.finish()
    }

    /// Iterate through all defined bidegrees in increasing order of stem.
    fn iter_stem(&self) -> StemIterator<'_, Self> {
        StemIterator {
            cc: self,
            current: Bidegree::n_s(self.min_degree(), 0),
            max_s: self.next_homological_degree(),
        }
    }

    /// Apply the quasi-inverse of the (s, t)th differential to the list of inputs and results.
    /// This defaults to applying `self.differentials(s).quasi_inverse(t)`, but in some cases
    /// the quasi-inverse might be stored separately on disk.
    ///
    /// This returns whether the application was successful
    #[must_use]
    fn apply_quasi_inverse<T, S>(&self, results: &mut [T], b: Bidegree, inputs: &[S]) -> bool
    where
        for<'a> &'a mut T: Into<FpSliceMut<'a>>,
        for<'a> &'a S: Into<FpSlice<'a>>,
    {
        assert_eq!(results.len(), inputs.len());
        if results.is_empty() {
            return true;
        }

        let mut iter = inputs.iter().zip_eq(results);
        let (input, result) = iter.next().unwrap();
        let d = self.differential(b.s());
        if d.apply_quasi_inverse(result.into(), b.t(), input.into()) {
            for (input, result) in iter {
                assert!(d.apply_quasi_inverse(result.into(), b.t(), input.into()));
            }
            true
        } else {
            false
        }
    }

    /// A directory used to save information about the chain complex.
    fn save_dir(&self) -> &SaveDirectory {
        &SaveDirectory::None
    }
}

/// An iterator returned by [`ChainComplex::iter_stem`]
pub struct StemIterator<'a, CC: ?Sized> {
    cc: &'a CC,
    current: Bidegree,
    max_s: i32,
}

impl<CC: ChainComplex + ?Sized> Iterator for StemIterator<'_, CC> {
    type Item = Bidegree;

    fn next(&mut self) -> Option<Self::Item> {
        if self.max_s == 0 {
            return None;
        }
        let cur = self.current;

        if cur.s() == self.max_s {
            self.current = Bidegree::n_s(cur.n() + 1, 0);
            return self.next();
        }
        if cur.t() > self.cc.module(cur.s()).max_computed_degree() {
            if cur.s() == 0 {
                return None;
            } else {
                self.current = Bidegree::n_s(cur.n() + 1, 0);
                return self.next();
            }
        }
        self.current = cur + Bidegree::n_s(0, 1);
        Some(cur)
    }
}

/// An augmented chain complex is a map of chain complexes C -> D that is a *quasi-isomorphism*. We
/// usually think of C as a resolution of D. The chain map must be a map of degree shift 0.
pub trait AugmentedChainComplex: ChainComplex {
    type TargetComplex: ChainComplex<Algebra = Self::Algebra>;
    type ChainMap: ModuleHomomorphism<
            Source = Self::Module,
            Target = <Self::TargetComplex as ChainComplex>::Module,
        >;

    fn target(&self) -> Arc<Self::TargetComplex>;
    fn chain_map(&self, s: i32) -> Arc<Self::ChainMap>;
}

/// A bounded chain complex is a chain complex C for which C_s = 0 for all s >= max_s
pub trait BoundedChainComplex: ChainComplex {
    fn max_s(&self) -> i32;

    fn euler_characteristic(&self, t: i32) -> isize {
        (0..self.max_s())
            .map(|s| (if s % 2 == 0 { 1 } else { -1 }) * self.module(s).dimension(t) as isize)
            .sum()
    }
}

/// `chain_maps` is required to be non-empty
pub struct ChainMap<F: ModuleHomomorphism> {
    pub s_shift: i32,
    pub chain_maps: Vec<F>,
}
