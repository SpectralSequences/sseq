#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::module::{BoundedModule, Module};
use fp::matrix::Subspace;
use fp::vector::{FpVector};
use once::OnceBiVec;
use std::sync::Arc;

/// Given a module `module`, this is the quotient of `module` by a collection of basis elements.
///
/// # Fields
///  * `module` - The original module
///  * `basis` - For each degree `d`, `basis[d]` is the list of basis elements of `module` that are
///  *not* quotiented out
///  * `mask` - This is the mask that corresponds to `basis`. Applying the mask will project to the
///  subspace defined by `basis`.
pub struct QuotientModule<M: Module> {
    pub module: Arc<M>,
    pub subspaces: OnceBiVec<Subspace>,
    pub basis_list: OnceBiVec<Vec<usize>>,
}

impl<M: Module> QuotientModule<M> {
    pub fn new(module: Arc<M>) -> Self {
        let min_deg = module.min_degree();
        QuotientModule {
            module,
            subspaces: OnceBiVec::new(min_deg),
            basis_list: OnceBiVec::new(min_deg),
        }
    }

    pub fn quotient(&mut self, degree: i32, element: &FpVector) {
        self.subspaces[degree].add_vector(element);
        self.flush(degree);
    }

    pub fn quotient_basis_elements(
        &mut self,
        degree: i32,
        elements: impl std::iter::Iterator<Item = usize>,
    ) {
        self.subspaces[degree].add_basis_elements(elements);
        self.flush(degree);
    }

    pub fn quotient_vectors(&mut self, degree: i32, vecs: Vec<FpVector>) {
        self.subspaces[degree].add_vectors(vecs.into_iter());
        self.flush(degree);
    }

    fn flush(&mut self, degree: i32) {
        let mut vec = Vec::with_capacity(self.basis_list[degree].len());
        for i in 0..self.module.dimension(degree) {
            if self.subspaces[degree].pivots()[i] < 0 {
                vec.push(i);
            }
        }
        self.basis_list[degree] = vec;
    }

    pub fn quotient_all(&mut self, degree: i32) {
        self.subspaces[degree].set_to_entire();
        self.basis_list[degree] = Vec::new();
    }

    pub fn act_on_original_basis(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) {
        self.module
            .act_on_basis(result, coeff, op_degree, op_index, mod_degree, mod_index);
        self.reduce(op_degree + mod_degree, result)
    }

    pub fn reduce(&self, degree: i32, vec: &mut FpVector) {
        self.subspaces[degree].reduce(vec);
    }

    pub fn old_basis_to_new(&self, degree: i32, new: &mut FpVector, old: &FpVector) {
        for (i, idx) in self.basis_list[degree].iter().enumerate() {
            new.add_basis_element(i, old.entry(*idx));
        }
    }
}

impl<M: Module> Module for QuotientModule<M> {
    type Algebra = M::Algebra;
    fn algebra(&self) -> Arc<Self::Algebra> {
        self.module.algebra()
    }
    fn name(&self) -> String {
        format!("Quotient of {}", self.module.name())
    }

    fn min_degree(&self) -> i32 {
        self.module.min_degree()
    }

    fn compute_basis(&self, degree: i32) {
        self.module.compute_basis(degree);

        for i in self.subspaces.len()..=degree {
            let dim = self.module.dimension(i);
            self.subspaces
                .push(Subspace::new(self.prime(), dim + 1, dim));
            self.basis_list.push((0..dim).collect::<Vec<_>>());
        }
    }

    fn max_computed_degree(&self) -> i32 {
        self.subspaces.len()
    }

    fn dimension(&self, degree: i32) -> usize {
        self.module.dimension(degree) - self.subspaces[degree].dimension()
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
        let target_deg = op_degree + mod_degree;

        let mut result_ = FpVector::new(self.prime(), self.module.dimension(target_deg));
        self.act_on_original_basis(
            &mut result_,
            coeff,
            op_degree,
            op_index,
            mod_degree,
            self.basis_list[mod_degree][mod_index],
        );
        self.reduce(target_deg, &mut result_);
        self.old_basis_to_new(target_deg, result, &result_);
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        self.module
            .basis_element_to_string(degree, self.basis_list[degree][idx])
    }
}

impl<M: Module + BoundedModule> BoundedModule for QuotientModule<M> {
    fn max_degree(&self) -> i32 {
        self.module.max_degree()
    }
}
