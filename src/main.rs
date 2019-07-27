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
// mod test;

#[cfg(test)]
extern crate rand;
extern crate spin;

#[macro_use]
extern crate lazy_static;

// use std::fmt;

// use crate::memory::{CVec, MemoryAllocator};

#[allow(unreachable_code)]
#[allow(non_snake_case)]
#[allow(unused_mut)]
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
    let mut x = fp_vector::FpVector::new_from_allocator(&s, p, 7, 0);
    let mut y = fp_vector::FpVector::new_from_allocator(&s, p, 7, 0);
    let v : [u32 ; 7] = [1,0,1,0,1, 1, 1];
    let w : [u32 ; 7] = [1,1,1,1,1, 0, 0];
    x.pack(&v);
    y.pack(&w);
    println!("x: {}\n",x);
    x.set_slice(1, 6);
    println!("x: {}\n",x);
    x.clear_slice();
    println!("x: {}\n",x);
    // let mut ys = y.slice(1, 6);
    println!("x: {}\ny: {}", x, y);
    y.add(&x,1);
    println!("x: {}\ny: {}", x, y);
    // println!("ys:   {}", ys);
    // println!("y: {}", y);
    return;

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
