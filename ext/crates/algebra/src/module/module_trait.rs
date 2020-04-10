use fp::prime::ValidPrime;

#[allow(unused_imports)]
use fp::vector::{FpVector, FpVectorT};

use std::sync::Arc;


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
        let p = self.prime();
        for (i, v) in op.iter_nonzero() {
            self.act(result, (coeff * v) % *p, op_degree, i, input_degree, input);
        }
    }

    fn generator_list_string(&self, degree: i32) -> String {
        let mut result = String::from("[");
        result += &(0..self.dimension(degree))
            .map(|idx| self.basis_element_to_string(degree, idx))
            .collect::<Vec<String>>()
            .join(", ");
        result += "]";
        result
    }

    fn element_to_string(&self, degree: i32, element: &FpVector) -> String {
        let mut result = String::new();
        let mut zero = true;
        for (idx, value) in element.iter_nonzero() {
            zero = false;
            if value != 1 {
                result.push_str(&format!("{} ", value));
            }
            let b = self.basis_element_to_string(degree, idx);
            result.push_str(&format!("{} + ", b));
        }
        if zero {
            result.push_str("0");
        } else {
            // Remove trailing " + "
            result.pop();
            result.pop();
            result.pop();
        }
        result
    }

    /// This truncates the module to `max_dim` and represents it as an `FDModule`. This retains the
    /// original name of the module
    fn truncate_to_fd_module(self: Arc<Self>, max_deg: i32) -> FDModule<Self::Algebra> {
        let name = self.name();
        let mut m = TruncatedModule::new(self, max_deg).to_fd_module();
        m.name = name;
        m
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

    fn act(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        input_degree: i32,
        input: &FpVector,
    ) {
        (&**self).act(result, coeff, op_degree, op_index, input_degree, input);
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
        (&**self).act_by_element(result, coeff, op_degree, op, input_degree, input);
    }

    fn generator_list_string(&self, degree: i32) -> String {
        (&**self).generator_list_string(degree)
    }

    fn element_to_string(&self, degree: i32, element: &FpVector) -> String {
        (&**self).element_to_string(degree, element)
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
