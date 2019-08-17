#![allow(dead_code)]
#![allow(unused_variables)]

pub mod once;
pub mod combinatorics;
pub mod fp_vector;
pub mod matrix;
pub mod block_structure;
pub mod algebra;
pub mod field;
pub mod adem_algebra;
pub mod milnor_algebra;
pub mod change_of_basis;
pub mod steenrod_parser;
pub mod steenrod_evaluator;
pub mod module;
pub mod module_homomorphism;
pub mod finite_dimensional_module;
pub mod free_module;
pub mod free_module_homomorphism;
pub mod finitely_presented_module;
pub mod chain_complex;
pub mod hom_space;
pub mod hom_pullback;
pub mod hom_complex;
pub mod resolution;
pub mod resolution_homomorphism;
#[cfg(target_arch = "wasm32")]
pub mod wasm_bindings;
mod cli_module_loaders;


#[cfg(test)]
extern crate rand;

#[cfg(test)]
extern crate rstest;

#[macro_use]
extern crate lazy_static;
extern crate enum_dispatch;

extern crate serde_json;
extern crate serde;

#[cfg(target_arch = "wasm32")]
extern crate wasm_bindgen;
extern crate web_sys;
extern crate bivec;

// Parser
extern crate nom;

use crate::algebra::{Algebra, AlgebraAny};
use crate::adem_algebra::AdemAlgebra;
use crate::milnor_algebra::MilnorAlgebra;
use crate::module::{FiniteModule, Module};
use crate::matrix::Matrix;
use crate::fp_vector::FpVectorT;
use crate::chain_complex::ChainComplexConcentratedInDegreeZero as CCDZ;
use crate::finite_dimensional_module::FiniteDimensionalModule as FDModule;
use crate::resolution::{Resolution, ModuleResolution};

use std::path::PathBuf;
use std::rc::Rc;
use std::cell::RefCell;
use std::error::Error;
use serde_json::value::Value;

pub struct Config {
    pub module_paths : Vec<PathBuf>,
    pub module_file_name : String,
    pub algebra_name : String,
    pub max_degree : i32
}


pub struct AlgebraicObjectsBundle<M : Module> {
    pub algebra : Rc<AlgebraAny>,
    pub module : Rc<M>,
    pub chain_complex : Rc<CCDZ<M>>,
    pub resolution : Rc<RefCell<ModuleResolution<M>>>
}

pub fn construct(config : &Config) -> Result<AlgebraicObjectsBundle<FiniteModule>, Box<dyn Error>> {
    let contents = load_module_from_file(config)?;
    let json = serde_json::from_str(&contents)?;

    construct_from_json(json, config.algebra_name.clone())
}

pub fn construct_from_json(mut json : Value, algebra_name : String) -> Result<AlgebraicObjectsBundle<FiniteModule>, Box<dyn Error>> {
    let algebra = construct_algebra_from_json(&json, algebra_name)?;
    let module = Rc::new(FiniteModule::from_json(Rc::clone(&algebra), &mut json)?);
    let chain_complex = Rc::new(CCDZ::new(Rc::clone(&module)));
    let resolution = Rc::new(RefCell::new(Resolution::new(Rc::clone(&chain_complex), None, None)));
    resolution.borrow_mut().set_self(Rc::downgrade(&resolution));

    let products_value = &json["products"];
    if !products_value.is_null() {
        let products = products_value.as_array().unwrap();
        for prod in products {
            let hom_deg = prod["hom_deg"].as_u64().unwrap() as u32;
            let int_deg = prod["int_deg"].as_i64().unwrap() as i32;
            let idx = prod["index"].as_u64().unwrap() as usize;
            let name = prod["name"].as_str().unwrap();
            resolution.borrow_mut().add_product(hom_deg, int_deg, idx, name.to_string());
        }
    }

    let self_maps = &json["self_maps"];
    if !self_maps.is_null() {
        for self_map in self_maps.as_array().unwrap() {
            let s = self_map["hom_deg"].as_u64().unwrap() as u32;
            let t = self_map["int_deg"].as_i64().unwrap() as i32;
            let name = self_map["name"].as_str().unwrap();

            let json_map_data = self_map["map_data"].as_array().unwrap();
            let json_map_data : Vec<&Vec<Value>> = json_map_data
                .iter()
                .map(|x| x.as_array().unwrap())
                .collect();

            let rows = json_map_data.len();
            let cols = json_map_data[0].len();
            let mut map_data = Matrix::new(algebra.prime(), rows, cols);
            for r in 0..rows {
                for c in 0..cols {
                    map_data[r].set_entry(c, json_map_data[r][c].as_u64().unwrap() as u32);
                }
            }
            resolution.borrow_mut().add_self_map(s, t, name.to_string(), map_data);
        }
    }

    Ok(AlgebraicObjectsBundle {
        algebra,
        module,
        chain_complex,
        resolution
    })
}
pub fn construct_algebra_from_json(json : &Value, mut algebra_name : String) -> Result<Rc<AlgebraAny>, Box<dyn Error>> {
    let p = json["p"].as_u64().unwrap() as u32;
    let algebra_list = json["algebra"].as_array();
    if let Some(list) = algebra_list {
        let list : Vec<&str> = list.iter().map(|x| x.as_str().unwrap()).collect();
        if !list.contains(&algebra_name.as_ref()) {
            println!("Module does not support algebra {}", algebra_name);
            println!("Using {} instead", list[0]);
            algebra_name = list[0].to_string();
        }
    }

    let mut algebra : AlgebraAny;
    match algebra_name.as_ref() {
        "adem" => algebra = AlgebraAny::from(AdemAlgebra::new(p, p != 2, false)),
        "milnor" => {
            let mut algebra_inner = MilnorAlgebra::new(p);
            let profile = &json["profile"];
            if !profile.is_null() {
                 if let Some(truncated) = profile["truncated"].as_bool() {
                     algebra_inner.profile.truncated = truncated;
                 }
                 if let Some(q_part) = profile["q_part"].as_u64() {
                     algebra_inner.profile.q_part = q_part as u32;
                 }
                 if let Some(p_part) = profile["p_part"].as_array() {
                     let p_part = p_part.into_iter().map(|x| x.as_u64().unwrap() as u32).collect();
                     algebra_inner.profile.p_part = p_part;
                 }
            }
            algebra = AlgebraAny::from(algebra_inner);
        }
        _ => { return Err(Box::new(InvalidAlgebraError { name : algebra_name })); }
    };
    Ok(Rc::new(algebra))
}
pub fn run_define_module() -> Result<String, Box<dyn Error>> {
    cli_module_loaders::interactive_module_define()
}

