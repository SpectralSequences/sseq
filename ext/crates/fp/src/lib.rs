#![feature(const_panic)]
#![feature(stdsimd)]

mod constants;
mod limb;

pub use constants::{MAX_MULTINOMIAL_LEN, NUM_PRIMES, PRIMES, PRIME_TO_INDEX_MAP};

pub mod matrix;
pub mod prime;
#[cfg(feature = "odd-primes")]
pub mod vector;
pub mod vector_2;
#[cfg(not(feature = "odd-primes"))]
pub use vector_2 as vector;

pub mod vector_inner;

pub(crate) mod simd;
