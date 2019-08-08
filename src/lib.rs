#![allow(dead_code)]
#![allow(unused_variables)]

mod once;
pub mod combinatorics;
pub mod fp_vector;
pub mod matrix;
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
mod resolution_homomorphism;
mod resolution_with_chain_maps;
mod wasm_bindings;
mod cli_module_loaders;


#[cfg(test)]
extern crate rand;

#[cfg(test)]
extern crate rstest;

#[macro_use]
extern crate lazy_static;
extern crate enum_dispatch;

extern crate serde_json;

extern crate wasm_bindgen;
extern crate web_sys;

use crate::algebra::{Algebra, AlgebraAny};
use crate::fp_vector::{FpVector, FpVectorT};
use crate::adem_algebra::AdemAlgebra;
use crate::milnor_algebra::MilnorAlgebra;
use crate::module::{FiniteModule, Module};
use crate::chain_complex::ChainComplexConcentratedInDegreeZero as CCDZ;
use crate::finite_dimensional_module::FiniteDimensionalModule as FDModule;
use crate::resolution::{Resolution, ModuleResolution};
use crate::resolution_with_chain_maps::ResolutionWithChainMaps;

use std::path::PathBuf;
use std::rc::Rc;
use std::error::Error;
use serde_json::value::Value;

pub struct Config {
    pub module_paths : Vec<PathBuf>,
    pub module_file_name : String,
    pub algebra_name : String,
    pub max_degree : i32
}


pub struct AlgebraicObjectsBundle<M : Module> {
    algebra : Rc<AlgebraAny>,
    module : Rc<M>,
    chain_complex : Rc<CCDZ<M>>,
    resolution : Rc<ModuleResolution<M>>
}

pub fn construct(config : &Config) -> Result<AlgebraicObjectsBundle<FiniteModule>, Box<dyn Error>> {
    let contents = load_module_from_file(config)?;
    let mut json : Value = serde_json::from_str(&contents)?;
    let p = json["p"].as_u64().unwrap() as u32;

    // You need a box in order to allow for different possible types implementing the same trait
    let algebra : Rc<AlgebraAny>;
    match config.algebra_name.as_ref() {
        "adem" => algebra = Rc::new(AlgebraAny::from(AdemAlgebra::new(p, p != 2, false))),
        "milnor" => algebra = Rc::new(AlgebraAny::from(MilnorAlgebra::new(p))),
        _ => { return Err(Box::new(InvalidAlgebraError { name : config.algebra_name.clone() })); }
    };    
    let module = Rc::new(FiniteModule::from_json(Rc::clone(&algebra), &config.algebra_name, &mut json)?);
    let chain_complex = Rc::new(CCDZ::new(Rc::clone(&module)));
    let resolution = Rc::new(Resolution::new(Rc::clone(&chain_complex), config.max_degree, None, None));
    Ok(AlgebraicObjectsBundle {
        algebra,
        module,
        chain_complex,
        resolution
    })
}

pub fn run_define_module() -> Result<String, Box<dyn Error>> {
    cli_module_loaders::interactive_module_define()
}

pub fn run_resolve(config : &Config) -> Result<String, Box<dyn Error>> {
    let bundle = construct(config)?;
    bundle.resolution.resolve_through_degree(config.max_degree);
    Ok(bundle.resolution.graded_dimension_string())
}


//use crate::fp_vector::FpVectorT;
// use crate::resolution_homomorphism::ResolutionHomomorphism;
#[allow(unreachable_code)]
pub fn run_test() {
    let p = 3;
    let max_degree = 80;
    let algebra = AdemAlgebra::new(p, p != 2, false);
    algebra.compute_basis(80);
    let idx = algebra.basis_element_to_index(&crate::adem_algebra::AdemBasisElement{
        degree : 60,
        excess : 0,
        bocksteins : 0,
        ps : vec![15]
    });
    let decomposition = algebra.decompose_basis_element(60, idx);
    println!("decomposition : {:?}", decomposition);

    return;
    let max_degree = 25;
    // let contents = std::fs::read_to_string("static/modules/S_3.json").unwrap();
    // S_3
    // let contents = r#"{"type" : "finite dimensional module","name": "$S_3$", "file_name": "S_3", "p": 3, "generic": true, "gens": {"x0": 0}, "sq_actions": [], "adem_actions": [], "milnor_actions": []}"#;
    // C2:
    let contents = r#"{"type" : "finite dimensional module", "name": "$C(2)$", "file_name": "C2", "p": 2, "generic": false, "gens": {"x0": 0, "x1": 1}, "sq_actions": [{"op": 1, "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}], "adem_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}], "milnor_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}]}"#;
    let mut json : Value = serde_json::from_str(&contents).unwrap();
    let p = json["p"].as_u64().unwrap() as u32;
    let algebra = Rc::new(AlgebraAny::from(AdemAlgebra::new(p, p != 2, false)));
    let module = Rc::new(FDModule::from_json(Rc::clone(&algebra), "adem", &mut json));
    let chain_complex = Rc::new(CCDZ::new(Rc::clone(&module)));
    let resolution = Rc::new(Resolution::new(Rc::clone(&chain_complex), max_degree, None, None)); 
    // resolution.resolve_through_degree(max_degree);
    // let f = ResolutionHomomorphism::new("test".to_string(), Rc::clone(&resolution), Rc::clone(&resolution), 1, 4);
    // let mut v = matrix::Matrix::new(p, 1, 1);
    // v[0].set_entry(0, 1);
    // f.extend_step(1, 4, Some(&mut v));
    // f.extend(3, 15);
    
    let mut res_with_maps = ResolutionWithChainMaps::new(Rc::clone(&resolution), Rc::clone(&resolution));
    let mut map_data = crate::matrix::Matrix::new(2, 1, 1);
    map_data[0].set_entry(0, 1);
    res_with_maps.add_self_map(4, 12, "v_1".to_string(), map_data);
    // res_with_maps.add_product(2, 12, 0, "beta".to_string());
    // res_with_maps.add_product(2, 9, 0, "\\alpha_{2}".to_string());
    res_with_maps.resolve_through_degree(max_degree);
    println!("{}", resolution.graded_dimension_string());
}



pub fn load_module_from_file(config : &Config) -> Result<String, Box<dyn Error>> {
    let mut result = None;
    for path in config.module_paths.iter() {
        let mut path = path.clone();
        path.push(&config.module_file_name);
        path.set_extension("json");
        result = std::fs::read_to_string(path).ok();
        if result.is_some() {
            break;
        }
    }
    return result.ok_or_else(|| Box::new(ModuleFileNotFoundError {
        name : config.module_file_name.clone()
    }) as Box<dyn Error>);
}

#[derive(Debug)]
struct ModuleFileNotFoundError {
    name : String
}

impl std::fmt::Display for ModuleFileNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Module file '{}' not found on path", &self.name)
    }
}

impl Error for ModuleFileNotFoundError {
    fn description(&self) -> &str {
        "Module file not found"
    }
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