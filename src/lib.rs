#![allow(dead_code)]
#![allow(unused_variables)]

pub mod combinatorics;
pub mod fp_vector;
pub mod matrix;
pub mod block_structure;
pub mod algebra;
pub mod change_of_basis;
pub mod steenrod_parser;
pub mod steenrod_evaluator;
pub mod module;
pub mod chain_complex;
pub mod resolution;
pub mod resolution_homomorphism;
mod cli_module_loaders;
mod yoneda;

use algebra::{Algebra, AlgebraAny};
use module::{FiniteModule, FDModule, Module, BoundedModule};
use module::homomorphism::{FiniteModuleHomomorphism};
use matrix::Matrix;
use fp_vector::FpVectorT;
use chain_complex::{FiniteChainComplex, ChainComplex};
use resolution::Resolution;
use resolution_homomorphism::ResolutionHomomorphism;
use yoneda::yoneda_representative;

use bivec::BiVec;
use query::*;

use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use serde_json::value::Value;

pub struct Config {
    pub module_paths : Vec<PathBuf>,
    pub module_file_name : String,
    pub algebra_name : String,
    pub max_degree : i32
}

pub type CCC = FiniteChainComplex<FiniteModule, FiniteModuleHomomorphism<FiniteModule>>;

pub struct AlgebraicObjectsBundle {
    pub chain_complex : Arc<CCC>,
    pub resolution : Arc<RwLock<Resolution<CCC>>>
}

pub fn construct(config : &Config) -> Result<AlgebraicObjectsBundle, Box<dyn Error>> {
    let contents = load_module_from_file(config)?;
    let json = serde_json::from_str(&contents)?;

    construct_from_json(json, config.algebra_name.clone())
}

pub fn construct_derived_resolution(json : Value, algebra_name : String) -> Result<AlgebraicObjectsBundle, Box<dyn Error>> {
    let algebra = Arc::new(AlgebraAny::from_json(&json, algebra_name)?);
    let unit_module = Arc::new(FiniteModule::from(FDModule::new(Arc::clone(&algebra), "unit".to_string(), BiVec::from_vec(0, vec![1]))));
    let unit_chain_complex : Arc<CCC> = Arc::new(FiniteChainComplex::ccdz(unit_module));
    let unit_resolution = Arc::new(Resolution::new(unit_chain_complex, None, None));

    let p = algebra.prime();
    let s = json["s"].as_u64().unwrap() as u32;
    let t = json["t"].as_i64().unwrap() as i32;
    let idx = json["idx"].as_u64().unwrap() as usize;

    unit_resolution.resolve_through_bidegree(s, t);

    let idx = unit_resolution.module(s).operation_generator_to_index(0, 0, t, idx);
    let yoneda = yoneda_representative(Arc::clone(&unit_resolution.inner), s, t, idx);
    let mut yoneda = FiniteChainComplex::from(yoneda);
    yoneda.pop();

    let yoneda = Arc::new(yoneda);
    Ok(AlgebraicObjectsBundle {
        chain_complex : Arc::clone(&yoneda),
        resolution : Arc::new(RwLock::new(Resolution::new(yoneda, None, None))),
    })
}

pub fn construct_from_json(mut json : Value, algebra_name : String) -> Result<AlgebraicObjectsBundle, Box<dyn Error>> {
    if json["type"].as_str().unwrap() == "derived cofiber" {
        return construct_derived_resolution(json, algebra_name);
    }
    let algebra = Arc::new(AlgebraAny::from_json(&json, algebra_name)?);
    let module = Arc::new(FiniteModule::from_json(Arc::clone(&algebra), &mut json)?);
    let chain_complex = Arc::new(FiniteChainComplex::ccdz(Arc::clone(&module)));
    let resolution = Arc::new(RwLock::new(Resolution::new(Arc::clone(&chain_complex), None, None)));

    let products_value = &mut json["products"];
    if !products_value.is_null() {
        let products = products_value.as_array_mut().unwrap();
        for prod in products {
            let hom_deg = prod["hom_deg"].as_u64().unwrap() as u32;
            let int_deg = prod["int_deg"].as_i64().unwrap() as i32;
            let class : Vec<u32> = serde_json::from_value(prod["class"].take()).unwrap();
            let name = prod["name"].as_str().unwrap();

            resolution.write().unwrap().add_product(hom_deg, int_deg, class, &name.to_string());
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
            resolution.write().unwrap().add_self_map(s, t, &name.to_string(), map_data);
        }
    }

    Ok(AlgebraicObjectsBundle {
        chain_complex,
        resolution
    })
}

pub fn run_define_module() -> Result<String, Box<dyn Error>> {
    cli_module_loaders::interactive_module_define()
}

