use itertools::Itertools;
use std::sync::Arc;


use fp::prime::ValidPrime;
use fp::vector::{FpVector, FpVectorT};

use crate::algebra::Algebra;
use crate::module::FDModule;
use crate::module::TruncatedModule;
use crate::module::bounded_module::BoundedModule;

pub trait Module: Send + Sync + 'static {
    type Algebra: Algebra;

    fn algebra(&self) -> Arc<Self::Algebra>;
    fn name(&self) -> String;
    fn min_degree(&self) -> i32;
    fn compute_basis(&self, _degree: i32) {}
    fn max_computed_degree(&self) -> i32;
    fn dimension(&self, degree: i32) -> usize;
    fn act_on_basis(
        &self,
        result: &mut FpVector,
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

    /// Returns a borrow of the value of the corresponding action on the basis element. This
    /// FpVector must be "pure", i.e. it is not sliced and the limbs are zero in indices greater
    /// than the dimension of the vector.
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
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        input_degree: i32,
        input: &FpVector,
    ) {
        assert!(input.dimension() <= self.dimension(input_degree));
        let p = self.prime();
        for (i, v) in input.iter_nonzero() {
            self.act_on_basis(
                result,
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
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op: &FpVector,
        input_degree: i32,
        input: &FpVector,
    ) {
        assert_eq!(input.dimension(), self.dimension(input_degree));
        assert_eq!(op.dimension(), self.algebra().dimension(op_degree, i32::max_value()));
        let p = self.prime();
        for (i, v) in op.iter_nonzero() {
            self.act(result, (coeff * v) % *p, op_degree, i, input_degree, input);
        }
    }

    fn act_by_element_on_basis(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op: &FpVector,
        input_degree: i32,
        input_index: usize,
    ) {
        assert_eq!(op.dimension(), self.algebra().dimension(op_degree, i32::max_value()));
        let p = self.prime();
        for (i, v) in op.iter_nonzero() {
            self.act_on_basis(result, (coeff * v) % *p, op_degree, i, input_degree, input_index);
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

    fn element_to_string(&self, degree: i32, element: &FpVector) -> String {
        let result = element.iter_nonzero().map(|(idx, value)|{
            let coeff = if value == 1 { 
                "".to_string()
            } else {
                format!("{} ", value)
            };
            let basis_elt = self.basis_element_to_string(degree, idx);
            format!("{}{}", coeff, basis_elt)
        }).join(" + ");
        if result.len() == 0 {
            "0".to_string()
        } else {
            result
        }
    }

    /// This truncates the module to `max_dim` and represents it as an `FDModule`. This retains the
    /// original name of the module
    fn truncate_to_fd_module(self: Arc<Self>, max_deg: i32) -> FDModule<Self::Algebra> {
        let name = self.name();
        let mut m = TruncatedModule::new(self, max_deg).to_fd_module();
        m.name = name;
        m
    }

    /// op1(op2(x)) - (op1*op2)(x)
    fn check_relation(&self, 
        result : &mut FpVector, scratch : &mut FpVector,
        outer_op_degree : i32, outer_op_index : usize, 
        inner_op_degree : i32, inner_op_index : usize,
        module_degree : i32, module_index : usize
    ) {
        result.set_scratch_vector_size(self.dimension(outer_op_degree + inner_op_degree + module_degree));
        result.set_to_zero_pure();
        scratch.set_scratch_vector_size(self.dimension(inner_op_degree + module_degree));
        scratch.set_to_zero_pure();
        self.act_on_basis(scratch, 1, inner_op_degree, inner_op_index, module_degree, module_index);
        self.act(result, 1, outer_op_degree, outer_op_index, inner_op_degree + module_degree, scratch);
        scratch.set_scratch_vector_size(self.algebra().dimension(outer_op_degree + inner_op_degree, i32::max_value()));
        scratch.set_to_zero_pure();
        self.algebra().multiply_basis_elements(scratch, 1,  outer_op_degree, outer_op_index, inner_op_degree, inner_op_index, i32::max_value());
        self.act_by_element_on_basis(result, *self.prime() - 1, outer_op_degree + inner_op_degree, scratch, module_degree, module_index);
    }

    fn check_relations(&self, max_degree : i32) -> Vec<((i32, usize, i32, usize, i32, usize), FpVector)> {
        let mut result = Vec::new();
        let algebra = self.algebra();
        let p = self.prime();
        let mut scratch_vec = FpVector::new(p, 0);
        let mut discrepancy_vec = FpVector::new(p, 0);
        algebra.compute_basis(max_degree);
        self.compute_basis(max_degree);
        for outer_op_degree in 0 ..= max_degree {
            for outer_op_index in 0 .. algebra.dimension(outer_op_degree, i32::max_value()) {
                for inner_op_degree in 0 ..= max_degree - outer_op_degree {
                    for inner_op_index in 0 .. algebra.dimension(inner_op_degree, i32::max_value()) {
                        for module_degree in 0 ..= max_degree - outer_op_degree - inner_op_degree {
                            for module_index in 0..self.dimension(module_degree) {
                                self.check_relation(&mut discrepancy_vec, &mut scratch_vec,
                                    outer_op_degree, outer_op_index, 
                                    inner_op_degree, inner_op_index,
                                    module_degree, module_index
                                );
                                if !discrepancy_vec.is_zero() {
                                    result.push((
                                        (outer_op_degree, outer_op_index, 
                                        inner_op_degree, inner_op_index,
                                        module_degree, module_index),
                                        discrepancy_vec.clone()
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
}

impl<A: Algebra> Module for Arc<dyn Module<Algebra = A>> {
    type Algebra = A;

    fn algebra(&self) -> Arc<Self::Algebra> {
        (&**self).algebra()
    }

    fn name(&self) -> String {
        (&**self).name()
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
        result: &mut FpVector,
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

impl std::error::Error for ModuleFailedRelationError {
    fn description(&self) -> &str {
        "Module failed a relation"
    }
}
