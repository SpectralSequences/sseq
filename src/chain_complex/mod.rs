mod hom_complex;
mod finite_chain_complex;
mod tensor_product_chain_complex;

use crate::fp_vector::{FpVector, FpVectorT};
use crate::matrix::Subspace;
use crate::algebra::{Algebra, AlgebraAny};
use crate::module::Module;
use crate::module::homomorphism::ModuleHomomorphism;
use std::sync::Arc;

pub use hom_complex::HomComplex;
pub use finite_chain_complex::{FiniteChainComplex, FiniteAugmentedChainComplex};
pub use tensor_product_chain_complex::TensorChainComplex;

pub enum ChainComplexGrading {
    Homological,
    Cohomological
}

/// A chain complex is defined to start in degree 0. The min_degree is the min_degree of the
/// modules in the chain complex, all of which must be the same.
pub trait ChainComplex : Send + Sync + 'static {
    type Module : Module;
    type Homomorphism : ModuleHomomorphism<Source=Self::Module, Target=Self::Module>;

    fn prime(&self) -> u32 {
        self.algebra().prime()
    }

    fn algebra(&self) -> Arc<AlgebraAny>;
    fn min_degree(&self) -> i32;
    fn zero_module(&self) -> Arc<Self::Module>;
    fn module(&self, homological_degree : u32) -> Arc<Self::Module>;

    // This returns the differential starting from the sth module.
    fn differential(&self, homological_degree : u32) -> Arc<Self::Homomorphism>;
    fn compute_through_bidegree(&self, homological_degree : u32, internal_degree : i32);

    fn set_homology_basis(&self, homological_degree : u32, internal_degree : i32, homology_basis : Vec<usize>);
    fn homology_basis(&self, homological_degree : u32, internal_degree : i32) -> &Vec<usize>;
    fn max_homology_degree(&self, homological_degree : u32) -> i32;

    fn compute_homology_through_bidegree(&self, homological_degree : u32, internal_degree : i32){
        self.compute_through_bidegree(homological_degree + 1, internal_degree);
        for i in 0 ..= homological_degree {
            for j in self.max_homology_degree(i) + 1 ..= internal_degree {
                self.compute_homology(i, j);
            }
        }
    }

    fn homology_dimension(&self, homological_degree : u32, internal_degree : i32) -> usize {
        self.homology_basis(homological_degree, internal_degree).len()
    }

    fn homology_gen_to_cocyle(&self, result : &mut FpVector, coeff : u32, homological_degree : u32, internal_degree : i32, index : usize){
        let row_index = self.homology_basis(homological_degree, internal_degree)[index];
        result.add(&self.differential(homological_degree).kernel(internal_degree).matrix[row_index], coeff);
    }

    fn compute_homology(&self, homological_degree : u32, internal_degree : i32){
        self.compute_through_bidegree(homological_degree + 1, internal_degree);
        let d_prev = self.differential(homological_degree);
        let d_cur = self.differential(homological_degree + 1);
        d_prev.compute_kernels_and_quasi_inverses_through_degree(internal_degree);
        d_cur.compute_kernels_and_quasi_inverses_through_degree(internal_degree);
        let kernel = d_prev.kernel(internal_degree);
        let image = d_cur.image(internal_degree);
        let homology_basis = Subspace::subquotient(Some(kernel), image.as_ref(), d_prev.source().dimension(internal_degree));
        self.set_homology_basis(homological_degree, internal_degree, homology_basis);
    }
}

pub trait CochainComplex : Send + Sync + 'static {
    type Module : Module;
    type Homomorphism : ModuleHomomorphism<Source=Self::Module, Target=Self::Module>;

    fn prime(&self) -> u32 {
        self.algebra().prime()
    }
    fn algebra(&self) -> Arc<AlgebraAny>;
    fn min_degree(&self) -> i32;
    fn zero_module(&self) -> Arc<Self::Module>;
    fn module(&self, homological_degree : u32) -> Arc<Self::Module>;
    fn differential(&self, homological_degree : u32) -> Arc<Self::Homomorphism>;
    fn compute_through_bidegree(&self, homological_degree : u32, degree : i32);

    fn set_cohomology_basis(&self, homological_degree : u32, internal_degree : i32, homology_basis : Vec<usize>);
    fn cohomology_basis(&self, homological_degree : u32, internal_degree : i32) -> &Vec<usize>;
    fn max_cohomology_degree(&self, homological_degree : u32) -> i32;

    fn compute_cohomology_through_bidegree(&self, homological_degree : u32, internal_degree : i32){
        self.compute_through_bidegree(homological_degree + 1, internal_degree);
        for i in 0 ..= homological_degree {
            for j in self.max_cohomology_degree(i) + 1 ..= internal_degree {
                self.compute_cohomology(i, j);
            }
        }
    }

    fn cohomology_dimension(&self, homological_degree : u32, internal_degree : i32) -> usize {
        self.cohomology_basis(homological_degree, internal_degree).len()
    }

    fn homology_gen_to_cocyle(&self, result : &mut FpVector, coeff : u32, homological_degree : u32, internal_degree : i32, index : usize){
        let row_index = self.cohomology_basis(homological_degree, internal_degree)[index];
        result.add(&self.differential(homological_degree).kernel(internal_degree).matrix[row_index], coeff);
    }

    fn compute_cohomology(&self, homological_degree : u32, internal_degree : i32){
        println!("==== {}, {}", homological_degree, internal_degree);
        self.compute_through_bidegree(homological_degree + 1, internal_degree);
        let d_cur = self.differential(homological_degree);
        let d_prev = self.differential(homological_degree + 1);
        d_prev.compute_kernels_and_quasi_inverses_through_degree(internal_degree);
        d_cur.compute_kernels_and_quasi_inverses_through_degree(internal_degree);
        let kernel = d_prev.kernel(internal_degree);
        let image = d_cur.image(internal_degree);
        let cohomology_basis = Subspace::subquotient(Some(kernel), image.as_ref(), d_prev.source().dimension(internal_degree));
        self.set_cohomology_basis(homological_degree, internal_degree, cohomology_basis);
    }
}

/// An augmented chain complex is a map of chain complexes C -> D that is a *quasi-isomorphism*. We
/// usually think of C as a resolution of D. The chain map must be a map of degree shift 0.
pub trait AugmentedChainComplex : ChainComplex {
    type TargetComplex : ChainComplex;
    type ChainMap : ModuleHomomorphism<Source=Self::Module, Target=<<Self as AugmentedChainComplex>::TargetComplex as ChainComplex>::Module>;

    fn target(&self) -> Arc<Self::TargetComplex>;
    fn chain_map(&self, s: u32) -> Arc<Self::ChainMap>;
}

/// A bounded chain complex is a chain complex C for which C_s = 0 for all s >= max_s
pub trait BoundedChainComplex : ChainComplex {
    fn max_s (&self) -> u32;
}


/// `chain_maps` is required to be non-empty
pub struct ChainMap<F : ModuleHomomorphism> {
    pub s_shift : u32,
    pub chain_maps : Vec<F>
}
