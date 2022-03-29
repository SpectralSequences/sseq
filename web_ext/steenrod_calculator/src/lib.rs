use algebra::steenrod_evaluator::SteenrodEvaluator;
use algebra::Algebra;
use fp::prime::ValidPrime;
use std::convert::TryFrom;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct SteenrodCalculator(SteenrodEvaluator);

#[wasm_bindgen]
impl SteenrodCalculator {
    pub fn new(p: u32) -> Option<SteenrodCalculator> {
        let p = ValidPrime::try_from(p).ok()?;
        Some(Self(SteenrodEvaluator::new(p)))
    }

    pub fn evaluate_adem(&self, input: &str) -> Result<String, JsValue> {
        self.0
            .evaluate_algebra_adem(input)
            .map(|(d, v)| self.0.adem.element_to_string(d, v.as_slice()))
            .map_err(|e| JsValue::from(e.to_string()))
    }

    pub fn evaluate_milnor(&self, input: &str) -> Result<String, JsValue> {
        self.0
            .evaluate_algebra_milnor(input)
            .map(|(d, v)| self.0.milnor.element_to_string(d, v.as_slice()))
            .map_err(|e| JsValue::from(e.to_string()))
    }
}
