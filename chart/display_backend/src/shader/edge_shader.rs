use std::collections::{BTreeMap, btree_map};
use std::convert::TryInto;

use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, WebGlVertexArrayObject, WebGlTexture};

use lyon::geom::math::{Point, Angle, Vector};
use lyon::tessellation::{VertexBuffers};

#[allow(unused_imports)]
use crate::log;
use crate::vector::{Vec4};
use crate::shader::Program;
use crate::webgl_wrapper::WebGlWrapper;

use crate::glyph::{GlyphInstance};
use crate::arrow::{Arrow, ArrowId};

use crate::shader::attributes::{Format, Type, NumChannels, Attribute, Attributes};
use crate::shader::data_texture::DataTexture;
use crate::shader::vertex_buffer::VertexBuffer;


const DASH_PATTERN_TEXTURE_WIDTH : usize = 512;

const ATTRIBUTES : Attributes = Attributes::new(&[
    Attribute::new("aColor", 4, Type::F32), // color
    Attribute::new("aStartPositionOffset", 4, Type::F32), // (start_position, start_offset)
    Attribute::new("aEndPositionOffset", 4, Type::F32), // (end_position, end_offset)
    Attribute::new("aGlyphScales_angle_thickness", 4, Type::F32), // (start_glyph_scale, end_glyph_scale, angle, thickness)

    Attribute::new("aStart", 4, Type::I16), // (startGlyph, vec3 startArrow = (NumVertices, HeaderIndex, VerticesIndex) )
    Attribute::new("aEnd", 4, Type::I16), // (endGlyph, vec3 endArrow = (NumVertices, HeaderIndex, VerticesIndex) )
    Attribute::new("aDashPattern", 4, Type::I16), // (dash_length, dash_index, dash_offset, dash_padding )
]);

const GLYPH_HULL_TEXTURE_UNIT : u32 = 0;
const ARROW_METRICS_TEXTURE_UNIT : u32 = 1;
const ARROW_PATHS_TEXTURE_UNIT : u32 = 2;
const DASH_PATTERNS_TEXTURE_UNIT : u32 = 3;

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct EdgeOptions {
    start_tip : Option<Arrow>, 
    end_tip : Option<Arrow>,
    angle : Angle,
    thickness : f32,
    dash_pattern : Vec<u8>,
    color : Vec4,
}

#[wasm_bindgen]
impl EdgeOptions {
    pub fn new() -> Self {
        Self {
            start_tip : None,
            end_tip : None,
            angle : Angle::zero(),
            thickness : 1.0,
            dash_pattern : vec![],
            color : Vec4::new(0.0, 0.0, 0.0, 1.0)
        }
    }

    pub fn set_color(&mut self, color : Vec4){
        self.color = color;
    }

    pub fn set_tips(&mut self, arrow : &Arrow) {
        self.start_tip = Some(arrow.clone());
        self.end_tip = Some(arrow.clone());
    }

    pub fn set_start_tip(&mut self, arrow : &Arrow) {
        self.start_tip = Some(arrow.clone());
    }

    pub fn set_end_tip(&mut self, arrow : &Arrow) {
        self.end_tip = Some(arrow.clone());
    }

    pub fn no_tips(&mut self) {
        self.start_tip = None;
        self.end_tip = None;
    }

    pub fn no_start_tip(&mut self) {
        self.start_tip = None;
    }

    pub fn no_end_tip(&mut self) {
        self.end_tip = None;
    }

    pub fn set_bend_degrees(&mut self, degrees : f32) {
        self.angle = Angle::degrees(degrees);
    }

    pub fn set_thickness(&mut self, thickness : f32) {
        self.thickness = thickness;
    }

    pub fn set_dash_pattern(&mut self, dash_pattern : Vec<u8>) {
        self.dash_pattern = dash_pattern;
    }
}


#[derive(Clone, Copy, Debug)]
#[repr(C, align(4))]
struct EdgeInstance {
    color : Vec4,
    start_position : Point, 
    start_offset : Vector,
    end_position : Point,
    end_offset : Vector,

    start_glyph_scale : f32,
    end_glyph_scale : f32,
    angle : f32,
    thickness : f32,
    
    start_glyph : u16,
    start_arrow : ArrowIndices,
    end_glyph : u16,
    end_arrow : ArrowIndices,

    dash_length : u16, 
    dash_index : u16, 
    dash_offset : u16, 
    dash_padding : u16,
}

