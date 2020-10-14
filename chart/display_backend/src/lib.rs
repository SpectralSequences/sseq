#![deny(unused_must_use)]
//#![allow(dead_code)]
//#![allow(unused_imports)]

mod console_log;
mod error;

mod convex_hull;

mod vector;

mod arrow;

mod webgl_wrapper;
mod shader;

mod coordinate_system;
mod canvas;


mod glyph;

use crate::canvas::Canvas;


use wasm_bindgen::prelude::*;


use web_sys::{WebGl2RenderingContext};

#[wasm_bindgen]
pub fn get_rust_canvas(context : &WebGl2RenderingContext) -> Result<Canvas, JsValue> {
    console_error_panic_hook::set_once();
    Ok(Canvas::new(context)?)
}

#[wasm_bindgen]
pub fn rust_main() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    // #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
