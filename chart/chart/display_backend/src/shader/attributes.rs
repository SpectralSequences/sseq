use std::convert::{TryInto, TryFrom};
use web_sys::{WebGl2RenderingContext, WebGlVertexArrayObject, WebGlBuffer, WebGlProgram};
use wasm_bindgen::JsValue;

use crate::webgl_wrapper::WebGlWrapper;

#[derive(Copy, Clone, Debug)]
pub enum Type {
    F32,
    U32,
    I16, U16,
    U8,
}


impl Type { 
    fn size(self) -> i32 {
        match self {
            Type::F32 => std::mem::size_of::<f32>() as i32,
            Type::U32 => std::mem::size_of::<u32>() as i32,
            Type::I16 => std::mem::size_of::<i16>() as i32,
            Type::U16 => std::mem::size_of::<u16>() as i32,
            Type::U8 => std::mem::size_of::<u8>() as i32,
        }
    }

    pub fn alignment(self) -> usize {
        match self {
            Type::F32 => std::mem::align_of::<f32>(),
            Type::U32 => std::mem::align_of::<u32>(),
            Type::I16 => std::mem::align_of::<i16>(),
            Type::U16 => std::mem::align_of::<u16>(),
            Type::U8 => std::mem::align_of::<u8>(),
        }
    }

    fn webgl_type(self) -> u32 {
        match self {
            Type::F32 => WebGl2RenderingContext::FLOAT,
            Type::U32 => WebGl2RenderingContext::UNSIGNED_INT,
            Type::I16 => WebGl2RenderingContext::SHORT,
            Type::U16 => WebGl2RenderingContext::UNSIGNED_SHORT,
            Type::U8 => WebGl2RenderingContext::UNSIGNED_BYTE,
        }
    }
}


#[derive(Copy, Clone, Debug)]
pub enum NumChannels {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
}

impl NumChannels {
    pub fn base_format(self) -> u32 {
        match self {
            NumChannels::One => WebGl2RenderingContext::RED,
            NumChannels::Two => WebGl2RenderingContext::RG,
            NumChannels::Three => WebGl2RenderingContext::RGB,
            NumChannels::Four => WebGl2RenderingContext::RGBA,
        }
    }
}

impl TryFrom<i32> for NumChannels {
    type Error = ();

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            x if x == Self::One as i32 => Ok(Self::One),
            x if x == Self::Two as i32 => Ok(Self::Two),
            x if x == Self::Three as i32 => Ok(Self::Three),
            x if x == Self::Four as i32 => Ok(Self::Four),
            _ => Err(()),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Format(pub Type, pub NumChannels);

impl Format {
    pub fn internal_format(self) -> u32 {
        match self {
            Format(Type::F32, NumChannels::One) => WebGl2RenderingContext::R32F,
            Format(Type::F32, NumChannels::Two) => WebGl2RenderingContext::RG32F,
            Format(Type::F32, NumChannels::Three) => WebGl2RenderingContext::RGB32F,
            Format(Type::F32, NumChannels::Four) => WebGl2RenderingContext::RGBA32F,

            Format(Type::U32, NumChannels::One) => WebGl2RenderingContext::R32UI,
            Format(Type::U32, NumChannels::Two) => WebGl2RenderingContext::RG32UI,
            Format(Type::U32, NumChannels::Three) => WebGl2RenderingContext::RGB32UI,
            Format(Type::U32, NumChannels::Four) => WebGl2RenderingContext::RGBA32UI,


            Format(Type::I16, NumChannels::One) => WebGl2RenderingContext::R16I,
            Format(Type::I16, NumChannels::Two) => WebGl2RenderingContext::RG16I,
            Format(Type::I16, NumChannels::Three) => WebGl2RenderingContext::RGB16I,
            Format(Type::I16, NumChannels::Four) => WebGl2RenderingContext::RGBA16I,

            Format(Type::U16, NumChannels::One) => WebGl2RenderingContext::R16UI,
            Format(Type::U16, NumChannels::Two) => WebGl2RenderingContext::RG16UI,
            Format(Type::U16, NumChannels::Three) => WebGl2RenderingContext::RGB16UI,
            Format(Type::U16, NumChannels::Four) => WebGl2RenderingContext::RGBA16UI,

            Format(Type::U8, NumChannels::One) => WebGl2RenderingContext::R8UI,
            Format(Type::U8, NumChannels::Two) => WebGl2RenderingContext::RG8UI,
            Format(Type::U8, NumChannels::Three) => WebGl2RenderingContext::RGB8UI,
            Format(Type::U8, NumChannels::Four) => WebGl2RenderingContext::RGBA8UI,
        }
    }

    pub fn size(self) -> i32 {
        let Format(ty, size) = self;
        ty.size() * (size as i32)
    }

    pub fn webgl_type(self) -> u32 {
        self.0.webgl_type()
    }

    pub fn base_format(self) -> u32 {
        self.1.base_format()
    }
}


pub struct Attribute {
    name : &'static str, 
    size : usize, 
    ty : Type,
}

impl Attribute {
    pub const fn new(name : &'static str, size : usize, ty : Type) -> Self {
        Self {
            name, size, ty
        }
    }
}


pub struct Attributes {
    attributes : &'static [Attribute]
}

impl Attributes {
    pub const fn new(attributes : &'static [Attribute]) -> Self {
        Self {
            attributes
        }
    }

    fn offset(&self, idx : usize) -> i32 {
        self.attributes[..idx].iter().map(|&Attribute {size, ty, ..}|
            ty.size() * (size as i32)
        ).sum()
    }
    
    fn stride(&self) -> i32 {
        self.offset(self.attributes.len())
    }

    pub fn set_up_vertex_array(&self, webgl : &WebGlWrapper, program : &WebGlProgram, attribute_state : Option<&WebGlVertexArrayObject>, attributes_buffer : Option<&WebGlBuffer>) -> Result<(), JsValue> {
        webgl.bind_vertex_array(attribute_state);
        // IMPORTANT: Must bind_buffer here!!!!
        // vertex_attrib_pointer uses the current bound buffer implicitly.
        webgl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, attributes_buffer);
    
        let stride = self.stride();
        for (idx, &Attribute {name, size, ty}) in self.attributes.iter().enumerate() {
            let size = size as i32;
            let loc = webgl.get_attrib_location(program, name).try_into().map_err(|_| name.to_string())?;
            let offset = self.offset(idx);
            webgl.enable_vertex_attrib_array(loc);
            match ty {
                Type::F32 => {webgl.vertex_attrib_pointer_with_i32(loc, size, ty.webgl_type(), false, stride, offset)},
                Type::U32 | Type::I16 | Type::U16 | Type::U8 
                    => {webgl.vertex_attrib_i_pointer_with_i32(loc, size, ty.webgl_type(), stride, offset)}
            };
            webgl.vertex_attrib_divisor(loc, 1);
        }
        webgl.bind_vertex_array(None);
        Ok(())
    }
    
}

