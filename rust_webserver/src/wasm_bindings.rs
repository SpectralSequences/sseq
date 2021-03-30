use crate::actions::*;
use crate::managers::*;
use algebra::steenrod_evaluator::SteenrodCalculator as SteenrodCalculator_;
use fp::prime::ValidPrime;
use js_sys::Function;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
pub struct Sender {
    f: Function,
}

impl Sender {
    pub fn new(f: Function) -> Self {
        Sender { f }
    }

    pub fn send(&self, msg: Message) -> error::Result<()> {
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
        let msg: Result<Message, serde_json::Error> = serde_json::from_str(&m);
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
        let msg: Result<Message, serde_json::Error> = serde_json::from_str(&m);
        if msg.is_err() {
            println!("Unable to understand message:\n{}", m);
            println!("Error: {:?}", msg);
        }

        let msg = msg.unwrap();

        self.s.process_message(msg).unwrap();
    }
}

#[wasm_bindgen]
pub struct SteenrodCalculator(SteenrodCalculator_);

#[wasm_bindgen]
impl SteenrodCalculator {
    pub fn new(p: u32) -> Self {
        Self(SteenrodCalculator_::new(ValidPrime::new(p)))
    }

    pub fn compute_basis(&self, degree: i32) {
        self.0.compute_basis(degree);
    }

    pub fn evaluate_adem(&self, input: &str) -> Result<String, JsValue> {
        self.0
            .evaluate_adem_to_string(input)
            .map_err(|err| JsValue::from(err.to_string()))
    }

    pub fn evaluate_milnor(&self, input: &str) -> Result<String, JsValue> {
        self.0
            .evaluate_milnor_to_string(input)
            .map_err(|err| JsValue::from(err.to_string()))
    }
}
