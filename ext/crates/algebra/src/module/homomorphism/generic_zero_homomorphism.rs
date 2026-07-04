use std::sync::Arc;

use crate::{
    algebra::{BaseRingOf, Scalar},
    linear_algebra::{BaseSliceMutOf, GradedDvr},
    module::{
        Module,
        homomorphism::{ModuleHomomorphism, ZeroHomomorphism},
    },
};

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
where
    BaseRingOf<S::Algebra>: GradedDvr,
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

    fn apply_to_basis_element(
        &self,
        _: BaseSliceMutOf<'_, <Self::Source as Module>::Algebra>,
        _: Scalar<<Self::Source as Module>::Algebra>,
        _: i32,
        _: usize,
    ) {
    }
}

impl<S: Module, T: Module<Algebra = S::Algebra>> ZeroHomomorphism<S, T>
    for GenericZeroHomomorphism<S, T>
where
    BaseRingOf<S::Algebra>: GradedDvr,
{
    fn zero_homomorphism(source: Arc<S>, target: Arc<T>, degree_shift: i32) -> Self {
        Self::new(source, target, degree_shift)
    }
}
