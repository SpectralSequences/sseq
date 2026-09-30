use std::sync::Arc;

use sseq::coordinates::MultiDegree;

use crate::module::{Module, ModuleExt, ZeroModule};

pub struct SuspensionModule<M: Module> {
    inner: Arc<M>,
    shift: i32,
}

impl<M: Module> SuspensionModule<M> {
    pub fn new(inner: Arc<M>, shift: i32) -> Self {
        Self { inner, shift }
    }
}

impl<M: Module> std::fmt::Display for SuspensionModule<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.shift == 0 {
            self.inner.fmt(f)
        } else {
            self.inner.fmt(f)?;
            write!(f, "[{}]", self.shift)
        }
    }
}

impl<M: Module> Module for SuspensionModule<M> {
    type Algebra = M::Algebra;

    fn compute_basis_multi(&self, degree: MultiDegree<1>) {
        let degree = i32::from(degree);
        self.inner.compute_basis(degree - self.shift);
    }

    fn is_unit(&self) -> bool {
        self.shift == 0 && self.inner.is_unit()
    }

    fn prime(&self) -> fp::prime::ValidPrime {
        self.inner.prime()
    }

    fn max_degree(&self) -> Option<i32> {
        self.inner.max_degree().map(|x| x + self.shift)
    }

    fn max_generator_degree(&self) -> Option<i32> {
        self.inner.max_degree().map(|x| x + self.shift)
    }

    fn total_dimension(&self) -> usize {
        self.inner.total_dimension()
    }

    fn act_multi(
        &self,
        result: fp::vector::FpSliceMut,
        coeff: u32,
        op_degree: MultiDegree<1>,
        op_index: usize,
        input_degree: MultiDegree<1>,
        input: fp::vector::FpSlice,
    ) {
        let op_degree = i32::from(op_degree);
        let input_degree = i32::from(input_degree);
        self.inner.act(
            result,
            coeff,
            op_degree,
            op_index,
            input_degree - self.shift,
            input,
        );
    }

    fn act_by_element_multi(
        &self,
        result: fp::vector::FpSliceMut,
        coeff: u32,
        op_degree: MultiDegree<1>,
        op: fp::vector::FpSlice,
        input_degree: MultiDegree<1>,
        input: fp::vector::FpSlice,
    ) {
        let op_degree = i32::from(op_degree);
        let input_degree = i32::from(input_degree);
        self.inner.act_by_element(
            result,
            coeff,
            op_degree,
            op,
            input_degree - self.shift,
            input,
        );
    }

    fn act_by_element_on_basis_multi(
        &self,
        result: fp::vector::FpSliceMut,
        coeff: u32,
        op_degree: MultiDegree<1>,
        op: fp::vector::FpSlice,
        input_degree: MultiDegree<1>,
        input_index: usize,
    ) {
        let op_degree = i32::from(op_degree);
        let input_degree = i32::from(input_degree);
        self.inner.act_by_element_on_basis(
            result,
            coeff,
            op_degree,
            op,
            input_degree - self.shift,
            input_index,
        );
    }

    fn element_to_string_multi(
        &self,
        degree: MultiDegree<1>,
        element: fp::vector::FpSlice,
    ) -> String {
        let degree = i32::from(degree);
        self.inner.element_to_string(degree - self.shift, element)
    }

    fn algebra(&self) -> std::sync::Arc<Self::Algebra> {
        self.inner.algebra()
    }

    fn min_degree(&self) -> i32 {
        self.inner.min_degree() + self.shift
    }

    fn max_computed_degree(&self) -> i32 {
        self.inner.max_computed_degree() + self.shift
    }

    fn dimension_multi(&self, degree: MultiDegree<1>) -> usize {
        let degree = i32::from(degree);
        self.inner.dimension(degree - self.shift)
    }

    fn act_on_basis_multi(
        &self,
        result: fp::vector::FpSliceMut,
        coeff: u32,
        op_degree: MultiDegree<1>,
        op_index: usize,
        mod_degree: MultiDegree<1>,
        mod_index: usize,
    ) {
        let op_degree = i32::from(op_degree);
        let mod_degree = i32::from(mod_degree);
        self.inner.act_on_basis(
            result,
            coeff,
            op_degree,
            op_index,
            mod_degree - self.shift,
            mod_index,
        );
    }

    fn basis_element_to_string_multi(&self, degree: MultiDegree<1>, idx: usize) -> String {
        let degree = i32::from(degree);
        self.inner.basis_element_to_string(degree - self.shift, idx)
    }
}

impl<M: ZeroModule> ZeroModule for SuspensionModule<M> {
    fn zero_module(algebra: Arc<Self::Algebra>, min_degree: i32) -> Self {
        let inner = Arc::new(M::zero_module(algebra, min_degree));
        Self { inner, shift: 0 }
    }
}
