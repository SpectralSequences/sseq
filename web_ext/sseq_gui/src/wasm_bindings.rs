use std::panic::AssertUnwindSafe;

use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::{actions::*, managers::*};

#[derive(Clone)]
pub struct Sender {
    f: Function,
}

impl Sender {
    pub fn new(f: Function) -> Self {
        Self { f }
    }

    pub fn send(&self, msg: Message) -> anyhow::Result<()> {
        let s = serde_json::to_string(&msg)?;
        self.f.call1(&JsValue::NULL, &JsValue::from(s)).unwrap();
        Ok(())
    }
}

#[wasm_bindgen]
pub struct Resolution {
    // The manager holds `Arc`/`RwLock`/`DashMap` and so is not `RefUnwindSafe`.
    // With `panic=unwind`, wasm-bindgen wraps every exported method in
    // `catch_unwind` and therefore requires the exported struct to be
    // `RefUnwindSafe`. A panic that escapes a method tears down this whole
    // instance, so there are no surviving broken invariants to guard against;
    // assert unwind safety to satisfy the bound.
    r: AssertUnwindSafe<ResolutionManager>,
}

#[wasm_bindgen]
impl Resolution {
    pub fn new(f: Function) -> Self {
        Self {
            r: AssertUnwindSafe(ResolutionManager::new(Sender::new(f))),
        }
    }

    pub fn run(&mut self, m: String) {
        match serde_json::from_str(&m) {
            Ok(msg) => self.r.process_message(msg),
            Err(e) => self
                .r
                .send_error(format!("Failed to parse message:\n{m}\nError: {e}")),
        }
    }
}

#[wasm_bindgen]
pub struct Sseq {
    // See the note on `Resolution::r` for why this is `AssertUnwindSafe`.
    s: AssertUnwindSafe<SseqManager>,
}

#[wasm_bindgen]
impl Sseq {
    pub fn new(f: Function) -> Self {
        Self {
            s: AssertUnwindSafe(SseqManager::new(Sender::new(f))),
        }
    }

    pub fn run(&mut self, m: String) {
        match serde_json::from_str(&m) {
            Ok(msg) => self.s.process_message(msg),
            Err(e) => self
                .s
                .send_error(format!("Failed to parse message:\n{m}\nError: {e}")),
        }
    }
}
