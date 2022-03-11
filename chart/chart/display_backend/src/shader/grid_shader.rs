#[allow(unused_imports)]
use crate::log;

use crate::vector::{Vec4};
use lyon::geom::math::{Point, vector};
use crate::webgl_wrapper::WebGlWrapper;
use crate::shader::Program;

use wasm_bindgen::JsValue;
use web_sys::WebGl2RenderingContext;
use crate::coordinate_system::CoordinateSystem;


pub struct GridShader {
    pub program : Program,
    x_grid_step : i32,
    y_grid_step : i32,
    color : Vec4,
    thickness : f32,
    offsets : Point,
    ready : bool,
}


impl GridShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let program = Program::new(
            webgl,
            // vertexShader :
            include_str!("grid.vert"),
            // fragmentShader :
            r#"#version 300 es
                precision highp float;
                uniform vec4 uColor;
                out vec4 outColor;
                void main() {
                    outColor = uColor;
                }
            "#
        )?;
        Ok(Self {
            program,
            x_grid_step : 1,
            y_grid_step : 1,
            color : Vec4::new(0.0, 0.0, 0.0, 1.0),
            thickness : 1.0,
            offsets : Point::new(0.0, 0.0),
            ready : false,
        })
    }
    // uniform mat3x2 uTransformationMatrix;
    // uniform vec2 uOrigin;
    // uniform vec2 uScale;
    // uniform ivec4 uChartRange; // (xmin, xmax, ymin, ymax)
    // uniform vec4 uScreenRange; // (xmin, xmax, ymin, ymax)
    // uniform ivec2 uGridStep; // (xGridStep, yGridStep)
    // uniform vec2 uGridOffset; // (xGridStep, yGridStep)

    pub fn grid_step(&mut self, x_grid_step : i32, y_grid_step : i32){
        self.x_grid_step = x_grid_step;
        self.y_grid_step = y_grid_step;
        self.ready = false;
    }

    #[allow(dead_code)]
    pub fn grid_offsets(&mut self, x_grid_offset : f32, y_grid_offset : f32){
        self.offsets = Point::new(x_grid_offset, y_grid_offset);
        self.ready = false;
    }

    pub fn color(&mut self, color : Vec4) {
        self.color = color;
        self.ready = false;
    }

    pub fn thickness(&mut self, thickness : f32){
        self.thickness = thickness;
        self.ready = false;
    }

    fn prepare(&mut self){
        if self.ready {
            return;
        }
        self.program.set_uniform_float("uThickness", self.thickness);
        self.program.set_uniform_vec4("uColor", self.color);
        self.program.set_uniform_point("uGridOffset", self.offsets);
        let loc = self.program.webgl.get_uniform_location(&self.program.program, "uGridStep");
        self.program.webgl.uniform2iv_with_i32_array(loc.as_ref(), &[self.x_grid_step, self.y_grid_step]);
        self.ready = true;
    }


    pub fn draw(&mut self, coordinate_system : CoordinateSystem) -> Result<(), JsValue> {
        self.program.use_program();
        self.prepare();
        self.program.set_uniform_transform("uTransformationMatrix", coordinate_system.transform);
        self.program.set_uniform_point("uOrigin", coordinate_system.origin);
        self.program.set_uniform_vector("uScale", coordinate_system.scale);

        let [mut chart_x_min, mut chart_y_min] : [i32; 2] = (coordinate_system.current_min_xy().floor() - vector(1.0, 1.0)).cast().to_array();
        let [chart_x_max, chart_y_max] : [i32; 2] = (coordinate_system.current_max_xy().ceil() + vector(1.0, 1.0)).cast().to_array();

        chart_x_min = (chart_x_min/self.x_grid_step) * self.x_grid_step;
        chart_y_min = (chart_y_min/self.y_grid_step) * self.x_grid_step;

        let (screen_x_min, screen_x_max) = coordinate_system.screen_x_range();
        let (screen_y_min, screen_y_max) = coordinate_system.screen_y_range();

        let num_vertical_grid_lines = (chart_x_max - chart_x_min + self.x_grid_step - 1) / self.x_grid_step + 1;
        let num_horizontal_grid_lines = (chart_y_max - chart_y_min + self.y_grid_step - 1) / self.y_grid_step + 1;
        let loc = self.program.webgl.get_uniform_location(&self.program.program, "uChartRange");
        self.program.webgl.uniform4iv_with_i32_array(loc.as_ref(), &[chart_x_min, chart_x_max, chart_y_min, chart_y_max]);
        self.program.set_uniform_vec4("uScreenRange", Vec4::new(screen_x_min, screen_x_max, screen_y_min, screen_y_max));

        // Without this check, it seems to freeze the computer when you zoom out very far.
        if num_vertical_grid_lines + num_horizontal_grid_lines > 10_000 {
            return Err("Scale too small!".into());
        }

        self.program.webgl.draw_arrays_instanced(
            WebGl2RenderingContext::TRIANGLES,
            0,
            6,
            num_vertical_grid_lines + num_horizontal_grid_lines
        );
        Ok(())
    }
}
