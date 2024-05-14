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
    r: ResolutionManager,
}

#[wasm_bindgen]
impl Resolution {
    pub fn new(f: Function) -> Self {
        Self {
            r: ResolutionManager::new(Sender::new(f)),
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
    s: SseqManager,
}

#[wasm_bindgen]
impl Sseq {
    pub fn new(f: Function) -> Self {
        Self {
            s: SseqManager::new(Sender::new(f)),
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
