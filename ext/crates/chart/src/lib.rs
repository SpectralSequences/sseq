use std::collections::HashMap;
use std::io::{Result, Write};

const STYLES: &str = r#"
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

const GRID_WIDTH: i32 = 20;
const MARGIN: i32 = 30;
#[rustfmt::skip]
const PATTERNS: [(f32, [(f32, f32); 7]); 7] = [
    (2.0, [(0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)]),
    (2.0, [(-3.0, 0.0), (3.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)]),
    (2.0, [(-3.0, 2.58), (3.0, 2.58), (0.0, -2.58), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)]),
    (2.0, [(-3.0, 3.0), (3.0, 3.0), (-3.0, -3.0), (3.0, -3.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)]),
    (1.5, [(-3.0, 3.0), (3.0, 3.0), (0.0, 0.0), (-3.0, -3.0), (3.0, -3.0), (0.0, 0.0), (0.0, 0.0)]),
    (1.5, [(-2.0, 4.0), (2.0, 4.0), (-2.0, 0.0), (2.0, 0.0), (-2.0, -4.0), (2.0, -4.0), (0.0, 0.0)]),
    (1.5, [(-2.0, 4.0), (2.0, 4.0), (-4.0, 0.0), (0.0, 0.0), (4.0, 0.0), (-2.0, -4.0), (2.0, -4.0)]),
];

pub struct Graph<T: Write> {
    out: T,
    max_x: i32,
    max_y: i32,
    num_nodes: HashMap<(i32, i32), usize>,
}

impl<T: Write> Drop for Graph<T> {
    fn drop(&mut self) {
        writeln!(self.out, "</svg>").unwrap();
    }
}

impl<T: Write> Graph<T> {
    pub fn new(mut out: T, max_x: i32, max_y: i32) -> Result<Self> {
        let width = max_x * GRID_WIDTH + 2 * MARGIN;
        let height = max_y * GRID_WIDTH + 2 * MARGIN;

        writeln!(
            out,
            r#"<svg version = "1.1" width="{width}" height="{height}" xmlns="http://www.w3.org/2000/svg">"#,
            width = width,
            height = height
        )?;

        writeln!(out, "<style>{}</style>", STYLES)?;

        for x in 0..=max_x {
            writeln!(
                out,
                r#"<line class="{class}" x1="{x}" x2="{x}" y1="{margin}" y2="{y_end}" />"#,
                class = if x % 4 == 0 { "major-grid" } else { "grid" },
                margin = MARGIN,
                x = MARGIN + x * GRID_WIDTH,
                y_end = height - MARGIN,
            )?;
            if x % 4 == 0 {
                writeln!(
                    out,
                    r#"<text class="x-label" x="{x}" y="{y}">{text}</text>"#,
                    x = MARGIN + x * GRID_WIDTH,
                    y = height - MARGIN + 3,
                    text = x,
                )?
            }
        }
        for y in 0..=max_y {
            writeln!(
                out,
                r#"<line class="{class}" x1="{margin}" x2="{x_end}" y1="{y}" y2="{y}" />"#,
                margin = MARGIN,
                class = if y % 4 == 0 { "major-grid" } else { "grid" },
                y = MARGIN + (max_y - y) * GRID_WIDTH,
                x_end = width - MARGIN,
            )?;
            if y % 4 == 0 {
                writeln!(
                    out,
                    r#"<text class="y-label" x="{x}" y="{y}">{text}</text>"#,
                    x = MARGIN - 5,
                    y = MARGIN + (max_y - y) * GRID_WIDTH,
                    text = y,
                )?
            }
        }

        Ok(Self {
            out,
            max_x,
            max_y,
            num_nodes: HashMap::new(),
        })
    }

    /// Returns r, x, y
    pub fn get_coords(&self, x: i32, y: i32, i: usize) -> (f32, f32, f32) {
        let n = *self.num_nodes.get(&(x, y)).unwrap();

        let (radius, patterns) = PATTERNS[n - 1];
        let offset = patterns[i];

        (
            radius,
            (x * GRID_WIDTH + MARGIN) as f32 + offset.0,
            ((self.max_y - y) * GRID_WIDTH + MARGIN) as f32 + offset.1,
        )
    }

    pub fn node(&mut self, x: i32, y: i32, n: usize) -> Result<()> {
        if n == 0 || x > self.max_x || y > self.max_y {
            return Ok(());
        }
        self.num_nodes.insert((x, y), n);

        for k in 0..n {
            let (r, x, y) = self.get_coords(x, y, k);
            writeln!(
                self.out,
                r#"<circle cx="{x}" cy="{y}" r="{r}"/>"#,
                x = x,
                y = y,
                r = r
            )?;
        }
        Ok(())
    }

    pub fn structline(
        &mut self,
        source: (i32, i32, usize),
        target: (i32, i32, usize),
        class: Option<&str>,
    ) -> Result<()> {
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
            r#"<line class="{class}" x1="{source_x}" x2="{target_x}" y1="{source_y}" y2="{target_y}" />"#,
            class = class.unwrap_or("structline"),
            source_x = source_x,
            source_y = source_y,
            target_x = target_x,
            target_y = target_y,
        )?;

        Ok(())
    }

    /// Print the legend for node patterns
    pub fn legend(mut out: T) -> Result<()> {
        writeln!(
            out,
            r#"<svg version = "1.1" width="700" height="100" xmlns="http://www.w3.org/2000/svg">"#,
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
                    k = k
                )?;
            }
        }
        writeln!(out, r#"</svg>"#,)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use expect_test::expect_file;

    #[test]
    fn test_legend() {
        let mut res: Vec<u8> = Vec::new();
        Graph::legend(&mut res).unwrap();

        expect_file!["../legend.svg"].assert_eq(std::str::from_utf8(&res).unwrap());
    }
}
