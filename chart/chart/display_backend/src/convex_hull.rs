// Stolen from: https://github.com/jobtalle/ConvexHull/tree/master/src/convexHull
// use crate::log;

use std::{borrow::Borrow, cmp::Ordering};

use euclid::default::Box2D;
use footile::{FillRule, Path2D, PathOp, Plotter, Transform};
use lyon::geom::math::{Angle, Point, Vector};
use pix::{chan::Channel, el::Pixel, matte::Matte8, Raster};

// Must be the same as the angle_resolution constants defined in edge.vert and hit_canvas.vert.
pub const ANGLE_RESOLUTION: usize = 180;
const RASTER_SCALE_FACTOR: f32 = 100.0;

fn raster_midpoint<P: Pixel>(raster: &Raster<P>) -> Point {
    Point::new((raster.width() / 2) as f32, (raster.height() / 2) as f32)
}

fn raster_contains_point<P: Pixel>(raster: &Raster<P>, point: Point) -> bool {
    raster.pixel(point.x as i32, point.y as i32).alpha() != P::Chan::MIN
}

fn raster_to_convex_hull_polygon<P: Pixel>(raster: &Raster<P>, precision: f32) -> Vec<Vector> {
    let mut convex_hull = sample_raster_outline(raster, Point::origin(), ANGLE_RESOLUTION);
    average_nearby_points(&mut convex_hull, precision);
    graham_scan(&mut convex_hull);
    // convex_hull.shrink_to_fit();
    convex_hull
}

fn scan_ray_for_nontransparent_pixel<P: Pixel>(
    raster: &Raster<P>,
    start_position: Point,
    direction: Vector,
    radius: i32,
) -> Point {
    // Scan for pixel with nonzero value on color channel "channel"
    for i in 0..radius {
        let current_position = start_position - direction * i as f32;
        // Check channel
        if raster_contains_point(raster, current_position) {
            return current_position;
        }
    }
    start_position - direction * radius as f32
}

fn sample_raster_outline<P: Pixel>(
    raster: &Raster<P>,
    pivot: Point,
    point_count: usize,
) -> Vec<Vector> {
    let angle_step = Angle::two_pi() / (point_count as f32);
    let half_dim = raster_midpoint(raster);

    let mut result = Vec::with_capacity(point_count);
    for i in 0..point_count {
        let angle = angle_step * (i as f32);
        let direction = Vector::from_angle_and_length(angle, 1.0);

        // Create edge points
        let abscos = direction.x.abs();
        let abssin = direction.y.abs();

        let radius = f32::min(half_dim.x / abscos, half_dim.y / abssin) - 1.0;
        let position = scan_ray_for_nontransparent_pixel(
            raster,
            half_dim + direction * radius,
            direction,
            f32::ceil(radius) as i32,
        );

        result.push(position - pivot);
    }
    result
}

// Average together collections of nearby points. In place.
fn average_nearby_points(points: &mut Vec<Vector>, trim_distance: f32) {
    let mut input_idx = 0;
    let mut output_idx = 0;
    while input_idx < points.len() {
        // Average the current point with as many later points as are closer than trim_distance to it.
        let current = points[input_idx];
        let (total, num_points) = points[input_idx + 1..]
            .iter()
            .take_while(|&&p| (p - current).length() < trim_distance)
            .fold((current, 1), |(total, num_points), &point| {
                (total + point, num_points + 1)
            });
        let average = total / (num_points as f32);
        // Put new average into input list
        points[output_idx] = average;
        output_idx += 1;
        input_idx += num_points;
    }
    // Shrink list to new length (panic if somehow output_idx > points.len())
    points.resize_with(output_idx, || unreachable!());
}

fn orientation(p: Vector, q: Vector, r: Vector) -> f32 {
    Vector::cross(r - q, q - p)
}

fn compare_magnitudes(p: Vector, q: Vector) -> Ordering {
    p.length().partial_cmp(&q.length()).unwrap()
}

#[derive(PartialEq, PartialOrd)]
struct NonNan(f32);

impl NonNan {
    fn new(val: f32) -> Option<NonNan> {
        if val.is_nan() {
            None
        } else {
            Some(NonNan(val))
        }
    }
}

