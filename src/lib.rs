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
mod finitely_presented_module;
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
    let contents = std::fs::read_to_string(format!("static/modules/{}.json", config.module_name))?;
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

use crate::fp_vector::FpVectorT;
use crate::module::Module;
use crate::free_module::FreeModule;
use crate::module_homomorphism::ModuleHomomorphism;
use crate::finitely_presented_module::FinitelyPresentedModule;
pub fn test(){
    let p = 2;
    let algebra : Rc<Algebra> = Rc::new(AdemAlgebra::new(p, p != 2, false));
    let mut fpmod = finitely_presented_module::FinitelyPresentedModule::new(Rc::clone(&algebra), "A/(Sq1,Sq2)".to_string(), 0);
    algebra.compute_basis(5);
    fpmod.generators.add_generators_immediate(0, 1);
    fpmod.generators.extend_by_zero(3);
    fpmod.relations.add_generators_immediate(0, 0);
    fpmod.relations.add_generators_immediate(1, 1);
    fpmod.relations.add_generators_immediate(2, 1);
    let mut output_matrix = matrix::Matrix::new(2, 1, 1);
    output_matrix[0].set_entry(0, 1);
    {
        let map = &mut fpmod.map;
        let mut map_lock = map.get_lock();
        map.add_generators_from_matrix_rows(&map_lock, 0, &mut output_matrix, 0, 0, 0);
        *map_lock += 1;
        map.add_generators_from_matrix_rows(&map_lock, 1, &mut output_matrix, 0, 0, 1);
        *map_lock += 1;
        map.add_generators_from_matrix_rows(&map_lock, 2, &mut output_matrix, 0, 0, 1);
        *map_lock += 1;
    }
    let max_degree = 20;
    let cc : Rc<CCDZ<FinitelyPresentedModule>> = Rc::new(CCDZ::new(Rc::new(fpmod)));
    let res = Box::new(Resolution::new(Rc::clone(&cc), max_degree, None, None));
    res.resolve_through_degree(max_degree);
    println!("{}", res.graded_dimension_string());
    std::process::exit(1);
}

pub fn run(config : &Config) -> Result<String, Box<dyn Error>> {
    let bundle = construct(&config)?;
    bundle.resolution.resolve_through_degree(config.max_degree);
    Ok(bundle.resolution.graded_dimension_string())
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
