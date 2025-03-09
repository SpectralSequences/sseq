use std::{collections::HashMap, fmt::Display, io};

use crate::coordinates::{Bidegree, BidegreeGenerator};

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

pub struct SvgBackend<T: io::Write> {
    out: T,
    max: Bidegree,
    num_nodes: HashMap<Bidegree, usize>,
}

impl<T: io::Write> SvgBackend<T> {
    const GRID_WIDTH: i32 = 20;
    const MARGIN: i32 = 30;
    const STYLES: &'static str = r#"
    circle {
        fill: black;
    }
    .structline {
        stroke: black;
        fill: none;
    }
    .d2 {
        stroke: blue;
    }
    .major-grid {
        stroke: black;
        opacity: 20%;
        shape-rendering: crispEdges;
        fill: none;
    }
    .grid {
        stroke: black;
        opacity: 10%;
        shape-rendering: crispEdges;
        fill: none;
    }
    .x-label {
     text-anchor: middle;
     dominant-baseline: text-before-edge;
    }
    .y-label {
     text-anchor: end;
     dominant-baseline: middle;
    }
    "#;

    /// Print the legend for node patterns
    pub fn legend(mut out: T) -> std::io::Result<()> {
        writeln!(
            out,
            r#"<svg version = "1.1" width="{}" height="100" xmlns="http://www.w3.org/2000/svg">"#,
            PATTERNS.len() * 100,
        )?;

        for (n, (_, row)) in PATTERNS.iter().enumerate() {
            writeln!(
                out,
                r#"<rect fill="none" stroke="black" width="80" height="80" y="10" x="{x}"/>"#,
                x = 10 + n * 100
            )?;
            for (k, offset) in row[0..=n].iter().enumerate() {
                writeln!(
                    out,
                    r#"<text x="{x}" y="{y}" text-anchor="middle" dominant-baseline="middle">{k}</text>"#,
                    x = 50.0 + n as f32 * 100.0 + offset.0 * 6.0,
                    y = 50.0 + offset.1 * 6.0,
                )?;
            }
        }
        writeln!(out, r#"</svg>"#,)?;

        Ok(())
    }

    /// Returns r, x, y
    fn get_coords(&self, g: BidegreeGenerator) -> (f32, f32, f32) {
        let n = *self.num_nodes.get(&g.degree()).unwrap();

        let (radius, patterns) = PATTERNS[n - 1];
        let offset = patterns[g.idx()];

        (
            radius,
            (g.x() * Self::GRID_WIDTH + Self::MARGIN) as f32 + offset.0,
            ((self.max - g.degree()).y() * Self::GRID_WIDTH + Self::MARGIN) as f32 + offset.1,
        )
    }

    pub fn new(out: T) -> Self {
        Self {
            out,
            max: Bidegree::zero(),
            num_nodes: HashMap::new(),
        }
    }
}

impl<T: io::Write> Backend for SvgBackend<T> {
    type Error = io::Error;

    const EXT: &'static str = "svg";

    fn header(&mut self, max: Bidegree) -> Result<(), Self::Error> {
        self.max = max;

        let width = self.max.x() * Self::GRID_WIDTH + 2 * Self::MARGIN;
        let height = self.max.y() * Self::GRID_WIDTH + 2 * Self::MARGIN;

        writeln!(
            self.out,
            r#"<svg version = "1.1" width="{width}" height="{height}" xmlns="http://www.w3.org/2000/svg">"#,
        )?;
        writeln!(self.out, "<style>{}</style>", Self::STYLES)
    }

    fn line(&mut self, start: Bidegree, end: Bidegree, style: &str) -> Result<(), Self::Error> {
        let height = self.max.y() * Self::GRID_WIDTH + 2 * Self::MARGIN;

        writeln!(
            self.out,
            r#"<line class="{style}" x1="{start_x}" x2="{end_x}" y1="{start_y}" y2="{end_y}" />"#,
            start_x = Self::MARGIN + start.x() * Self::GRID_WIDTH,
            end_x = Self::MARGIN + end.x() * Self::GRID_WIDTH,
            start_y = height - Self::MARGIN - start.y() * Self::GRID_WIDTH,
            end_y = height - Self::MARGIN - end.y() * Self::GRID_WIDTH,
        )
    }

    fn text(
        &mut self,
        b: Bidegree,
        content: impl Display,
        orientation: Orientation,
    ) -> Result<(), Self::Error> {
        let (offset, class) = match orientation {
            Orientation::Left => ((-5, 0), "y-label"),
            Orientation::Right => unimplemented!(),
            Orientation::Below => ((0, 3), "x-label"),
            Orientation::Above => unimplemented!(),
        };

        writeln!(
            self.out,
            r#"<text class="{class}" x="{x}" y="{y}">{content}</text>"#,
            x = Self::MARGIN + b.x() * Self::GRID_WIDTH + offset.0,
            y = Self::MARGIN + (self.max - b).y() * Self::GRID_WIDTH + offset.1,
        )
    }

