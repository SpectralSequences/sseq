extern crate rust_ext;
extern crate bivec;
extern crate serde_json;
#[cfg(feature = "concurrent")]
extern crate threadpool;

#[cfg(target_arch = "wasm32")]
extern crate wasm_bindgen;

mod sseq;

pub mod actions;
pub mod managers;
#[cfg(target_arch = "wasm32")]
pub mod wasm_bindings;

#[cfg(target_arch = "wasm32")]
pub type Sender = wasm_bindings::Sender;

#[cfg(not(target_arch = "wasm32"))]
pub type Sender = std::sync::mpsc::Sender<actions::Message>;
