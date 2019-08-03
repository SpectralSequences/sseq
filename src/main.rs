#![allow(dead_code)]
#![allow(unused_variables)]
#[allow(unused_imports)]

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
mod chain_complex;
mod resolution;

#[cfg(test)]
extern crate rand;
extern crate spin;

#[macro_use]
extern crate lazy_static;

use serde_json::value::Value;

use crate::algebra::Algebra;
use crate::adem_algebra::AdemAlgebra;
#[cfg(test)]
use crate::milnor_algebra::MilnorAlgebra;
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

#[allow(non_snake_case)]
fn run(config : Config) -> Result<(), Box<Error>> {
    let contents = std::fs::read_to_string(format!("static/modules/{}.json", config.module_name))?;
    let mut json : Value = serde_json::from_str(&contents)?;
    let p = json["p"].as_u64().unwrap() as u32;
    let max_degree = config.max_degree;

    let A = AdemAlgebra::new(p, p != 2, false, max_degree);
    A.compute_basis(max_degree);
    let M = FiniteDimensionalModule::from_json(&A, "adem", &mut json);
    let CC = ChainComplexConcentratedInDegreeZero::new(&M);
    let res = Resolution::new(&CC, max_degree, None, None);
    res.resolve_through_degree(max_degree);
    println!("{}", res.graded_dimension_string());
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

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn milnor_vs_adem() {
        compare("S_2", 30);
        compare("C2", 30);
        compare("Joker", 30);
        compare("RP4", 30);
        compare("Csigma", 30);
        compare("S_3", 30);
        compare("Calpha", 30);
        compare("C3", 60);
    }

    fn compare(filename : &str, max_degree : i32) {
        let contents = std::fs::read_to_string(format!("static/modules/{}.json", filename)).unwrap();

        assert_eq!(run_adem(&contents, max_degree), run_milnor(&contents, max_degree));
    }

    fn run_adem(contents : &str, max_degree : i32) -> String {
        let mut json : Value = serde_json::from_str(contents).unwrap();
        let p = json["p"].as_u64().unwrap() as u32;

        let A = AdemAlgebra::new(p, p != 2, false, max_degree);
        A.compute_basis(max_degree);
        let M = FiniteDimensionalModule::from_json(&A, "adem", &mut json);
        let CC = ChainComplexConcentratedInDegreeZero::new(&M);
        let res = Resolution::new(&CC, max_degree, None, None);
        res.resolve_through_degree(max_degree);

        res.graded_dimension_string()
    }

    fn run_milnor(contents : &str, max_degree : i32) -> String {
        let mut json : Value = serde_json::from_str(contents).unwrap();
        let p = json["p"].as_u64().unwrap() as u32;

        let A = MilnorAlgebra::new(p);
        A.compute_basis(max_degree);
        let M = FiniteDimensionalModule::from_json(&A, "milnor", &mut json);
        let CC = ChainComplexConcentratedInDegreeZero::new(&M);
        let res = Resolution::new(&CC, max_degree, None, None);
        res.resolve_through_degree(max_degree);

        res.graded_dimension_string()
    }

}
