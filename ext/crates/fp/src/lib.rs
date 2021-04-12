#![feature(const_panic)]
#![feature(vec_extend_from_within)]
#![feature(stdsimd)]

pub mod matrix;
pub mod prime;
#[cfg(feature = "odd-primes")]
pub mod vector;
pub mod vector_2;
#[cfg(not(feature = "odd-primes"))]
pub use vector_2 as vector;

pub mod vector_inner;

pub(crate) mod simd;
