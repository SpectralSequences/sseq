use super::{FreeModuleHomomorphism, ModuleHomomorphism};
use crate::module::{FreeModule, Module};
use std::sync::Arc;

/// A composition of two module homomorphisms. This has a more efficient implementation if the
/// first map is a free module homomorphism. Without specialization, we stick to implementing this
/// case.
///
/// This can have a more efficient [`ModuleHomomorphism::apply`] implementation but we do not need
/// it so it's not yet implemented.
pub struct CompositionHomomorphism<'a, M: Module, T: ModuleHomomorphism<Source = M>> {
    left: &'a FreeModuleHomomorphism<M>,
    right: &'a T,
}

impl<'a, M: Module, T: ModuleHomomorphism<Source = M>> CompositionHomomorphism<'a, M, T> {
    pub fn new(left: &'a FreeModuleHomomorphism<M>, right: &'a T) -> Self {
        assert!(Arc::ptr_eq(&left.target(), &right.source()));
        Self { left, right }
    }
}

impl<'a, M: Module, T: ModuleHomomorphism<Source = M>> ModuleHomomorphism
    for CompositionHomomorphism<'a, M, T>
{
    type Source = FreeModule<M::Algebra>;
    type Target = T::Target;

    fn source(&self) -> Arc<Self::Source> {
        self.left.source()
    }

    fn target(&self) -> Arc<Self::Target> {
        self.right.target()
    }

    fn degree_shift(&self) -> i32 {
        self.left.degree_shift() + self.right.degree_shift()
    }

    fn apply_to_basis_element(
        &self,
        result: fp::vector::SliceMut,
        coeff: u32,
        input_degree: i32,
        input_idx: usize,
    ) {
        self.right.apply(
            result,
            coeff,
            input_degree - self.left.degree_shift(),
            self.left.output(input_degree, input_idx).as_slice(),
        );
    }
}
