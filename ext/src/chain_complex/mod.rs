mod chain_homotopy;
mod finite_chain_complex;
mod tensor_product_chain_complex;

use crate::utils::unicode_num;
use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::{FreeModule, Module};
use algebra::Algebra;
use fp::matrix::Subquotient;
use fp::prime::ValidPrime;
use fp::vector::{FpVector, Slice, SliceMut};
use std::sync::Arc;

use itertools::Itertools;

// pub use hom_complex::HomComplex;
pub use chain_homotopy::ChainHomotopy;
pub use finite_chain_complex::{FiniteAugmentedChainComplex, FiniteChainComplex};
pub use tensor_product_chain_complex::TensorChainComplex;

pub enum ChainComplexGrading {
    Homological,
    Cohomological,
}

pub trait FreeChainComplex:
    ChainComplex<
    Module = FreeModule<<Self as ChainComplex>::Algebra>,
    Homomorphism = FreeModuleHomomorphism<FreeModule<<Self as ChainComplex>::Algebra>>,
>
{
    fn graded_dimension_string(&self) -> String {
        let mut result = String::new();
        let min_degree = self.min_degree();
        for s in (0..self.next_homological_degree()).rev() {
            let module = self.module(s);

            for t in min_degree + s as i32..=module.max_computed_degree() {
                result.push(unicode_num(module.number_of_gens_in_degree(t)));
                result.push(' ');
            }
            result.push('\n');
            // If it is empty so far, don't print anything
            if result.trim_start().is_empty() {
                result = String::new();
            }
        }
        result
    }

    /// Computes the filtration one product. This returns None if the source or target is out of
    /// range.
    fn filtration_one_product(
        &self,
        op_deg: i32,
        op_idx: usize,
        target_s: u32,
        target_t: i32,
    ) -> Option<Vec<Vec<u32>>> {
        let source_t = target_t - op_deg;
        let source_s = target_s.overflowing_sub(1).0;
        if target_s == 0
            || target_s >= self.next_homological_degree()
            || source_t - (source_s as i32) < self.min_degree()
        {
            return None;
        }

        let source = self.module(target_s - 1);
        let target = self.module(target_s);

        if target_t > target.max_computed_degree() {
            return None;
        }

        let source_dim = source.number_of_gens_in_degree(source_t);
        let target_dim = target.number_of_gens_in_degree(target_t);

        let d = self.differential(target_s);

        let mut products = vec![Vec::with_capacity(target_dim); source_dim];
        for i in 0..target_dim {
            let dx = d.output(target_t, i);

            for (j, row) in products.iter_mut().enumerate() {
                let idx = source.operation_generator_to_index(op_deg, op_idx, source_t, j);
                row.push(dx.entry(idx));
            }
        }

        Some(products)
    }

    fn number_of_gens_in_bidegree(&self, s: u32, t: i32) -> usize {
        self.module(s).number_of_gens_in_degree(t)
    }

    fn cocycle_string(&self, s: u32, t: i32, idx: usize) -> String {
        let d = self.differential(s);
        let target = d.target();
        let result_vector = d.output(t, idx);

        target.element_to_string_pretty(s, t, result_vector.as_slice())
    }
}

