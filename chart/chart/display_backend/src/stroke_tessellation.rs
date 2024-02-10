use lyon::tessellation as tess;
use tess::{
    math::{Point, Vector},
    StrokeAttributes, StrokeVertexConstructor,
};

#[derive(Copy, Clone, Debug)]
pub struct PositionNormal {
    pub position: Point,
    pub normal: Vector,
}

/// A simple vertex constructor that just takes the position.
pub struct PositionNormalConstructor;

impl StrokeVertexConstructor<PositionNormal> for PositionNormalConstructor {
    fn new_vertex(&mut self, position: Point, attributes: StrokeAttributes) -> PositionNormal {
        let normal = if attributes.side().is_right() {
            attributes.normal()
        } else {
            Vector::new(0.0, 0.0)
        };
        PositionNormal { position, normal }
    }
}
