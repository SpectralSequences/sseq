use std::error::Error;
use std::sync::Arc;
use parking_lot::RwLock;
use serde_json::value::Value;

use std::path::PathBuf;
use algebra::{Algebra, SteenrodAlgebra};
use algebra::module::{FiniteModule, Module, BoundedModule};
use algebra::module::homomorphism::FreeModuleHomomorphism;
use fp::matrix::Matrix;
use fp::vector::FpVectorT;
use crate::chain_complex::{FiniteChainComplex, ChainMap};
use crate::resolution::Resolution;
use crate::yoneda::yoneda_representative;

use crate::CCC;

pub struct Config {
    pub module_paths : Vec<PathBuf>,
    pub module_file_name : String,
    pub algebra_name : String,
    pub max_degree : i32
}

pub struct AlgebraicObjectsBundle {
    pub chain_complex : Arc<CCC>,
    pub module: Arc<FiniteModule>,
    pub resolution : Arc<RwLock<Resolution<CCC>>>
}

pub fn construct(config : &Config) -> Result<AlgebraicObjectsBundle, Box<dyn Error>> {
    let contents = load_module_from_file(config)?;
    let json = serde_json::from_str(&contents)?;

    construct_from_json(json, config.algebra_name.clone())
}

pub fn construct_from_json(mut json : Value, algebra_name : String) -> Result<AlgebraicObjectsBundle, Box<dyn Error>> {
    let algebra = Arc::new(SteenrodAlgebra::from_json(&json, algebra_name)?);
    let module = Arc::new(FiniteModule::from_json(Arc::clone(&algebra), &mut json)?);
    let mut chain_complex = Arc::new(FiniteChainComplex::ccdz(Arc::clone(&module)));
    let mut resolution = Resolution::new(Arc::clone(&chain_complex), None, None);

    let cofiber = &json["cofiber"];
    if !cofiber.is_null() {
        let s = cofiber["s"].as_u64().unwrap() as u32;
        let t = cofiber["t"].as_i64().unwrap() as i32;
        let idx = cofiber["idx"].as_u64().unwrap() as usize;

        resolution.resolve_through_bidegree(s, t + module.max_degree());

        let map = FreeModuleHomomorphism::new(resolution.module(s), Arc::clone(&module), t);
        let mut new_output = Matrix::new(module.prime(), resolution.module(s).number_of_gens_in_degree(t), 1);
        new_output[idx].set_entry(0, 1);

        let lock = map.lock();
        map.add_generators_from_matrix_rows(&lock, t, &new_output);
        drop(lock);
        map.extend_by_zero_safe(module.max_degree() + t);

        let cm = ChainMap {
            s_shift : s,
            chain_maps : vec![map]
        };
        let yoneda = yoneda_representative(Arc::clone(&resolution.inner), cm);
        let mut yoneda = FiniteChainComplex::from(yoneda);
        yoneda.pop();

        chain_complex = Arc::new(yoneda);
        resolution = Resolution::new(Arc::clone(&chain_complex), None, None);
    }

    let products_value = &mut json["products"];
    if !products_value.is_null() {
        let products = products_value.as_array_mut().unwrap();
        for prod in products {
            let hom_deg = prod["hom_deg"].as_u64().unwrap() as u32;
            let int_deg = prod["int_deg"].as_i64().unwrap() as i32;
            let class : Vec<u32> = serde_json::from_value(prod["class"].take()).unwrap();
            let name = prod["name"].as_str().unwrap();

            resolution.add_product(hom_deg, int_deg, class, &name.to_string());
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
            resolution.add_self_map(s, t, &name.to_string(), map_data);
        }
    }
    Ok(AlgebraicObjectsBundle {
        chain_complex,
        module,
        resolution : Arc::new(RwLock::new(resolution))
    })
}

pub fn load_module_from_file(config : &Config) -> Result<String, Box<dyn Error>> {
    let mut result = None;
    for path in &config.module_paths {
        let mut path = path.clone();
        path.push(&config.module_file_name);
        path.set_extension("json");
        result = std::fs::read_to_string(path).ok();
        if result.is_some() {
            break;
        }
    }
    result.ok_or_else(|| Box::new(ModuleFileNotFoundError {
        name : config.module_file_name.clone()
    }) as Box<dyn Error>)
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
