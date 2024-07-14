//! Types and traits for working with various algebras and modules, with
//! a focus on the Steenrod algebra and its modules.

// TODO: Write descriptions of each module therein.

#![deny(clippy::use_self)]

pub mod module;
pub mod steenrod_evaluator;
pub(crate) mod steenrod_parser;

//pub mod dense_bigraded_algebra;

mod algebra;

pub use crate::algebra::*;

pub(crate) fn module_gens_from_json(
    gens: &serde_json::Value,
) -> (
    bivec::BiVec<usize>,
    bivec::BiVec<Vec<String>>,
    impl for<'a> Fn(&'a str) -> anyhow::Result<(i32, usize)> + '_,
) {
    use anyhow::anyhow;
    let gens = gens.as_object().unwrap();

    let degrees = gens
        .iter()
        .map(|(_, x)| x.as_i64().unwrap() as i32)
        .collect::<Vec<_>>();

    let min_degree = degrees.iter().copied().min().unwrap_or(0);
    let max_degree = degrees.iter().copied().max().unwrap_or(-1) + 1;

    let mut gen_to_idx = rustc_hash::FxHashMap::default();
    let mut graded_dimension = bivec::BiVec::with_capacity(min_degree, max_degree);
    let mut gen_names = bivec::BiVec::with_capacity(min_degree, max_degree);

    for _ in min_degree..max_degree {
        graded_dimension.push(0);
        gen_names.push(vec![]);
    }

    for (name, degree) in gens {
        let degree = degree.as_i64().unwrap() as i32;
        gen_names[degree].push(name.clone());
        gen_to_idx.insert(name.clone(), (degree, graded_dimension[degree]));
        graded_dimension[degree] += 1;
    }
    (graded_dimension, gen_names, move |gen| {
        gen_to_idx
            .get(gen)
            .copied()
            .ok_or_else(|| anyhow!("Invalid generator: {gen}"))
    })
}

#[cfg(test)]
pub mod tests {
    pub fn joker_json() -> serde_json::Value {
        use serde_json::json;

        json!({
            "type" : "finite dimensional module",
            "p": 2,
            "gens": {
                "x0": 0,
                "x1": 1,
                "x2": 2,
                "x3": 3,
                "x4": 4
            },
            "actions": [
                "Sq1 x0 = x1",
                "Sq2 x1 = x3",
                "Sq1 x3 = x4",
                "Sq2 x0 = x2",
                "Sq2 x2 = x4"
            ]
        })
    }
}