// Arrow metrics used to position the arrow.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct ArrowMetrics {
    tip_end : f32,
    back_end : f32,
    visual_tip_end : f32,
    visual_back_end : f32,
    line_end : f32,
}

// The part of the arrow data that we store directly into the attributes buffer.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct ArrowIndices {
    num_vertices : u16,
    metrics_index : u16, // Where to find the rest of the arrow metrics in the ArrowHeader data texture
    vertices_index : u16 // Where to find the arrow vertices.
}

pub struct EdgeShader {
    webgl : WebGlWrapper,
    program : Program,
    
    edge_instances : VertexBuffer<EdgeInstance>,
    max_vertices : i32,
    attribute_state : Option<WebGlVertexArrayObject>,
    
    // This stores the distance to the boundary at each angle.
    // Angle resolution is 2 degrees, so the float at index n is the distance to boundary at 2n degrees.
    glyph_convex_hulls : DataTexture<f32>,


    // Arrow indices are stored in each EdgeInstance, contains number of arrows and index into arrow_metrics and arrow_paths
    tip_map : BTreeMap<ArrowId, ArrowIndices>,
    arrow_metrics_data : DataTexture<ArrowMetrics>,
    arrow_path_data : DataTexture<Point>,

    // The dash texture we use like a 1D texture with linear blending.
    // So instead of leaving the management to DataTexture, we will manage it ourselves.
    dash_data : Vec<u8>,
    dash_texture : Option<WebGlTexture>,
    dash_texture_num_rows : usize, // record how big texture is to decide when to reallocate.
    dash_map : BTreeMap<Vec<u8>, (u16, u16)>,

    ready : bool,
}


