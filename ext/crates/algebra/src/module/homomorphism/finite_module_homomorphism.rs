use std::sync::Arc;

use crate::algebra::SteenrodAlgebra;
use crate::module::homomorphism::FPModuleT;
use crate::module::homomorphism::{
    BoundedModuleHomomorphism, FPModuleHomomorphism, GenericZeroHomomorphism, IdentityHomomorphism,
    ModuleHomomorphism, ZeroHomomorphism,
};
use crate::module::{FiniteModule, FreeModule, SteenrodModule};
use fp::matrix::{QuasiInverse, Subspace};
use fp::vector::SliceMut;

impl FPModuleT for FiniteModule {
    fn fp_idx_to_gen_idx(&self, degree: i32, index: usize) -> usize {
        self.as_fp_module()
            .unwrap()
            .fp_idx_to_gen_idx(degree, index)
    }
    fn gen_idx_to_fp_idx(&self, degree: i32, index: usize) -> isize {
        self.as_fp_module()
            .unwrap()
            .gen_idx_to_fp_idx(degree, index)
    }
    fn generators(&self) -> Arc<FreeModule<SteenrodAlgebra>> {
        self.as_fp_module().unwrap().generators()
    }
}

impl<M: SteenrodModule> From<BoundedModuleHomomorphism<FiniteModule, M>>
    for FiniteModuleHomomorphism<M>
{
    fn from(f: BoundedModuleHomomorphism<FiniteModule, M>) -> Self {
        FiniteModuleHomomorphism {
            source: f.source(),
            target: f.target(),
            map: FMHI::FD(f),
        }
    }
}

impl<M: SteenrodModule> From<FPModuleHomomorphism<FiniteModule, M>>
    for FiniteModuleHomomorphism<M>
{
    fn from(f: FPModuleHomomorphism<FiniteModule, M>) -> Self {
        FiniteModuleHomomorphism {
            source: f.source(),
            target: f.target(),
            map: FMHI::FP(f),
        }
    }
}

// Finite Module Homomorphism Interior
#[allow(clippy::upper_case_acronyms)]
enum FMHI<M: SteenrodModule> {
    FD(BoundedModuleHomomorphism<FiniteModule, M>),
    FP(FPModuleHomomorphism<FiniteModule, M>),
    RP(GenericZeroHomomorphism<FiniteModule, M>),
}

pub struct FiniteModuleHomomorphism<M: SteenrodModule> {
    source: Arc<FiniteModule>,
    target: Arc<M>,
    map: FMHI<M>,
}

impl<M: SteenrodModule> ModuleHomomorphism for FiniteModuleHomomorphism<M> {
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
            FMHI::RP(f) => f.degree_shift(),
            FMHI::FP(f) => f.degree_shift(),
        }
    }

    fn apply_to_basis_element(
        &self,
        result: SliceMut,
        coeff: u32,
        input_degree: i32,
        input_index: usize,
    ) {
        match &self.map {
            FMHI::FD(f) => f.apply_to_basis_element(result, coeff, input_degree, input_index),
            FMHI::RP(f) => f.apply_to_basis_element(result, coeff, input_degree, input_index),
            FMHI::FP(f) => f.apply_to_basis_element(result, coeff, input_degree, input_index),
        }
    }

    fn quasi_inverse(&self, degree: i32) -> Option<&QuasiInverse> {
        match &self.map {
            FMHI::FD(f) => f.quasi_inverse(degree),
            FMHI::RP(f) => f.quasi_inverse(degree),
            FMHI::FP(f) => f.quasi_inverse(degree),
        }
    }

    fn kernel(&self, degree: i32) -> Option<&Subspace> {
        match &self.map {
            FMHI::FD(f) => f.kernel(degree),
            FMHI::RP(f) => f.kernel(degree),
            FMHI::FP(f) => f.kernel(degree),
        }
    }

    fn image(&self, degree: i32) -> Option<&Subspace> {
        match &self.map {
            FMHI::FD(f) => f.image(degree),
            FMHI::RP(f) => f.image(degree),
            FMHI::FP(f) => f.image(degree),
        }
    }

    fn compute_auxiliary_data_through_degree(&self, degree: i32) {
        match &self.map {
            FMHI::FD(f) => f.compute_auxiliary_data_through_degree(degree),
            FMHI::RP(f) => f.compute_auxiliary_data_through_degree(degree),
            FMHI::FP(f) => f.compute_auxiliary_data_through_degree(degree),
        }
    }
}

impl<M: SteenrodModule> ZeroHomomorphism<FiniteModule, M> for FiniteModuleHomomorphism<M> {
    fn zero_homomorphism(source: Arc<FiniteModule>, target: Arc<M>, degree_shift: i32) -> Self {
        let map = match &*source {
            FiniteModule::FDModule(_) => FMHI::FD(BoundedModuleHomomorphism::zero_homomorphism(
                Arc::clone(&source),
                Arc::clone(&target),
                degree_shift,
            )),
            FiniteModule::RealProjectiveSpace(_) => {
                FMHI::RP(GenericZeroHomomorphism::zero_homomorphism(
                    Arc::clone(&source),
                    Arc::clone(&target),
                    degree_shift,
                ))
            }
            FiniteModule::FPModule(_) => FMHI::FP(FPModuleHomomorphism::zero_homomorphism(
                Arc::clone(&source),
                Arc::clone(&target),
                degree_shift,
            )),
        };
        FiniteModuleHomomorphism {
            source,
            target,
            map,
        }
    }
}

impl IdentityHomomorphism<FiniteModule> for FiniteModuleHomomorphism<FiniteModule> {
    fn identity_homomorphism(source: Arc<FiniteModule>) -> Self {
        let map = match &*source {
            FiniteModule::FDModule(_) => FMHI::FD(
                BoundedModuleHomomorphism::identity_homomorphism(Arc::clone(&source)),
            ),
            FiniteModule::RealProjectiveSpace(_) => {
                panic!("Identity morphism not supported for RealProjectiveSpace")
            }
            FiniteModule::FPModule(_) => FMHI::FP(FPModuleHomomorphism::identity_homomorphism(
                Arc::clone(&source),
            )),
        };
        FiniteModuleHomomorphism {
            source: Arc::clone(&source),
            target: source,
            map,
        }
    }
}
