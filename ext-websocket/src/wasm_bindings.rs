use js_sys::Function;
use crate::actions::*;
use crate::managers::*;
use wasm_bindgen::prelude::*;
use std::error::Error;

#[derive(Clone)]
pub struct Sender {
    f : Function
}

impl Sender {
    pub fn new(f : Function) -> Self {
        Sender { f }
    }

    pub fn send(&self, msg : Message) -> Result<(), Box<dyn Error>>{
        let s = serde_json::to_string(&msg).unwrap();
        self.f.call1(&JsValue::NULL, &JsValue::from(s)).unwrap();
        Ok(())
    }
}

#[wasm_bindgen]
pub struct Resolution {
    r : ResolutionManager
}

#[wasm_bindgen]
impl Resolution {
    pub fn new(f : Function) -> Self {
        Self {
            r : ResolutionManager::new(Sender::new(f))
        }
    }

    pub fn run(&mut self, m : String) {
        let msg : Result<Message, serde_json::Error> = serde_json::from_str(&m);
        if msg.is_err() {
            println!("Unable to understand message:\n{}", m);
            println!("Error: {:?}", msg);
        }

        let msg = msg.unwrap();

        self.r.process_message(msg).unwrap();
    }
}

#[wasm_bindgen]
pub struct Sseq {
    s : SseqManager
}

#[wasm_bindgen]
impl Sseq {
    pub fn new(f : Function) -> Self {
        Self {
            s : SseqManager::new(Sender::new(f))
        }
    }

    pub fn run(&mut self, m : String) {
        let msg : Result<Message, serde_json::Error> = serde_json::from_str(&m);
        if msg.is_err() {
            println!("Unable to understand message:\n{}", m);
            println!("Error: {:?}", msg);
        }

        let msg = msg.unwrap();

        self.s.process_message(msg).unwrap();
    }
}
