#![allow(dead_code)]
#![allow(unused_variables)]
#[allow(unused_imports)]

mod memory;
mod once;
mod combinatorics;
mod fp_vector;
mod matrix;
mod algebra;
mod adem_algebra;
mod module;
mod module_homomorphism;
mod finite_dimensional_module;
mod free_module;
mod free_module_homomorphism;
mod chain_complex;
mod resolution;

#[cfg(test)]
extern crate rand;
extern crate spin;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate rental;

use serde_json::value::Value;

use crate::algebra::Algebra;
use crate::adem_algebra::AdemAlgebra;
use crate::module::Module;
use crate::finite_dimensional_module::FiniteDimensionalModule;
use crate::chain_complex::ChainComplexConcentratedInDegreeZero;
use crate::resolution::Resolution;

use std::error::Error;

#[allow(unreachable_code)]
#[allow(non_snake_case)]
#[allow(unused_mut)]
fn main() {
    let args : Vec<_> = std::env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        std::process::exit(1);
    });

    if let Err(e) = run(config) {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
}

fn run(config : Config) -> Result<(), Box<Error>> {
    let contents = std::fs::read_to_string(format!("static/modules/{}.json", config.module_name))?;
    let mut json : Value = serde_json::from_str(&contents)?;
    let p = json["p"].as_u64().unwrap() as u32;
    let max_degree = config.max_degree;
    let A = AdemAlgebra::new(p, p != 2, false, max_degree);
    A.compute_basis(max_degree);
    let M = finite_dimensional_module::FiniteDimensionalModule::adem_module_from_json(&A, &mut json);
    let CC = ChainComplexConcentratedInDegreeZero::new(&M);
    let res = Resolution::new(&CC, max_degree, None, None);
    res.resolve_through_degree(max_degree);
    println!("{}", res.graded_dimension_string());
    res.get_cocycle_string(1, 3, 0);
    Ok(())
}

struct Config {
    module_name : String,
    max_degree : i32
}

impl Config {
    fn new(args: &[String]) -> Result<Self, String> {
        if args.len() < 3 {
            return Err("Not enough arguments".to_string());
        }
        let module_name = args[1].clone();
        let max_deg_result : Result<i32,_> = args[2].parse();
        
        if let Err(error) = max_deg_result {
            return Err(format!("{} in argument max_degree.", error));
        }
        let max_degree = max_deg_result.unwrap();
        Ok(Self { module_name, max_degree })
    }
}