impl<CC> FreeChainComplex for CC where
    CC: ChainComplex<
        Module = FreeModule<Self::Algebra>,
        Homomorphism = FreeModuleHomomorphism<FreeModule<Self::Algebra>>,
    >
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
    fn module(&self, homological_degree: u32) -> Arc<Self::Module>;

    /// This returns the differential starting from the sth module.
    fn differential(&self, s: u32) -> Arc<Self::Homomorphism>;

    /// If the complex has been computed at bidegree (s, t). This means the module has been
    /// computed at (s, t), and so has the differential at (s, t). In the case of a free module,
    /// the target of the differential, namely the bidegree (s - 1, t), need not be computed, as
    /// long as all the generators hit by the differential have already been computed.
    fn has_computed_bidegree(&self, s: u32, t: i32) -> bool;

    /// Ensure all bidegrees less than or equal to (s, t) have been computed
    fn compute_through_bidegree(&self, s: u32, t: i32);

    /// The first s such that `self.module(s)` is not defined.
    fn next_homological_degree(&self) -> u32;

    fn set_homology_basis(
        &self,
        homological_degree: u32,
        internal_degree: i32,
        homology_basis: Vec<usize>,
    );
    fn homology_basis(&self, homological_degree: u32, internal_degree: i32) -> &Vec<usize>;
    fn max_homology_degree(&self, homological_degree: u32) -> i32;

    fn compute_homology_through_bidegree(&self, homological_degree: u32, internal_degree: i32) {
        self.compute_through_bidegree(homological_degree + 1, internal_degree);
        for i in 0..=homological_degree {
            for j in self.max_homology_degree(i) + 1..=internal_degree {
                self.compute_homology(i, j);
            }
        }
    }

    fn homology_dimension(&self, homological_degree: u32, internal_degree: i32) -> usize {
        self.homology_basis(homological_degree, internal_degree)
            .len()
    }

    fn homology_gen_to_cocyle(
        &self,
        result: &mut FpVector,
        coeff: u32,
        homological_degree: u32,
        internal_degree: i32,
        index: usize,
    ) {
        let row_index = self.homology_basis(homological_degree, internal_degree)[index];
        result.add(
            &self
                .differential(homological_degree)
                .kernel(internal_degree)
                .unwrap()[row_index],
            coeff,
        );
    }

    fn compute_homology(&self, homological_degree: u32, internal_degree: i32) {
        self.compute_through_bidegree(homological_degree + 1, internal_degree);
        let d_prev = self.differential(homological_degree);
        let d_cur = self.differential(homological_degree + 1);
        d_prev.compute_auxiliary_data_through_degree(internal_degree);
        d_cur.compute_auxiliary_data_through_degree(internal_degree);
        let kernel = d_prev.kernel(internal_degree).unwrap();
        let image = d_cur.image(internal_degree).unwrap();
        let homology_basis = Subquotient::subquotient(kernel, image);
        self.set_homology_basis(homological_degree, internal_degree, homology_basis);
    }

    /// Iterate through all defind bidegrees in increasing order of stem. The return values are of
    /// the form `(s, n, t)`.
    fn iter_stem(&self) -> StemIterator<'_, Self> {
        StemIterator {
            cc: self,
            n: self.min_degree(),
            s: 0,
            max_s: self.next_homological_degree(),
        }
    }

    /// Apply the quasi-inverse of the (s, t)th differential to the list of inputs and results.
    /// This defaults to applying `self.differentials(s).quasi_inverse(t)`, but in some cases
    /// the quasi-inverse might be stored separately on disk.
    ///
    /// This returns whether the application was successful
    #[must_use]
    fn apply_quasi_inverse<T, S>(&self, results: &mut [T], s: u32, t: i32, inputs: &[S]) -> bool
    where
        for<'a> &'a mut T: Into<SliceMut<'a>>,
        for<'a> &'a S: Into<Slice<'a>>,
    {
        assert_eq!(results.len(), inputs.len());
        if results.is_empty() {
            return true;
        }

        let mut iter = inputs.iter().zip_eq(results);
        let (input, result) = iter.next().unwrap();
        let d = self.differential(s);
        if d.apply_quasi_inverse(result.into(), t, input.into()) {
            for (input, result) in iter {
                assert!(d.apply_quasi_inverse(result.into(), t, input.into()));
            }
            true
        } else {
            false
        }
    }

    /// A directory used to save information about the chain complex.
    fn save_dir(&self) -> Option<&std::path::Path> {
        None
    }

    /// Get the save file of a bidegree
    fn save_file(
        &self,
        kind: crate::save::SaveKind,
        s: u32,
        t: i32,
    ) -> crate::save::SaveFile<Self::Algebra> {
        crate::save::SaveFile {
            algebra: self.algebra(),
            kind,
            s,
            t,
            idx: None,
        }
    }
}

/// An iterator returned by [`ChainComplex::iter_stem`]
pub struct StemIterator<'a, CC: ?Sized> {
    cc: &'a CC,
    n: i32,
    s: u32,
    max_s: u32,
}

