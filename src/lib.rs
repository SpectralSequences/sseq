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
use std::io::{stdin, stdout, Write};
use std::fmt::Display;
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
struct ModuleFailedRelationError {
    relation : String,
    value : String
}

impl std::fmt::Display for ModuleFailedRelationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Relation failed:\n    {}  !=  0\nInstead it is equal to {}\n", &self.relation, &self.value)
    }
}

impl Error for ModuleFailedRelationError {
    fn description(&self) -> &str {
        "Module failed a relation"
    }
}

pub struct AlgebraicObjectsBundle<M : Module> {
    algebra : Rc<AlgebraAny>,
    module : Rc<M>,
    chain_complex : Rc<CCDZ<M>>,
    resolution : Rc<ModuleResolution<M>>
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

fn query_with_default<T : FromStr + Display>(prompt : &str, default : T) -> T {
    loop {
        print!("{} (default {}): ", prompt, default);
        stdout().flush().unwrap();
        let mut input = String::new();
        stdin().read_line(&mut input).expect(&format!("Error reading for prompt: {}", prompt));
        let trimmed = input.trim();
        if trimmed.len() == 0 {
            return default;
        }
        if let Ok(res) = trimmed.parse::<T>() {
            return res;
        }
        println!("Invalid input. Try again");
    }
}

fn query_with_default_no_default_indicated<T : FromStr + Display>(prompt : &str, default : T) -> T {
    loop {
        print!("{}: ", prompt);
        stdout().flush().unwrap();
        let mut input = String::new();
        stdin().read_line(&mut input).expect(&format!("Error reading for prompt: {}", prompt));
        let trimmed = input.trim();
        if trimmed.len() == 0 {
            return default;
        }
        if let Ok(res) = trimmed.parse::<T>() {
            return res;
        }
        println!("Invalid input. Try again");
    }
}


pub fn run_interactive() -> Result<String, Box<dyn Error>>{
    // Query for prime and max_degree
    let mut p;
    loop {
        p = query_with_default("p", 2);
        if crate::combinatorics::is_valid_prime(p) {
            break;
        }
        println!("Invalid input. Try again");
    }

    let max_degree = query_with_default("Max degree", 30);

    let algebra : Rc<AlgebraAny>;
    loop {
        match query_with_default("Algebra basis (adem/milnor)", "adem".to_string()).as_ref() {
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
    let finished_degree = usize::max_value();
    loop {
        let gen_deg = query_with_default_no_default_indicated::<usize>("Generator degree", finished_degree);
        if gen_deg == finished_degree {
            println!("This is the list of generators and degrees:");
            for i in 0..gens.len() {
                for gen in &gens[i] {
                    print!("({}, {}) ", i, gen)
                }
            }
            print!("\n");
            if query::<String>("Is it okay? (yes/no)").starts_with("y") {
                break;
            } else {
                if query::<String>("Reset generator list? (yes/no)").starts_with("y") {
                    gens = Vec::new();
                }
                continue;
            }
        }
        while gens.len() <= gen_deg {
            gens.push(Vec::new());
        }
        let gen_name = query_with_default("Generator name", format!("x{}{}",gen_deg, gens[gen_deg].len()));        
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
                let mut output_vec = FpVector::new(p, gens[output_deg].len(), 0);
                for op_idx in 0..algebra.get_dimension(deg_diff, -1) {
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
                            module.set_action_vector(deg_diff, op_idx, input_deg as i32, idx, &output_vec);
                            break;
                        }
                    } else {
                        let decomposition = algebra.decompose_basis_element(deg_diff, op_idx);
                        println!("decomposition : {:?}", decomposition);
                        for (coef, (deg_1, idx_1), (deg_2, idx_2)) in decomposition {
                            let mut tmp_output = FpVector::new(p, gens[deg_2 as usize + input_deg].len(), 0);
                            module.act_on_basis(&mut tmp_output, 1, deg_2, idx_2, input_deg as i32, idx);
                            module.act(&mut output_vec, coef, deg_1, idx_1, deg_2 + input_deg as i32, &tmp_output);
                        }
                        println!("computed {} action on {}: {}", algebra.basis_element_to_string(deg_diff, op_idx), gens[input_deg][idx], output_vec);
                        module.set_action_vector(deg_diff, op_idx, input_deg as i32, idx, &output_vec);
                    }
                    output_vec.set_to_zero();
                }
                for op_idx in 0..algebra.get_dimension(deg_diff, -1) {
                    let relations = algebra.get_relations_to_check(deg_diff);
                    for relation in relations {
                        for (coef, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                            let mut tmp_output = FpVector::new(p, gens[*deg_2 as usize + input_deg].len(), 0);
                            module.act_on_basis(&mut tmp_output, 1, *deg_2, *idx_2, input_deg as i32, idx);
                            module.act(&mut output_vec, *coef, *deg_1, *idx_1, *deg_2 + input_deg as i32, &tmp_output);                        
                        }
                        if !output_vec.is_zero() {
                            let mut relation_string = String::new();
                            for (coef, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                                relation_string.push_str(&format!("{} * {} * {}  +  ", 
                                    *coef, 
                                    &algebra.basis_element_to_string(*deg_1, *idx_1), 
                                    &algebra.basis_element_to_string(*deg_2, *idx_2))
                                );
                            }
                            relation_string.pop(); relation_string.pop(); relation_string.pop();
                            relation_string.pop(); relation_string.pop();

                            let value_string = module.element_to_string(output_deg as i32, &output_vec);
                            return Err(Box::new(ModuleFailedRelationError {relation : relation_string, value : value_string}));
                        }
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
// use crate::resolution_homomorphism::ResolutionHomomorphism;
pub fn test(config : &Config){
    test_no_config();
}
#[allow(unreachable_code)]
pub fn test_no_config(){
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

pub fn run(config : &Config) -> Result<String, Box<dyn Error>> {
    let bundle = construct(config)?;
    bundle.resolution.resolve_through_degree(config.max_degree);
    Ok(bundle.resolution.graded_dimension_string())
}

pub struct Config {
    pub module_paths : Vec<PathBuf>,
    pub module_file_name : String,
    pub algebra_name : String,
    pub max_degree : i32
}