impl EdgeShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let program = Program::new(
            webgl.clone(), 
            include_str!("edge.vert"),
            include_str!("edge.frag")
        )?;
        let attribute_state = webgl.create_vertex_array();
        let edge_instances = VertexBuffer::new(webgl.clone());
        ATTRIBUTES.set_up_vertex_array(&webgl, &program.program, attribute_state.as_ref(), edge_instances.buffer.as_ref())?;

        let glyph_convex_hulls = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Four));
        let arrow_metrics_data = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Four));
        let arrow_path_data = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Two));
        
        program.use_program();
        program.set_uniform_texture_unit("uGlyphConvexHulls", GLYPH_HULL_TEXTURE_UNIT);
        program.set_uniform_texture_unit("uArrowMetrics", ARROW_METRICS_TEXTURE_UNIT);
        program.set_uniform_texture_unit("uArrowPaths", ARROW_PATHS_TEXTURE_UNIT);
        program.set_uniform_texture_unit("uDashPatterns", DASH_PATTERNS_TEXTURE_UNIT);
        let index = webgl.get_uniform_block_index(&program.program, "Transform");
        webgl.uniform_block_binding(&program.program, index, 0);

        let dash_texture = webgl.create_texture();
        let mut dash_map = BTreeMap::new();
        dash_map.insert(vec![], (0, 0));

        Ok(Self {
            webgl,
            program,

            edge_instances,
            max_vertices : 0,
            attribute_state,

            glyph_convex_hulls,

            tip_map : BTreeMap::new(),
            arrow_metrics_data,
            arrow_path_data,
            
            dash_data : Vec::new(),
            dash_texture,
            dash_texture_num_rows : 0,
            dash_map,
            ready : false,
        })
    }

    // Set up dash data for a dash pattern.
    // Each dash pattern (other than the solid one) will go into its own row of the DashTexture.
    // We store the length of the DashPattern separately: because each DashPattern has a different length,
    // we have to manually wrap the dash pattern in the shader using mod (rather than just using TEXTURE_WRAP_S = REPEAT).
    // If a dash pattern looks like [..., a, b, ...] then we turn that into a "on" bytes followed by b "off" bytes.
    // To ensure that the starting edge when we start the pattern over is correctly blended, we add one extra "on" byte at the end.
    // The total of all of the entries must be <= DASH_PATTERN_TEXTURE_WIDTH.
    // (Instead of requiring this we could also dynamically widen the texture...)
    fn dash_data(&mut self, dash_pattern : Vec<u8>) -> Result<(u16, u16), JsValue> {
        let entry = self.dash_map.entry(dash_pattern);
        Ok(match entry {
            btree_map::Entry::Occupied(oe) => *oe.get(),
            btree_map::Entry::Vacant(ve) => {
                let dash_pattern = ve.key();
                let mut pattern_len : u16 = dash_pattern.iter().map(|&b| b as u16).sum();
                if dash_pattern.len() % 2 == 1 {
                    pattern_len *= 2;
                }
                if pattern_len > DASH_PATTERN_TEXTURE_WIDTH as u16 {
                    return Err(format!("Dash pattern too long. Max of {} total.", DASH_PATTERN_TEXTURE_WIDTH - 1).into())
                }
                let orig_dash_data_len = self.dash_data.len();
                let dash_pattern_row = orig_dash_data_len / DASH_PATTERN_TEXTURE_WIDTH;
                for (i, &e) in dash_pattern.iter().enumerate() {
                    let value = if i%2 == 1 { 0 } else { 255 };
                    for _ in 0..e {
                        self.dash_data.extend(&[value]);
                    }
                }
                // If pattern has odd length, then double it up with its negation
                if dash_pattern.len() % 2 == 1 {
                    for (i, &e) in dash_pattern.iter().enumerate() {
                        let value = if i%2 == 1 { 255 } else { 0 };
                        for _ in 0..e {
                            self.dash_data.extend(&[value]);
                        }
                    }
                }
                // An extra sentinel "on" value. This is so that the final "off" segment is correctly linearly blended
                // with the inital "on" segment when we wrap around. If the length of the dash pattern is exactly
                // DASH_PATTERN_TEXTURE_WIDTH, then we have no space for this but also the blending occurs automatically.
                // (I assume that's how TEXTURE_WRAP_S = REPEAT must work...)
                if pattern_len < DASH_PATTERN_TEXTURE_WIDTH as u16 {
                    self.dash_data.extend(&[255]);
                }
                self.dash_data.resize_with(orig_dash_data_len +  DASH_PATTERN_TEXTURE_WIDTH, ||0);
                *ve.insert((dash_pattern_row as u16, pattern_len))
            }
        })
    }

    // Increase number of texture rows to make space if necessary.
    // TODO: Maybe double number of rows for each realloc?
    fn ensure_dash_texture_size(&mut self){
        let num_rows = self.dash_data.len() / DASH_PATTERN_TEXTURE_WIDTH;
        if num_rows <= self.dash_texture_num_rows {
            return;
        }
        self.dash_texture_num_rows = num_rows;
        self.webgl.delete_texture(self.dash_texture.as_ref());
        self.dash_texture = self.webgl.create_texture();
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.dash_texture.as_ref());
        self.webgl.tex_storage_2d(
            WebGl2RenderingContext::TEXTURE_2D,
            1, // mip levels
            WebGl2RenderingContext::R8,
            DASH_PATTERN_TEXTURE_WIDTH as i32, self.dash_texture_num_rows as i32
        );
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MAG_FILTER, WebGl2RenderingContext::LINEAR as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MIN_FILTER, WebGl2RenderingContext::LINEAR as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_S, WebGl2RenderingContext::REPEAT as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_T, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
    }

    fn upload_dash_texture_data(&mut self) -> Result<(), JsValue>{
        self.ensure_dash_texture_size();
        let num_rows = self.dash_data.len() / DASH_PATTERN_TEXTURE_WIDTH;
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.dash_texture.as_ref());
        self.webgl.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_u8_array(
            WebGl2RenderingContext::TEXTURE_2D, 
            0, // mip level
            0, 0, // xoffset, yoffset: i32,
            DASH_PATTERN_TEXTURE_WIDTH as i32, num_rows as i32, // width, height
            WebGl2RenderingContext::RED, // format: u32,
            WebGl2RenderingContext::UNSIGNED_BYTE, // type_: u32,
            Some(&self.dash_data) // pixels: Option<&Object>
        )?; 
        Ok(())
    }

    fn bind_dash_data(&mut self, texture_unit : u32) -> Result<(), JsValue> {
        self.webgl.active_texture(WebGl2RenderingContext::TEXTURE0 + texture_unit);
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.dash_texture.as_ref());
        if !self.dash_data.is_empty() {
            self.upload_dash_texture_data()?;
        }
        Ok(())
    }

    pub fn clear_all(&mut self){
        self.tip_map.clear();
        self.edge_instances.clear();
        self.arrow_metrics_data.clear();
        self.arrow_path_data.clear();
        self.ready = false;
    }

    pub fn clear_edge_instances(&mut self){
        self.max_vertices = 0;
        self.edge_instances.clear();
        self.ready = false;
    }    

    // Get arrow tip data to put into the attribute buffer.
    // If the given arrow has never been used before, we tesselate it and add the path and metrics to the relevant data textures.
    fn arrow_tip_data(&mut self, arrow : &Arrow) -> Result<ArrowIndices, JsValue> {
        let entry = self.tip_map.entry(arrow.uuid);
        match entry {
            btree_map::Entry::Occupied(oe) => Ok(*oe.get()),
            btree_map::Entry::Vacant(ve) => {
                let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
                arrow.tesselate_into_buffers(&mut buffers)?;

                let vertices_index = self.arrow_path_data.len();
                let num_vertices = buffers.indices.len();
                let metrics_index : Result<u16, _> = (std::mem::size_of_val(self.arrow_metrics_data.data())/4).try_into();
                let metrics_index = metrics_index.map_err(|_| "Too many total arrow heads.")?;
                self.arrow_path_data.append(buffers.indices.iter().map(|&i| buffers.vertices[i as usize]));
                self.arrow_metrics_data.push(ArrowMetrics {     
                    tip_end : arrow.tip_end,
                    back_end : arrow.back_end,
                    visual_tip_end : arrow.visual_tip_end,
                    visual_back_end : arrow.visual_back_end,
                    line_end : arrow.line_end, 
                });

                let arrow_indices = ArrowIndices {
                    num_vertices : num_vertices as u16,
                    metrics_index,
                    vertices_index : vertices_index as u16,
                };
                Ok(*ve.insert(arrow_indices))
            }
        }
    }

    // Edge shader takes the convex hull as a list of distances from center (to decide how far to travel along path)
    pub fn add_glyph_hull<It : ExactSizeIterator<Item = Vector>>(&mut self, convex_hull : It){
        self.glyph_convex_hulls.append(convex_hull.map(|v| v.length()));
    }


    pub fn add_edge(&mut self, 
        start : GlyphInstance, 
        end : GlyphInstance, 
        start_glyph_id : usize,
        end_glyph_id : usize,
        options : &EdgeOptions,
        // start_tip : Option<&Arrow>, end_tip : Option<&Arrow>,
        // angle : Angle,
        // thickness : f32,
        // dash_pattern : &[u8],
    ) -> Result<(), JsValue> {
        let start_arrow = options.start_tip.as_ref().map(|tip| self.arrow_tip_data(tip)).unwrap_or_else(|| Ok(Default::default()))?;
        let end_arrow = options.end_tip.as_ref().map(|tip| self.arrow_tip_data(tip)).unwrap_or_else(|| Ok(Default::default()))?;
        let start_glyph_idx = start_glyph_id as u16;
        let end_glyph_idx = end_glyph_id as u16;
        let (dash_index, dash_length) = self.dash_data(options.dash_pattern.to_vec())?;

        let edge_num_vertices = if options.angle == Angle::zero() { 6 } else { 12 };
        self.max_vertices = self.max_vertices.max((start_arrow.num_vertices + end_arrow.num_vertices + edge_num_vertices) as i32);

        self.ready = false;
        self.edge_instances.push(EdgeInstance {
            color : options.color,
            start_position : start.position,
            start_offset : start.offset,
            end_position : end.position,
            end_offset : end.offset,
            start_glyph : start_glyph_idx,
            end_glyph : end_glyph_idx,
            start_glyph_scale : start.scale,
            end_glyph_scale : end.scale,
            angle : options.angle.radians,
            thickness : options.thickness,

            start_arrow,
            end_arrow,

            dash_length,
            dash_index,
            dash_offset : 0,
            dash_padding : 0,
        });
        Ok(())
    }


    pub fn draw(&mut self) -> Result<(), JsValue> {
        if self.edge_instances.is_empty() {
            return Ok(());
        }
        self.program.use_program();
        self.edge_instances.prepare();
        self.glyph_convex_hulls.bind(GLYPH_HULL_TEXTURE_UNIT)?;
        self.arrow_metrics_data.bind(ARROW_METRICS_TEXTURE_UNIT)?;
        self.arrow_path_data.bind(ARROW_PATHS_TEXTURE_UNIT)?;
        self.bind_dash_data(DASH_PATTERNS_TEXTURE_UNIT)?;

        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        
        self.webgl.draw_arrays_instanced(
            WebGl2RenderingContext::TRIANGLES,
            0,
            self.max_vertices,
            self.edge_instances.len() as i32
        );
        Ok(())
    }
}