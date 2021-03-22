#![cfg_attr(rustfmt, rustfmt_skip)]
use std::error::Error;
use std::sync::Arc;
use serde_json::{json, Value};

use std::path::PathBuf;
use algebra::{Algebra, SteenrodAlgebra};
use algebra::module::{FiniteModule, Module, BoundedModule};
use algebra::module::homomorphism::FreeModuleHomomorphism;
use fp::matrix::Matrix;
use crate::chain_complex::{FreeChainComplex, FiniteChainComplex, ChainMap};
use crate::resolution::Resolution;
#[cfg(feature = "yoneda")]
use crate::yoneda::yoneda_representative;

use crate::CCC;

pub struct Config {
    pub module_paths : Vec<PathBuf>,
    pub module_file_name : String,
    pub algebra_name : String,
    pub max_degree : i32
}

pub fn construct(config : &Config) -> error::Result<Resolution<CCC>> {
    let contents = load_module_from_file(config)?;
    let json = serde_json::from_str(&contents)?;

    construct_from_json(json, &config.algebra_name)
}

pub fn construct_from_json(mut json : Value, algebra_name : &str) -> error::Result<Resolution<CCC>> {
    let algebra = Arc::new(SteenrodAlgebra::from_json(&json, algebra_name)?);
    let module = Arc::new(FiniteModule::from_json(Arc::clone(&algebra), &mut json)?);
    let mut chain_complex = Arc::new(FiniteChainComplex::ccdz(Arc::clone(&module)));
    let mut resolution = Resolution::new(Arc::clone(&chain_complex), None, None);

    let cofiber = &json["cofiber"];
    #[cfg(feature = "yoneda")]
    if !cofiber.is_null() {
        let s = cofiber["s"].as_u64().unwrap() as u32;
        let t = cofiber["t"].as_i64().unwrap() as i32;
        let idx = cofiber["idx"].as_u64().unwrap() as usize;

        resolution.resolve_through_bidegree(s, t + module.max_degree());

        let map = FreeModuleHomomorphism::new(resolution.module(s), Arc::clone(&module), t);
        let mut new_output = Matrix::new(module.prime(), resolution.module(s).number_of_gens_in_degree(t), 1);
        new_output[idx].set_entry(0, 1);

        let lock = map.lock();
        map.add_generators_from_matrix_rows(&lock, t, new_output.as_slice_mut());
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

    #[cfg(not(feature = "yoneda"))]
    if !cofiber.is_null() {
        panic!("cofiber not supported. Compile with yoneda feature enabled");
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
    Ok(resolution)
}

pub fn load_module_from_file(config : &Config) -> error::Result<String> {
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
    result.ok_or_else(|| ModuleFileNotFoundError {
        name : config.module_file_name.clone()
    }.into())
}

pub fn construct_s_2(algebra: &str) -> Resolution<CCC> {
    let json = json!({
        "type" : "finite dimensional module",
        "p": 2,
        "gens": {"x0": 0},
        "actions": []
    });
    construct_from_json(json, algebra).unwrap()
}

#[macro_export]
macro_rules! load_s_2 {
    ($resolution:ident, $algebra:literal, $path:literal) => {
        use saveload::Load;

        let mut resolution = ext::utils::construct_s_2($algebra);

        if std::path::Path::new($path).exists() {
            let f = std::fs::File::open($path).unwrap();
            let mut f = std::io::BufReader::new(f);
            resolution = ext::resolution::Resolution::load(&mut f, &resolution.complex()).unwrap();
        }
        let $resolution = resolution;
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

const RED_ANSI_CODE: &str = "\x1b[31;1m";
const WHITE_ANSI_CODE: &str = "\x1b[0m";

pub fn ascii_num(n: usize) -> char {
    match n {
        0 => ' ',
        1 => '·',
        2 => ':',
        3 => '∴',
        4 => '⁘',
        5 => '⁙',
        6 | 7 | 8 | 9 => (b'0' + n as u8) as char,
        _ => '*',
    }
}

pub fn print_resolution_color<C: FreeChainComplex, S: std::hash::BuildHasher>(res: &C, max_s: u32, max_t: i32, highlight: &std::collections::HashMap<(u32, i32), u32, S>) {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    for s in (0 ..= max_s).rev() {
        for t in s as i32 ..= max_t {
            if matches!(highlight.get(&(s, t)), None | Some(0)) {
                write!(stdout, "{}{}{} ", RED_ANSI_CODE, ascii_num(res.module(s).number_of_gens_in_degree(t)), WHITE_ANSI_CODE).unwrap();
            } else {
                write!(stdout, "{} ", ascii_num(res.module(s).number_of_gens_in_degree(t))).unwrap();
            }
        }
        writeln!(stdout).unwrap();
    }
}

use std::hash::{Hash, Hasher, BuildHasher};
use std::collections::HashMap;

pub trait HashMapTuple<A, B, C> {
    fn get_tuple(&self, a: &A, b: &B) -> Option<&C>;
}

impl<A: Eq + Hash, B: Eq + Hash, C, S: BuildHasher> HashMapTuple<A, B, C> for HashMap<(A, B), C, S> {
    fn get_tuple(&self, a: &A, b: &B) -> Option<&C> {
        let mut hasher = self.hasher().build_hasher();
        a.hash(&mut hasher);
        b.hash(&mut hasher);
        let raw_entry = self.raw_entry();

        raw_entry.from_hash(
            hasher.finish(),
            |v| &v.0 == a && &v.1 == b,
        ).map(|(_, y)| y)
    }
}

/// Iterate through all pairs (s, f, t) such that f = t - s, s <= max_s and t <= max_t
pub fn iter_stems(max_s: u32, max_t: i32) -> impl Iterator<Item=(u32, i32, i32)> {
    (0..=max_t)
        .map(move |f| {
            (0..=std::cmp::min(max_s, (max_t - f) as u32))
                .map(move |s| (s, f, f + s as i32))
        })
        .flatten()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_hashmap_tuple() {
        let mut x: HashMap<(u32, u32), bool> = HashMap::new();
        x.insert((5, 3), true);

        assert_eq!(x.get_tuple(&5, &3), Some(&true));
        assert_eq!(x.get_tuple(&3, &5), None);
        assert_eq!(x.get_tuple(&7, &12), None);

    }
}
