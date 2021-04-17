mod finite_chain_complex;
#[cfg(feature = "extras")]
mod tensor_product_chain_complex;

use crate::utils::ascii_num;
use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::{FreeModule, Module};
use algebra::Algebra;
use fp::matrix::Subquotient;
use fp::prime::ValidPrime;
use fp::vector::FpVector;
use std::sync::Arc;

// pub use hom_complex::HomComplex;
pub use finite_chain_complex::{FiniteAugmentedChainComplex, FiniteChainComplex};
#[cfg(feature = "extras")]
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
        for s in (0..=self.max_homological_degree()).rev() {
            let module = self.module(s);

            for t in min_degree + s as i32..=module.max_computed_degree() {
                result.push(ascii_num(module.number_of_gens_in_degree(t)));
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
pub trait ChainComplex: Send + Sync + 'static {
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

    /// A concurrent version of compute_through_bidegree_concurrent. This defaults to the
    /// non-concurrent version
    #[cfg(feature = "concurrent")]
    #[allow(unused_variables)]
    fn compute_through_bidegree_concurrent(
        &self,
        s: u32,
        t: i32,
        bucket: &thread_token::TokenBucket,
    ) {
        self.compute_through_bidegree(s, t);
    }

    /// The largest s such that `self.module(s)` is defined.
    fn max_homological_degree(&self) -> u32;

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
}

pub trait CochainComplex: Send + Sync + 'static {
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
