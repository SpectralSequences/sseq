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
use module::{FiniteModule, Module, BoundedModule};
use module::homomorphism::{FiniteModuleHomomorphism, ModuleHomomorphism, FreeModuleHomomorphism};
use matrix::Matrix;
use fp_vector::{FpVector, FpVectorT};
use chain_complex::{FiniteChainComplex, ChainComplex, TensorChainComplex, ChainMap};
use resolution::Resolution;
use resolution_homomorphism::ResolutionHomomorphism;
use yoneda::{yoneda_representative_element, yoneda_representative};

use bivec::BiVec;
use query::*;

use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use std::collections::VecDeque;
use serde_json::value::Value;

use std::{thread, thread::JoinHandle, sync::mpsc};

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

pub fn construct_from_json(mut json : Value, algebra_name : String) -> Result<AlgebraicObjectsBundle, Box<dyn Error>> {
    let algebra = Arc::new(AlgebraAny::from_json(&json, algebra_name)?);
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
        map.add_generators_from_matrix_rows(&lock, t, &mut new_output, 0, 0);
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
        resolution : Arc::new(RwLock::new(resolution))
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
        let x : i32= query_with_default_no_default_indicated("t - s", 200, |x : i32| Ok(x));
        let s : u32 = query_with_default_no_default_indicated("s", 200, |x : u32| Ok(x));
        let i : usize = query_with_default_no_default_indicated("idx", 200, |x : usize| Ok(x));

        let start = Instant::now();
        let t = x + s as i32;
        resolution.resolve_through_bidegree(s + 1, t + 1);

        println!("Resolving time: {:?}", start.elapsed());

        let start = Instant::now();
        let yoneda = Arc::new(yoneda_representative_element(Arc::clone(&resolution.inner), s, t, i));

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
            assert_eq!(check[t], module.dimension(t) as i32, "Incorrect Euler characteristic at t = {}", t);
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
            FiniteModule::FPModule(_) => {
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

pub fn run_steenrod() -> Result<String, Box<dyn Error>> {
    let k = r#"{"type" : "finite dimensional module","name": "$S_2$", "file_name": "S_2", "p": 2, "generic": false, "gens": {"x0": 0}, "adem_actions": []}"#;
    let k = serde_json::from_str(k).unwrap();
    let bundle = construct_from_json(k, "adem".to_string()).unwrap();
    let resolution = bundle.resolution.read().unwrap();
    let p = 2;
    let num_threads = query_with_default_no_default_indicated("Number of threads", 2, |x : usize| Ok(x));

    loop {
        let x : i32= query_with_default_no_default_indicated("t - s", 8, |x : i32| Ok(x));
        let s : u32 = query_with_default_no_default_indicated("s", 3, |x : u32| Ok(x));
        let idx : usize = query_with_default_no_default_indicated("idx", 0, |x : usize| Ok(x));

        let t = s as i32 + x;
        print!("Resolving ext: ");
        let start = Instant::now();
        resolution.resolve_through_bidegree(2 * s, 2 * t);
        println!("{:?}", start.elapsed());

        print!("Computing Yoneda representative: ");
        let start = Instant::now();
        let yoneda = Arc::new(yoneda_representative_element(Arc::clone(&resolution.inner), s, t, idx));
        println!("{:?}", start.elapsed());

        print!("Dimensions of Yoneda representative: 1");
        let mut check = vec![0; t as usize + 1];
        for s in 0 ..= s {
            let module = yoneda.module(s);
            print!(" {}", module.total_dimension());

            for t in 0 ..= t {
                check[t as usize] += (if s % 2 == 0 { 1 } else { -1 }) * module.dimension(t) as i32;
            }
        }
        println!("");

        // We check that lifting the identity returns the original class. Even if the
        // algorithm in yoneda.rs is incorrect, this ensures that a posteriori we happened
        // to have a valid Yoneda representative. (Not really --- we don't check it is exact, just
        // that its Euler characteristic is 0 in each degree)
        print!("Checking Yoneda representative: ");
        let start = Instant::now();
        {
            assert_eq!(check[0], 1, "Incorrect Euler characteristic at t = 0");
            for t in 1 ..= t as usize {
                assert_eq!(check[t], 0, "Incorrect Euler characteristic at t = {}", t);
            }
            let f = ResolutionHomomorphism::new("".to_string(), Arc::downgrade(&resolution.inner), Arc::downgrade(&yoneda), 0, 0);
            let mut mat = Matrix::new(p, 1, 1);
            mat[0].set_entry(0, 1);
            f.extend_step(0, 0, Some(&mut mat));

            f.extend(s, t);
            let final_map = f.get_map(s);
            let num_gens = resolution.inner.number_of_gens_in_bidegree(s, t);
            for i_ in 0 .. num_gens {
                assert_eq!(final_map.output(t, i_).dimension(), 1);
                if i_ == idx {
                    assert_eq!(final_map.output(t, i_).entry(0), 1);
                } else {
                    assert_eq!(final_map.output(t, i_).entry(0), 0);
                }
            }
        }
        println!("{:?}", start.elapsed());

        let square = Arc::new(TensorChainComplex::new(Arc::clone(&yoneda), Arc::clone(&yoneda)));

        print!("Computing quasi_inverses: ");
        let start = Instant::now();
        square.compute_through_bidegree(2 * s, 2 * t);
        for s in 0 ..= 2 * s {
            square.differential(s as u32).compute_kernels_and_quasi_inverses_through_degree(2 * t);
        }
        println!("{:?}", start.elapsed());

        println!("Computing Steenrod operations: ");
        let start = Instant::now();

        let mut delta = Vec::with_capacity(s as usize);

        for i in 0 ..= s {
            let mut maps : Vec<Arc<FreeModuleHomomorphism<_>>> = Vec::with_capacity(2 * s as usize - 1);

            for s in 0 ..= 2 * s - i {
                let source = resolution.inner.module(s);
                let target = square.module(s + i);

                let map = FreeModuleHomomorphism::new(Arc::clone(&source), Arc::clone(&target), 0);
                maps.push(Arc::new(map));
            }
            delta.push(maps);
        }

        // We use the formula d Δ_i + Δ_i d = Δ_{i-1} + τΔ_{i-1}
        for i in 0 ..= s {
            // Δ_i is a map C_s -> C_{s + i}. So to hit C_{2s}, we only need to compute up to 2
            // * s - i
            let start = Instant::now();

            let mut handles : VecDeque<JoinHandle<()>> = VecDeque::with_capacity(num_threads);
            let mut last_receiver : Option<mpsc::Receiver<()>> = None;

            for s in 0 ..= 2 * s - i {
                if i == 0 && s == 0 {
                    let map = &delta[0][0];
                    let mut lock = map.lock();
                    map.add_generators_from_matrix_rows(&lock, 0, &mut Matrix::from_vec(p, &[vec![1]]), 0, 0);
                    *lock += 1;
                    map.extend_by_zero(&lock, 2 * t);
                    continue;
                }

                if handles.len() == num_threads {
                    handles.pop_front().unwrap().join().unwrap();
                }

                let square = Arc::clone(&square);

                let source = resolution.inner.module(s);
                let target = square.module(s + i);

                let dtarget_module = square.module(s + i - 1);

                let d_res = resolution.inner.differential(s);
                let d_target = square.differential(s + i as u32);

                let map = Arc::clone(&delta[i as usize][s as usize]);
                let prev_map = match s { 0 => None, _ => Some(Arc::clone(&delta[i as usize][s as usize - 1])) };

                let prev_delta = match i { 0 => None, _ => Some(Arc::clone(&delta[i as usize - 1][s as usize])) };

                let (sender, new_receiver) = mpsc::channel();

                let handle = thread::Builder::new().name(format!("Delta_{}, s = {}", i, s)).spawn(move || {
                    for t in 0 ..= 2 * t {
                        if let Some(recv) = &last_receiver {
                            recv.recv().unwrap();
                        }
                        let num_gens = source.number_of_gens_in_degree(t);

                        let fx_dim = target.dimension(t);
                        let fdx_dim = dtarget_module.dimension(t);

                        if fx_dim == 0 || fdx_dim == 0 || num_gens == 0 {
                            let mut lock = map.lock();
                            map.extend_by_zero(&lock, t);
                            *lock += 1;
                            sender.send(()).unwrap();
                            continue;
                        }

                        let mut output_matrix = Matrix::new(p, num_gens, fx_dim);
                        let mut result = FpVector::new(p, fdx_dim);
                        for j in 0 .. num_gens {
                            if let Some(m) = &prev_delta {
                                // Δ_{i-1} x
                                let prevd = m.output(t, j);

                                // τ Δ_{i-1}x
                                square.swap(&mut result, prevd, s + i as u32 - 1, t);
                                result.add(prevd, 1);
                            }

                            if let Some(m) = &prev_map {
                                let dx = d_res.output(t, j);
                                m.apply(&mut result, 1, t, dx);
                            }
                            d_target.apply_quasi_inverse(&mut output_matrix[j], t, &result);

                            result.set_to_zero();
                        }
                        let mut lock = map.lock();
                        map.add_generators_from_matrix_rows(&lock, t, &mut output_matrix, 0, 0);
                        *lock += 1;
                        sender.send(()).unwrap();
                    }
                });
                last_receiver = Some(new_receiver);
                handles.push_back(handle.unwrap());
            }
            for handle in handles.into_iter() {
                handle.join().unwrap();
            }
            let final_map = &delta[i as usize][(2 * s - i) as usize];
            let num_gens = resolution.inner.number_of_gens_in_bidegree(2 * s - i, 2 * t);
            println!("Sq^{} x_{{{}, {}}}^({}) = [{}] ({:?})", s - i, t-s as i32, s, idx, (0 .. num_gens).map(|k| format!("{}", final_map.output(2 * t, k).entry(0))).collect::<Vec<_>>().join(", "), start.elapsed());
        }
        println!("Computing Steenrod operations: {:?}", start.elapsed());
    }
}

pub fn run_test() {
    let k = r#"{"type" : "finite dimensional module","name": "$S_2$", "file_name": "S_2", "p": 2, "generic": false, "gens": {"x0": 0}, "adem_actions": []}"#;
    let k = serde_json::from_str(k).unwrap();
    let bundle = construct_from_json(k, "adem".to_string()).unwrap();
    let resolution = bundle.resolution.read().unwrap();
    let p = 2;

    let x : i32 = 30;
    let s : u32 = 6;
    let idx : usize = 0;

    let t = s as i32 + x;
    let start = Instant::now();
    resolution.resolve_through_bidegree(s, t);

    let yoneda = Arc::new(yoneda_representative_element(Arc::clone(&resolution.inner), s, t, idx));
    let square = Arc::new(TensorChainComplex::new(Arc::clone(&yoneda), Arc::clone(&yoneda)));

    square.compute_through_bidegree(6, 48);
    println!("Starting test");
    for deg in 0 ..= 30 {
        let mut result = FpVector::new(p, square.module(6).dimension(deg + 18));
        for i in 0 .. square.module(6).dimension(deg) {
            square.module(6).act_on_basis(&mut result, 1, 18, 2, deg, i);
        }
        let mut result = FpVector::new(p, square.module(5).dimension(deg + 18));
        for i in 0 .. square.module(5).dimension(deg) {
            square.module(5).act_on_basis(&mut result, 1, 18, 3, deg, i);
        }
    }
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
