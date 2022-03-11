#[allow(unused_imports)]
use create::log;
use create::error::convert_tessellation_error;

use lazy_static::lazy_static;
use arrayvec::ArrayVec;

use std::rc::Rc;
use uuid::Uuid;

use wasm_bindgen::prelude::*;
use euclid::default::Box2D;
use footile::{Pt, PathOp, Path2D};
use fonterator::{self as font, Font}; // For parsing font file.
use lyon::geom::math::{point, Point, vector, Vector, Angle, Transform};
use lyon::path::{Path, PathEvent, iterator::PathIterator};
use lyon::tessellation::{
    geometry_builder,
    StrokeTessellator, StrokeOptions,
    FillTessellator, FillOptions, VertexBuffers
};



use create::vector::{Vec4};
use create::convex_hull::{ConvexHull};
#[allow(unused_imports)]
use create::stroke_tessellation::{PositionNormal, PositionNormalConstructor};

const FONT_SIZE: f32 = 32.0;
const SCALE_FACTOR : f32 = 100.0; // TODO: what/why is SCALE_FACTOR? What are the units here?

lazy_static!{
    static ref STIX_FONT : Font<'static> = {
        font::Font::new().push(include_bytes!("../fonts/STIX2Math.otf") as &[u8]).expect("Failed to parse font file")
    };
}



fn pt_to_euclid(p : Pt) -> Point {
    point(p.0, p.1)
}

fn euclid_pt_to_footile_pt(p : Point) -> Pt {
    Pt(p.x, p.y)
}

fn pathop_bounding_box<'a, T : Iterator<Item=&'a PathOp>>(path : T) -> Box2D<f32> {
    Box2D::from_points(path.flat_map(|path_op|{
        let mut result = ArrayVec::<[_; 3]>::new();
        match path_op {
            PathOp::Close() => {},
            PathOp::Move(to) => result.push(pt_to_euclid(*to)),
            PathOp::Line(to) => result.push(pt_to_euclid(*to)),
            PathOp::Quad(ctrl, to) => {
                result.push(pt_to_euclid(*ctrl));
                result.push(pt_to_euclid(*to));
            }
            PathOp::Cubic(ctrl1, ctrl2, to) =>{
                result.push(pt_to_euclid(*ctrl1));
                result.push(pt_to_euclid(*ctrl2));
                result.push(pt_to_euclid(*to));
            }
            PathOp::PenWidth(_) => {}
        };
        result.into_iter()
    }))
}

fn footile_path_to_lyon_path<T : Iterator<Item=PathOp>>(path : T) -> impl Iterator<Item=PathEvent> {
    let mut first = point(0.0, 0.0);
    let mut from = point(0.0, 0.0);
    path.filter_map(move |path_op| {
        let result; //= None;
        match path_op {
            PathOp::Close() => {
                result = Some(PathEvent::End { last : from, first, close : true});
            }
            PathOp::Move(to) => {
                let to = pt_to_euclid(to);
                result = Some(PathEvent::Begin { at : to });
                first = to;
                from = to;
            }
            PathOp::Line(to) => {
                let to = pt_to_euclid(to);
                result = Some(PathEvent::Line { from, to });
                from = to;
            }
            PathOp::Quad(ctrl, to) => {
                let ctrl = pt_to_euclid(ctrl);
                let to = pt_to_euclid(to);
                result = Some(PathEvent::Quadratic { from, ctrl, to });
                from = to;
            }
            PathOp::Cubic(ctrl1, ctrl2, to) => {
                let ctrl1 = pt_to_euclid(ctrl1);
                let ctrl2 = pt_to_euclid(ctrl2);
                let to = pt_to_euclid(to);
                result = Some(PathEvent::Cubic { from, ctrl1, ctrl2, to });
                from = to;
            }
            PathOp::PenWidth(_) => {unimplemented!()}
        }
        result
    })
}

fn scale_lyon_path<T : Iterator<Item=PathEvent>>(path : T, scale : f32) -> impl Iterator<Item=PathEvent>{
    path.map(move |event| event.transformed(&Transform::scale(scale, scale)))
}

