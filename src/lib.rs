#![allow(dead_code)]
#![allow(unused_variables)]

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
mod wasm_bindings;

#[cfg(test)]
extern crate rand;

#[macro_use]
extern crate lazy_static;

extern crate serde_derive;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate rental;

extern crate wasm_bindgen;
extern crate web_sys;

