use std::fmt::Display;

use crate::coordinates::{Bidegree, BidegreeGenerator};

pub mod svg;
pub mod tikz;

pub use svg::SvgBackend;
pub use tikz::TikzBackend;

#[rustfmt::skip]
const PATTERNS: [(f32, &[(f32, f32)]); 12] = [
    (2.0, &[(0.0, 0.0)]),
    (2.0, &[(-3.0, 0.0), (3.0, 0.0)]),
    (2.0, &[(-3.0, 2.58), (3.0, 2.58), (0.0, -2.58)]),
    (2.0, &[(-3.0, 3.0), (3.0, 3.0), (-3.0, -3.0), (3.0, -3.0)]),
    (1.5, &[(-3.0, 3.0), (3.0, 3.0), (0.0, 0.0), (-3.0, -3.0), (3.0, -3.0)]),
    (1.5, &[(-2.0, 4.0), (2.0, 4.0), (-2.0, 0.0), (2.0, 0.0), (-2.0, -4.0), (2.0, -4.0)]),
    (1.5, &[(-2.0, 4.0), (2.0, 4.0), (-4.0, 0.0), (0.0, 0.0), (4.0, 0.0), (-2.0, -4.0), (2.0, -4.0)]),
    (1.5, &[(-4.0, 4.0), (0.0, 4.0), (4.0, 4.0), (-4.0, 0.0), (0.0, 0.0), (4.0, 0.0), (-2.0, -4.0), (2.0, -4.0)]),
    (1.5, &[(-4.0, 4.0), (0.0, 4.0), (4.0, 4.0), (-4.0, 0.0), (0.0, 0.0), (4.0, 0.0), (-4.0, -4.0), (0.0, -4.0), (4.0, -4.0)]),
    (1.5, &[(-4.0, 4.0), (0.0, 4.0), (4.0, 4.0), (-4.0, 0.0), (-1.3, 0.0), (1.3, 0.0), (4.0, 0.0), (-4.0, -4.0), (0.0, -4.0), (4.0, -4.0)]),
    (1.5, &[(-4.0, 4.0), (-1.3, 4.0), (1.3, 4.0), (4.0, 4.0), (-4.0, 0.0), (-1.3, 0.0), (1.3, 0.0), (4.0, 0.0), (-4.0, -4.0), (0.0, -4.0), (4.0, -4.0)]),
    (1.5, &[(-4.0, 4.0), (-1.3, 4.0), (1.3, 4.0), (4.0, 4.0), (-4.0, 0.0), (-1.3, 0.0), (1.3, 0.0), (4.0, 0.0), (-4.0, -4.0), (-1.3, -4.0), (1.3, -4.0), (4.0, -4.0)]),
];

pub enum Orientation {
    Left,
    Right,
    Above,
    Below,
}

pub trait Backend {
    type Error;

    /// If the backend writes to a file, this is the extension commonly taken by the file type
    const EXT: &'static str = "";

    fn header(&mut self, max: Bidegree) -> Result<(), Self::Error>;
    fn line(&mut self, start: Bidegree, end: Bidegree, style: &str) -> Result<(), Self::Error>;

    fn text(
        &mut self,
        b: Bidegree,
        content: impl Display,
        orientation: Orientation,
    ) -> Result<(), Self::Error>;

    // We don't use BidegreeGenerator here because `n` represents the order of a bidegree instead of
    // an index of an element within a bidegree
    fn node(&mut self, b: Bidegree, n: usize) -> Result<(), Self::Error>;

    fn structline(
        &mut self,
        source: BidegreeGenerator,
        target: BidegreeGenerator,
        style: Option<&str>,
    ) -> Result<(), Self::Error>;

    fn init(&mut self, max: Bidegree) -> Result<(), Self::Error> {
        self.header(max)?;

        for x in 0..=max.x() {
            let on_x_axis = Bidegree::x_y(x, 0);
            self.line(
                on_x_axis,
                Bidegree::x_y(x, max.y()),
                if x % 4 == 0 { "major-grid" } else { "grid" },
            )?;
            if x % 4 == 0 {
                self.text(on_x_axis, x, Orientation::Below)?;
            }
        }
        for y in 0..=max.y() {
            let on_y_axis = Bidegree::x_y(0, y);
            self.line(
                on_y_axis,
                Bidegree::x_y(max.x(), y),
                if y % 4 == 0 { "major-grid" } else { "grid" },
            )?;
            if y % 4 == 0 {
                self.text(on_y_axis, y, Orientation::Left)?;
            }
        }
        Ok(())
    }

    fn structline_matrix(
        &mut self,
        source: Bidegree,
        target: Bidegree,
        matrix: Vec<Vec<u32>>,
        class: Option<&str>,
    ) -> Result<(), Self::Error> {
        for (k, row) in matrix.into_iter().enumerate() {
            for (l, v) in row.into_iter().enumerate() {
                if v != 0 {
                    self.structline(
                        BidegreeGenerator::new(source, k),
                        BidegreeGenerator::new(target, l),
                        class,
                    )?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect_file;

    use super::*;

    #[test]
    fn test_legend() {
        let mut res: Vec<u8> = Vec::new();
        SvgBackend::legend(&mut res).unwrap();

        expect_file!["./legend.svg"].assert_eq(std::str::from_utf8(&res).unwrap());
    }
}
