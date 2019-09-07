use js_sys::Function;
use crate::actions::*;
use crate::ResolutionManager;
use crate::SseqManager;
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
            r : ResolutionManager {
                sender : Sender::new(f),
                resolution : None,
                is_unit : false
            }
        }
    }

    pub fn run(&mut self, m : String) {
        let msg : Result<Message, serde_json::Error> = serde_json::from_str(&m);
        if msg.is_err() {
            println!("Unable to understand message:\n{}", m);
            println!("Error: {:?}", msg);
        }

        let msg = msg.unwrap();
        let isblock = match msg.action { Action::BlockRefresh(_) => true, _ => false };
        let target_sseq = msg.sseq;

        self.r.process_message(msg).unwrap();
        if !isblock {
            self.r.sender.send(Message {
                recipients : vec![],
                sseq : target_sseq,
                action : Action::from(Complete {})
            }).unwrap();
        }
    }
}

#[wasm_bindgen]
pub struct Sseq {
    r : SseqManager
}

#[wasm_bindgen]
impl Sseq {
    pub fn new(f : Function) -> Self {
        Self {
            r : SseqManager {
                sender : Sender::new(f),
                sseq : None,
                unit_sseq : None
            }
        }
    }

    pub fn run(&mut self, m : String) {
        let msg : Result<Message, serde_json::Error> = serde_json::from_str(&m);
        if msg.is_err() {
            println!("Unable to understand message:\n{}", m);
            println!("Error: {:?}", msg);
        }

        let msg = msg.unwrap();

        let user = match msg.action {
            Action::AddClass(_) => false,
            Action::AddProduct(_) => false,
            Action::Complete(_) => false,
            Action::Resolving(_) => false,
            _ => true
        };
        let target_sseq = msg.sseq;

        self.r.process_message(msg).unwrap();

        if user {
            self.r.sender.send(Message {
                recipients : vec![],
                sseq : target_sseq,
                action : Action::from(Complete {})
            }).unwrap();
        }
    }
}
