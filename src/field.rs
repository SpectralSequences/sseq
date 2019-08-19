use serde_json::Value;
use serde_json::json;

use crate::fp_vector::{FpVector, FpVectorT};
use crate::algebra::Algebra;

pub struct Field {
    prime : u32
}

impl Field {
    pub fn new(p : u32) -> Self {
        Self {
            prime : p
        }
    }
}

impl Algebra for Field {
    fn algebra_type(&self) -> &str {
        &"field"
    }

    /// Returns the prime the algebra is over.
    fn prime(&self) -> u32 {
        self.prime
    }

    fn name(&self) -> &str {
        &"field"
    }

    fn compute_basis(&self, degree : i32){}

    /// Gets the dimension of the algebra in degree `degree`.
    fn dimension(&self, degree : i32, excess : i32) -> usize {
        if degree == 0 { 1 } else { 0 }
    }

    fn multiply_basis_elements(&self, result : &mut FpVector, coeff : u32, r_degree : i32, r_idx : usize, s_degree: i32, s_idx : usize, excess : i32) {
        result.add_basis_element(0, coeff)
    }

    fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
        vec![]
    }

    /// Converts a JSON object into a basis element. The way basis elements are represented by JSON
    /// objects is to be specified by the algebra itself, and will be used by module
    /// specifications.
    fn json_to_basis(&self, json : Value) -> (i32, usize) {
        (0, 0)
    }

    fn json_from_basis(&self, degree : i32, idx : usize) -> Value {
        json!({})
    }

    /// Converts a basis element into a string for display.
    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        assert!(degree == 0);
        "1".to_string()
    }

    fn element_to_string(&self, degree : i32, element : &FpVector) -> String {
        assert!(degree == 0);
        format!("{}", element.entry(0))
    }

    fn generators(&self, degree : i32) -> Vec<usize> {
        vec![]
    }

    fn decompose_basis_element(&self, degree : i32, idx : usize) -> Vec<(u32, (i32, usize), (i32, usize))> {
        vec![]
    }

    fn relations_to_check(&self, degree : i32) -> Vec<Vec<(u32, (i32, usize), (i32, usize))>>{
        vec![]
    }
}