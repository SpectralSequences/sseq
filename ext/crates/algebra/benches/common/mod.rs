//! Shared helpers for the algebra crate's criterion benchmarks.
//!
//! These are generic over *any* [`Algebra`] or [`Module`], so the same routines can benchmark the
//! Adem and Milnor bases (see `multiplication.rs`) or any of the predefined Steenrod modules in
//! `ext/steenrod_modules` (see `module_action.rs`).
//!
//! Every bench target that needs these includes the file with `mod common;`; each target compiles
//! its own copy, which is the standard criterion pattern for sharing code between benches.

// Not every bench target uses every helper, and each compiles this module independently, so silence
// the unavoidable dead-code warnings.
#![allow(dead_code)]

use std::{hint::black_box, path::PathBuf, sync::Arc};

use algebra::{
    Algebra, AlgebraType, SteenrodAlgebra,
    module::{Module, SteenrodModule, steenrod_module},
};
use criterion::{BenchmarkGroup, Throughput, measurement::WallTime};
use fp::{
    prime::{Prime, ValidPrime},
    vector::FpVector,
};
use serde_json::Value;

/// The directory shipping the predefined Steenrod modules. We read the JSON directly rather than
/// going through `ext::utils`, because the `ext` crate depends on `algebra` (a dependency cycle).
pub const STEENROD_MODULES_DIR: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../steenrod_modules");

/// Build a dense element of the given length with every entry non-zero and reduced mod `p`. Used to
/// exercise the general-element multiplication and action code paths.
pub fn dense_vector(p: ValidPrime, len: usize) -> FpVector {
    let mut v = FpVector::new(p, len);
    let modulus = p.as_u32() - 1;
    for i in 0..len {
        // Entries in `1..p`, so the vector is genuinely dense.
        v.set_entry(i, (i as u32 % modulus) + 1);
    }
    v
}

// -------------------------------------------------------------------------------------------------
// Algebra multiplication
// -------------------------------------------------------------------------------------------------

