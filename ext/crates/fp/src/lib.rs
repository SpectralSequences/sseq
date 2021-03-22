#![feature(const_panic)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::unreadable_literal)]

pub mod matrix;
pub mod prime;
#[cfg(feature = "odd-primes")]
pub mod vector;
pub mod vector_2;
#[cfg(not(feature = "odd-primes"))]
pub use vector_2 as vector;

pub mod vector_inner;
