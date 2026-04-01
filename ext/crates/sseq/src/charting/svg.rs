use std::{collections::HashMap, fmt::Display, io};

use crate::{
    charting::{Backend, Orientation, PATTERNS},
    coordinates::{Bidegree, BidegreeGenerator},
};

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

    /// Returns (radius, x, y) for the given generator.
    ///
    /// # Panics
    ///
    /// Panics if `node()` was not previously called for `g.degree()`.
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
        let _ = writeln!(self.out, "</svg>");
    }
}
