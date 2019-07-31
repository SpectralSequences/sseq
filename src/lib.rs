// #![feature(plugin_registrar)]
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

use crate::algebra::Algebra;
use crate::module::Module;
use crate::resolution::Resolution;

#[cfg(test)]
extern crate rand;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;


#[macro_use]
extern crate rental;

// #[cfg(target_arch = "wasm32")]
extern crate wasm_bindgen;
// #[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;


extern crate web_sys;
use web_sys::console;



// #[cfg(not(target_arch = "wasm32"))]
// use wasm_bindgen_noop::wasm_bindgen;

// use std::fmt;



// #[wasm_bindgen(start)]
// pub fn main_js() -> Result<(), JsValue> {
//     // This provides better error messages in debug mode.
//     // It's disabled in release mode so it doesn't bloat up the file size.
//     // #[cfg(debug_assertions)]
//     // console_error_panic_hook::set_once();


//     // Your code goes here!
//     console::log_1(&JsValue::from_str("Hello world!"));
//     let p = 2;
//     let max_degree = 20;
//     let A = adem_algebra::AdemAlgebra::new(p, p != 2, false, max_degree);
//     A.compute_basis(max_degree);
//     let M = finite_dimensional_module::FiniteDimensionalModule::new(&A, "k".to_string(), 0, 1, vec![1]);
//     // println!("M.min_degree: {}", M.get_min_degree());
//     let CC = chain_complex::ChainComplexConcentratedInDegreeZero::new(&M);
//     let res = resolution::Resolution::new(&CC, max_degree, None, None);
//     // res.get_module(0);
//     // println!("res.min_degree: {}", res.get_min_degree());
//     resolve_through_degree(&res, max_degree);
//     console::log_1(&JsValue::from_str(&res.graded_dimension_string()));



//     Ok(())
// }

#[allow(unreachable_code)]
#[allow(non_snake_case)]
#[allow(unused_mut)]
#[allow(unused_variables)]
#[allow(non_snake_case)]
pub fn run(){
    let p = 2;
    let max_degree = 20;
    let A = adem_algebra::AdemAlgebra::new(p, p != 2, false, max_degree);
    A.compute_basis(max_degree);
    let M = finite_dimensional_module::FiniteDimensionalModule::new(&A, "k".to_string(), 0, 1, vec![1]);
    // println!("M.min_degree: {}", M.get_min_degree());
    let CC = chain_complex::ChainComplexConcentratedInDegreeZero::new(&M);
    let res = resolution::Resolution::new(&CC, max_degree, None, None);
    // res.get_module(0);
    // println!("res.min_degree: {}", res.get_min_degree());
    res.resolve_through_degree(max_degree);
    println!("{}", res.graded_dimension_string());
}


// #[wasm_bindgen]



#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let p = 2;
    let s = memory::MemoryTable::new();
    // // s.push_stack_frame();
    fp_vector::initialize_limb_bit_index_table(p);
    combinatorics::initialize_prime(p);

    // let mut A = adem_algebra::AdemAlgebra::new(p, p != 2, false);
    // A.generate_basis(20);
    // for (i, basis) in A.basis_table.iter().enumerate() {
    //     print!("{}: ", i);
    //     println!("[{}]", basis.iter().fold(String::new(), |acc, num| acc + &num.to_string() + ", "));
    // }
    // println!("\n\n");
    // let r_deg = 13;
    // let r_idx = 1;
    // let s_deg = 4;
    // let s_idx = 0;
    // let out_deg = r_deg + s_deg;
    // let mut result = fp_vector::FpVector::new(p, A.get_dimension(out_deg, -1), 0);
    // A.multiply(&mut result, 1, r_deg, r_idx, s_deg, s_idx, -1);
    // println!("{} * {} = {}", A.basis_element_to_string(r_deg, r_idx), A.basis_element_to_string(s_deg, s_idx),  A.element_to_string(out_deg, result));
    // // return;

    // x.unpack(&mut v);
    // println!("{:?}", v);
    // println!("x:{}", x);
    // println!("{}", s);

    // println!("{}", s);
    // let x = matrix::Matrix::new_from_allocator(&s, p, 200, 100);
    // println!("{}", s);
    // let y = matrix::Matrix::new_from_allocator(&s, p, 200, 100);
    // println!("{}", s);
    // let z = matrix::Matrix::new_from_allocator(&s, p, 200, 100);
    // println!("{}", s);
    // let z1 = matrix::Matrix::new_from_allocator(&s, p, 200, 100);
    // println!("{}", s);
    // let z2 = matrix::Matrix::new_from_allocator(&s, p, 200, 100);
    // println!("{}", s);
    // let z3 = matrix::Matrix::new_from_allocator(&s, p, 200, 100);
    // println!("{}", s);
    // // let mut m = matrix::Matrix::new_from_allocator(&s, p, 1, 1);
    // // let mut m = matrix::Matrix::new_from_allocator(&s, p, 1, 1);
    // // let mut m = matrix::Matrix::new_from_allocator(&s, p, 1, 1);

    // let mut m = matrix::Matrix::new_from_allocator(&s, p, 5, 7);
    // println!("{}", s);
    // let matrix_initialization = [
    //     [2,0,1,0,2,1,1],
    //     [1,1,1,0,2,1,2],
    //     [1,0,1,0,2,0,1],
    //     [1,1,2,0,2,0,0],
    //     [1,2,0,0,2,2,2]
    // ];
    // for (i,x) in matrix_initialization.iter().enumerate(){
    //     m[i].pack(x);
    // }

    // let mut pivots : CVec<isize> = s.alloc_vec(7);
    // println!("m: {}", m);
    // m.row_reduce(&mut pivots);
    // println!("m: {}", m);
    // println!("pivots: {}", pivots);


    // let x = s.alloc(10, 64/8);
    // unsafe{
    //     *(x.offset(1)) = 1;
    // }
    // println!("{:p}", x);
    // println!("{}", s);
    // let y = &mut s.alloc_vec(10);
    // y[0] = 11;
    // y[1] = 13;
    // println!("   {}", y);
    // println!("{}", s);
    // let z = s.alloc(10, 64/8);
    // println!("{:p}", z);
    // println!("{}", s);
    // // s.pop_stack_frame();
    // println!("{}", s);


    // println!("3^3 = {}", combinatorics::integer_power(3,3));
    // println!("3^3 = {}", combinatorics::power_mod(5, 3,3));
    // println!("log3(29) = {}", combinatorics::logp(3, 29));
    // let exp : [u32; 10] = [0; 10];
    // println!("base 3: {:?}", exp);
    // combinatorics::intialize_prime(29);
    // for i in 1..29 {
    //     println!("{}^{{-1}} = {}", i, combinatorics::inverse(29, i));
    // }

    // combinatorics::intialize_prime(7);
    // for n in 0..20 {
    //     for k in 0..20 {
    //         print!("{} ", combinatorics::binomial(7, n, k));
    //     }
    //     print!("\n");
    // }

}
