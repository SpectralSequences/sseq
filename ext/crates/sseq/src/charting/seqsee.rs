use std::{collections::BTreeSet, fmt::Display, io};

use serde_json::{Map, Value, json};

use crate::{
    charting::{Backend, Orientation},
    coordinates::{Bidegree, BidegreeGenerator},
};

/// A [`Backend`] that emits [SeqSee](https://github.com/JoeyBF/SeqSee) JSON.
///
/// SeqSee is a generic visualization tool for spectral sequences. It consumes a JSON file
/// conforming to its [schema] and produces a self-contained, interactive HTML chart. This backend
/// targets that schema so that the charts produced elsewhere in this crate can be rendered by
/// SeqSee.
///
/// The document is assembled in memory and written out when the backend is dropped, since the JSON
/// object must group all nodes together and all edges together, whereas the [`Backend`] callbacks
/// interleave them.
///
/// [schema]: https://raw.githubusercontent.com/JoeyBF/SeqSee/refs/heads/master/seqsee/input_schema.json
pub struct SeqSeeBackend<T: io::Write> {
    out: T,
    max: Bidegree,
    /// Map from node id to its SeqSee node object.
    nodes: Map<String, Value>,
    /// The list of SeqSee edge objects.
    edges: Vec<Value>,
    /// The set of style names referenced by edges. Each becomes an attribute alias in the header so
    /// that the edges referencing it validate against the schema.
    styles: BTreeSet<String>,
}

impl<T: io::Write> SeqSeeBackend<T> {
    pub fn new(out: T) -> Self {
        Self {
            out,
            max: Bidegree::zero(),
            nodes: Map::new(),
            edges: Vec::new(),
            styles: BTreeSet::new(),
        }
    }
}

/// Whether a style name denotes a differential, i.e. it is `d` followed by a page number.
///
/// Differentials are given a distinct color to match the other backends in this crate.
fn is_differential(style: &str) -> bool {
    style
        .strip_prefix('d')
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
}

impl<T: io::Write> Backend for SeqSeeBackend<T> {
    type Error = io::Error;

    const EXT: &'static str = "json";

    fn header(&mut self, max: Bidegree) -> Result<(), Self::Error> {
        self.max = max;
        Ok(())
    }

    // SeqSee draws its own grid based on the chart dimensions in the header, so there is nothing to
    // emit for the grid lines.
    fn line(&mut self, _start: Bidegree, _end: Bidegree, _style: &str) -> Result<(), Self::Error> {
        Ok(())
    }

    // SeqSee draws its own axis labels, so there is nothing to emit here.
    fn text(
        &mut self,
        _b: Bidegree,
        _content: impl Display,
        _orientation: Orientation,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn node(&mut self, b: Bidegree, n: usize) -> Result<(), Self::Error> {
        if n == 0 || b.x() > self.max.x() || b.y() > self.max.y() {
            return Ok(());
        }

        for k in 0..n {
            // The `{:#}` (alternate) form of the `Display` impl is `(x,y,idx)`. Reusing it here and
            // in `structline` keeps node ids and edge endpoints in sync.
            let id = format!("{:#}", BidegreeGenerator::new(b, k));
            self.nodes.insert(
                id,
                json!({
                    "x": b.x(),
                    "y": b.y(),
                    "position": k,
                }),
            );
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

        let mut edge = Map::new();
        edge.insert("source".to_string(), Value::String(format!("{source:#}")));
        edge.insert("target".to_string(), Value::String(format!("{target:#}")));
        if let Some(style) = style {
            self.styles.insert(style.to_string());
            edge.insert("attributes".to_string(), json!([style]));
        }
        self.edges.push(Value::Object(edge));
        Ok(())
    }
}

impl<T: io::Write> Drop for SeqSeeBackend<T> {
    fn drop(&mut self) {
        // Register every referenced style as an attribute alias so that the edges validate.
        let mut attributes = Map::new();
        for style in &self.styles {
            let attr = if is_differential(style) {
                json!([{ "color": "blue" }])
            } else {
                json!([])
            };
            attributes.insert(style.clone(), attr);
        }

        let document = json!({
            "header": {
                "chart": {
                    "width": { "min": 0, "max": self.max.x() },
                    "height": { "min": 0, "max": self.max.y() },
                },
                "aliases": {
                    "attributes": attributes,
                },
            },
            "nodes": std::mem::take(&mut self.nodes),
            "edges": std::mem::take(&mut self.edges),
        });

        let _ = serde_json::to_writer_pretty(&mut self.out, &document);
        let _ = writeln!(self.out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seqsee_output() {
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut backend = SeqSeeBackend::new(&mut buf);
            // `init` draws the grid and axis labels via `line`/`text`, which are no-ops here.
            backend.init(Bidegree::x_y(2, 2)).unwrap();

            backend.node(Bidegree::x_y(0, 0), 1).unwrap();
            backend.node(Bidegree::x_y(0, 1), 1).unwrap();
            backend.node(Bidegree::x_y(1, 1), 1).unwrap();

            // A multiplication by `h0` (a filtration-one product).
            backend
                .structline(
                    BidegreeGenerator::new(Bidegree::x_y(0, 0), 0),
                    BidegreeGenerator::new(Bidegree::x_y(0, 1), 0),
                    Some("h0"),
                )
                .unwrap();
            // A `d2` differential.
            backend
                .structline(
                    BidegreeGenerator::new(Bidegree::x_y(1, 1), 0),
                    BidegreeGenerator::new(Bidegree::x_y(0, 1), 0),
                    Some("d2"),
                )
                .unwrap();
            // Out of bounds: dropped silently.
            backend.node(Bidegree::x_y(5, 5), 1).unwrap();
        }

        // Compare parsed values rather than the raw string so the assertion does not depend on
        // JSON object key ordering, which varies with whether serde_json's `preserve_order`
        // feature is enabled by feature unification in the surrounding build.
        let produced: Value = serde_json::from_slice(&buf).unwrap();
        let expected = json!({
            "header": {
                "chart": {
                    "width": { "min": 0, "max": 2 },
                    "height": { "min": 0, "max": 2 },
                },
                "aliases": {
                    "attributes": {
                        "h0": [],
                        "d2": [{ "color": "blue" }],
                    },
                },
            },
            "nodes": {
                "(0,0,0)": { "x": 0, "y": 0, "position": 0 },
                "(0,1,0)": { "x": 0, "y": 1, "position": 0 },
                "(1,1,0)": { "x": 1, "y": 1, "position": 0 },
            },
            "edges": [
                { "source": "(0,0,0)", "target": "(0,1,0)", "attributes": ["h0"] },
                { "source": "(1,1,0)", "target": "(0,1,0)", "attributes": ["d2"] },
            ],
        });
        assert_eq!(produced, expected);
    }

    #[test]
    fn test_is_differential() {
        assert!(is_differential("d2"));
        assert!(is_differential("d17"));
        assert!(!is_differential("d"));
        assert!(!is_differential("h0"));
        assert!(!is_differential("delta"));
    }
}
