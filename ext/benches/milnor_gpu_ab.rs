//! A/B microbenchmark: GPU batch Milnor-multiply vs the CPU `get_partial_matrix`.
//!
//! Measures whether offloading the Milnor multiply to the GPU beats the CPU per-term path
//! *including* host↔device transfer.
//!
//! It resolves `S_2` with Nassau's algorithm, finds the largest `get_partial_matrix`
//! launch (by total element-term work), then times, best-of-N:
//! - **CPU**: the real `get_partial_matrix(degree, inputs)` (parallel, the default
//!   per-term path) — this is the baseline to beat.
//! - **GPU**: extract the `(R, s)` products of that launch via the public API, then
//!   `multiply_batch_on_gpu` (marshalling + upload + kernel + readback).
//!
//! Caveats (all *favour* the GPU, so a GPU loss here is conclusive): the GPU timing
//! excludes identity (`Sq(∅)`) copies and F₂ result *placement* (bit offsets) — both
//! cheap and not the multiply work being offloaded.
//!
//! Run under the `gpu` dev shell (unsandboxed):
//! `nix develop .#gpu --command cargo bench --bench milnor_gpu_ab --features gpu -- <stem> <max_s>`

#[cfg(not(feature = "gpu"))]
fn main() {
    eprintln!("milnor_gpu_ab requires --features gpu (and a live CUDA device).");
}

