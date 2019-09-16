use std::sync::Arc;

use crate::fp_vector::{FpVector};
use crate::matrix::{Subspace, QuasiInverse};
use crate::module::{Module, FiniteModule, FreeModule, BoundedModule};
use crate::module::homomorphism::{ModuleHomomorphism, FPModuleHomomorphism, BoundedModuleHomomorphism, ZeroHomomorphismT};
use crate::module::homomorphism::FPModuleT;

impl BoundedModule for FiniteModule {
    fn max_degree(&self) -> i32 {
        match self {
            FiniteModule::FDModule(m) => m.max_degree(),
            FiniteModule::FPModule(m) => panic!("Finitely Presented Module is not bounded")
        }
    }
}

impl FPModuleT for FiniteModule {
    fn fp_idx_to_gen_idx(&self, input_degree : i32, input_index : usize) -> usize {
        match self {
             FiniteModule::FDModule(m) => panic!("Finite Dimensional Module is not finitely presented"),
             FiniteModule::FPModule(m) => m.fp_idx_to_gen_idx(input_degree, input_index)
        }
    }
    fn generators(&self) -> &Arc<FreeModule> {
        match self {
             FiniteModule::FDModule(m) => panic!("Finite Dimensional Module is not finitely presented"),
             FiniteModule::FPModule(m) => &m.generators
        }
    }
}

impl<M : Module> From<BoundedModuleHomomorphism<FiniteModule, M>> for FiniteModuleHomomorphism<M> {
    fn from(f : BoundedModuleHomomorphism<FiniteModule, M>) -> Self {
        FiniteModuleHomomorphism {
            source : f.source(),
            target : f.target(),
            map : FMHI::FD(f)
        }
    }
}

impl<M : Module> From<FPModuleHomomorphism<FiniteModule, M>> for FiniteModuleHomomorphism<M> {
    fn from(f : FPModuleHomomorphism<FiniteModule, M>) -> Self {
        FiniteModuleHomomorphism {
            source : f.source(),
            target : f.target(),
            map : FMHI::FP(f)
        }
    }
}

// Finite Module Homomorphism Interior
enum FMHI<M : Module> {
    FD(BoundedModuleHomomorphism<FiniteModule, M>),
    FP(FPModuleHomomorphism<FiniteModule, M>)
}

pub struct FiniteModuleHomomorphism<M : Module> {
    source : Arc<FiniteModule>,
    target : Arc<M>,
    map : FMHI<M>
}

impl<M : Module> ModuleHomomorphism for FiniteModuleHomomorphism<M> {
    type Source = FiniteModule;
    type Target = M;

    fn source(&self) -> Arc<Self::Source> {
        Arc::clone(&self.source)
    }

    fn target(&self) -> Arc<Self::Target> {
        Arc::clone(&self.target)
    }

    fn degree_shift(&self) -> i32 {
        match &self.map {
            FMHI::FD(f) => f.degree_shift(),
            FMHI::FP(f) => f.degree_shift()
        }
    }

    fn apply_to_basis_element(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_index : usize){
        match &self.map {
            FMHI::FD(f) => f.apply_to_basis_element(result, coeff, input_degree, input_index),
            FMHI::FP(f) => f.apply_to_basis_element(result, coeff, input_degree, input_index)
        }
    }

    fn quasi_inverse(&self, degree : i32) -> &QuasiInverse {
        match &self.map {
            FMHI::FD(f) => f.quasi_inverse(degree),
            FMHI::FP(f) => f.quasi_inverse(degree)
        }
    }

    fn kernel(&self, degree : i32) -> &Subspace {
        match &self.map {
            FMHI::FD(f) => f.kernel(degree),
            FMHI::FP(f) => f.kernel(degree)
        }
    }

    fn compute_kernels_and_quasi_inverses_through_degree(&self, degree : i32) {
        match &self.map {
            FMHI::FD(f) => f.compute_kernels_and_quasi_inverses_through_degree(degree),
            FMHI::FP(f) => f.compute_kernels_and_quasi_inverses_through_degree(degree)
        }
    }
}

impl<M : Module> ZeroHomomorphismT<FiniteModule, M> for FiniteModuleHomomorphism<M> {
    fn zero_homomorphism(source : Arc<FiniteModule>, target : Arc<M>, degree_shift : i32) -> Self {
        let map = match &*source {
            FiniteModule::FDModule(m) => FMHI::FD(BoundedModuleHomomorphism::zero_homomorphism(Arc::clone(&source), Arc::clone(&target), degree_shift)),
            FiniteModule::FPModule(m) => FMHI::FP(FPModuleHomomorphism::zero_homomorphism(Arc::clone(&source), Arc::clone(&target), degree_shift))
        };
        FiniteModuleHomomorphism {
            source,
            target,
            map
        }
    }
}
