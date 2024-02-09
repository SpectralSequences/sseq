use lyon::geom::math::{Point, Vector};
use wasm_bindgen::JsValue;
use web_sys::{
    WebGl2RenderingContext, WebGlFramebuffer, WebGlRenderbuffer, WebGlTexture,
    WebGlVertexArrayObject,
};

#[allow(unused_imports)]
use crate::log;
use crate::{
    convex_hull::ANGLE_RESOLUTION,
    coordinate_system::{BufferDimensions, CoordinateSystem},
    glyph::GlyphInstance,
    shader::{
        attributes::{Attribute, Attributes, Format, NumChannels, Type},
        data_texture::DataTexture,
        vertex_buffer::VertexBuffer,
        Program,
    },
    webgl_wrapper::WebGlWrapper,
};

const ATTRIBUTES: Attributes = Attributes::new(&[
    Attribute::new("aPositionOffset", 4, Type::F32),
    Attribute::new("aScale", 1, Type::F32),
    Attribute::new("aGlyphIndex", 2, Type::U16), // (index, padding)
]);

const GLYPH_HULL_TEXTURE_UNIT: u32 = 0;

#[derive(Debug)]
struct ShaderGlyphHeader {
    index: u16,
    padding: u16,
}

#[derive(Debug)]
struct ShaderGlyphInstance {
    position: Point,
    offset: Vector,
    scale: f32,
    glyph: ShaderGlyphHeader,
}

pub struct HitCanvasShader {
    webgl: WebGlWrapper,
    program: Program,
    hit_canvas_buffer_dimensions: BufferDimensions,
    hit_canvas_framebuffer: Option<WebGlFramebuffer>,
    hit_canvas_texture: Option<WebGlTexture>,
    hit_canvas_depth_buffer: Option<WebGlRenderbuffer>,

    attribute_state: Option<WebGlVertexArrayObject>,

    glyph_convex_hulls: DataTexture<Vector>,

    glyph_instances: VertexBuffer<ShaderGlyphInstance>,
    ready: bool,
}

