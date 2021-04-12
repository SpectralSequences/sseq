use itertools::Itertools;
use std::sync::Arc;

use fp::prime::ValidPrime;
use fp::vector::{FpVector, Slice, SliceMut};

use crate::algebra::Algebra;
#[cfg(feature = "extras")]
use crate::module::BoundedModule;
#[cfg(feature = "extras")]
use crate::module::FDModule;
#[cfg(feature = "extras")]
use crate::module::TruncatedModule;

pub trait Module: std::fmt::Display + Send + Sync + 'static {
    type Algebra: Algebra;

    fn algebra(&self) -> Arc<Self::Algebra>;
    fn min_degree(&self) -> i32;
    fn compute_basis(&self, _degree: i32) {}
    fn max_computed_degree(&self) -> i32;
    fn dimension(&self, degree: i32) -> usize;
    fn act_on_basis(
        &self,
        result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    );

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String;

    /// Whether this is the unit module.
    fn is_unit(&self) -> bool {
        false
    }

    fn prime(&self) -> ValidPrime {
        self.algebra().prime()
    }

    /// Whether act_on_basis_borrow is available.
    fn borrow_output(&self) -> bool {
        false
    }

    /// Returns a borrow of the value of the corresponding action on the basis element.
    fn act_on_basis_borrow(
        &self,
        _op_degree: i32,
        _op_index: usize,
        _mod_degree: i32,
        _mod_index: usize,
    ) -> &FpVector {
        unimplemented!()
    }

    fn act(
        &self,
        mut result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        input_degree: i32,
        input: Slice,
    ) {
        assert!(input.dimension() <= self.dimension(input_degree));
        let p = self.prime();
        for (i, v) in input.iter_nonzero() {
            self.act_on_basis(
                result.copy(),
                (coeff * v) % *p,
                op_degree,
                op_index,
                input_degree,
                i,
            );
        }
    }

    fn act_by_element(
        &self,
        mut result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op: Slice,
        input_degree: i32,
        input: Slice,
    ) {
        assert_eq!(input.dimension(), self.dimension(input_degree));
        assert_eq!(
            op.dimension(),
            self.algebra().dimension(op_degree, i32::max_value())
        );
        let p = self.prime();
        for (i, v) in op.iter_nonzero() {
            self.act(
                result.copy(),
                (coeff * v) % *p,
                op_degree,
                i,
                input_degree,
                input,
            );
        }
    }

    fn act_by_element_on_basis(
        &self,
        mut result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op: Slice,
        input_degree: i32,
        input_index: usize,
    ) {
        assert_eq!(
            op.dimension(),
            self.algebra().dimension(op_degree, i32::max_value())
        );
        let p = self.prime();
        for (i, v) in op.iter_nonzero() {
            self.act_on_basis(
                result.copy(),
                (coeff * v) % *p,
                op_degree,
                i,
                input_degree,
                input_index,
            );
        }
    }

    fn basis_string_list(&self, degree: i32) -> Vec<String> {
        (0..self.dimension(degree))
            .map(|idx| self.basis_element_to_string(degree, idx))
            .collect()
        // let formatter = (0..self.dimension(degree))
        //     .map(|idx| self.basis_element_to_string(degree, idx))
        //     .format(", ");
        // format!("[{}]", formatter)
    }

    fn element_to_string(&self, degree: i32, element: Slice) -> String {
        let result = element
            .iter_nonzero()
            .map(|(idx, value)| {
                let coeff = if value == 1 {
                    "".to_string()
                } else {
                    format!("{} ", value)
                };
                let basis_elt = self.basis_element_to_string(degree, idx);
                format!("{}{}", coeff, basis_elt)
            })
            .join(" + ");
        if result.is_empty() {
            "0".to_string()
        } else {
            result
        }
    }

    /// This truncates the module to `max_dim` and represents it as an `FDModule`. This retains the
    /// original name of the module
    #[cfg(feature = "extras")]
    fn truncate_to_fd_module(self: Arc<Self>, max_deg: i32) -> FDModule<Self::Algebra> {
        let name = self.to_string();
        let mut m = TruncatedModule::new(self, max_deg).to_fd_module();
        m.name = name;
        m
    }

    /// op1(op2(x)) - (op1*op2)(x)
    fn check_relation(
        &self,
        result: &mut FpVector,
        scratch: &mut FpVector,
        outer_op_degree: i32,
        outer_op_index: usize,
        inner_op_degree: i32,
        inner_op_index: usize,
        module_degree: i32,
        module_index: usize,
    ) {
        result.set_scratch_vector_size(
            self.dimension(outer_op_degree + inner_op_degree + module_degree),
        );
        scratch.set_scratch_vector_size(self.dimension(inner_op_degree + module_degree));
        self.act_on_basis(
            scratch.as_slice_mut(),
            1,
            inner_op_degree,
            inner_op_index,
            module_degree,
            module_index,
        );
        self.act(
            result.as_slice_mut(),
            1,
            outer_op_degree,
            outer_op_index,
            inner_op_degree + module_degree,
            scratch.as_slice(),
        );
        // println!("scratch 1 : {}", self.element_to_string(inner_op_degree + module_degree, &scratch));
        // println!("result 1 : {}", self.element_to_string(outer_op_degree + inner_op_degree + module_degree, &result));
        scratch.set_scratch_vector_size(
            self.algebra()
                .dimension(outer_op_degree + inner_op_degree, i32::max_value()),
        );
        self.algebra().multiply_basis_elements(
            scratch.as_slice_mut(),
            1,
            outer_op_degree,
            outer_op_index,
            inner_op_degree,
            inner_op_index,
            i32::max_value(),
        );
        self.act_by_element_on_basis(
            result.as_slice_mut(),
            *self.prime() - 1,
            outer_op_degree + inner_op_degree,
            scratch.as_slice(),
            module_degree,
            module_index,
        );
        // println!("result 2 : {}", self.element_to_string(outer_op_degree + inner_op_degree + module_degree, &result));
    }

    /// Input: degree through which to check.
    /// Output: Vec of discrepancies.
    fn check_relations(
        &self,
        max_degree: i32,
    ) -> Vec<((i32, usize, i32, usize, i32, usize), FpVector)> {
        let mut result = Vec::new();
        let algebra = self.algebra();
        let p = self.prime();
        let mut scratch_vec = FpVector::new(p, 0);
        let mut discrepancy_vec = FpVector::new(p, 0);
        algebra.compute_basis(max_degree);
        self.compute_basis(max_degree);
        for outer_op_degree in 0..=max_degree {
            for outer_op_index in 0..algebra.dimension(outer_op_degree, i32::max_value()) {
                for inner_op_degree in 0..=max_degree - outer_op_degree {
                    for inner_op_index in 0..algebra.dimension(inner_op_degree, i32::max_value()) {
                        for module_degree in 0..=max_degree - outer_op_degree - inner_op_degree {
                            for module_index in 0..self.dimension(module_degree) {
                                self.check_relation(
                                    &mut discrepancy_vec,
                                    &mut scratch_vec,
                                    outer_op_degree,
                                    outer_op_index,
                                    inner_op_degree,
                                    inner_op_index,
                                    module_degree,
                                    module_index,
                                );
                                if !discrepancy_vec.is_zero() {
                                    result.push((
                                        (
                                            outer_op_degree,
                                            outer_op_index,
                                            inner_op_degree,
                                            inner_op_index,
                                            module_degree,
                                            module_index,
                                        ),
                                        discrepancy_vec.clone(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        result
    }

    fn test_relations(&self, max_degree: i32, max_failures_to_display: usize) {
        let discrepancies = self.check_relations(max_degree);
        let algebra = self.algebra();
        if !discrepancies.is_empty() {
            let formatter = discrepancies.iter().take(max_failures_to_display).format_with("\n\n   ========= \n\n  ", 
                |(
                    tuple,
                    discrepancy_vec
                ), f| {
                    let &(outer_op_degree, outer_op_index,
                        inner_op_degree, inner_op_index,
                        module_degree, module_index)
                    = tuple;
                    f(&format_args!(
                        "{outer_op_degree}, {outer_op_index}, {inner_op_degree}, {inner_op_index}, {module_degree}, {module_index}\n\
                        {op1}({op2}({m})) - ({op1} * {op2})({m}) == {disc}",
                        outer_op_degree = outer_op_degree,
                        outer_op_index = outer_op_index,
                        inner_op_degree = inner_op_degree,
                        inner_op_index = inner_op_index,
                        module_degree = module_degree,
                        module_index = module_index,
                        op1 = algebra.basis_element_to_string(outer_op_degree, outer_op_index),
                        op2 = algebra.basis_element_to_string(inner_op_degree, inner_op_index),
                        m = self.basis_element_to_string(module_degree, module_index),
                        disc = self.element_to_string(outer_op_degree + inner_op_degree + module_degree, discrepancy_vec.as_slice())
                    ))
                }
            );
            panic!("Discrepancies:\n  {}", formatter);
        }
    }
}

impl<A: Algebra> Module for Arc<dyn Module<Algebra = A>> {
    type Algebra = A;

    fn algebra(&self) -> Arc<Self::Algebra> {
        (&**self).algebra()
    }

    fn min_degree(&self) -> i32 {
        (&**self).min_degree()
    }

    fn max_computed_degree(&self) -> i32 {
        (&**self).max_computed_degree()
    }

    fn compute_basis(&self, degree: i32) {
        (&**self).compute_basis(degree);
    }
    fn dimension(&self, degree: i32) -> usize {
        (&**self).dimension(degree)
    }

    fn act_on_basis(
        &self,
        result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) {
        (&**self).act_on_basis(result, coeff, op_degree, op_index, mod_degree, mod_index);
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        (&**self).basis_element_to_string(degree, idx)
    }

    // Whether this is the unit module.
    fn is_unit(&self) -> bool {
        (&**self).is_unit()
    }

    fn prime(&self) -> ValidPrime {
        (&**self).prime()
    }

    /// Whether act_on_basis_borrow is available.
    fn borrow_output(&self) -> bool {
        (&**self).borrow_output()
    }

    /// Returns a borrow of the value of the corresponding action on the basis element. This
    /// FpVector must be "pure", i.e. it is not sliced and the limbs are zero in indices greater
    /// than the dimension of the vector.
    fn act_on_basis_borrow(
        &self,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) -> &FpVector {
        (&**self).act_on_basis_borrow(op_degree, op_index, mod_degree, mod_index)
    }
}

#[derive(Debug)]
pub struct ModuleFailedRelationError {
    pub relation: String,
    pub value: String,
}

impl std::fmt::Display for ModuleFailedRelationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Relation failed:\n    {}  !=  0\nInstead it is equal to {}\n",
            &self.relation, &self.value
        )
    }
}

impl std::error::Error for ModuleFailedRelationError {}
