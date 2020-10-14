#[allow(unused_imports)]
use crate::log; 
use crate::error::convert_tessellation_error;

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


use crate::vector::{Vec4};

use crate::convex_hull::ConvexHull;

const FONT_SIZE: f32 = 32.0;


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

fn footile_path_to_lyon_path<T : Iterator<Item=PathOp>>(path : T) -> Vec<PathEvent> {
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
    }).collect()
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

struct GlyphComponent {
    path : Vec<PathEvent>,
    stroke : Option<StrokeOptions>, 
    fill : Option<FillOptions>,
}


#[wasm_bindgen]
pub struct GlyphBuilder {
    paths : Vec<GlyphComponent>,
    bounding_box : Box2D<f32>,
    convex_hull_path_idx : usize
}


#[wasm_bindgen]
impl GlyphBuilder {
    pub fn from_stix(character : &str) -> Self {
        let path : Vec<_> = STIX_FONT.render(
            character,
            (512.0 - 64.0) / FONT_SIZE,
            font::TextAlign::Center
        ).0.collect();
        let bounding_box = pathop_bounding_box(path.iter());
        let component = GlyphComponent {
            path : footile_path_to_lyon_path(path.iter().copied()),
            stroke : Some(StrokeOptions::default().with_line_width(2.0).with_tolerance(0.2)),
            fill : Some(FillOptions::default().with_tolerance(0.2)),
        };
        Self {
            paths : vec![component],
            bounding_box,
            convex_hull_path_idx : 0,
        }
    }

    pub fn empty() -> Self {
        Self {
            paths : vec![],
            bounding_box : Box2D::new(point(0.0, 0.0), point(0.0, 0.0)),
            convex_hull_path_idx : 0,
        }
    }

    pub fn boxed(&mut self, padding : f32) {
        let padding = padding / 100.0;
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
            path : footile_path_to_lyon_path(box_path.iter().copied()),
            stroke : Some(StrokeOptions::default().with_line_width(4.0).with_tolerance(0.2)),
            fill : None,
        };        
        self.convex_hull_path_idx = self.paths.len();
        self.paths.push(component);
    }

    pub fn circled(&mut self, padding : f32) {
        let padding = padding / 100.0;
        let bounding_box = self.bounding_box.inflate(padding, padding);
        let radius = bounding_box.min.distance_to(bounding_box.max)/2.0;
        let center = bounding_box.min.lerp(bounding_box.max, 0.5);
        self.bounding_box = Box2D::new(center - vector(radius, radius), center + vector(radius, radius));
        let mut circle_path = Path::builder();
        circle_path.move_to(center - vector(radius, 0.0));
        circle_path.arc(center, vector(radius, radius), Angle::two_pi(), Angle::zero());
        circle_path.close();
        let circle_path : Vec<_> = circle_path.build().iter().collect();
        let component = GlyphComponent {
            path : circle_path,
            stroke : Some(StrokeOptions::default().with_line_width(4.0).with_tolerance(0.2)),
            fill : None,
        };            
        self.convex_hull_path_idx = self.paths.len();
        self.paths.push(component);
    }

    pub fn build(self) -> Glyph {
        let GlyphBuilder { paths, bounding_box, convex_hull_path_idx } = self;
        let convex_hull = Rc::new(ConvexHull::from_path(lyon_path_to_footile_path(paths[convex_hull_path_idx].path.iter().copied()), bounding_box));
        let paths = Rc::new(paths);
        Glyph { 
            paths,
            convex_hull,
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
    pub(crate) uuid : GlyphUuid
}

impl Glyph { 
    pub(crate) fn tessellate_fill(&self,
        buffers : &mut VertexBuffers<Point, u16>,
        scale : f32
    ) -> Result<(), JsValue> {
        let mut vertex_builder = geometry_builder::simple_builder(buffers);
        let mut fill_tessellator = FillTessellator::new();
        let transform = Transform::identity().then_translate(- self.convex_hull.center().to_vector()).then_scale(scale, scale);
        for &GlyphComponent { ref path, stroke : _stroke,  fill } in self.paths.iter() {
            if let Some(options) = fill {
                let path = path.iter().copied().transformed(&transform);
                fill_tessellator.tessellate(path, &options, &mut vertex_builder).map_err(convert_tessellation_error)?;
            }
        }        
        Ok(())
    }

    pub(crate) fn tessellate_stroke(&self,
        buffers : &mut VertexBuffers<Point, u16>,
        scale : f32
    ) -> Result<(), JsValue> {
        let mut vertex_builder = geometry_builder::simple_builder(buffers);
        let mut stroke_tessellator = StrokeTessellator::new();
        let transform = Transform::identity().then_translate(- self.convex_hull.center().to_vector()).then_scale(scale, scale);
        for &GlyphComponent { ref path, stroke, fill : _fill } in &*self.paths {
            if let Some(options) = stroke {
                let path = path.iter().copied().transformed(&transform);
                stroke_tessellator.tessellate(path, &options, &mut vertex_builder).map_err(convert_tessellation_error)?;
            }
        }
        Ok(())
    }

    pub(crate) fn boundary(&self) -> &Vec<Vector> {
        &self.convex_hull.outline
    }
}


#[wasm_bindgen]
#[derive(Clone)]
pub struct GlyphInstance {
    pub(crate) glyph : Glyph,
    pub(crate) position : Point,
    pub(crate) offset : Vector,
    pub(crate) scale : f32,
    pub(crate) stroke_color : Vec4,
    pub(crate) fill_color : Vec4,
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
        stroke_color : Vec4, 
        fill_color : Vec4
    ) -> Self {
        Self {
            glyph,
            position,
            offset,
            scale,
            stroke_color,
            fill_color,
        }
    }
}