fn lyon_path_to_footile_path<T : Iterator<Item=PathEvent>>(path : T) -> Vec<PathOp> {
    path.filter_map(move |path_event| {
        match path_event {
            PathEvent::End { close : false, ..} => {
                None
            }
            PathEvent::End { close : true, ..} => {
                Some(PathOp::Close())
            }
            PathEvent::Begin { at : to } => {
                let to = euclid_pt_to_footile_pt(to);
                Some(PathOp::Move(to))
            }
            PathEvent::Line { to, .. } => {
                let to = euclid_pt_to_footile_pt(to);
                Some(PathOp::Line(to))
            }
            PathEvent::Quadratic { ctrl, to, .. } => {
                let ctrl = euclid_pt_to_footile_pt(ctrl);
                let to = euclid_pt_to_footile_pt(to);
                Some(PathOp::Quad(ctrl, to))
            }
            PathEvent::Cubic { ctrl1, ctrl2, to, .. } => {
                let ctrl1 = euclid_pt_to_footile_pt(ctrl1);
                let ctrl2 = euclid_pt_to_footile_pt(ctrl2);
                let to = euclid_pt_to_footile_pt(to);
                Some(PathOp::Cubic(ctrl1, ctrl2, to))
            }
        }
    }).collect()
}

#[derive(Copy, Clone, Debug)]
enum PathType {
    Foreground,
    #[allow(dead_code)]
    Background,
    Boundary,
    BackgroundAndBoundary
}

#[derive(Clone, Debug)]
struct GlyphComponent {
    path : Vec<PathEvent>,
    path_type : PathType // Which of the parts of the glyph should this contribute to?
}


#[wasm_bindgen]
pub struct GlyphBuilder {
    paths : Vec<GlyphComponent>, // List of paths. The last one is assumed to determine the convex hull
    bounding_box : Box2D<f32>,
}


#[wasm_bindgen]
impl GlyphBuilder {
    pub fn from_stix(character : &str, scale : f32, whole_shape : bool) -> Self {
        let path : Vec<_> = STIX_FONT.render(
            character,
            (512.0 - 64.0) / FONT_SIZE, // What the heck is this?
            font::TextAlign::Center
        ).0.collect();
        let bounding_box = pathop_bounding_box(path.iter()).scale(scale, scale);
        let component = GlyphComponent {
            path : scale_lyon_path(footile_path_to_lyon_path(path.iter().copied()), scale).collect(),
            path_type : if whole_shape { PathType::BackgroundAndBoundary } else { PathType::Foreground }
        };
        Self {
            paths : vec![component],
            bounding_box,
        }
    }

    pub fn empty() -> Self {
        Self {
            paths : vec![],
            bounding_box : Box2D::new(point(0.0, 0.0), point(0.0, 0.0)),
        }
    }

    pub fn boxed(&mut self, padding : f32, include_background : bool ) {
        self.bounding_box = self.bounding_box.inflate(padding, padding);
        let Point { x : xmin, y : ymin, ..} = self.bounding_box.min;
        let Point { x : xmax, y : ymax, ..} = self.bounding_box.max;
        let box_path = Path2D::default().absolute()
            .move_to(xmin, ymin)
            .line_to(xmax, ymin)
            .line_to(xmax, ymax)
            .line_to(xmin, ymax)
            .close().finish();
        let component = GlyphComponent {
            path : footile_path_to_lyon_path(box_path.iter().copied()).collect(),
            path_type : if include_background { PathType::BackgroundAndBoundary } else { PathType::Boundary },
        };
        self.paths.push(component);
    }

    pub fn circled(&mut self, padding : f32, num_circles : i32, circle_gap : f32, include_background : bool) {
        let bounding_box = self.bounding_box.inflate(padding, padding);
        let radius = bounding_box.min.distance_to(bounding_box.max)/2.0;
        let center = bounding_box.min.lerp(bounding_box.max, 0.5);
        let max_radius = radius + (num_circles as f32) * circle_gap;
        self.bounding_box = Box2D::new(center, center).inflate(max_radius, max_radius);
        let mut circle_path = Path::builder();
        circle_path.move_to(center - vector(radius, 0.0));
        circle_path.arc(center, vector(radius, radius), Angle::two_pi(), Angle::zero());
        circle_path.close();
        let circle_path : Vec<_> = circle_path.build().iter().collect();
        let component = GlyphComponent {
            path : circle_path,
            path_type : if include_background { PathType::BackgroundAndBoundary } else { PathType::Boundary },
        };
        self.paths.push(component);
        for i in 1..num_circles {
            let radius = radius + (i as f32) * circle_gap;
            let mut circle_path = Path::builder();
            circle_path.move_to(center - vector(radius, 0.0));
            circle_path.arc(center, vector(radius, radius), Angle::two_pi(), Angle::zero());
            circle_path.close();
            let circle_path : Vec<_> = circle_path.build().iter().collect();
            let component = GlyphComponent {
                path : circle_path,
                path_type : PathType::Boundary,
            };
            self.paths.push(component);
        }
    }

