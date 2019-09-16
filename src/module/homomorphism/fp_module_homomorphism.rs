use std::sync::Arc;

use crate::fp_vector::{FpVector};
use crate::matrix::{Subspace, QuasiInverse};
use crate::module::{Module, FPModule};
use crate::module::homomorphism::{ModuleHomomorphism, FreeModuleHomomorphism, ZeroHomomorphismT};

pub struct FPModuleHomomorphism<M : Module> {
    source : Arc<FPModule>,
    underlying_map : Arc<FreeModuleHomomorphism<M>>
}

impl<M : Module> ModuleHomomorphism for FPModuleHomomorphism<M> {
    type Source = FPModule;
    type Target = M;

    fn source(&self) -> Arc<Self::Source> {
        Arc::clone(&self.source)
    }

    fn target(&self) -> Arc<Self::Target> {
        self.underlying_map.target()
    }

    fn degree_shift(&self) -> i32 {
        self.underlying_map.degree_shift()
    }

    fn apply_to_basis_element(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_index : usize){
        let idx = self.source.fp_idx_to_gen_idx(input_degree, input_index);
        self.underlying_map.extend_by_zero_safe(input_degree);
        self.underlying_map.apply_to_basis_element(result, coeff, input_degree, idx);
    }

    fn quasi_inverse(&self, degree : i32) -> &QuasiInverse {
        &self.underlying_map.quasi_inverse[degree]
    }

    fn kernel(&self, degree : i32) -> &Subspace {
        &self.underlying_map.kernel[degree]
    }

    fn compute_kernels_and_quasi_inverses_through_degree(&self, degree : i32) {
        let lock = self.underlying_map.lock();

        let kernel_len = self.underlying_map.kernel.len();
        let qi_len = self.underlying_map.quasi_inverse.len();
        assert_eq!(kernel_len, qi_len);
        for i in kernel_len ..= degree {
            let (kernel, qi) = self.kernel_and_quasi_inverse(degree);
            self.underlying_map.kernel.push(kernel);
            self.underlying_map.quasi_inverse.push(qi);
        }
    }
}

impl<M : Module> ZeroHomomorphismT<FPModule, M> for FPModuleHomomorphism<M> {
    fn zero_homomorphism(source : Arc<FPModule>, target : Arc<M>, degree_shift : i32) -> Self {
        let underlying_map = Arc::new(FreeModuleHomomorphism::new(Arc::clone(&source.generators), target, degree_shift));
        FPModuleHomomorphism {
            source, underlying_map
        }
    }
}
