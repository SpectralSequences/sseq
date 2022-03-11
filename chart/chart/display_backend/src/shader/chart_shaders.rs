use std::collections::{BTreeMap, btree_map};

use wasm_bindgen::JsValue;

use web_sys::{WebGlBuffer, WebGl2RenderingContext};
use lyon::geom::math::{Point, Vector};

#[allow(unused_imports)]
use create::log;
use create::webgl_wrapper::WebGlWrapper;

use create::vector::{JsPoint, Vec4};
use create::coordinate_system::CoordinateSystem;
use create::glyph::{Glyph, GlyphUuid, GlyphInstance};

use create::shader::{GridShader, GlyphShader, HitCanvasShader, EdgeShader, EdgeOptions};

// I'm pretty confused by the way matrices are laid out in opengl.
// Apparently a mat3x2 has 3 "columns" and 2 "rows". std140 layout requires
// that each "column" of a matrix be padded out to a Vec4, so transform has 3 columns each with 4 entries.
// The last two entries of each column are padding.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
struct UniformBufferData {
    transform : [f32; 12],
    origin : Point,
    scale : Vector,
    glyph_scale : f32,
    // We have to be aligned to a multiple of 4 floats. This padding brings us up to 20.
    padding : [f32 ; 3]
}

impl From<CoordinateSystem> for UniformBufferData {
    fn from(c : CoordinateSystem) -> Self {
        let transform_array = c.transform.to_array();
        Self {
            transform : [
                transform_array[0], transform_array[1], 0.0, 0.0,
                transform_array[2], transform_array[3], 0.0, 0.0,
                transform_array[4], transform_array[5], 0.0, 0.0,
            ],
            origin : c.origin,
            scale : c.scale,
            glyph_scale : c.glyph_scale,
            padding : [0.0, 0.0, 0.0]
        }
    }
}


static GRID_LIGHT_COLOR : Vec4 = Vec4::new(0.0, 0.0, 0.0, 30.0 / 255.0);
static GRID_DARK_COLOR : Vec4 = Vec4::new(0.0, 0.0, 0.0, 90.0 / 255.0);

pub struct ChartShaders {
    webgl : WebGlWrapper,
    glyph_map : BTreeMap<GlyphUuid, usize>,
    uniform_buffer : Option<WebGlBuffer>,
    minor_grid_shader : GridShader,
    major_grid_shader : GridShader,
    glyph_shader : GlyphShader,
    edge_shader : EdgeShader,
    hit_canvas_shader : HitCanvasShader,
}

impl ChartShaders {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let uniform_buffer = webgl.create_buffer();
        webgl.bind_buffer(WebGl2RenderingContext::UNIFORM_BUFFER, uniform_buffer.as_ref());
        webgl.buffer_data_with_i32(WebGl2RenderingContext::UNIFORM_BUFFER, std::mem::size_of::<UniformBufferData>() as i32, WebGl2RenderingContext::DYNAMIC_DRAW);
        webgl.bind_buffer_base(WebGl2RenderingContext::UNIFORM_BUFFER, 0, uniform_buffer.as_ref());
        webgl.bind_buffer(WebGl2RenderingContext::UNIFORM_BUFFER, None);

        let mut minor_grid_shader = GridShader::new(webgl.clone())?;
        minor_grid_shader.thickness(0.5);
        minor_grid_shader.color(GRID_LIGHT_COLOR);
        minor_grid_shader.grid_step(2, 2);

        let mut major_grid_shader = GridShader::new(webgl.clone())?;
        major_grid_shader.thickness(0.5);
        major_grid_shader.color(GRID_DARK_COLOR);
        major_grid_shader.grid_step(10, 10);

