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
use crate::module::Module;
use crate::finite_dimensional_module::FiniteDimensionalModule;
use crate::chain_complex::{ChainComplex, ChainComplexConcentratedInDegreeZero};
use crate::resolution::Resolution;

use std::error::Error;
use serde_json::value::Value;

pub struct AlgebraicObjectsBundle<'a> {
    algebra : Box<Algebra>,
    module : Option<Box<Module>>,
    chain_complex : Box<ChainComplex>,
    resolution : Box<Resolution<'a>>
}


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

pub fn run(config : Config) -> Result<String, Box<Error>> {
    let max_degree = config.max_degree;
    let bundle = construct(config)?;
    bundle.resolution.resolve_through_degree(max_degree);
    Ok(bundle.resolution.graded_dimension_string())
}


#[allow(non_snake_case)]
pub fn construct(config : Config) -> Result<AlgebraicObjectsBundle<'static>, Box<Error>> {
    let contents = std::fs::read_to_string(format!("static/modules/{}.json", config.module_name))?;
    let mut json : Value = serde_json::from_str(&contents)?;
    let p = json["p"].as_u64().unwrap() as u32;
    let max_degree = config.max_degree;

    // You need a box in order to allow for different possible types implementing the same trait
    let algebra : Box<Algebra>;
    match config.algebra_name.as_ref() {
        "adem" => algebra = Box::new(AdemAlgebra::new(p, p != 2, false)),
        "milnor" => algebra = Box::new(MilnorAlgebra::new(p)),
        _ => { println!("Invalid algebra"); return Err(Box::new(InvalidAlgebraError { name : config.algebra_name })); }
    };
    let algebra_borrow_cast : &'static Box<Algebra> = unsafe { std::mem::transmute(&algebra) };
    let module : Box<Module> = Box::new(FiniteDimensionalModule::from_json(&**algebra_borrow_cast, &config.algebra_name, &mut json));
    let module_borrow_cast : &'static Box<Module> = unsafe { std::mem::transmute(&module) };
    let cc : Box<ChainComplex> = Box::new(ChainComplexConcentratedInDegreeZero::new(&**module_borrow_cast));
    let cc_borrow_cast : &'static Box<ChainComplex> = unsafe { std::mem::transmute(&cc) };
    let res = Box::new(Resolution::new(&**cc_borrow_cast, max_degree, None, None));
    Ok(AlgebraicObjectsBundle {
        algebra,
        module : Some(module),
        chain_complex : cc,
        resolution : res
    })
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
