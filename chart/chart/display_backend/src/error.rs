use lyon::tessellation::TessellationError;
use wasm_bindgen::JsValue;

pub fn convert_tessellation_error(err: TessellationError) -> JsValue {
    JsValue::from_str(&format!("{:?}", err))
}
