use crate::module::homomorphism::{ModuleHomomorphism, ZeroHomomorphism};
use crate::module::Module;
use fp::vector::SliceMut;
use std::sync::Arc;

pub struct GenericZeroHomomorphism<S: Module, T: Module<Algebra = S::Algebra>> {
    source: Arc<S>,
    target: Arc<T>,
    degree_shift: i32,
}

impl<S: Module, T: Module<Algebra = S::Algebra>> GenericZeroHomomorphism<S, T> {
    pub fn new(source: Arc<S>, target: Arc<T>, degree_shift: i32) -> Self {
        Self {
            source,
            target,
            degree_shift,
        }
    }
}

impl<S: Module, T: Module<Algebra = S::Algebra>> ModuleHomomorphism
    for GenericZeroHomomorphism<S, T>
{
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

    fn apply_to_basis_element(&self, _: SliceMut, _: u32, _: i32, _: usize) {}
}

impl<S: Module, T: Module<Algebra = S::Algebra>> ZeroHomomorphism<S, T>
    for GenericZeroHomomorphism<S, T>
{
    fn zero_homomorphism(source: Arc<S>, target: Arc<T>, degree_shift: i32) -> Self {
        Self::new(source, target, degree_shift)
    }
}