/// Benchmark multiplication in `algebra` for each `(r_degree, s_degree)` pair in `pairs`.
///
/// For each pair we register two benchmarks:
///  - `basis_products`: multiply every pair of basis elements via
///    [`Algebra::multiply_basis_elements`], the required multiplication primitive.
///  - `element_product`: a single [`Algebra::multiply_element_by_element`] of two dense elements,
///    exercising the general-element path.
pub fn bench_algebra_multiplication<A: Algebra>(
    group: &mut BenchmarkGroup<WallTime>,
    label: &str,
    algebra: &A,
    pairs: &[(i32, i32)],
) {
    let p = algebra.prime();
    for &(r_degree, s_degree) in pairs {
        let out_degree = r_degree + s_degree;
        // `compute_basis` is cumulative, so this readies every degree up to `out_degree`.
        algebra.compute_basis(out_degree);
        let r_dim = algebra.dimension(r_degree);
        let s_dim = algebra.dimension(s_degree);
        let out_dim = algebra.dimension(out_degree);
        if r_dim == 0 || s_dim == 0 {
            continue;
        }

        group.throughput(Throughput::Elements((r_dim * s_dim) as u64));

        {
            let mut scratch = FpVector::new(p, out_dim);
            group.bench_function(
                format!("{label}/basis_products/{r_degree}x{s_degree}"),
                |b| {
                    b.iter(|| {
                        scratch.set_to_zero();
                        for i in 0..r_dim {
                            for j in 0..s_dim {
                                algebra.multiply_basis_elements(
                                    scratch.as_slice_mut(),
                                    1,
                                    r_degree,
                                    i,
                                    s_degree,
                                    j,
                                );
                            }
                        }
                        black_box(&scratch);
                    });
                },
            );
        }

        {
            let r = dense_vector(p, r_dim);
            let s = dense_vector(p, s_dim);
            let mut scratch = FpVector::new(p, out_dim);
            group.bench_function(
                format!("{label}/element_product/{r_degree}x{s_degree}"),
                |b| {
                    b.iter(|| {
                        scratch.set_to_zero();
                        algebra.multiply_element_by_element(
                            scratch.as_slice_mut(),
                            1,
                            r_degree,
                            r.as_slice(),
                            s_degree,
                            s.as_slice(),
                        );
                        black_box(&scratch);
                    });
                },
            );
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Module action
// -------------------------------------------------------------------------------------------------

/// The lowest few positive operation degrees in which `algebra` is non-trivial, up to `max` of them
/// and bounded by `op_cap`. Keeps the number of benchmark cases per module small and bounded.
fn representative_op_degrees(algebra: &SteenrodAlgebra, op_cap: i32, max: usize) -> Vec<i32> {
    let mut degrees = Vec::new();
    let mut d = 1;
    while d <= op_cap && degrees.len() < max {
        algebra.compute_basis(d);
        if algebra.dimension(d) > 0 {
            degrees.push(d);
        }
        d += 1;
    }
    degrees
}

/// Benchmark the module action of `module` up to total degree `cap`.
///
/// For a handful of representative operation degrees, we register two benchmarks that sweep over all
/// module degrees (bounded by `cap`, which keeps infinite modules such as `RP_inf` in check):
///  - `act_on_basis`: apply every operation basis element to every module basis element via the
///    required primitive [`Module::act_on_basis`].
///  - `act`: apply every operation basis element to a dense element via [`Module::act`].
pub fn bench_module_action(
    group: &mut BenchmarkGroup<WallTime>,
    label: &str,
    module: &dyn Module<Algebra = SteenrodAlgebra>,
    cap: i32,
) {
    let algebra = module.algebra();
    let p = module.prime();

    let min_degree = module.min_degree();
    let max_degree = module.max_degree().map_or(cap, |m| m.min(cap));
    if max_degree < min_degree {
        return;
    }

    let op_cap = cap.min(12);
    let op_degrees = representative_op_degrees(&algebra, op_cap, 4);

    for op_degree in op_degrees {
        // Ready every module (and algebra) degree we might touch.
        algebra.compute_basis(op_degree);
        module.compute_basis(max_degree + op_degree);
        let op_dim = algebra.dimension(op_degree);
        if op_dim == 0 {
            continue;
        }

        // The module degrees that actually contribute work at this operation degree.
        let sweep: Vec<(i32, usize, usize)> = (min_degree..=max_degree)
            .filter_map(|mod_degree| {
                let mod_dim = module.dimension(mod_degree);
                let out_dim = module.dimension(mod_degree + op_degree);
                (mod_dim > 0 && out_dim > 0).then_some((mod_degree, mod_dim, out_dim))
            })
            .collect();
        if sweep.is_empty() {
            continue;
        }

        let total_pairs: u64 = sweep
            .iter()
            .map(|&(_, mod_dim, _)| (op_dim * mod_dim) as u64)
            .sum();
        group.throughput(Throughput::Elements(total_pairs));

        {
            let mut scratch = FpVector::new(p, 0);
            group.bench_function(format!("{label}/act_on_basis/op{op_degree}"), |b| {
                b.iter(|| {
                    for &(mod_degree, mod_dim, out_dim) in &sweep {
                        scratch.set_scratch_vector_size(out_dim);
                        scratch.set_to_zero();
                        for op_idx in 0..op_dim {
                            for mod_idx in 0..mod_dim {
                                module.act_on_basis(
                                    scratch.as_slice_mut(),
                                    1,
                                    op_degree,
                                    op_idx,
                                    mod_degree,
                                    mod_idx,
                                );
                            }
                        }
                        black_box(&scratch);
                    }
                });
            });
        }

        {
            let inputs: Vec<(i32, FpVector, usize)> = sweep
                .iter()
                .map(|&(mod_degree, mod_dim, out_dim)| {
                    (mod_degree, dense_vector(p, mod_dim), out_dim)
                })
                .collect();
            let mut scratch = FpVector::new(p, 0);
            group.bench_function(format!("{label}/act/op{op_degree}"), |b| {
                b.iter(|| {
                    for op_idx in 0..op_dim {
                        for (mod_degree, input, out_dim) in &inputs {
                            scratch.set_scratch_vector_size(*out_dim);
                            scratch.set_to_zero();
                            module.act(
                                scratch.as_slice_mut(),
                                1,
                                op_degree,
                                op_idx,
                                *mod_degree,
                                input.as_slice(),
                            );
                            black_box(&scratch);
                        }
                    }
                });
            });
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Loading the predefined Steenrod modules
// -------------------------------------------------------------------------------------------------

/// All `*.json` module files shipped in [`STEENROD_MODULES_DIR`], sorted by file name for a stable
/// benchmark ordering.
pub fn module_json_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(STEENROD_MODULES_DIR)
        .unwrap_or_else(|e| panic!("failed to read {STEENROD_MODULES_DIR}: {e}"))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    files
}

/// A parsed module JSON together with its file stem (used as the benchmark label).
pub struct ModuleSpec {
    pub name: String,
    pub json: Value,
}

/// Parse every module file, returning `(name, json)` for each. Files that fail to read or parse are
/// reported to stderr and skipped.
pub fn load_module_specs() -> Vec<ModuleSpec> {
    let mut specs = Vec::new();
    for path in module_json_files() {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("<unknown>")
            .to_owned();
        match std::fs::read_to_string(&path)
            .map_err(anyhow::Error::from)
            .and_then(|s| serde_json::from_str::<Value>(&s).map_err(anyhow::Error::from))
        {
            Ok(json) => specs.push(ModuleSpec { name, json }),
            Err(e) => eprintln!("skipping module {name}: could not parse JSON ({e})"),
        }
    }
    specs
}

/// Whether the module spec requires machinery only available in the `ext` crate. The one such case
/// is a `cofiber`, which is realized as a Yoneda representative via `Resolution` /
/// `yoneda_representative` — neither of which lives in the `algebra` crate. Such modules cannot be
/// constructed from raw JSON here and are skipped.
pub fn requires_ext_machinery(json: &Value) -> bool {
    !json["cofiber"].is_null()
}

/// Whether the module's optional `"algebra"` restriction permits the given basis. Modules without an
/// `"algebra"` field support both bases.
pub fn module_supports(json: &Value, ty: AlgebraType) -> bool {
    match json["algebra"].as_array() {
        None => true,
        Some(list) => {
            let wanted = ty.to_string();
            list.iter().any(|x| x.as_str() == Some(&wanted))
        }
    }
}

/// Build the algebra and module for a given spec and basis, using only the `algebra` crate's JSON
/// constructors.
pub fn load_named_module(
    json: &Value,
    ty: AlgebraType,
) -> anyhow::Result<(Arc<SteenrodAlgebra>, SteenrodModule)> {
    let algebra = Arc::new(SteenrodAlgebra::from_json(json, ty, false)?);
    let module = steenrod_module::from_json(Arc::clone(&algebra), json)?;
    Ok((algebra, module))
}
