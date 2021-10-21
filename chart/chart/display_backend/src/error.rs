use wasm_bindgen::JsValue;

use lyon::tessellation::TessellationError;

pub fn convert_tessellation_error(err : TessellationError) -> JsValue {
    JsValue::from_str(&format!("{:?}", err))
}
