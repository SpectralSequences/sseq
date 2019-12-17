use serde_json::Value;
use serde_json::json;

use crate::fp_vector::{FpVector, FpVectorT};
use crate::algebra::{Algebra, Bialgebra};

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
    #[cfg(feature = "prime-two")]
    fn prime(&self) -> u32 {
        2
    }

    #[cfg(not(feature = "prime-two"))]
    fn prime(&self) -> u32 {
        self.prime
    }

    fn name(&self) -> &str {
        &"field"
    }

    fn compute_basis(&self, _degree : i32){}

    /// Gets the dimension of the algebra in degree `degree`.
    fn dimension(&self, degree : i32, _excess : i32) -> usize {
        if degree == 0 { 1 } else { 0 }
    }

    fn multiply_basis_elements(&self, result : &mut FpVector, coeff : u32, _r_degree : i32, _r_idx : usize, _s_degree: i32, _s_idx : usize, _excess : i32) {
        result.add_basis_element(0, coeff)
    }

    fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
        vec![]
    }

    /// Converts a JSON object into a basis element. The way basis elements are represented by JSON
    /// objects is to be specified by the algebra itself, and will be used by module
    /// specifications.
    fn json_to_basis(&self, _json : Value) -> (i32, usize) {
        (0, 0)
    }

    fn json_from_basis(&self, _degree : i32, _idx : usize) -> Value {
        json!({})
    }

    /// Converts a basis element into a string for display.
    fn basis_element_to_string(&self, degree : i32, _idx : usize) -> String {
        assert!(degree == 0);
        "1".to_string()
    }

    fn element_to_string(&self, degree : i32, element : &FpVector) -> String {
        assert!(degree == 0);
        format!("{}", element.entry(0))
    }

    fn generators(&self, _degree : i32) -> Vec<usize> {
        vec![]
    }

    fn decompose_basis_element(&self, _degree : i32, _idx : usize) -> Vec<(u32, (i32, usize), (i32, usize))> {
        vec![]
    }

    fn relations_to_check(&self, _degree : i32) -> Vec<Vec<(u32, (i32, usize), (i32, usize))>>{
        vec![]
    }
}

impl Bialgebra for Field {
    fn coproduct (&self, _op_deg : i32, _op_idx : usize) -> Vec<(i32, usize, i32, usize)> {
        vec![(1, 0, 1, 0)]
    }
    fn decompose (&self, _op_deg : i32, _op_idx : usize) -> Vec<(i32, usize)> {
        vec![(1, 0)]
    }
}
