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


#[cfg(test)]
extern crate rand;

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
use crate::module::Module;
use crate::finite_dimensional_module::FiniteDimensionalModule as FDModule;
use crate::finitely_presented_module::FinitelyPresentedModule as FPModule;
use crate::chain_complex::ChainComplexConcentratedInDegreeZero as CCDZ;
use crate::resolution::{Resolution, ModuleResolution};
use crate::resolution_with_chain_maps::ResolutionWithChainMaps;

use std::io::{stdin, stdout, Write};
use std::str::FromStr;
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

#[derive(Debug)]
struct UnknownModuleType {
    module_type : String
}

impl std::fmt::Display for UnknownModuleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown module type: {}", &self.module_type)
    }
}

impl Error for UnknownModuleType {
    fn description(&self) -> &str {
        "Unknown module type"
    }
}

pub struct AlgebraicObjectsBundle<M : Module> {
    algebra : Rc<AlgebraAny>,
    module : Rc<M>,
    chain_complex : Rc<CCDZ<M>>,
    resolution : Rc<ModuleResolution<M>>
}

pub enum AlgebraicObjectsBundleChoice {
    FinitelyPresented(AlgebraicObjectsBundle<FPModule>),
    FiniteDimensional(AlgebraicObjectsBundle<FDModule>)
}

impl AlgebraicObjectsBundleChoice {
    pub fn resolve_through_degree(&self, max_degree : i32) {
        match self {
            AlgebraicObjectsBundleChoice::FinitelyPresented(bundle) => bundle.resolution.resolve_through_degree(max_degree),
            AlgebraicObjectsBundleChoice::FiniteDimensional(bundle) => bundle.resolution.resolve_through_degree(max_degree),
        }
    }

    pub fn graded_dimension_string(&self) -> String {
        match self {
            AlgebraicObjectsBundleChoice::FinitelyPresented(bundle) => bundle.resolution.graded_dimension_string(),
            AlgebraicObjectsBundleChoice::FiniteDimensional(bundle) => bundle.resolution.graded_dimension_string(),
        }        
    }
}

pub fn construct_helper<M : Module + Sized>(config : &Config, mut json : Value) -> Result<AlgebraicObjectsBundle<M>, Box<dyn Error>> {
    let p = json["p"].as_u64().unwrap() as u32;

    // You need a box in order to allow for different possible types implementing the same trait
    let algebra : Rc<AlgebraAny>;
    match config.algebra_name.as_ref() {
        "adem" => algebra = Rc::new(AlgebraAny::from(AdemAlgebra::new(p, p != 2, false))),
        "milnor" => algebra = Rc::new(AlgebraAny::from(MilnorAlgebra::new(p))),
        _ => { return Err(Box::new(InvalidAlgebraError { name : config.algebra_name.clone() })); }
    };    
    let module = Rc::new(M::from_json(Rc::clone(&algebra), &config.algebra_name, &mut json));
    let chain_complex = Rc::new(CCDZ::new(Rc::clone(&module)));
    let resolution = Rc::new(Resolution::new(Rc::clone(&chain_complex), config.max_degree, None, None));
    Ok(AlgebraicObjectsBundle {
        algebra,
        module,
        chain_complex,
        resolution
    })
}

pub fn construct(config : &Config) -> Result<AlgebraicObjectsBundleChoice, Box<dyn Error>> {
    let contents = std::fs::read_to_string(&config.module_path)?;
    let json : Value = serde_json::from_str(&contents)?;
    let module_type = &json["type"].as_str().unwrap();
    match module_type {
        &"finite dimensional module" => {
            let bundle = construct_helper(config, json)?;
            Ok(AlgebraicObjectsBundleChoice::FiniteDimensional(bundle))
        },
        &"finitely presented module" => {
            let bundle = construct_helper(config, json)?;
            Ok(AlgebraicObjectsBundleChoice::FinitelyPresented(bundle))
        }
        _ => Err(Box::new(UnknownModuleType { module_type : module_type.to_string() }))
    }
}

fn query<T : FromStr>(prompt : &str) -> T {
    loop {
        print!("{} : ", prompt);
        stdout().flush().unwrap();
        let mut input = String::new();
        stdin().read_line(&mut input).expect(&format!("Error reading for prompt: {}", prompt));
        if let Ok(res) = input.trim().parse::<T>() {
            return res;
        }
        println!("Invalid input. Try again");
    }
}