impl HitCanvasShader {
    pub fn new(webgl: WebGlWrapper) -> Result<Self, JsValue> {
        let program = Program::new(
            webgl.clone(),
            include_str!("hit_canvas.vert"),
            r#"#version 300 es
                precision highp float;
                flat in vec4 fColor;
                in vec2 vPosition;
                out vec4 outColor;
                void main() {
                    gl_FragDepth = length(vPosition) / 2000.0;
                    outColor = fColor;
                }
            "#,
        )?;

        let attribute_state = webgl.create_vertex_array();
        let glyph_instances = VertexBuffer::new(webgl.clone());

        ATTRIBUTES.set_up_vertex_array(
            &webgl,
            &program.program,
            attribute_state.as_ref(),
            glyph_instances.buffer.as_ref(),
        )?;

        let glyph_convex_hulls =
            DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Four));

        program.use_program();
        program.set_uniform_texture_unit("uGlyphConvexHulls", GLYPH_HULL_TEXTURE_UNIT);
        let index = webgl.get_uniform_block_index(&program.program, "Transform");
        webgl.uniform_block_binding(&program.program, index, 0);

        Ok(Self {
            webgl,
            program,
            hit_canvas_buffer_dimensions: BufferDimensions::new(1, 1, 0.0),
            hit_canvas_texture: None,
            hit_canvas_framebuffer: None,
            hit_canvas_depth_buffer: None,

            glyph_convex_hulls,
            glyph_instances,

            attribute_state,
            ready: false,
        })
    }

    fn initialize_hit_canvas(&mut self, dimensions: BufferDimensions) {
        if self.hit_canvas_buffer_dimensions == dimensions {
            return;
        }
        self.webgl
            .delete_framebuffer(self.hit_canvas_framebuffer.as_ref());
        self.webgl.delete_texture(self.hit_canvas_texture.as_ref());
        self.webgl
            .delete_renderbuffer(self.hit_canvas_depth_buffer.as_ref());
        self.hit_canvas_framebuffer = self.webgl.create_framebuffer();
        self.hit_canvas_texture = self.webgl.create_texture();
        self.hit_canvas_depth_buffer = self.webgl.create_renderbuffer();

        self.webgl.bind_texture(
            WebGl2RenderingContext::TEXTURE_2D,
            self.hit_canvas_texture.as_ref(),
        );
        self.webgl.bind_renderbuffer(
            WebGl2RenderingContext::RENDERBUFFER,
            self.hit_canvas_depth_buffer.as_ref(),
        );
        self.webgl.tex_storage_2d(
            WebGl2RenderingContext::TEXTURE_2D,
            1, // mip levels
            WebGl2RenderingContext::RGBA8,
            dimensions.pixel_width(),
            dimensions.pixel_height(), // width, height
        );
        self.webgl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MAG_FILTER,
            WebGl2RenderingContext::NEAREST as i32,
        );
        self.webgl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MIN_FILTER,
            WebGl2RenderingContext::NEAREST as i32,
        );
        self.webgl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_S,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );
        self.webgl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_T,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );

        self.webgl.bind_framebuffer(
            WebGl2RenderingContext::FRAMEBUFFER,
            self.hit_canvas_framebuffer.as_ref(),
        );
        self.webgl.framebuffer_texture_2d(
            WebGl2RenderingContext::FRAMEBUFFER,
            WebGl2RenderingContext::COLOR_ATTACHMENT0,
            WebGl2RenderingContext::TEXTURE_2D,
            self.hit_canvas_texture.as_ref(),
            0, // level
        );
        self.webgl.framebuffer_renderbuffer(
            WebGl2RenderingContext::FRAMEBUFFER,
            WebGl2RenderingContext::DEPTH_ATTACHMENT,
            WebGl2RenderingContext::RENDERBUFFER,
            self.hit_canvas_depth_buffer.as_ref(),
        );
        self.webgl.renderbuffer_storage(
            WebGl2RenderingContext::RENDERBUFFER,
            WebGl2RenderingContext::DEPTH_COMPONENT16,
            dimensions.pixel_width(),
            dimensions.pixel_height(),
        );
    }

    pub fn add_glyph_hull<It: ExactSizeIterator<Item = Vector>>(&mut self, convex_hull: It) {
        self.glyph_convex_hulls.append(convex_hull);
    }

    pub fn add_glyph_instance(
        &mut self,
        glyph_instance: GlyphInstance,
        glyph_index: usize,
    ) -> Result<(), JsValue> {
        self.glyph_instances.push(ShaderGlyphInstance {
            position: glyph_instance.position,
            offset: glyph_instance.offset,
            scale: glyph_instance.scale,
            glyph: ShaderGlyphHeader {
                index: glyph_index as u16,
                padding: 0,
            },
        });
        self.ready = false;
        Ok(())
    }

    pub fn clear_all(&mut self) {
        self.glyph_instances.clear();
        self.glyph_convex_hulls.clear();
        self.ready = false;
    }

    pub fn clear_glyph_instances(&mut self) {
        self.glyph_instances.clear();
        self.ready = false;
    }

    pub fn draw(&mut self, coordinate_system: CoordinateSystem) -> Result<(), JsValue> {
        if self.glyph_instances.is_empty() {
            return Ok(());
        }
        self.program.use_program();
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());

        self.webgl.bind_framebuffer(
            WebGl2RenderingContext::FRAMEBUFFER,
            self.hit_canvas_framebuffer.as_ref(),
        );
        self.initialize_hit_canvas(coordinate_system.buffer_dimensions);
        self.webgl.clear_color(0.0, 0.0, 0.0, 0.0);
        self.webgl.clear(
            WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT,
        );
        self.webgl.disable(WebGl2RenderingContext::BLEND);
        self.webgl.enable(WebGl2RenderingContext::DEPTH_TEST);

        // This has a side-effect of adjusting texture bindings, so it has to occur before glyph_boundary_data.bind().

        self.glyph_instances.prepare();
        self.glyph_convex_hulls.bind(GLYPH_HULL_TEXTURE_UNIT)?;

        let num_instances = self.glyph_instances.len() as i32;
        let num_vertices = ANGLE_RESOLUTION as i32;
        self.webgl.draw_arrays_instanced(
            WebGl2RenderingContext::TRIANGLE_FAN,
            0,
            num_vertices,
            num_instances,
        );

        self.webgl
            .bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        self.webgl.bind_vertex_array(None);
        self.webgl
            .render_to_canvas(coordinate_system.buffer_dimensions);
        self.webgl.disable(WebGl2RenderingContext::DEPTH_TEST);
        self.webgl.enable(WebGl2RenderingContext::BLEND);
        Ok(())
    }

    pub fn object_underneath_pixel(
        &self,
        coordinate_system: CoordinateSystem,
        point: Point,
    ) -> Result<Option<u32>, JsValue> {
        let mut data = [0; 4];
        self.webgl.bind_framebuffer(
            WebGl2RenderingContext::FRAMEBUFFER,
            self.hit_canvas_framebuffer.as_ref(),
        );
        let density = coordinate_system.buffer_dimensions.density();
        let pixel_height = coordinate_system.buffer_dimensions.pixel_height();
        self.webgl.read_pixels_with_opt_u8_array(
            (point.x as f64 * density) as i32,                // x
            pixel_height - (point.y as f64 * density) as i32, // y
            1,
            1,                                     // width, height
            WebGl2RenderingContext::RGBA,          // format
            WebGl2RenderingContext::UNSIGNED_BYTE, // type
            Some(&mut data),                       // array to hold result
        )?;
        self.webgl
            .bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        let id = u32::from_le_bytes(data);
        Ok(id.checked_sub(1))
    }
}