pub fn run_resolve(config : &Config) -> Result<String, Box<dyn Error>> {
    let bundle = construct(config)?;
    let res = bundle.resolution.borrow();
    res.resolve_through_degree(config.max_degree);
    Ok(res.graded_dimension_string())
}


//use crate::fp_vector::FpVectorT;
// use crate::resolution_homomorphism::ResolutionHomomorphism;
#[allow(unreachable_code)]
#[allow(unused_mut)]
pub fn run_test() {
    let p = 2;
    let max_degree = 16;
    let adem = AdemAlgebra::new(p, p != 2, false);
    let milnor = MilnorAlgebra::new(p);//, p != 2
    adem.compute_basis(max_degree);
    milnor.compute_basis(max_degree);
    let idx = adem.basis_element_to_index(&crate::adem_algebra::AdemBasisElement {
        degree : 7,
        excess : 0,
        bocksteins : 0,
        ps : vec![4, 2, 1]
    });
    let mut result = fp_vector::FpVector::new(p, adem.get_dimension(7, -1));
    change_of_basis::adem_to_milnor_on_basis(&adem, &milnor, &mut result, 1, 7, idx);
    println!("Sq4 Sq2 Sq1 ==> {}\n\n", milnor.element_to_string(7, &result));
    for i in 2..3 {
        let degree = (1 << (i + 1)) - 1;
        let mut result = crate::fp_vector::FpVector::new(p, adem.get_dimension(degree, -1));
        crate::change_of_basis::get_adem_q(&adem, &milnor, &mut result, 1, i);
        println!("Q{} ==> {}", i, adem.element_to_string(degree, &result));
    }
    
    let p = 2;
    let max_degree = 30;
    let adem = AdemAlgebra::new(p, p != 2, false);
    let milnor = MilnorAlgebra::new(p);//, p != 2
    adem.compute_basis(max_degree);
    milnor.compute_basis(max_degree);
    let degree = 9;
    let i = 4;
    let dim = adem.get_dimension(degree, -1);
    let mut adem_result = crate::fp_vector::FpVector::new(p, dim);
    // crate::change_of_basis::milnor_to_adem_on_basis(&adem, &milnor, &mut adem_result, 1, degree, i);
    return;

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

    let max_degree = 25;
    // let contents = std::fs::read_to_string("static/modules/S_3.json").unwrap();
    // S_3
    // let contents = r#"{"type" : "finite dimensional module","name": "$S_3$", "file_name": "S_3", "p": 3, "generic": true, "gens": {"x0": 0}, "sq_actions": [], "adem_actions": [], "milnor_actions": []}"#;
    // C2:
    let contents = r#"{"type" : "finite dimensional module", "name": "$C(2)$", "file_name": "C2", "p": 2, "generic": false, "gens": {"x0": 0, "x1": 1}, "sq_actions": [{"op": 1, "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}], "adem_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}], "milnor_actions": [{"op": [1], "input": "x0", "output": [{"gen": "x1", "coeff": 1}]}]}"#;
    let mut json : Value = serde_json::from_str(&contents).unwrap();
    let p = json["p"].as_u64().unwrap() as u32;
    let algebra = Rc::new(AlgebraAny::from(AdemAlgebra::new(p, p != 2, false)));
    let module = Rc::new(FDModule::from_json(Rc::clone(&algebra), &mut json));
    let chain_complex = Rc::new(CCDZ::new(Rc::clone(&module)));
    let resolution = Rc::new(RefCell::new(Resolution::new(Rc::clone(&chain_complex), None, None)));
    // resolution.borrow_mut().resolve_through_degree(max_degree);
    // let f = ResolutionHomomorphism::new("test".to_string(), Rc::clone(&resolution), Rc::clone(&resolution), 1, 4);
    // let mut v = matrix::Matrix::new(p, 1, 1);
    // v[0].set_entry(0, 1);
    // f.extend_step(1, 4, Some(&mut v));
    // f.extend(3, 15);
    
//    let mut res_with_maps = ResolutionWithChainMaps::new(Rc::clone(&resolution), Rc::clone(&resolution));
//    let mut map_data = crate::matrix::Matrix::new(2, 1, 1);
//    map_data[0].set_entry(0, 1);
//    // res_with_maps.add_self_map(4, 12, "v_1".to_string(), map_data);
//    res_with_maps.add_product(2, 12, 0, "beta".to_string());
//    res_with_maps.add_product(2, 9, 0, "\\alpha_{2}".to_string());
//    res_with_maps.resolve_through_degree(max_degree);
//    println!("{}", resolution.borrow().graded_dimension_string());
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
