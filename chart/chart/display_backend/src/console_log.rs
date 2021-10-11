use wasm_bindgen::prelude::*;


#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just `log(..)`
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn log_1(s: &JsValue);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn log_str(s: &str);    
}

#[macro_export]
macro_rules! log {
    () => { log_str("") };
    ($($arg:tt)*) => { crate::console_log::log_str(&format!($($arg)*)) };
}