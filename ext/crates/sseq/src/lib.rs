#![deny(clippy::use_self)]
// Rust 2024 compatibility lints
#![deny(rust_2024_compatibility)]
// The `expr` fragment will change in Rust 2024
#![allow(edition_2024_expr_fragment_specifier)]
// Drop order will change in Rust 2024
#![allow(tail_expr_drop_order)]
// impl Trait will capture more lifetimes in Rust 2024
#![allow(impl_trait_overcaptures)]

mod bigraded;
pub mod coordinates;
mod differential;
mod sseq;

pub use bigraded::*;
pub use differential::*;

pub use crate::sseq::*;