impl<'a, CC: ChainComplex> Iterator for StemIterator<'a, CC> {
    // (s, n, t)
    type Item = (u32, i32, i32);
    fn next(&mut self) -> Option<Self::Item> {
        if self.max_s == 0 {
            return None;
        }
        let s = self.s;
        let n = self.n;
        let t = self.n + self.s as i32;

        if s == self.max_s {
            self.n += 1;
            self.s = 0;
            return self.next();
        }
        if t > self.cc.module(s).max_computed_degree() {
            if s == 0 {
                return None;
            } else {
                self.n += 1;
                self.s = 0;
                return self.next();
            }
        }
        self.s += 1;
        Some((s, n, t))
    }
}

pub trait CochainComplex: Send + Sync {
    type Algebra: Algebra;
    type Module: Module<Algebra = Self::Algebra>;
    type Homomorphism: ModuleHomomorphism<Source = Self::Module, Target = Self::Module>;

    fn prime(&self) -> ValidPrime {
        self.algebra().prime()
    }
    fn algebra(&self) -> Arc<<Self::Module as Module>::Algebra>;
    fn min_degree(&self) -> i32;
    fn zero_module(&self) -> Arc<Self::Module>;
    fn module(&self, homological_degree: u32) -> Arc<Self::Module>;
    fn differential(&self, homological_degree: u32) -> Arc<Self::Homomorphism>;
    fn compute_through_bidegree(&self, homological_degree: u32, degree: i32);

    fn set_cohomology_basis(
        &self,
        homological_degree: u32,
        internal_degree: i32,
        homology_basis: Vec<usize>,
    );
    fn cohomology_basis(&self, homological_degree: u32, internal_degree: i32) -> &Vec<usize>;
    fn max_cohomology_degree(&self, homological_degree: u32) -> i32;

    fn compute_cohomology_through_bidegree(&self, homological_degree: u32, internal_degree: i32) {
        self.compute_through_bidegree(homological_degree + 1, internal_degree);
        for i in 0..=homological_degree {
            for j in self.max_cohomology_degree(i) + 1..=internal_degree {
                self.compute_cohomology(i, j);
            }
        }
    }

    fn cohomology_dimension(&self, homological_degree: u32, internal_degree: i32) -> usize {
        self.cohomology_basis(homological_degree, internal_degree)
            .len()
    }

    fn homology_gen_to_cocyle(
        &self,
        result: &mut FpVector,
        coeff: u32,
        homological_degree: u32,
        internal_degree: i32,
        index: usize,
    ) {
        let row_index = self.cohomology_basis(homological_degree, internal_degree)[index];
        result.add(
            &self
                .differential(homological_degree)
                .kernel(internal_degree)
                .unwrap()[row_index],
            coeff,
        );
    }

    fn compute_cohomology(&self, homological_degree: u32, internal_degree: i32) {
        println!("==== {}, {}", homological_degree, internal_degree);
        self.compute_through_bidegree(homological_degree + 1, internal_degree);
        let d_cur = self.differential(homological_degree);
        let d_prev = self.differential(homological_degree + 1);
        d_prev.compute_auxiliary_data_through_degree(internal_degree);
        d_cur.compute_auxiliary_data_through_degree(internal_degree);
        let kernel = d_prev.kernel(internal_degree).unwrap();
        let image = d_cur.image(internal_degree).unwrap();
        let cohomology_basis = Subquotient::subquotient(kernel, image);
        self.set_cohomology_basis(homological_degree, internal_degree, cohomology_basis);
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
    fn chain_map(&self, s: u32) -> Arc<Self::ChainMap>;
}

/// A bounded chain complex is a chain complex C for which C_s = 0 for all s >= max_s
pub trait BoundedChainComplex: ChainComplex {
    fn max_s(&self) -> u32;
}

/// `chain_maps` is required to be non-empty
pub struct ChainMap<F: ModuleHomomorphism> {
    pub s_shift: u32,
    pub chain_maps: Vec<F>,
}
