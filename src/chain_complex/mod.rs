mod hom_complex;
mod finite_chain_complex;

use crate::fp_vector::{FpVector, FpVectorT};
use crate::matrix::Subspace;
use crate::algebra::{Algebra, AlgebraAny};
use crate::module::{Module, ZeroModule, OptionModule};
use crate::module::homomorphism::{ModuleHomomorphism, ZeroHomomorphism};
use std::sync::Arc;

pub use hom_complex::HomComplex;
pub use finite_chain_complex::{FiniteChainComplex, FiniteAugmentedChainComplex};

pub enum ChainComplexGrading {
    Homological,
    Cohomological
}

/// A chain complex is defined to start in degree 0. The min_degree is the min_degree of the
/// modules in the chain complex, all of which must be the same.
pub trait ChainComplex {
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

pub trait CochainComplex {
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


pub struct ChainComplexConcentratedInDegreeZero<M : Module> {
    module : Arc<OptionModule<M>>,
    zero_module : Arc<OptionModule<M>>,
    d0 : Arc<ZeroHomomorphism<OptionModule<M>, OptionModule<M>>>,
    d1 : Arc<ZeroHomomorphism<OptionModule<M>, OptionModule<M>>>,
    other_ds : Arc<ZeroHomomorphism<OptionModule<M>, OptionModule<M>>>
}

impl<M : Module> ChainComplexConcentratedInDegreeZero<M> {
    pub fn new(module : Arc<M>) -> Self {
        let zero_module_inner = Arc::new(ZeroModule::new(Arc::clone(&module.algebra()), module.min_degree()));
        let zero_module = Arc::new(OptionModule::Zero(Arc::clone(&zero_module_inner)));
        let some_module = Arc::new(OptionModule::Some(Arc::clone(&module)));
        Self {
            d0 : Arc::new(ZeroHomomorphism::new(Arc::clone(&some_module), Arc::clone(&zero_module), 0)),
            d1 : Arc::new(ZeroHomomorphism::new(Arc::clone(&zero_module), Arc::clone(&some_module), 0)),
            other_ds : Arc::new(ZeroHomomorphism::new(Arc::clone(&zero_module), Arc::clone(&zero_module), 0)),
            module : some_module,
            zero_module
        }
    }
}

impl<M : Module> ChainComplex for ChainComplexConcentratedInDegreeZero<M> {
    type Module = OptionModule<M>;
    type Homomorphism = ZeroHomomorphism<Self::Module, Self::Module>;

    fn algebra(&self) -> Arc<AlgebraAny> {
        self.module.algebra()
    }

    fn set_homology_basis(&self, homological_degree : u32, internal_degree : i32, homology_basis : Vec<usize>){
        unimplemented!()
    }

    fn homology_basis(&self, homological_degree : u32, internal_degree : i32) -> &Vec<usize>{
        unimplemented!()
    }

    fn max_homology_degree(&self, homological_degree : u32) -> i32 {
        unimplemented!()
    }

    fn zero_module(&self) -> Arc<OptionModule<M>>{
        Arc::clone(&self.zero_module)
    }

    fn module(&self, homological_degree : u32) -> Arc<OptionModule<M>> {
        if homological_degree == 0 {
            Arc::clone(&self.module)
        } else {
            Arc::clone(&self.zero_module)
        }
    }

    fn min_degree(&self) -> i32 {
        self.module.min_degree()
    }

    fn differential(&self, homological_degree : u32) -> Arc<ZeroHomomorphism<OptionModule<M>, OptionModule<M>>> {
        match homological_degree {
            0 => Arc::clone(&self.d0),
            1 => Arc::clone(&self.d1),
            _ => Arc::clone(&self.other_ds)
        }
    }

    fn compute_through_bidegree(&self, homological_degree : u32, degree : i32) {
        if homological_degree == 0 {
            self.module.compute_basis(degree);
        }
    }
}

pub trait AugmentedChainComplex : ChainComplex {
    type TargetComplex : ChainComplex;
    type ChainMap : ModuleHomomorphism<Source=Self::Module, Target=<<Self as AugmentedChainComplex>::TargetComplex as ChainComplex>::Module>;

    fn target(&self) -> Arc<Self::TargetComplex>;
    fn chain_map(&self, s: u32) -> Arc<Self::ChainMap>;
}

