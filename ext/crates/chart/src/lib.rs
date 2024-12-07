#![deny(clippy::use_self)]

use std::{collections::HashMap, fmt::Display, io};

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

    fn header(&mut self, max_x: i32, max_y: i32) -> Result<(), Self::Error>;
    fn line(
        &mut self,
        start_x: i32,
        end_x: i32,
        start_y: i32,
        end_y: i32,
        style: &str,
    ) -> Result<(), Self::Error>;

    fn text(
        &mut self,
        x: i32,
        y: i32,
        content: impl Display,
        orientation: Orientation,
    ) -> Result<(), Self::Error>;
    fn node(&mut self, x: i32, y: i32, n: usize) -> Result<(), Self::Error>;

    fn structline(
        &mut self,
        source: (i32, i32, usize),
        target: (i32, i32, usize),
        style: Option<&str>,
    ) -> Result<(), Self::Error>;

    fn init(&mut self, max_x: i32, max_y: i32) -> Result<(), Self::Error> {
        self.header(max_x, max_y)?;

        for x in 0..=max_x {
            self.line(
                x,
                x,
                0,
                max_y,
                if x % 4 == 0 { "major-grid" } else { "grid" },
            )?;
            if x % 4 == 0 {
                self.text(x, 0, x, Orientation::Below)?;
            }
        }
        for y in 0..=max_y {
            self.line(
                0,
                max_x,
                y,
                y,
                if y % 4 == 0 { "major-grid" } else { "grid" },
            )?;
            if y % 4 == 0 {
                self.text(0, y, y, Orientation::Left)?;
            }
        }
        Ok(())
    }

    fn structline_matrix(
        &mut self,
        source: (i32, i32),
        target: (i32, i32),
        matrix: Vec<Vec<u32>>,
        class: Option<&str>,
    ) -> Result<(), Self::Error> {
        for (k, row) in matrix.into_iter().enumerate() {
            for (l, v) in row.into_iter().enumerate() {
                if v != 0 {
                    self.structline((source.0, source.1, k), (target.0, target.1, l), class)?;
                }
            }
        }
        Ok(())
    }
}

pub struct SvgBackend<T: io::Write> {
    out: T,
    max_x: i32,
    max_y: i32,
    num_nodes: HashMap<(i32, i32), usize>,
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
    fn get_coords(&self, x: i32, y: i32, i: usize) -> (f32, f32, f32) {
        let n = *self.num_nodes.get(&(x, y)).unwrap();

        let (radius, patterns) = PATTERNS[n - 1];
        let offset = patterns[i];

        (
            radius,
            (x * Self::GRID_WIDTH + Self::MARGIN) as f32 + offset.0,
            ((self.max_y - y) * Self::GRID_WIDTH + Self::MARGIN) as f32 + offset.1,
        )
    }

    pub fn new(out: T) -> Self {
        Self {
            out,
            max_x: 0,
            max_y: 0,
            num_nodes: HashMap::new(),
        }
    }
}

impl<T: io::Write> Backend for SvgBackend<T> {
    type Error = io::Error;

    const EXT: &'static str = "svg";

    fn header(&mut self, max_x: i32, max_y: i32) -> Result<(), Self::Error> {
        self.max_x = max_x;
        self.max_y = max_y;

        let width = self.max_x * Self::GRID_WIDTH + 2 * Self::MARGIN;
        let height = self.max_y * Self::GRID_WIDTH + 2 * Self::MARGIN;

        writeln!(
            self.out,
            r#"<svg version = "1.1" width="{width}" height="{height}" xmlns="http://www.w3.org/2000/svg">"#,
        )?;
        writeln!(self.out, "<style>{}</style>", Self::STYLES)
    }

    fn line(
        &mut self,
        start_x: i32,
        end_x: i32,
        start_y: i32,
        end_y: i32,
        style: &str,
    ) -> Result<(), Self::Error> {
        let height = self.max_y * Self::GRID_WIDTH + 2 * Self::MARGIN;

        writeln!(
            self.out,
            r#"<line class="{style}" x1="{start_x}" x2="{end_x}" y1="{start_y}" y2="{end_y}" />"#,
            start_x = Self::MARGIN + start_x * Self::GRID_WIDTH,
            end_x = Self::MARGIN + end_x * Self::GRID_WIDTH,
            start_y = height - Self::MARGIN - start_y * Self::GRID_WIDTH,
            end_y = height - Self::MARGIN - end_y * Self::GRID_WIDTH,
        )
    }

