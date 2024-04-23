use lyon::geom::math::{point, vector};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};

// use std::f32::consts::PI;
#[allow(unused_imports)]
use crate::log;
use crate::{
    coordinate_system::{BufferDimensions, CoordinateSystem},
    glyph::{Glyph, GlyphInstance},
    shader::{ChartShaders, EdgeOptions},
    vector::{JsPoint, Vec4},
    webgl_wrapper::WebGlWrapper,
};

#[wasm_bindgen]
pub struct Canvas {
    canvas: HtmlCanvasElement,
    coordinate_system: CoordinateSystem,
    chart_shaders: ChartShaders,
}

#[wasm_bindgen]
impl Canvas {
    #[wasm_bindgen(constructor)]
    pub fn new(webgl_context: &WebGl2RenderingContext) -> Result<Self, JsValue> {
        let webgl = WebGlWrapper::new(webgl_context.clone());
        let canvas = webgl.canvas()?;
        let chart_shaders = ChartShaders::new(webgl.clone())?;

        let coordinate_system = CoordinateSystem::new();
        Ok(Self {
            canvas,
            coordinate_system,
            chart_shaders,
        })
    }

    // Returns : [xNearest, yNearest, distance]
    pub fn nearest_gridpoint(&self, point: &JsPoint) -> Vec<f32> {
        let pt = point.into();
        let nearest = self
            .coordinate_system
            .transform_point(self.coordinate_system.inverse_transform_point(pt).round());
        vec![nearest.x, nearest.y, nearest.distance_to(pt)]
    }

    pub fn transform_point(&self, point: &JsPoint) -> JsPoint {
        self.coordinate_system.transform_point(point.into()).into()
    }

    pub fn transform_x(&self, x: f32) -> f32 {
        self.coordinate_system.transform_x(x)
    }

    pub fn transform_y(&self, y: f32) -> f32 {
        self.coordinate_system.transform_y(y)
    }

    pub fn scale_x(&self, x: f32) -> f32 {
        x * self.coordinate_system.scale.x
    }

    pub fn scale_y(&self, y: f32) -> f32 {
        y * self.coordinate_system.scale.y
    }

    pub fn inverse_transform_point(&self, point: &JsPoint) -> JsPoint {
        self.coordinate_system
            .inverse_transform_point(point.into())
            .into()
    }

    pub fn glyph_position(&self, position: &JsPoint, offset: &JsPoint) -> JsPoint {
        self.coordinate_system
            .glyph_position(position.into(), offset.into())
            .into()
    }

    pub fn set_margins(
        &mut self,
        left_margin: i32,
        right_margin: i32,
        bottom_margin: i32,
        top_margin: i32,
    ) {
        self.coordinate_system
            .set_margins(left_margin, right_margin, bottom_margin, top_margin);
    }

    pub fn set_padding(&mut self, padding: f32) {
        self.coordinate_system.set_padding(padding);
    }

    // For the publicly exposed version we update the "natural scale"
    pub fn set_current_xrange(&mut self, xmin: f32, xmax: f32) {
        self.coordinate_system.set_current_xrange(xmin, xmax);
        self.coordinate_system.update_natural_ratio();
    }

    // For the publicly exposed version we update the "natural scale"
    pub fn set_current_yrange(&mut self, ymin: f32, ymax: f32) {
        self.coordinate_system.set_current_yrange(ymin, ymax);
        self.coordinate_system.update_natural_ratio();
    }

    pub fn current_xrange(&mut self) -> Vec<f32> {
        let (a, b) = self.coordinate_system.current_xrange();
        vec![a, b]
    }

    pub fn current_yrange(&mut self) -> Vec<f32> {
        let (a, b) = self.coordinate_system.current_yrange();
        vec![a, b]
    }

    pub fn set_max_xrange(&mut self, xmin: f32, xmax: f32) {
        self.coordinate_system.set_max_xrange(xmin, xmax);
    }

    pub fn set_max_yrange(&mut self, ymin: f32, ymax: f32) {
        self.coordinate_system.set_max_yrange(ymin, ymax);
    }

    pub fn translate(&mut self, delta: JsPoint) {
        self.coordinate_system.translate(delta);
    }

    pub fn scale_around(&mut self, scale: f32, center: JsPoint) -> Result<(), JsValue> {
        self.coordinate_system.scale_around(scale, center)?;
        Ok(())
    }

    pub fn set_glyph_scale(&mut self, glyph_scale: f32) {
        self.coordinate_system.set_glyph_scale(glyph_scale);
    }

    pub fn apply_transform(&self, p: JsPoint) -> JsPoint {
        self.coordinate_system
            .transform
            .transform_point(p.into())
            .into()
    }

    pub fn resize(&mut self, width: i32, height: i32, density: f64) -> Result<(), JsValue> {
        let new_dimensions = BufferDimensions::new(width, height, density);
        if new_dimensions == self.coordinate_system.buffer_dimensions {
            return Ok(());
        }
        let current_xrange = self.coordinate_system.current_xrange();
        let current_yrange = self.coordinate_system.current_yrange();
        self.coordinate_system.buffer_dimensions = new_dimensions;
        self.canvas
            .style()
            .set_property("width", &format!("{}px", new_dimensions.width()))?;
        self.canvas
            .style()
            .set_property("height", &format!("{}px", new_dimensions.height()))?;
        self.canvas.set_width(new_dimensions.pixel_width() as u32);
        self.canvas.set_height(new_dimensions.pixel_height() as u32);

        self.coordinate_system.reset_transform();
        // Make sure not to update "natural scale."
        self.coordinate_system
            .set_current_xrange(current_xrange.0, current_xrange.1);
        self.coordinate_system
            .set_current_yrange(current_yrange.0, current_yrange.1);
        Ok(())
    }

