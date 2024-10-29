#![deny(clippy::use_self)]
// Rust 2024 compatibility lints
#![deny(rust_2024_compatibility)]
// The `expr` fragment will change in Rust 2024
#![allow(edition_2024_expr_fragment_specifier)]
// Drop order will change in Rust 2024
#![allow(tail_expr_drop_order)]
// impl Trait will capture more lifetimes in Rust 2024
#![allow(impl_trait_overcaptures)]

mod constants;
mod limb;

pub use constants::{MAX_MULTINOMIAL_LEN, NUM_PRIMES, PRIMES, PRIME_TO_INDEX_MAP};

pub mod field;
pub mod matrix;
pub mod prime;
pub mod vector;

pub(crate) mod simd;

// This is useful for traits that want to implement `Arbitrary`. This lets us specify that they
// should be subtraits of `Arbitrary` iff the `proptest` feature is enabled.
#[cfg(not(feature = "proptest"))]
pub(crate) trait MaybeArbitrary<Params> {}

#[cfg(feature = "proptest")]
pub(crate) trait MaybeArbitrary<Params>:
    proptest::arbitrary::Arbitrary<Parameters = Params>
{
}

#[cfg(feature = "odd-primes")]
pub const ODD_PRIMES: bool = true;
#[cfg(not(feature = "odd-primes"))]
pub const ODD_PRIMES: bool = false;
