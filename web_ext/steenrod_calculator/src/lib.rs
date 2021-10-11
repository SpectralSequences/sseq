use algebra::steenrod_evaluator::{evaluate_algebra_adem, evaluate_algebra_milnor};
use algebra::{AdemAlgebra, Algebra, MilnorAlgebra};
use fp::prime::ValidPrime;
use std::convert::TryFrom;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct SteenrodCalculator {
    adem_algebra: AdemAlgebra,
    milnor_algebra: MilnorAlgebra,
}

#[wasm_bindgen]
impl SteenrodCalculator {
    pub fn new(p: u32) -> Option<SteenrodCalculator> {
        let p = ValidPrime::try_from(p).ok()?;
        Some(Self {
            adem_algebra: AdemAlgebra::new(p, *p != 2, false, false),
            milnor_algebra: MilnorAlgebra::new(p),
        })
    }

    pub fn compute_basis(&self, degree: i32) {
        self.adem_algebra.compute_basis(degree);
        self.milnor_algebra.compute_basis(degree);
    }

    pub fn evaluate_adem(&self, input: &str) -> Result<String, JsValue> {
        evaluate_algebra_adem(&self.adem_algebra, &self.milnor_algebra, input)
            .map(|(d, v)| self.adem_algebra.element_to_string(d, v.as_slice()))
            .map_err(|e| JsValue::from(e.to_string()))
    }

    pub fn evaluate_milnor(&self, input: &str) -> Result<String, JsValue> {
        evaluate_algebra_milnor(&self.adem_algebra, &self.milnor_algebra, input)
            .map(|(d, v)| self.milnor_algebra.element_to_string(d, v.as_slice()))
            .map_err(|e| JsValue::from(e.to_string()))
    }
}
