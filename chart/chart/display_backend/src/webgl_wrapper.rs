use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};
use wasm_bindgen::{JsValue, JsCast};
use std::ops::Deref;


#[allow(unused_imports)]
use create::log;
use create::coordinate_system::BufferDimensions;



#[derive(Clone)]
pub struct WebGlWrapper {
    pub inner : WebGl2RenderingContext
}

impl Deref for WebGlWrapper {
    type Target = WebGl2RenderingContext;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl WebGlWrapper {
    pub fn new(inner : WebGl2RenderingContext) -> Self {
        Self { inner }
    }

    pub fn canvas(&self) -> Result<HtmlCanvasElement, JsValue> {
        Ok(self.inner.canvas().ok_or("context.canvas is undefined")?.dyn_into()?)
    }

    // pub fn dimensions(&self) -> Result<BufferDimensions, JsValue> {
    //     let canvas = self.canvas()?;
    //     let width = canvas.client_width();
    //     let height = canvas.client_height();
    //     let density = WebGlWrapper::pixel_density();
    //     Ok(BufferDimensions::new(width, height, density))
    // }

    pub fn pixel_density() -> f64 {
        web_sys::window().unwrap().device_pixel_ratio()
    }

    pub fn point_to_pixels(points : f32) -> f32 {
        ((points as f64) * WebGlWrapper::pixel_density()) as f32
    }

    pub fn viewport_dimensions(&self, dimensions : BufferDimensions) {
        self.inner.viewport(0, 0, dimensions.pixel_width(), dimensions.pixel_height());
    }

    pub fn render_to_canvas(&self, dimensions : BufferDimensions) {
        self.inner.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        self.viewport_dimensions(dimensions);
    }

    // pub fn copy_blend_mode(&self){
    //     self.disable(WebGl2RenderingContext::BLEND);
    // }

    pub fn premultiplied_blend_mode(&self){
        self.enable(WebGl2RenderingContext::BLEND);
        self.blend_func(WebGl2RenderingContext::ONE, WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA);
    }
}