        let glyph_shader = GlyphShader::new(webgl.clone())?;
        let hit_canvas_shader = HitCanvasShader::new(webgl.clone())?;
        let edge_shader = EdgeShader::new(webgl.clone())?;
        Ok(Self {
            webgl,
            glyph_map : BTreeMap::new(),
            uniform_buffer,
            minor_grid_shader,
            major_grid_shader,
            glyph_shader,
            hit_canvas_shader,
            edge_shader,
        })
    }

    pub fn clear_all(&mut self) {
        self.glyph_map.clear();
        self.glyph_shader.clear_all();
        self.hit_canvas_shader.clear_all();
        self.edge_shader.clear_all();
    }

    pub fn clear_glyphs(&mut self) {
        self.glyph_shader.clear_glyph_instances();
        self.hit_canvas_shader.clear_glyph_instances();
    }

    pub fn clear_edges(&mut self) {
        self.edge_shader.clear_edge_instances();
    }

    fn glyph_index(&mut self, glyph : &Glyph) -> Result<usize, JsValue>{
        let next_index = self.glyph_map.len();
        let entry = self.glyph_map.entry(glyph.uuid);
        Ok(match entry {
            btree_map::Entry::Occupied(oe) => *oe.get(),
            btree_map::Entry::Vacant(ve) => {
                self.glyph_shader.add_glyph_data(glyph)?;
                self.edge_shader.add_glyph_hull(glyph.boundary().iter().copied());
                self.hit_canvas_shader.add_glyph_hull(glyph.boundary().iter().copied());
                *ve.insert(next_index)
            }
        })
    }

    pub fn add_glyph_instance(&mut self, glyph_instance : GlyphInstance) -> Result<(), JsValue> {
        let glyph_index = self.glyph_index(&glyph_instance.glyph)?;
        self.glyph_shader.add_glyph_instance(glyph_instance.clone(), glyph_index);
        self.hit_canvas_shader.add_glyph_instance(glyph_instance, glyph_index)?;
        Ok(())
    }

    pub fn add_edge(&mut self, start : GlyphInstance, end : GlyphInstance, options : &EdgeOptions) -> Result<(), JsValue> {
        let start_glyph_index = self.glyph_index(&start.glyph)?;
        let end_glyph_index = self.glyph_index(&end.glyph)?;
        self.edge_shader.add_edge(start, end, start_glyph_index, end_glyph_index, options)?;
        Ok(())
    }

    pub fn object_underneath_pixel(&self, coordinate_system : CoordinateSystem, p : JsPoint) -> Result<Option<u32>, JsValue> {
        self.hit_canvas_shader.object_underneath_pixel(coordinate_system, p.into())
    }


    pub fn update_canvas_dimensions(&self, coord_system : CoordinateSystem){
        let left = (coord_system.left_margin as f64 * coord_system.buffer_dimensions.density()) as i32;
        let bottom = (coord_system.bottom_margin as f64 * coord_system.buffer_dimensions.density()) as i32;
        let width = ((coord_system.buffer_dimensions.width() - coord_system.left_margin - coord_system.right_margin) as f64  * coord_system.buffer_dimensions.density()) as i32;
        let height = ((coord_system.buffer_dimensions.height() - coord_system.top_margin - coord_system.bottom_margin) as f64  * coord_system.buffer_dimensions.density()) as i32;
        self.webgl.scissor(left, bottom, width, height);
        self.webgl.viewport_dimensions(coord_system.buffer_dimensions);
    }

    fn update_uniform_buffer(&self, coordinate_system : CoordinateSystem){
        self.webgl.bind_buffer(WebGl2RenderingContext::UNIFORM_BUFFER, self.uniform_buffer.as_ref());
        let uniform_data : UniformBufferData = coordinate_system.into();
        let ptr = &uniform_data as *const UniformBufferData as *const u8;
        let len = std::mem::size_of::<UniformBufferData>();
        let data = unsafe {
            std::slice::from_raw_parts(ptr, len)
        };
        self.webgl.buffer_sub_data_with_i32_and_u8_array(WebGl2RenderingContext::UNIFORM_BUFFER, 0, data);
        self.webgl.bind_buffer(WebGl2RenderingContext::UNIFORM_BUFFER, None);
    }

    fn enable_clip(&self){
        self.webgl.enable(WebGl2RenderingContext::SCISSOR_TEST);
    }

    fn disable_clip(&self){
        self.webgl.disable(WebGl2RenderingContext::SCISSOR_TEST);
    }

    pub fn render(&mut self, coordinate_system : CoordinateSystem) -> Result<(), JsValue> {
        self.update_canvas_dimensions(coordinate_system);
        self.update_uniform_buffer(coordinate_system);
        self.disable_clip();
        self.webgl.clear_color(0.0, 0.0, 0.0, 0.0);
        self.webgl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
        self.enable_clip();
        self.webgl.premultiplied_blend_mode();
        self.minor_grid_shader.draw(coordinate_system)?;
        self.major_grid_shader.draw(coordinate_system)?;

        self.glyph_shader.draw()?;
        self.edge_shader.draw()?;
        self.hit_canvas_shader.draw(coordinate_system)?;
        Ok(())
    }
}
