use crate::module::{BoundedModule, Module};
use fp::vector::FpVector;
use std::sync::Arc;

/// A module M where we quotient out everything above degree `truncation`
pub struct TruncatedModule<M: Module> {
    pub module: Arc<M>,
    pub truncation: i32,
}

impl<M: Module> TruncatedModule<M> {
    pub fn new(module: Arc<M>, truncation: i32) -> Self {
        TruncatedModule { module, truncation }
    }
}

impl<M: Module> BoundedModule for TruncatedModule<M> {
    fn max_degree(&self) -> i32 {
        self.truncation
    }
}

impl<M: Module> Module for TruncatedModule<M> {
    type Algebra = M::Algebra;

    fn algebra(&self) -> Arc<Self::Algebra> {
        self.module.algebra()
    }
    fn name(&self) -> &str {
        ""
    }

    fn min_degree(&self) -> i32 {
        self.module.min_degree()
    }

    fn compute_basis(&self, degree: i32) {
        self.module
            .compute_basis(std::cmp::min(degree, self.truncation));
    }

    fn dimension(&self, degree: i32) -> usize {
        if degree > self.truncation {
            0
        } else {
            self.module.dimension(degree)
        }
    }

    fn act_on_basis(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) {
        if op_degree + mod_degree <= self.truncation {
            self.module
                .act_on_basis(result, coeff, op_degree, op_index, mod_degree, mod_index);
        }
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        if degree > self.truncation {
            "".to_string()
        } else {
            self.module.basis_element_to_string(degree, idx)
        }
    }
}
