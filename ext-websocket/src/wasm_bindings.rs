use js_sys::Function;
use crate::actions::*;
use crate::managers::*;
use wasm_bindgen::prelude::*;
use std::error::Error;
use std::rc::Rc;
use rust_ext::steenrod_evaluator;

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



#[wasm_bindgen]
pub struct SteenrodCalculator {
    pimpl : *const steenrod_evaluator::SteenrodCalculator,

}

#[wasm_bindgen]
impl SteenrodCalculator {
    pub fn new(p : u32) -> Self {
        let calculator = steenrod_evaluator::SteenrodCalculator::new(p);
        let boxed_calculator = Rc::new(calculator);
        Self {
            pimpl : Rc::into_raw(boxed_calculator)
        }
    }

    pub fn compute_basis(&self, degree : i32) {
        self.to_calculator().compute_basis(degree);
    }

    fn to_calculator(&self) -> Rc<steenrod_evaluator::SteenrodCalculator> {
        let raw = unsafe { Rc::from_raw(self.pimpl) };
        let clone = Rc::clone(&raw);
        std::mem::forget(raw);
        clone
    }

    pub fn evaluate_adem(&self, input : &str) -> Result<String, JsValue> {
        self.to_calculator().evaluate_adem_to_string(input).map_err(|err| JsValue::from(err.to_string()))
    }

    // pub fn evaluate_milnor(&self, input : &str) -> Result<(i32, FpVector), Box<dyn Error>> {
    //     self.to_calculator().evaluate_milnor(input)
    // }    

    pub fn free(self) {
        let _drop_me = unsafe { Rc::from_raw(self.pimpl) };
    }
}