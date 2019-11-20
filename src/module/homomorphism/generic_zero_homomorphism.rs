use crate::module::Module;
use crate::fp_vector::FpVector;
use crate::matrix::{Subspace, QuasiInverse};
use crate::module::homomorphism::{ModuleHomomorphism, ZeroHomomorphism};
use std::sync::Arc;


pub struct GenericZeroHomomorphism<S : Module, T : Module> {
    source : Arc<S>,
    target : Arc<T>,
    degree_shift : i32
}

impl<S : Module, T : Module> GenericZeroHomomorphism<S, T> {
    pub fn new(source : Arc<S>, target : Arc<T>, degree_shift : i32) -> Self {
        Self {
            source,
            target,
            degree_shift
        }
    }
}

impl<S : Module, T : Module> ModuleHomomorphism for GenericZeroHomomorphism<S, T> {
    type Source = S;
    type Target = T;

    fn source(&self) -> Arc<Self::Source> {
        Arc::clone(&self.source)
    }

    fn target(&self) -> Arc<Self::Target> {
        Arc::clone(&self.target)
    }

    fn degree_shift(&self) -> i32 {
        self.degree_shift
    }

    fn apply_to_basis_element(&self, _ : &mut FpVector, _ : u32, _ : i32, _ : usize) {}

    fn quasi_inverse(&self, _ : i32) -> &QuasiInverse { unimplemented!() }
    fn kernel(&self, _  : i32) -> &Subspace { unimplemented!() }
    fn compute_kernels_and_quasi_inverses_through_degree(&self, _  : i32) {}
}

impl<S : Module, T : Module> ZeroHomomorphism<S, T> for GenericZeroHomomorphism<S, T> {
    fn zero_homomorphism(source : Arc<S>, target : Arc<T>, degree_shift : i32) -> Self {
        Self::new(source, target, degree_shift)
    }
}