    pub fn build(self, tolerance : f32, line_width : f32) -> Glyph {
        let GlyphBuilder { paths, bounding_box } = self;
        let convex_hull = Rc::new(ConvexHull::from_path(
            lyon_path_to_footile_path(paths.last().unwrap().path.iter().copied()),
            bounding_box
        ));
        let paths = Rc::new(paths);
        Glyph {
            paths,
            convex_hull,
            tolerance,
            line_width,
            max_scale : SCALE_FACTOR,
            uuid : GlyphUuid(Uuid::new_v4())
        }
    }
}

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct GlyphUuid(Uuid);

#[wasm_bindgen]
#[derive(Clone)]
pub struct Glyph {
    paths : Rc<Vec<GlyphComponent>>,
    convex_hull : Rc<ConvexHull>,
    tolerance : f32,
    line_width : f32,
    max_scale : f32,
    pub(create) uuid : GlyphUuid
}

impl Glyph {
    // pub fn width_scale(&self) -> f32 {
    //     SCALE_FACTOR / self.
    // }

    pub(create) fn tessellate_background(&self, buffers : &mut VertexBuffers<Point, u16>) -> Result<(), JsValue> {
        let mut vertex_builder = geometry_builder::simple_builder(buffers);
        let mut fill_tessellator = FillTessellator::new();
        let options = FillOptions::default().with_tolerance(self.tolerance / self.max_scale);
        let transform = Transform::identity().then_translate(- self.convex_hull.center().to_vector());
        for &GlyphComponent { ref path, path_type } in &*self.paths {
            if let PathType::Background | PathType::BackgroundAndBoundary = path_type {
                let path = path.iter().copied().transformed(&transform);
                fill_tessellator.tessellate(path, &options, &mut vertex_builder).map_err(convert_tessellation_error)?;
            }
        }
        Ok(())
    }

    pub(create) fn tessellate_boundary(&self, buffers : &mut VertexBuffers<Point /*PositionNormal*/, u16>,) -> Result<(), JsValue> {
        // let mut vertex_builder = geometry_builder::BuffersBuilder::new(buffers, PositionNormalConstructor {});
        let mut vertex_builder = geometry_builder::simple_builder(buffers);
        let mut stroke_tessellator = StrokeTessellator::new();
        let options = StrokeOptions::default().with_line_width(self.line_width).with_tolerance(self.tolerance / self.max_scale);
        let transform = Transform::identity().then_translate(- self.convex_hull.center().to_vector());
        for &GlyphComponent { ref path, path_type} in &*self.paths {
            if let PathType::Boundary | PathType::BackgroundAndBoundary = path_type {
                let path = path.iter().copied().transformed(&transform);
                stroke_tessellator.tessellate(path, &options, &mut vertex_builder).map_err(convert_tessellation_error)?;
            }
        }
        Ok(())
    }

    pub(create) fn tessellate_foreground(&self, buffers : &mut VertexBuffers<Point, u16>) -> Result<(), JsValue> {
        let mut vertex_builder = geometry_builder::simple_builder(buffers);
        let mut fill_tessellator = FillTessellator::new();
        let options = FillOptions::default().with_tolerance(self.tolerance / self.max_scale);
        let transform = Transform::identity().then_translate(- self.convex_hull.center().to_vector());
        for &GlyphComponent { ref path, path_type } in &*self.paths {
            if let PathType::Foreground = path_type {
                let path = path.iter().copied().transformed(&transform);
                fill_tessellator.tessellate(path, &options, &mut vertex_builder).map_err(convert_tessellation_error)?;
            }
        }
        Ok(())
    }

    pub(create) fn boundary(&self) -> &Vec<Vector> {
        &self.convex_hull.outline
    }
}


#[wasm_bindgen]
#[derive(Clone)]
pub struct GlyphInstance {
    pub(create) glyph : Glyph,
    pub(create) position : Point,
    pub(create) offset : Vector,
    pub(create) scale : f32,
    pub(create) background_color : Vec4,
    pub(create) border_color : Vec4,
    pub(create) foreground_color : Vec4,
}

#[wasm_bindgen]
impl GlyphInstance {
    pub fn inner_radius(&self) -> f32 {
        self.glyph.convex_hull.inner_radius
    }

    pub fn outer_radius(&self) -> f32 {
        self.glyph.convex_hull.outer_radius
    }
}


impl GlyphInstance {
    pub fn new(
        glyph : Glyph,
        position : Point,
        offset : Vector,
        scale : f32,
        background_color : Vec4,
        border_color : Vec4,
        foreground_color : Vec4
    ) -> Self {
        Self {
            glyph,
            position,
            offset,
            scale,
            background_color,
            border_color,
            foreground_color,
        }
    }
}