#[cfg(feature = "gpu")]
fn main() {
    use std::time::Instant;

    use algebra::{
        MilnorAlgebra,
        milnor_gpu::{GpuProduct, multiply_batch_on_gpu},
        module::{Module, homomorphism::ModuleHomomorphism},
    };
    use ext::{
        chain_complex::ChainComplex,
        utils::{construct_nassau, init_logging},
    };
    use sseq::coordinates::Bidegree;

    // With `--features logging` this installs the tracing subscriber so the
    // resolution's per-bidegree spans stream to stderr (RUST_LOG=info); without
    // it, it's a no-op NoSubscriber.
    let _ = init_logging();

    let args: Vec<String> = std::env::args().collect();
    let stem: i32 = args.get(1).and_then(|x| x.parse().ok()).unwrap_or(80);
    let filt: i32 = args.get(2).and_then(|x| x.parse().ok()).unwrap_or(42);
    let n_runs: u32 = args.get(3).and_then(|x| x.parse().ok()).unwrap_or(3);
    let max_t = stem + filt;

    eprintln!("Resolving S_2 (Nassau) through stem {stem}, filtration {filt} ...");
    let res = construct_nassau(("S_2", "milnor"), None).unwrap();
    let t0 = Instant::now();
    res.compute_through_stem(Bidegree::n_s(stem, filt));
    eprintln!("  resolved in {:.1}s", t0.elapsed().as_secs_f64());

    // The algebra (shared) — build the seqno tables the GPU path indexes with.
    let algebra: std::sync::Arc<MilnorAlgebra> = res.differential(1).target().algebra();
    algebra.compute_seqno_tables(max_t);

    // Cheap traversal that only *counts* the multiply work of one full launch (all
    // source basis elements as inputs) at bidegree (s, t) — used to find the biggest
    // launch without allocating products for every bidegree.
    let count_work = |s: i32, t: i32| -> (usize, usize) {
        let diff = res.differential(s);
        let source = diff.source();
        let target = diff.target();
        let shift = diff.degree_shift();
        let (mut n_products, mut n_terms) = (0usize, 0usize);
        for input_index in 0..source.dimension(t) {
            let ogp = source.index_to_op_gen(t, input_index);
            if ogp.generator_degree < diff.min_degree() || ogp.operation_degree == 0 {
                continue;
            }
            let out_on_gen = diff.output(ogp.generator_degree, ogp.generator_index);
            let act_input_degree = ogp.generator_degree - shift;
            for gd in target
                .iter_gen_offsets::<2>([act_input_degree, act_input_degree + ogp.operation_degree])
            {
                let (input_start, input_end) = (gd.start[0], gd.end[0]);
                if input_start >= out_on_gen.len() {
                    break;
                }
                let s_slice = out_on_gen.as_slice().restrict(input_start, input_end);
                if !s_slice.is_zero() {
                    n_products += 1;
                    n_terms += s_slice.iter_nonzero().count();
                }
            }
        }
        (n_products, n_terms)
    };

    // Extract the (R, s) products of one full launch at bidegree (s, t), mirroring
    // apply_to_basis_element → FreeModule::act via the public API. Identity operations
    // (Sq(∅)) carry no multiply work and are skipped.
    let extract = |s: i32, t: i32| -> Vec<GpuProduct> {
        let diff = res.differential(s);
        let source = diff.source();
        let target = diff.target();
        let shift = diff.degree_shift();
        let mut products = Vec::new();
        for input_index in 0..source.dimension(t) {
            let ogp = source.index_to_op_gen(t, input_index);
            if ogp.generator_degree < diff.min_degree() || ogp.operation_degree == 0 {
                continue;
            }
            let out_on_gen = diff.output(ogp.generator_degree, ogp.generator_index);
            let act_input_degree = ogp.generator_degree - shift;
            for gd in target
                .iter_gen_offsets::<2>([act_input_degree, act_input_degree + ogp.operation_degree])
            {
                let (input_start, input_end) = (gd.start[0], gd.end[0]);
                if input_start >= out_on_gen.len() {
                    break;
                }
                let s_slice = out_on_gen.as_slice().restrict(input_start, input_end);
                if s_slice.is_zero() {
                    continue;
                }
                products.push(GpuProduct {
                    r_degree: ogp.operation_degree,
                    r_idx: ogp.operation_index,
                    s_degree: act_input_degree - gd.gen_deg,
                    term_indices: s_slice.iter_nonzero().map(|(i, _)| i).collect(),
                    row: input_index,
                    out_offset: gd.start[1],
                });
            }
        }
        products
    };

    // Find the launch with the most element-term work across the resolved region.
    let mut best: Option<(i32, i32, usize, usize)> = None; // (s, t, n_products, n_terms)
    eprintln!("Scanning {filt} filtrations for the largest launch ...");
    for s in 1..=filt {
        eprintln!("  scan s={s}/{filt} (best so far: {best:?})");
        for t in s..=(s + stem) {
            let diff = res.differential(s);
            if diff.source().dimension(t) == 0 || diff.target().dimension(t) == 0 {
                continue;
            }
            let (n_products, n_terms) = count_work(s, t);
            if n_products == 0 {
                continue;
            }
            if best.is_none_or(|(_, _, _, bt)| n_terms > bt) {
                best = Some((s, t, n_products, n_terms));
            }
        }
    }

    let Some((s, t, n_products, n_terms)) = best else {
        eprintln!("No non-trivial launch found; resolve to a larger bidegree.");
        return;
    };

    let source_dim = res.differential(s).source().dimension(t);
    let target_dim = res.differential(s).target().dimension(t);
    eprintln!(
        "\nLargest launch: (s={s}, t={t})  rows={source_dim}  cols={target_dim}\n  {n_products} \
         products, {n_terms} element-terms"
    );

    let inputs: Vec<usize> = (0..source_dim).collect();
    let diff = res.differential(s);

    // Best-of-N wall time helper (borrows locals via &mut dyn).
    let best_of = |n: u32, f: &mut dyn FnMut()| -> f64 {
        let mut best = f64::INFINITY;
        for _ in 0..n {
            let t0 = Instant::now();
            f();
            best = best.min(t0.elapsed().as_secs_f64());
        }
        best
    };

    // CPU baseline: the real get_partial_matrix (parallel, per-term path).
    let cpu = best_of(n_runs, &mut || {
        let m = diff.get_partial_matrix(t, &inputs);
        std::hint::black_box(&m);
    });

    // GPU candidate, split into host extraction and the batch multiply itself.
    let num_rows = source_dim.max(1);
    let extract_time = best_of(n_runs, &mut || {
        let p = extract(s, t);
        std::hint::black_box(&p);
    });
    let products = extract(s, t);
    let batch_time = best_of(n_runs, &mut || {
        let out = multiply_batch_on_gpu(&algebra, target_dim, num_rows, &products);
        std::hint::black_box(&out);
    });
    let gpu = extract_time + batch_time;

    eprintln!("\n=== A/B (best of {n_runs}) ===");
    eprintln!("  CPU get_partial_matrix : {:8.3} ms", cpu * 1e3);
    eprintln!("  GPU extract (host)     : {:8.3} ms", extract_time * 1e3);
    eprintln!("  GPU batch multiply     : {:8.3} ms", batch_time * 1e3);
    eprintln!("  GPU total              : {:8.3} ms", gpu * 1e3);
    eprintln!("  speedup (CPU / GPU)    : {:8.2}×", cpu / gpu);
    if gpu < cpu {
        eprintln!("  → GPU wins: the invasive wire-in is justified.");
    } else {
        eprintln!("  → CPU wins at this scale on this device.");
    }
    eprintln!("(set MILNOR_GPU_TIMING=1 for the batch's marshal/device split)");
}
