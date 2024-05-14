#![deny(clippy::use_self)]

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
