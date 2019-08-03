#![allow(dead_code)]
#![allow(unused_variables)]

mod once;
mod combinatorics;
mod fp_vector;
mod matrix;
mod algebra;
mod adem_algebra;
mod milnor_algebra;
mod module;
mod module_homomorphism;
mod finite_dimensional_module;
mod free_module;
mod free_module_homomorphism;
mod chain_complex;
mod resolution;
mod wasm_bindings;

#[cfg(test)]
extern crate rand;

#[macro_use]
extern crate lazy_static;

extern crate serde_derive;
extern crate serde;
extern crate serde_json;

extern crate wasm_bindgen;
extern crate web_sys;

use crate::algebra::Algebra;
use crate::adem_algebra::AdemAlgebra;
use crate::milnor_algebra::MilnorAlgebra;
use crate::finite_dimensional_module::FiniteDimensionalModule;
use crate::chain_complex::ChainComplexConcentratedInDegreeZero;
use crate::resolution::Resolution;

use std::error::Error;
use serde_json::value::Value;

#[derive(Debug)]
struct InvalidAlgebraError {
    name : String
}

impl std::fmt::Display for InvalidAlgebraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid algebra: {}", &self.name)
    }
}

impl Error for InvalidAlgebraError {
    fn description(&self) -> &str {
        "Invalid algebra supplied"
    }
}

macro_rules! construct {
    ($config: expr, $algebra:ident, $module:ident, $cc:ident, $res:ident) => {
        let contents = std::fs::read_to_string(format!("static/modules/{}.json", $config.module_name))?;
        let mut json : Value = serde_json::from_str(&contents)?;
        let p = json["p"].as_u64().unwrap() as u32;

        // You need a box in order to allow for different possible types implementing the same trait
        let $algebra : Box<Algebra>;
        match $config.algebra_name.as_ref() {
            "adem" => $algebra = Box::new(AdemAlgebra::new(p, p != 2, false)),
            "milnor" => $algebra = Box::new(MilnorAlgebra::new(p)),
            _ => { return Err(Box::new(InvalidAlgebraError { name : $config.algebra_name })); }
        };
        let $module = FiniteDimensionalModule::from_json(&*$algebra, &$config.algebra_name, &mut json);
        let $cc = ChainComplexConcentratedInDegreeZero::new(&$module);
        let $res = Box::new(Resolution::new(&$cc, $config.max_degree, None, None));
    };
}

pub fn run(config : Config) -> Result<String, Box<Error>> {
    construct!(config, algebra, module, cc, res);

    res.resolve_through_degree(config.max_degree);
    Ok(res.graded_dimension_string())
}

pub struct Config {
    pub module_name : String,
    pub algebra_name : String,
    pub max_degree : i32
}

impl Config {
    pub fn new(args: &[String]) -> Result<Self, String> {
        if args.len() < 4 {
            return Err("Not enough arguments".to_string());
        }
        let module_name = args[1].clone();
        let algebra_name = args[2].clone();
        let max_deg_result : Result<i32,_> = args[3].parse();

        if let Err(error) = max_deg_result {
            return Err(format!("{} in argument max_degree.", error));
        }
        let max_degree = max_deg_result.unwrap();
        Ok(Self { module_name, algebra_name, max_degree })
    }
}