pub fn run_resolve(config : &Config) -> Result<String, Box<dyn Error>> {
    let bundle = construct(config)?;
    let res = bundle.resolution.read().unwrap();
    res.resolve_through_degree(config.max_degree);
    // let hom = HomComplex::new(Arc::clone(&res), Arc::clone(&bundle.module));
    // hom.compute_cohomology_through_bidegree(res.max_computed_homological_degree(), res.max_computed_degree());
    Ok(res.graded_dimension_string())
}

pub fn run_yoneda(config : &Config) -> Result<String, Box<dyn Error>> {
    let bundle = construct(config)?;
    let module = bundle.chain_complex.module(0);
    let resolution = bundle.resolution.read().unwrap();
    let min_degree = resolution.min_degree();
    let p = resolution.prime();

    loop {
        let x : i32= query_with_default_no_default_indicated("x", 200, |x : i32| Ok(x));
        let s : u32 = query_with_default_no_default_indicated("s", 200, |x : u32| Ok(x));
        let i : usize = query_with_default_no_default_indicated("idx", 200, |x : usize| Ok(x));

        let start = Instant::now();
        let t = x + s as i32;
        resolution.resolve_through_bidegree(s + 1, t + 1);

        println!("Resolving time: {:?}", start.elapsed());

        let idx = resolution.module(s).operation_generator_to_index(0, 0, t, i);
        let start = Instant::now();
        let yoneda = Arc::new(yoneda_representative(Arc::clone(&resolution.inner), s, t, idx));

        println!("Finding representative time: {:?}", start.elapsed());

        let f = ResolutionHomomorphism::new("".to_string(), Arc::downgrade(&resolution.inner), Arc::downgrade(&yoneda), 0, 0);
        let mut mat = Matrix::new(p, 1, 1);
        mat[0].set_entry(0, 1);
        f.extend_step(0, 0, Some(&mut mat));

        f.extend(s, t);
        let final_map = f.get_map(s);
        let num_gens = resolution.inner.number_of_gens_in_bidegree(s, t);
        for i_ in 0 .. num_gens {
            assert_eq!(final_map.output(t, i_).dimension(), 1);
            if i_ == i {
                assert_eq!(final_map.output(t, i_).entry(0), 1);
            } else {
                assert_eq!(final_map.output(t, i_).entry(0), 0);
            }
        }

        let mut check = BiVec::from_vec(min_degree, vec![0; t as usize + 1 - min_degree as usize]);
        for s in 0 ..= s {
            let module = yoneda.module(s);

            println!("Dimension of {}th module is {}", s, module.total_dimension());

            for t in min_degree ..= t {
                check[t] += (if s % 2 == 0 { 1 } else { -1 }) * module.dimension(t) as i32;
            }
        }
        for t in min_degree ..= t {
            assert_eq!(check[t], module.dimension(t) as i32);
        }

        let filename = query("Output file name (empty to skip)", |result : String| Ok(result));

        if filename.is_empty() {
            continue;
        }

        let mut module_strings = Vec::with_capacity(s as usize + 2);
        match &*module {
            FiniteModule::FDModule(m) => {
                module_strings.push(m.to_minimal_json());
            }
            FiniteModule::FPModule(m) => {
                // This should never happen
                panic!();
            }
        };

        for s in 0 ..= s {
            match &*yoneda.module(s) {
                FiniteModule::FDModule(m) => module_strings.push(m.to_minimal_json()),
                _ => panic!()
            }
        }

        let mut output_path_buf = PathBuf::from(format!("{}", filename));
        output_path_buf.set_extension("json");
        std::fs::write(&output_path_buf, Value::from(module_strings).to_string()).unwrap();
    }
}

pub fn run_test() {
    let p : u32= query_with_default_no_default_indicated("p", 200, |x : u32| Ok(x));
    let x : i32= query_with_default_no_default_indicated("t - s", 200, |x : i32| Ok(x));
    let s : u32 = query_with_default_no_default_indicated("s", 200, |x : u32| Ok(x));
    let i : usize = query_with_default_no_default_indicated("idx", 200, |x : usize| Ok(x));
    let max_degree : i32 = query_with_default_no_default_indicated("max_degree", 200, |x : i32| Ok(x));

    let content = format!(r#"{{"type" : "derived cofiber","name": "$S_2$", "file_name": "S_2", "p": 2, "generic": false, "s": {}, "t": {}, "idx": {}}}"#, s, x + s as i32, i);
    let json = serde_json::from_str(&content).unwrap();
    let bundle = construct_derived_resolution(json, "adem".to_string()).unwrap();

    let resolution = bundle.resolution.read().unwrap();
    resolution.resolve_through_degree(max_degree);
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
