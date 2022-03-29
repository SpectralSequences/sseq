//! Types and traits for working with various algebras and modules, with
//! a focus on the Steenrod algebra and its modules.

// TODO: Write descriptions of each module therein.

pub mod change_of_basis;
pub mod module;
pub mod steenrod_evaluator;
pub mod steenrod_parser;

//pub mod dense_bigraded_algebra;

mod algebra;

pub use crate::algebra::*;

#[cfg(feature = "json")]
pub(crate) fn module_gens_from_json(
    gens: &serde_json::Value,
) -> (
    bivec::BiVec<usize>,
    bivec::BiVec<Vec<String>>,
    rustc_hash::FxHashMap<String, (i32, usize)>,
) {
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
    (graded_dimension, gen_names, gen_to_idx)
}