impl Eq for NonNan {}

impl Ord for NonNan {
    fn cmp(&self, other: &NonNan) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

fn graham_scan(points: &mut Vec<Vector>) {
    // Find minimum Y
    let (min_idx, _) = points
        .iter()
        .enumerate()
        .min_by_key(|(_idx, v)| NonNan::new(v.y).unwrap())
        .unwrap();

    // Put minimum at zero
    points.swap(0, min_idx);

    let compare_point = points[0];
    points.sort_by(
		move |&p1, &p2| // sort first by the handedness of (compare_point, p1, p2) then by distance from compare_pt.
		orientation(compare_point, p1, p2).partial_cmp(&0.0).unwrap().then_with(|| compare_magnitudes(p1 - compare_point, p2 - compare_point))
	);

    // Create & initialize stack
    let mut stack_length: usize = 3;
    for i in 3..points.len() {
        // Seems like this could lead to an infinite loop here...
        // Luckily, Rust will panic if stack_index becomes less than 2
        while orientation(
            points[stack_length - 2],
            points[stack_length - 1],
            points[i],
        ) >= 0.0
        {
            stack_length -= 1;
        }
        points[stack_length] = points[i];
        stack_length += 1;
    }
    // Shrink list to new length (panic if somehow output_idx > points.len())
    points.resize_with(stack_length, || unreachable!());
}

fn rasterize_polygon(polygon: &[Vector], width: u32, height: u32) -> Raster<Matte8> {
    let mut path_builder = Path2D::default();
    path_builder = path_builder.absolute().move_to(polygon[0].x, polygon[0].y);
    for v in &polygon[1..] {
        path_builder = path_builder.line_to(v.x, v.y);
    }
    let path = path_builder.close().finish();
    let mut p = Plotter::new(Raster::<Matte8>::with_clear(width, height));
    p.fill(FillRule::NonZero, path.iter(), Matte8::new(255));
    p.raster()
}

pub struct ConvexHull {
    pub outline: Vec<Vector>,
    pub inner_radius: f32,
    pub outer_radius: f32,
    bounding_box: Box2D<f32>,
}

impl ConvexHull {
    pub fn from_path<T>(path: T, bounding_box: Box2D<f32>) -> Self
    where
        T: IntoIterator,
        T::Item: Borrow<PathOp>,
    {
        let width_and_height = bounding_box.max - bounding_box.min;
        let raster_scale = RASTER_SCALE_FACTOR / f32::min(width_and_height.x, width_and_height.y);
        let width_and_height = (width_and_height * raster_scale).ceil();
        let width = width_and_height.x as u32;
        let height = width_and_height.y as u32;

        let transform = Transform::with_translate(-bounding_box.min.x, -bounding_box.min.y)
            .scale(raster_scale, raster_scale);

        let mut p = Plotter::new(Raster::<Matte8>::with_clear(width, height));
        p.set_transform(transform)
            .fill(FillRule::NonZero, path, Matte8::new(255));
        let raster = p.raster();
        let polygon = raster_to_convex_hull_polygon(&raster, 0.1);
        let convex_raster = rasterize_polygon(&polygon, width, height);
        let mut outline = sample_raster_outline(
            &convex_raster,
            raster_midpoint(&convex_raster),
            ANGLE_RESOLUTION,
        );
        for v in &mut outline {
            *v /= raster_scale;
        }
        let inner_radius = outline
            .iter()
            .map(|p| NonNan::new(p.length()))
            .min()
            .unwrap()
            .unwrap()
            .0;
        let outer_radius = outline
            .iter()
            .map(|p| NonNan::new(p.length()))
            .max()
            .unwrap()
            .unwrap()
            .0;
        Self {
            outline,
            bounding_box,
            inner_radius,
            outer_radius,
        }
    }

    #[allow(dead_code)]
    pub fn find_boundary_point(&self, angle: Angle) -> Vector {
        let index = ((ANGLE_RESOLUTION as f32) * (angle.positive() / Angle::two_pi())) as usize;
        self.outline[index]
    }

    pub fn center(&self) -> Point {
        self.bounding_box.max.lerp(self.bounding_box.min, 0.5)
    }
}
