//! Benchmarks for the module action, run against every predefined module in
//! `ext/steenrod_modules`.
//!
//! Each module is loaded directly from its JSON specification (via the `algebra` crate's own
//! constructors) and benchmarked under both the Milnor and Adem bases it supports. Modules that
//! require `ext`-level machinery (i.e. those defined via a `cofiber`) are skipped.
//!
//! To benchmark a single module, filter by name, e.g.
//! `cargo bench --bench module_action -- Joker`.

mod common;

use algebra::AlgebraType;
use criterion::{Criterion, criterion_group, criterion_main};
use pprof::criterion::{Output, PProfProfiler};

/// Cap on total degree, so infinite or high-dimensional modules (`RP_inf`, finitely presented
/// modules, …) stay bounded.
const DEGREE_CAP: i32 = 20;

fn module_action(c: &mut Criterion) {
    for spec in common::load_module_specs() {
        if common::requires_ext_machinery(&spec.json) {
            eprintln!(
                "skipping module {}: cofiber modules require ext-level resolution machinery",
                spec.name
            );
            continue;
        }

        for ty in [AlgebraType::Milnor, AlgebraType::Adem] {
            if !common::module_supports(&spec.json, ty) {
                continue;
            }
            let basis = ty.to_string();
            let (_algebra, module) = match common::load_named_module(&spec.json, ty) {
                Ok(pair) => pair,
                Err(e) => {
                    eprintln!("skipping module {} ({basis}): {e}", spec.name);
                    continue;
                }
            };

            let mut g = c.benchmark_group(format!("module_action/{}", spec.name));
            common::bench_module_action(&mut g, &basis, &*module, DEGREE_CAP);
            g.finish();
        }
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(std::time::Duration::from_secs(3))
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = module_action
}
criterion_main!(benches);