pub fn run_interactive() -> Result<String, Box<dyn Error>>{
    // Query for prime and max_degree
    let p = query::<u32>("p");
    let max_degree = query::<i32>("Max degree");

    let algebra : Rc<AlgebraAny>;
    loop {
        match query::<String>("Algebra basis (adem/milnor)").as_ref() {
            "adem" => { algebra = Rc::new(AlgebraAny::from(AdemAlgebra::new(p, p != 2, false))); break },
            "milnor" => { algebra = Rc::new(AlgebraAny::from(MilnorAlgebra::new(p))); break },
            _ => ()
        };
        println!("Invalid input. Try again");
    }

    // Query for generators
    println!("Input generators. Press return to finish.");
    stdout().flush()?;

    let mut gens = Vec::new();
    loop {
         let gen_name = query::<String>("Generator name");
         if gen_name.is_empty() {
             println!("This is the list of generators and degrees:");
             for i in 0..gens.len() {
                 for gen in &gens[i] {
                     print!("({}, {}) ", i, gen)
                 }
             }
             print!("\n");
             if query::<String>("Is it okay? (yes/no)") == "yes" {
                 break;
             } else {
                 gens = Vec::new();
                 continue;
             }
         }
         let gen_deg = query::<usize>("Generator degree");
         while gens.len() <= gen_deg {
             gens.push(Vec::new());
         }
         gens[gen_deg].push(gen_name);
    }

    let graded_dim = gens.iter().map(Vec::len).collect();

    algebra.compute_basis(std::cmp::max(max_degree, gens.len() as i32));

    let generators : Vec<Vec<usize>> = (0..gens.len()+1).map(|d| algebra.get_algebra_generators(d as i32)).collect();

    let mut module = FDModule::new(Rc::clone(&algebra), "".to_string(), 0, graded_dim);

    println!("Input actions. Write the value of the action in the form 'a x0 + b x1 + ...' where a, b are non-negative integers and x0, x1 are names of the generators. The coefficient can be omitted if it is 1");

    let len = gens.len();
    for input_deg in (0..len).rev() {
        for idx in 0..gens[input_deg].len() {
            for output_deg in (input_deg+1)..len {
                let deg_diff = (output_deg - input_deg) as i32;
                if gens[output_deg].len() == 0 {
                    continue;
                }

                for op_idx in 0..algebra.get_dimension(deg_diff, -1) {
                    let mut output_vec = FpVector::new(p, gens[output_deg].len(), 0);

                    if generators[deg_diff as usize].contains(&op_idx) {
                        'outer: loop {
                            let result = query::<String>(&format!("{} {}", algebra.basis_element_to_string(deg_diff, op_idx), gens[input_deg][idx]));

                            if result == "0" {
                                break;
                            }
                            for term in result.split("+") {
                                let term = term.trim();
                                let parts : Vec<&str> = term.split(" ").collect();
                                if parts.len() == 1 {
                                    match gens[output_deg].iter().position(|d| d == &parts[0]) {
                                        Some(i) => output_vec.add_basis_element(i, 1),
                                        None => { println!("Invalid value. Try again"); continue 'outer }
                                    };
                                } else if parts.len() == 2 {
                                    let gen_idx = match gens[output_deg].iter().position(|d| d == &parts[1]) {
                                        Some(i) => i,
                                        None => { println!("Invalid value. Try again"); continue 'outer }
                                    };
                                    let coef = match parts[1].parse::<u32>() {
                                        Ok(c) => c,
                                        _ => { println!("Invalid value. Try again"); continue 'outer }
                                    };
                                    output_vec.add_basis_element(gen_idx, coef);
                                } else {
                                    println!("Invalid value. Try again"); continue 'outer;
                                }
                            }
                            module.set_action_vector(deg_diff, op_idx, input_deg as i32, idx, output_vec);
                            break;
                        }
                    } else {
                        let decomposition = algebra.decompose_basis_element(deg_diff, op_idx);
                        for (coef, (deg_1, idx_1), (deg_2, idx_2)) in decomposition {
                            let mut tmp_output = FpVector::new(p, gens[deg_2 as usize + input_deg].len(), 0);
                            module.act_on_basis(&mut tmp_output, 1, deg_2, idx_2, input_deg as i32, idx);
                            module.act(&mut output_vec, coef, deg_1, idx_1, deg_2 + input_deg as i32, &tmp_output);
                        }
                        module.set_action_vector(deg_diff, op_idx, input_deg as i32, idx, output_vec);
                    }
                }
            }
        }
    }
    let chain_complex = Rc::new(CCDZ::new(Rc::new(module)));
    let resolution = Rc::new(Resolution::new(Rc::clone(&chain_complex), max_degree, None, None));

    resolution.resolve_through_degree(max_degree);
    Ok(resolution.graded_dimension_string())
}

//use crate::fp_vector::FpVectorT;
use crate::resolution_homomorphism::ResolutionHomomorphism;
pub fn test(config : &Config){
    test_no_config();
}

pub fn test_no_config(){
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

pub fn run(config : &Config) -> Result<String, Box<dyn Error>> {
    let bundle = construct(&config)?;
    bundle.resolve_through_degree(config.max_degree);
    Ok(bundle.graded_dimension_string())
}

pub struct Config {
    pub module_path : String,
    pub algebra_name : String,
    pub max_degree : i32
}
