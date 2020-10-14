use std::convert::TryInto;


use wasm_bindgen::JsValue;
use web_sys::{
    WebGl2RenderingContext, 
    WebGlVertexArrayObject
};

use lyon::geom::math::{Point, Vector};

use lyon::tessellation::{VertexBuffers};

#[allow(unused_imports)]
use crate::log;
use crate::webgl_wrapper::WebGlWrapper;
use crate::shader::Program;
use crate::vector::Vec4;

use crate::glyph::{GlyphInstance, Glyph};

use crate::shader::attributes::{Format, Type, NumChannels,  Attribute, Attributes};
use crate::shader::data_texture::DataTexture;
use crate::shader::vertex_buffer::VertexBuffer;


const ATTRIBUTES : Attributes = Attributes::new(&[
    Attribute::new("aPositionOffset", 4, Type::F32), // (position, offset)
    Attribute::new("aScale", 1, Type::F32),
    Attribute::new("aColors", 4, Type::U16),
    Attribute::new("aGlyphData", 4, Type::U16), // ShaderGlyphHeader: (index, num_fill_vertices, num_stroke_vertices, padding)
]);

const GLYPH_PATHS_TEXTURE_UNIT : u32 = 0;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct ShaderGlyphHeader {
    index : u16,
    num_fill_triangles : u16,
    num_stroke_triangles : u16,
    padding : u16,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct ShaderGlyphInstance {
    position : Point,
    offset : Vector,
    scale : f32,
    fill_color : [u16;2],
    stroke_color : [u16;2],
    
    // aGlyphData
    glyph : ShaderGlyphHeader
}


fn vec4_to_u8_array(v : Vec4) -> [u16;2] {
    [u16::from_le_bytes([
        (v.x * 255.0) as u8, 
        (v.y * 255.0) as u8, 
    ]),
    u16::from_le_bytes([
        (v.z * 255.0) as u8, 
        (v.w * 255.0) as u8, 
    ])]
}



pub struct GlyphShader {
    webgl : WebGlWrapper,
    pub(in crate::shader) program : Program,
    attribute_state : Option<WebGlVertexArrayObject>,
    glyph_instances : VertexBuffer<ShaderGlyphInstance>,
    max_triangles : i32,
    
    glyph_map : Vec<ShaderGlyphHeader>,
    glyph_paths : DataTexture<Point>,

    ready : bool,
}



impl GlyphShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let program = Program::new(
            webgl.clone(), 
            include_str!("glyph.vert"),
            // include_str!("glyph.frag"),
            r#"#version 300 es
                precision highp float;
                flat in vec4 fColor;
                out vec4 outColor;
                void main() {
                    outColor = fColor;
                }
            "#
        )?;

        let glyph_instances = VertexBuffer::new(webgl.clone());
        let attribute_state = webgl.create_vertex_array();

        ATTRIBUTES.set_up_vertex_array(&webgl, &program.program, attribute_state.as_ref(), glyph_instances.buffer.as_ref())?;

        let glyph_paths = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Two));
        program.use_program();
        program.set_uniform_texture_unit("uGlyphPaths", GLYPH_PATHS_TEXTURE_UNIT);
        let index = webgl.get_uniform_block_index(&program.program, "Transform");
        webgl.uniform_block_binding(&program.program, index, 0);
        Ok(Self {
            webgl,
            program,
            glyph_map : Vec::new(),

            attribute_state,
            glyph_instances, 
            max_triangles : 0,
            
            glyph_paths,
            ready : false
        })
    }

    pub fn clear_all(&mut self){
        self.glyph_paths.clear();
        self.glyph_map.clear();
        self.clear_glyph_instances();
    }

    pub fn clear_glyph_instances(&mut self){
        self.max_triangles = 0;
        self.glyph_instances.clear();
        self.ready = false;
    }

    pub(in crate::shader) fn add_glyph_data(&mut self, glyph : &Glyph) -> Result<(), JsValue> {
        let index = self.glyph_paths.len() / 3;
        let index : Result<u16, _> = index.try_into();
        let index = index.map_err(|_| "Too many total glyph vertices : max number of triangles in all glyphs is 65535.")?;

        let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
        let scale = 100.0;
        
        glyph.tessellate_fill(&mut buffers, scale)?;
        let num_fill_triangles = buffers.indices.len()  / 3;
        self.glyph_paths.append(buffers.indices.iter().map(|&i| buffers.vertices[i as usize]));
        
        buffers.vertices.clear();
        buffers.indices.clear();

        glyph.tessellate_stroke(&mut buffers, scale)?;
        let num_stroke_triangles = buffers.indices.len() / 3;
        self.glyph_paths.append(buffers.indices.iter().map(|&i| buffers.vertices[i as usize]));
        
        let num_fill_triangles = num_fill_triangles.try_into().unwrap();
        let num_stroke_triangles  = num_stroke_triangles.try_into().unwrap();
        self.glyph_map.push(ShaderGlyphHeader {
            index, 
            num_fill_triangles, 
            num_stroke_triangles,
            padding : 0
        });
        Ok(())
    }

    pub fn add_glyph_instance(&mut self, glyph_instance : GlyphInstance, glyph_index : usize) {
        let glyph = self.glyph_map[glyph_index];
        self.max_triangles = self.max_triangles.max((glyph.num_fill_triangles + glyph.num_stroke_triangles) as i32);
        self.glyph_instances.push(ShaderGlyphInstance {
            position : glyph_instance.position,
            offset : glyph_instance.offset,
            scale : glyph_instance.scale / 100.0,
            fill_color : vec4_to_u8_array(glyph_instance.fill_color),
            stroke_color : vec4_to_u8_array(glyph_instance.stroke_color),
            glyph 
        });
        self.ready = false;
    }

    pub fn draw(&mut self) -> Result<(), JsValue> {
        if self.glyph_instances.is_empty() {
            return Ok(());
        }
        
        self.program.use_program();
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.glyph_instances.prepare();
        self.glyph_paths.bind(GLYPH_PATHS_TEXTURE_UNIT)?;

        let num_instances = self.glyph_instances.len() as i32;
        self.webgl.draw_arrays_instanced(
            WebGl2RenderingContext::TRIANGLES,
            0,
            self.max_triangles * 3,
            num_instances
        );
        self.webgl.bind_vertex_array(None);
        Ok(())
    }
}