mod chain_homotopy;
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
    prime::ValidPrime,
    vector::{FpSlice, FpSliceMut},
};
use itertools::Itertools;
use sseq::coordinates::{Bidegree, BidegreeGenerator};

use crate::{save::SaveDirectory, utils::unicode_num};

pub enum ChainComplexGrading {
    Homological,
    Cohomological,
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

    /// Like [`number_of_gens_in_bidegree`](Self::number_of_gens_in_bidegree), but returns `None`
    /// for any bidegree outside the currently computed range instead of panicking.
    ///
    /// `number_of_gens_in_bidegree` panics out of range on either axis: `module(b.s())` indexes the
    /// homological degrees (panicking unless `0 <= b.s() < next_homological_degree()`), and the
    /// module's `number_of_gens_in_degree(b.t())` indexes its computed degrees (panicking for
    /// `b.t() > max_computed_degree()`; it returns 0 below `min_degree()`). `None` distinguishes
    /// "not computed" from a computed bidegree that genuinely has 0 generators (`Some(0)`).
    fn try_number_of_gens_in_bidegree(&self, b: Bidegree) -> Option<usize> {
        if b.s() < 0 || b.s() >= self.next_homological_degree() {
            return None;
        }
        let m = self.module(b.s());
        if b.t() < m.min_degree() || b.t() > m.max_computed_degree() {
            return None;
        }
        Some(m.number_of_gens_in_degree(b.t()))
    }