    pub fn clear_all(&mut self) {
        self.chart_shaders.clear_all();
    }

    pub fn clear(&mut self) {
        self.clear_glyphs();
        self.clear_edges();
    }

    pub fn clear_glyphs(&mut self) {
        self.chart_shaders.clear_glyphs();
    }

    pub fn clear_edges(&mut self) {
        self.chart_shaders.clear_edges();
    }

    pub fn add_glyph(
        &mut self,
        point: &JsPoint,
        offset: &JsPoint,
        glyph: &Glyph,
        scale: f32,
        background_color: &Vec4,
        border_color: &Vec4,
        foreground_color: &Vec4,
    ) -> Result<GlyphInstance, JsValue> {
        let glyph_instance = GlyphInstance::new(
            glyph.clone(),
            point.into(),
            offset.into(),
            scale,
            *background_color,
            *border_color,
            *foreground_color,
        );
        self.chart_shaders
            .add_glyph_instance(glyph_instance.clone())?;
        Ok(glyph_instance)
    }

    pub fn add_edge(
        &mut self,
        start_glyph_instance: &GlyphInstance,
        end_glyph_instance: &GlyphInstance,
        edge_options: &EdgeOptions,
    ) -> Result<(), JsValue> {
        self.chart_shaders.add_edge(
            start_glyph_instance.clone(),
            end_glyph_instance.clone(),
            edge_options,
        )?;
        Ok(())
    }

    pub fn test_edge_shader(
        &mut self,
        start_position: &JsPoint,
        start_offset: &JsPoint,
        end_position: &JsPoint,
        end_offset: &JsPoint,
        start_glyph: &Glyph,
        end_glyph: &Glyph,
        scale: f32,
        edge_options: &EdgeOptions,
    ) -> Result<(), JsValue> {
        self.clear();
        let start_glyph = GlyphInstance::new(
            start_glyph.clone(),
            start_position.into(),
            start_offset.into(),
            scale,
            Vec4::new(1.0, 0.0, 0.0, 0.5),
            Vec4::new(0.0, 0.0, 0.0, 0.5),
            Vec4::new(1.0, 0.0, 0.0, 0.5),
        );
        let end_glyph = GlyphInstance::new(
            end_glyph.clone(),
            end_position.into(),
            end_offset.into(),
            scale,
            Vec4::new(0.0, 0.0, 1.0, 0.5),
            Vec4::new(0.0, 1.0, 0.0, 0.5),
            Vec4::new(0.0, 0.0, 1.0, 0.5),
        );
        self.chart_shaders.add_glyph_instance(start_glyph.clone())?;
        self.chart_shaders.add_glyph_instance(end_glyph.clone())?;

        self.chart_shaders
            .add_edge(start_glyph, end_glyph, edge_options)?;

        Ok(())
    }

    pub fn test_speed_setup(
        &mut self,
        glyph1: &Glyph,
        glyph2: &Glyph,
        xy_max: usize,
        scale: f32,
        edge_options: &EdgeOptions,
    ) -> Result<(), JsValue> {
        self.clear();
        let mut glyph_instances = Vec::new();

        for x in 0..xy_max {
            for y in 0..xy_max {
                // let s = if (x + y) % 2 == 1 { &glyph1 } else { &glyph2 };
                // let r = x as f32 /  xy_max as f32;
                // let b = y as f32 /  xy_max as f32;
                // let glyph_instance = GlyphInstance::new(s.clone(), point(x as f32, y as f32), scale, Vec4::new(r, 0.0, b, 1.0), Vec4::new(b, 0.0, r, 1.0));

                let glyph = if (x + y) % 2 == 1 { glyph1 } else { glyph2 };
                let glyph_instance = GlyphInstance::new(
                    glyph.clone(),
                    point(x as f32, y as f32),
                    vector(0.0, 0.0),
                    scale,
                    Vec4::new(0.0, 0.0, 1.0, 0.5),
                    Vec4::new(0.0, 0.0, 0.0, 0.5),
                    Vec4::new(0.0, 0.0, 1.0, 0.5),
                );
                self.chart_shaders
                    .add_glyph_instance(glyph_instance.clone())?;
                glyph_instances.push(glyph_instance);
            }
        }
        let x_max = xy_max;
        let y_max = xy_max;

        for x in 1..x_max {
            for y in 0..y_max {
                let source = {
                    let y = 0;
                    glyph_instances[x * y_max + y].clone()
                };
                let target = {
                    let x = x - 1;
                    glyph_instances[x * y_max + y].clone()
                };
                self.chart_shaders.add_edge(source, target, edge_options)?;
            }
        }
        Ok(())
    }

    pub fn render(&mut self) -> Result<(), JsValue> {
        self.chart_shaders.render(self.coordinate_system)
    }

    pub fn object_underneath_pixel(&self, p: JsPoint) -> Result<Option<u32>, JsValue> {
        self.chart_shaders
            .object_underneath_pixel(self.coordinate_system, p.into())
    }
}
