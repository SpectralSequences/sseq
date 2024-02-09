use std::convert::From;

use lyon::geom::math::{Point, Vector};
use wasm_bindgen::prelude::*;

// use derive_more::{From, Add, Sub, Mul, Div, AddAssign, SubAssign, MulAssign, DivAssign, Sum};

#[wasm_bindgen(inspectable)]
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct JsPoint {
    pub x: f32,
    pub y: f32,
}

impl From<(f32, f32)> for JsPoint {
    fn from((px, py): (f32, f32)) -> Self {
        Self::new(px, py)
    }
}

impl From<JsPoint> for Point {
    fn from(p: JsPoint) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<JsPoint> for Vector {
    fn from(p: JsPoint) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<Point> for JsPoint {
    fn from(p: Point) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<Vector> for JsPoint {
    fn from(p: Vector) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<&Point> for JsPoint {
    fn from(p: &Point) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<&Vector> for JsPoint {
    fn from(p: &Vector) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<&JsPoint> for Point {
    fn from(p: &JsPoint) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<&JsPoint> for Vector {
    fn from(p: &JsPoint) -> Self {
        Self::new(p.x, p.y)
    }
}

#[wasm_bindgen]
impl JsPoint {
    #[wasm_bindgen(constructor)]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[wasm_bindgen(inspectable)]
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[wasm_bindgen]
impl Vec4 {
    #[wasm_bindgen(constructor)]
    pub fn new_js(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self::new(x, y, z, w)
    }
}

impl Vec4 {
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}
