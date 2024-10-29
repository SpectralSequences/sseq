#![deny(clippy::use_self)]
// Rust 2024 compatibility lints
#![deny(rust_2024_compatibility)]
// The `expr` fragment will change in Rust 2024
#![allow(edition_2024_expr_fragment_specifier)]
// Drop order will change in Rust 2024
#![allow(tail_expr_drop_order)]
// impl Trait will capture more lifetimes in Rust 2024
#![allow(impl_trait_overcaptures)]
// `if let` now drops the binding before entering the `else` block in Rust 2024. This lint is
// currently only supported on nightly.
#![allow(unknown_lints, if_let_rescope)]
#![deny(unknown_lints)]

pub mod sseq;

pub mod actions;
pub mod managers;
pub mod resolution_wrapper;
#[cfg(target_arch = "wasm32")]
pub mod wasm_bindings;

#[cfg(target_arch = "wasm32")]
pub type Sender = wasm_bindings::Sender;

#[cfg(not(target_arch = "wasm32"))]
pub type Sender = std::sync::mpsc::Sender<actions::Message>;
