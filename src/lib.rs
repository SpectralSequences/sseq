#![allow(clippy::many_single_char_names)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![warn(clippy::default_trait_access)]
#![warn(clippy::if_not_else)]
#![warn(clippy::needless_continue)]
#![warn(clippy::redundant_closure_for_method_calls)]
#![warn(clippy::explicit_iter_loop)]
#![warn(clippy::explicit_into_iter_loop)]
mod sseq;

pub mod actions;
pub mod managers;
#[cfg(target_arch = "wasm32")]
pub mod wasm_bindings;

#[cfg(target_arch = "wasm32")]
pub type Sender = wasm_bindings::Sender;

#[cfg(not(target_arch = "wasm32"))]
pub type Sender = std::sync::mpsc::Sender<actions::Message>;