    fn text(
        &mut self,
        x: i32,
        y: i32,
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
            x = Self::MARGIN + x * Self::GRID_WIDTH + offset.0,
            y = Self::MARGIN + (self.max_y - y) * Self::GRID_WIDTH + offset.1,
        )
    }

    fn node(&mut self, x: i32, y: i32, n: usize) -> Result<(), Self::Error> {
        if n == 0 || x > self.max_x || y > self.max_y {
            return Ok(());
        }
        self.num_nodes.insert((x, y), n);

        for k in 0..n {
            let (r, x, y) = self.get_coords(x, y, k);
            writeln!(self.out, r#"<circle cx="{x}" cy="{y}" r="{r}"/>"#,)?;
        }
        Ok(())
    }

    fn structline(
        &mut self,
        source: (i32, i32, usize),
        target: (i32, i32, usize),
        style: Option<&str>,
    ) -> Result<(), Self::Error> {
        if source.0 > self.max_x
            || source.1 > self.max_y
            || target.0 > self.max_x
            || target.1 > self.max_y
        {
            return Ok(());
        }

        let (_, source_x, source_y) = self.get_coords(source.0, source.1, source.2);
        let (_, target_x, target_y) = self.get_coords(target.0, target.1, target.2);

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
    max_x: i32,
    max_y: i32,
    num_nodes: HashMap<(i32, i32), usize>,
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
            max_x: 0,
            max_y: 0,
            num_nodes: HashMap::new(),
        }
    }

    /// Returns r, x, y
    fn get_coords(&self, x: i32, y: i32, i: usize) -> (f32, f32, f32) {
        let n = *self.num_nodes.get(&(x, y)).unwrap();

        let (radius, patterns) = PATTERNS[n - 1];
        let offset = patterns[i];

        (
            radius / 20.0,
            x as f32 + offset.0 / 20.0,
            // We subtract because in Tikz the origin is the bottom-left while in SVG it is the
            // top-left
            y as f32 - offset.1 / 20.0,
        )
    }
}

impl<T: io::Write> Backend for TikzBackend<T> {
    type Error = std::io::Error;

    const EXT: &'static str = "tex";

    fn header(&mut self, max_x: i32, max_y: i32) -> Result<(), Self::Error> {
        self.max_x = max_x;
        self.max_y = max_y;
        writeln!(self.out, "{}", Self::HEADER)
    }

    fn line(
        &mut self,
        start_x: i32,
        end_x: i32,
        start_y: i32,
        end_y: i32,
        style: &str,
    ) -> Result<(), Self::Error> {
        writeln!(
            self.out,
            r#"\draw [{style}] ({start_x}, {start_y}) -- ({end_x}, {end_y});"#,
        )
    }

    fn text(
        &mut self,
        x: i32,
        y: i32,
        content: impl Display,
        orientation: Orientation,
    ) -> Result<(), Self::Error> {
        let offset = match orientation {
            Orientation::Left => "left",
            Orientation::Right => "right",
            Orientation::Below => "below",
            Orientation::Above => "above",
        };

        writeln!(self.out, r#"\node [{offset}] at ({x}, {y}) {{{content}}};"#,)
    }

    fn node(&mut self, x: i32, y: i32, n: usize) -> Result<(), Self::Error> {
        if n == 0 || x > self.max_x || y > self.max_y {
            return Ok(());
        }
        self.num_nodes.insert((x, y), n);

        for k in 0..n {
            let (r, x, y) = self.get_coords(x, y, k);
            writeln!(self.out, r#"\draw [fill] ({x}, {y}) circle ({r});"#,)?;
        }
        Ok(())
    }

    fn structline(
        &mut self,
        source: (i32, i32, usize),
        target: (i32, i32, usize),
        style: Option<&str>,
    ) -> Result<(), Self::Error> {
        if source.0 > self.max_x
            || source.1 > self.max_y
            || target.0 > self.max_x
            || target.1 > self.max_y
        {
            return Ok(());
        }

        let (_, source_x, source_y) = self.get_coords(source.0, source.1, source.2);
        let (_, target_x, target_y) = self.get_coords(target.0, target.1, target.2);

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
