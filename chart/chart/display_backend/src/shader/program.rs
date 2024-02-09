use lyon::geom::math::{Point, Transform, Vector};
use wasm_bindgen::JsValue;
use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlShader};

use crate::{vector::Vec4, webgl_wrapper::WebGlWrapper};

pub struct Program {
    pub webgl: WebGlWrapper,
    pub program: WebGlProgram,
}

impl Program {
    pub fn new(
        webgl: WebGlWrapper,
        vertex_shader: &str,
        fragment_shader: &str,
    ) -> Result<Self, JsValue> {
        let vert_shader =
            compile_shader(&webgl, WebGl2RenderingContext::VERTEX_SHADER, vertex_shader)?;
        let frag_shader = compile_shader(
            &webgl,
            WebGl2RenderingContext::FRAGMENT_SHADER,
            fragment_shader,
        )?;
        let program = link_program(&webgl, &vert_shader, &frag_shader)?;
        Ok(Program { webgl, program })
    }

    pub fn use_program(&self) {
        self.webgl.use_program(Some(&self.program));
    }

    pub fn set_uniform_float(&self, name: &str, x: f32) {
        let loc = self.webgl.get_uniform_location(&self.program, name);
        self.webgl.uniform1f(loc.as_ref(), x);
    }

    pub fn set_uniform_texture_unit(&self, name: &str, x: u32) {
        let loc = self.webgl.get_uniform_location(&self.program, name);
        self.webgl
            .uniform1iv_with_i32_array(loc.as_ref(), &[x as i32]);
    }

    pub fn set_uniform_point(&self, name: &str, v2: Point) {
        let loc = self.webgl.get_uniform_location(&self.program, name);
        self.webgl
            .uniform2fv_with_f32_array(loc.as_ref(), &v2.to_array());
    }

    pub fn set_uniform_vector(&self, name: &str, v2: Vector) {
        let loc = self.webgl.get_uniform_location(&self.program, name);
        self.webgl
            .uniform2fv_with_f32_array(loc.as_ref(), &v2.to_array());
    }

    pub fn set_uniform_vec4(&self, name: &str, v4: Vec4) {
        let loc = self.webgl.get_uniform_location(&self.program, name);
        self.webgl
            .uniform4fv_with_f32_array(loc.as_ref(), &[v4.x, v4.y, v4.z, v4.w]);
    }

    pub fn set_uniform_transform(&self, name: &str, transform: Transform) {
        let loc = self.webgl.get_uniform_location(&self.program, name);
        self.webgl
            .uniform_matrix3x2fv_with_f32_array(loc.as_ref(), false, &transform.to_array());
    }
}

fn compile_shader(
    webgl: &WebGlWrapper,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let webgl = &webgl.inner;
    let shader = webgl
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    webgl.shader_source(&shader, source);
    webgl.compile_shader(&shader);

    if webgl
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(webgl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

fn link_program(
    webgl: &WebGlWrapper,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let webgl = &webgl.inner;
    let program = webgl
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    webgl.attach_shader(&program, vert_shader);
    webgl.attach_shader(&program, frag_shader);
    webgl.link_program(&program);

    if webgl
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(webgl
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}