    /// Iterate through all nonzero bidegrees in increasing order of stem.
    fn iter_nonzero_stem(&self) -> impl Iterator<Item = Bidegree> + '_ {
        self.iter_stem()
            .filter(move |&b| self.number_of_gens_in_bidegree(b) > 0)
    }

    /// Like [`iter_nonzero_stem`](Self::iter_nonzero_stem), but the returned iterator owns a
    /// shared handle (`Arc`) to the chain complex instead of borrowing it.
    ///
    /// This is the owning analogue of [`iter_nonzero_stem`]: it walks the same bidegrees as
    /// [`iter_stem_owned`](ChainComplex::iter_stem_owned) and keeps only those with a nonzero
    /// number of generators. The filter uses
    /// [`try_number_of_gens_in_bidegree`](Self::try_number_of_gens_in_bidegree) (mapping the
    /// out-of-range `None` to 0); every bidegree the stem walk yields is in the computed range,
    /// so this matches `number_of_gens_in_bidegree(b) > 0` exactly.
    fn iter_nonzero_stem_owned(self: Arc<Self>) -> impl Iterator<Item = Bidegree>
    where
        Self: Sized,
    {
        let cc = Arc::clone(&self);
        self.iter_stem_owned()
            .filter(move |&b| cc.try_number_of_gens_in_bidegree(b).unwrap_or(0) > 0)
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

    /// Like [`module`](Self::module), but returns `None` for any homological degree outside the
    /// defined range `[0, next_homological_degree())` instead of panicking.
    ///
    /// The resolution backends index their internal module table (an `OnceBiVec`) here, panicking
    /// unless `0 <= s < next_homological_degree()`; this is the non-panicking sibling used to guard
    /// such an access (e.g. from the Python bindings).
    fn try_module(&self, s: i32) -> Option<Arc<Self::Module>> {
        if s < 0 || s >= self.next_homological_degree() {
            None
        } else {
            Some(self.module(s))
        }
    }

    /// Like [`differential`](Self::differential), but returns `None` for any homological degree
    /// outside the defined range `[0, next_homological_degree())` instead of panicking.
    ///
    /// The resolution backends index their internal differential table (an `OnceVec`) here,
    /// panicking unless `0 <= s < next_homological_degree()`; this is the non-panicking sibling used
    /// to guard such an access.
    fn try_differential(&self, s: i32) -> Option<Arc<Self::Homomorphism>> {
        if s < 0 || s >= self.next_homological_degree() {
            None
        } else {
            Some(self.differential(s))
        }
    }

    /// Iterate through all defined bidegrees in increasing order of stem.
    fn iter_stem(&self) -> StemIterator<'_, Self> {
        StemIterator {
            cc: self,
            current: Bidegree::n_s(self.min_degree(), 0),
            max_s: self.next_homological_degree(),
        }
    }

    /// Like [`iter_stem`](Self::iter_stem), but the returned iterator owns a shared handle
    /// (`Arc`) to the chain complex instead of borrowing it.
    ///
    /// This yields exactly the same bidegrees, in the same order, with the same bounds as
    /// [`iter_stem`] (both delegate to the shared [`stem_step`] cursor logic). Because the
    /// iterator does not borrow `self`, it is `'static` (when `Self: 'static`) and can be stored
    /// in long-lived owners such as FFI handles.
    fn iter_stem_owned(self: Arc<Self>) -> OwnedStemIterator<Self>
    where
        Self: Sized,
    {
        let current = Bidegree::n_s(self.min_degree(), 0);
        let max_s = self.next_homological_degree();
        OwnedStemIterator {
            cc: self,
            current,
            max_s,
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

    /// Get the save file of a bidegree
    fn save_file(
        &self,
        kind: crate::save::SaveKind,
        b: Bidegree,
    ) -> crate::save::SaveFile<Self::Algebra> {
        crate::save::SaveFile {
            algebra: self.algebra(),
            kind,
            b,
            idx: None,
        }
    }
}

/// Advance a stem-walk cursor by one step, shared by [`StemIterator`] (borrowing) and
/// [`OwnedStemIterator`] (owning) so the two stay in lockstep.
///
/// `cc` is the chain complex being walked, `current` the mutable cursor (next bidegree to
/// consider), and `max_s` the exclusive homological-degree bound (`next_homological_degree()` at
/// the time the iterator was created). Returns the next defined bidegree in increasing order of
/// stem, or `None` once the walk is exhausted.
fn stem_step<CC: ChainComplex + ?Sized>(
    cc: &CC,
    current: &mut Bidegree,
    max_s: i32,
) -> Option<Bidegree> {
    loop {
        if max_s == 0 {
            return None;
        }
        let cur = *current;

        if cur.s() == max_s {
            *current = Bidegree::n_s(cur.n() + 1, 0);
            continue;
        }
        if cur.t() > cc.module(cur.s()).max_computed_degree() {
            if cur.s() == 0 {
                return None;
            } else {
                *current = Bidegree::n_s(cur.n() + 1, 0);
                continue;
            }
        }
        *current = cur + Bidegree::n_s(0, 1);
        return Some(cur);
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
        stem_step(self.cc, &mut self.current, self.max_s)
    }
}

/// The owning analogue of [`StemIterator`], returned by [`ChainComplex::iter_stem_owned`]. It
/// holds a shared handle (`Arc`) to the chain complex rather than a borrow, so it can outlive any
/// particular reference and be stored in long-lived owners. It yields the same bidegrees, in the
/// same order, as [`StemIterator`] (both delegate to [`stem_step`]).
pub struct OwnedStemIterator<CC: ?Sized> {
    cc: Arc<CC>,
    current: Bidegree,
    max_s: i32,
}

impl<CC: ChainComplex + ?Sized> Iterator for OwnedStemIterator<CC> {
    type Item = Bidegree;

    fn next(&mut self) -> Option<Self::Item> {
        stem_step(&*self.cc, &mut self.current, self.max_s)
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

    /// Like [`chain_map`](Self::chain_map), but returns `None` for any homological degree outside
    /// the bounded range `[0, max_s())` instead of panicking.
    ///
    /// [`chain_map`](Self::chain_map) indexes the internal `chain_maps` `Vec` and panics out of
    /// range; for a complex augmented from a [`FiniteChainComplex`] there is one chain map per
    /// module, so `max_s()` (the number of nonzero modules) is the defined upper bound. This is the
    /// non-panicking sibling used to guard such an access.
    fn try_chain_map(&self, s: i32) -> Option<Arc<Self::ChainMap>>
    where
        Self: BoundedChainComplex,
    {
        if s < 0 || s >= self.max_s() {
            None
        } else {
            Some(self.chain_map(s))
        }
    }
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
