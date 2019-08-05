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
extern crate enum_dispatch;

extern crate serde_json;

extern crate wasm_bindgen;
extern crate web_sys;

use crate::algebra::Algebra;
use crate::adem_algebra::AdemAlgebra;
use crate::milnor_algebra::MilnorAlgebra;
// use crate::module::Module;
use crate::finite_dimensional_module::{FiniteDimensionalModule as FDModule, OptionFDModule};
use crate::module_homomorphism::{ZeroHomomorphism}; //ModuleHomomorphism
use crate::chain_complex::{ChainComplexConcentratedInDegreeZero as CCDZ}; // ChainComplex,
use crate::resolution::Resolution;

use std::rc::Rc;
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

pub struct AlgebraicObjectsBundle {
    algebra : Rc<dyn Algebra>,
    module : Option<Rc<FDModule>>,
    chain_complex : Rc<CCDZ<FDModule>>,
    resolution : Box<Resolution<
                    OptionFDModule, 
                    ZeroHomomorphism<OptionFDModule, OptionFDModule>,
                    CCDZ<FDModule>
                >>
}

pub fn construct(config : &Config) -> Result<AlgebraicObjectsBundle, Box<dyn Error>> {
    let contents = std::fs::read_to_string(&config.module_path)?;
    let mut json : Value = serde_json::from_str(&contents)?;
    let p = json["p"].as_u64().unwrap() as u32;

    // You need a box in order to allow for different possible types implementing the same trait
    let algebra : Rc<dyn Algebra>;
    match config.algebra_name.as_ref() {
        "adem" => algebra = Rc::new(AdemAlgebra::new(p, p != 2, false)),
        "milnor" => algebra = Rc::new(MilnorAlgebra::new(p)),
        _ => { return Err(Box::new(InvalidAlgebraError { name : config.algebra_name.clone() })); }
    };
    let module : Rc<FDModule> = Rc::new(FDModule::from_json(Rc::clone(&algebra), &config.algebra_name, &mut json));
    let cc : Rc<CCDZ<FDModule>> = Rc::new(CCDZ::new(Rc::clone(&module)));
    let res = Box::new(Resolution::new(Rc::clone(&cc), config.max_degree, None, None));

    Ok(AlgebraicObjectsBundle {
        algebra,
        module : Some(module),
        chain_complex: cc,
        resolution: res
    })
}

pub fn run(config : &Config) -> Result<String, Box<dyn Error>> {
    let bundle = construct(&config)?;
    bundle.resolution.resolve_through_degree(config.max_degree);
    Ok(bundle.resolution.graded_dimension_string())
}

pub struct Config {
    pub module_path : String,
    pub algebra_name : String,
    pub max_degree : i32
}
