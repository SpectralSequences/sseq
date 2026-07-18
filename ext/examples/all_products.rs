//! Benchmark helper: compute *all* left-multiplication-by-generator products on a resolved module.
//!
//! By default this extends every product map together via [`ExtAlgebra::extend_all_products`], which
//! batches the quasi-inverse solve at each output bidegree (one shared solve per bidegree). Set
//! `EXT_PER_MAP=1` to fall back to extending each map on its own instead, and compare timings.
//!
//! Extending a product map is the chain-map lift that repeatedly calls
//! [`ChainComplex::apply_quasi_inverse`], so this exercises the quasi-inverse path across the whole
//! plane. Set `EXT_NASSAU_NO_SAVE_QI=1` on the resolve to store only the differentials, or
//! `EXT_NASSAU_RECOMPUTE_QI=1` here to force recompute-on-demand even when quasi-inverses are saved.
//!
//! With `EXT_DUMP_PRODUCTS=1` it prints the full multiplication table (sorted) instead of a timing
//! line, so the batched and per-map paths can be diffed.

use std::{sync::Arc, time::Instant};

use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    ext_algebra::ExtAlgebra,
    utils::query_module,
};

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    // Loading (from the save dir) is untimed; only the product-map extension is timed.
    let resolution = Arc::new(query_module(None, true)?);
    let e2 = ExtAlgebra::from_resolution(resolution)?;

    let per_map = std::env::var_os("EXT_PER_MAP").is_some();

    let start = Instant::now();
    let mut num_maps = 0usize;
    if per_map {
        // Old path: extend each product map on its own (recomputes each qi once per map).
        for b in e2.resolution().iter_stem() {
            for g in e2.basis(b) {
                e2.generator_product_map(g).extend_all();
                num_maps += 1;
            }
        }
    } else {
        // Batched path: extend all product maps together in bidegree-major order (one shared qi
        // solve per output bidegree).
        num_maps = e2
            .resolution()
            .iter_stem()
            .map(|b| e2.resolution().number_of_gens_in_bidegree(b))
            .sum();
        e2.extend_all_products();
    }
    let elapsed = start.elapsed();

    let mode = if per_map { "per-map" } else { "batched" };
    eprintln!("all_products ({mode}): extended {num_maps} product maps in {elapsed:.3?}");

    // Correctness dump: the full multiplication table, sorted so batched/per-map can be diffed.
    if std::env::var_os("EXT_DUMP_PRODUCTS").is_some() {
        let mut lines = Vec::new();
        for mb in e2.resolution().iter_stem() {
            for mg in e2.basis(mb) {
                let x = e2.generator(mg);
                for b in e2.unit().iter_nonzero_stem() {
                    let Some(rows) = e2.multiply_into(&x, b) else {
                        continue;
                    };
                    for (g, row) in e2.unit_basis(b).into_iter().zip(rows.iter()) {
                        let coords: Vec<u32> = row.iter().collect();
                        if coords.iter().any(|&c| c != 0) {
                            lines.push(format!("x_{mg} · x_{g} = {coords:?}"));
                        }
                    }
                }
            }
        }
        lines.sort();
        for l in lines {
            println!("{l}");
        }
    } else {
        println!("{num_maps} {}", elapsed.as_secs_f64());
    }
    Ok(())
}