    fn node(&mut self, b: Bidegree, n: usize) -> Result<(), Self::Error> {
        if n == 0 || b.x() > self.max.x() || b.y() > self.max.y() {
            return Ok(());
        }
        self.num_nodes.insert(b, n);

        for k in 0..n {
            let (r, x, y) = self.get_coords(BidegreeGenerator::new(b, k));
            writeln!(self.out, r#"<circle cx="{x}" cy="{y}" r="{r}"/>"#,)?;
        }
        Ok(())
    }

    fn structline(
        &mut self,
        source: BidegreeGenerator,
        target: BidegreeGenerator,
        style: Option<&str>,
    ) -> Result<(), Self::Error> {
        if source.x() > self.max.x()
            || source.y() > self.max.y()
            || target.x() > self.max.x()
            || target.y() > self.max.y()
        {
            return Ok(());
        }

        let (_, source_x, source_y) = self.get_coords(source);
        let (_, target_x, target_y) = self.get_coords(target);

        writeln!(
            self.out,
            r#"<line class="{style}" x1="{source_x}" x2="{target_x}" y1="{source_y}" y2="{target_y}" />"#,
            style = match &style {
                Some(x) => format!("structline {x}"),
                None => String::from("structline"),
            },
        )?;

        Ok(())
    }
}

impl<T: io::Write> Drop for SvgBackend<T> {
    fn drop(&mut self) {
        writeln!(self.out, "</svg>").unwrap();
    }
}

pub struct TikzBackend<T: io::Write> {
    out: T,
    max: Bidegree,
    num_nodes: HashMap<Bidegree, usize>,
}

impl<T: io::Write> TikzBackend<T> {
    const HEADER: &'static str = r"\begin{tikzpicture}[
  major-grid/.style={ opacity = 0.2 },
  grid/.style={ opacity = 0.1 },
  d2/.style={ blue },
]";

    pub fn new(out: T) -> Self {
        Self {
            out,
            max: Bidegree::zero(),
            num_nodes: HashMap::new(),
        }
    }

    /// Returns r, x, y
    fn get_coords(&self, g: BidegreeGenerator) -> (f32, f32, f32) {
        let n = *self.num_nodes.get(&g.degree()).unwrap();

        let (radius, patterns) = PATTERNS[n - 1];
        let offset = patterns[g.idx()];

        (
            radius / 20.0,
            g.x() as f32 + offset.0 / 20.0,
            // We subtract because in Tikz the origin is the bottom-left while in SVG it is the
            // top-left
            g.y() as f32 - offset.1 / 20.0,
        )
    }
}

impl<T: io::Write> Backend for TikzBackend<T> {
    type Error = std::io::Error;

    const EXT: &'static str = "tex";

    fn header(&mut self, max: Bidegree) -> Result<(), Self::Error> {
        self.max = max;
        writeln!(self.out, "{}", Self::HEADER)
    }

    fn line(&mut self, start: Bidegree, end: Bidegree, style: &str) -> Result<(), Self::Error> {
        writeln!(
            self.out,
            r#"\draw [{style}] ({start_x}, {start_y}) -- ({end_x}, {end_y});"#,
            start_x = start.x(),
            start_y = start.y(),
            end_x = end.x(),
            end_y = end.y(),
        )
    }

    fn text(
        &mut self,
        b: Bidegree,
        content: impl Display,
        orientation: Orientation,
    ) -> Result<(), Self::Error> {
        let offset = match orientation {
            Orientation::Left => "left",
            Orientation::Right => "right",
            Orientation::Below => "below",
            Orientation::Above => "above",
        };

        writeln!(
            self.out,
            r#"\node [{offset}] at ({x}, {y}) {{{content}}};"#,
            x = b.x(),
            y = b.y()
        )
    }

    fn node(&mut self, b: Bidegree, n: usize) -> Result<(), Self::Error> {
        if n == 0 || b.x() > self.max.x() || b.y() > self.max.y() {
            return Ok(());
        }
        self.num_nodes.insert(b, n);

        for k in 0..n {
            let (r, x, y) = self.get_coords(BidegreeGenerator::new(b, k));
            writeln!(self.out, r#"\draw [fill] ({x}, {y}) circle ({r});"#,)?;
        }
        Ok(())
    }

    fn structline(
        &mut self,
        source: BidegreeGenerator,
        target: BidegreeGenerator,
        style: Option<&str>,
    ) -> Result<(), Self::Error> {
        if source.x() > self.max.x()
            || source.y() > self.max.y()
            || target.x() > self.max.x()
            || target.y() > self.max.y()
        {
            return Ok(());
        }

        let (_, source_x, source_y) = self.get_coords(source);
        let (_, target_x, target_y) = self.get_coords(target);

        writeln!(
            self.out,
            r#"\draw [{style}] ({source_x}, {source_y}) -- ({target_x}, {target_y});"#,
            style = style.unwrap_or(""),
        )?;

        Ok(())
    }
}

impl<T: io::Write> Drop for TikzBackend<T> {
    fn drop(&mut self) {
        writeln!(self.out, r#"\end{{tikzpicture}}"#).unwrap();
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

        expect_file!["../legend.svg"].assert_eq(std::str::from_utf8(&res).unwrap());
    }
}
