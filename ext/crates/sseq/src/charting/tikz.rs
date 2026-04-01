use std::{collections::HashMap, fmt::Display, io};

use crate::{
    charting::{Backend, Orientation, PATTERNS},
    coordinates::{Bidegree, BidegreeGenerator},
};

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
