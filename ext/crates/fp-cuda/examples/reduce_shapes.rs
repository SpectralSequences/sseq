//! Device vs CPU reduction across shapes, from near-square to very wide.
//!
//! `reduce_timing` measures half-rank squares only, and a square cannot separate the two candidate
//! quantities for the dispatch gate: its short side and its total size move together. This example
//! varies them independently, which is what [`fp::blas`]'s size-based gate was chosen against. The
//! numbers it produced, and the shapes the resolution actually reduces, are in
//! `crates/fp-cuda/EXPERIMENTS.md`.
//!
//! The CPU baseline is `row_reduce_cpu`, the M4RI reduction the gate actually falls back to, so a
//! ratio here is the speedup against what production really runs.
//!
//! ```sh
//! cargo run --release -p fp-cuda --example reduce_shapes            # census shapes + controls
//! cargo run --release -p fp-cuda --example reduce_shapes -- 2877x1622037
//! ```

use std::time::Instant;

use fp_cuda::GpuContext;

mod common;
use common::{half_rank, upload_matrix};

/// The dispatch gate this example exists to justify, mirrored from `fp`'s `DEFAULT_RR_MIN_WORK`.
///
/// `fp` is a dev-dependency here and the predicate is crate-private, so the value cannot be read
/// from it. Keep the two in step.
const GATE_WORK: u64 = 100_000_000_000;

/// Time each shape on the device and on the CPU, and print the ratio.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let shapes: Vec<(usize, usize, &'static str)> = if args.is_empty() {
        vec![
            // Square controls, the shape earlier measurements used.
            (4096, 4096, "square control"),
            (8192, 8192, "square control"),
            // Wide shapes taken from the census, cheapest first.
            (587, 1_524_934, "b=(287,5)"),
            (657, 1_692_321, "b=(293,5)"),
            (1131, 611_461, "b=(255,9)"),
            (1676, 1_686_395, "b=(266,7)"),
            (2877, 1_622_037, "b=(253,10)"),
            (3055, 1_770_153, "b=(262,8)"),
        ]
    } else {
        // Reject a malformed shape rather than skipping it: silently dropping one turns a typo
        // into a short run that looks like it measured what was asked for.
        args.iter()
            .map(|a| {
                let parsed = a.split_once('x').and_then(|(r, c)| {
                    Some((r.trim().parse().ok()?, c.trim().parse().ok()?, "user"))
                });
                parsed.unwrap_or_else(|| {
                    eprintln!("not a shape: {a:?} (expected ROWSxCOLS, e.g. 2877x1622037)");
                    std::process::exit(2);
                })
            })
            .collect()
    };

    let gpu = GpuContext::new(0)?;
    println!("=== device vs CPU reduction, half-rank ===");
    println!("  The CPU baseline is M4RI, the path the gate falls back to.\n");
    println!(
        "  {:>6} {:>10} {:>8} {:>8} {:>10} {:>10} {:>9} {:>7}  shape",
        "rows", "cols", "aspect", "MB", "device", "cpu-m4ri", "speedup", "gated?"
    );

    for (rows, cols, label) in shapes {
        let mm = half_rank(rows, cols);
        let mb = rows as f64 * cols as f64 / 8.0 / (1 << 20) as f64;
        let aspect = cols as f64 / rows as f64;
        let rank = (rows.min(cols) / 2) as u64;
        let gated = if rank * rank * cols as u64 >= GATE_WORK {
            "GPU"
        } else {
            "CPU"
        };

        // Device: upload + full reduce + sync, excluding download (matches reduce_timing).
        let t0 = Instant::now();
        let mut dm = upload_matrix(&gpu, &mm)?;
        let (_perm, r, _piv) = gpu.row_reduce_dev(&mut dm)?;
        let dev = t0.elapsed().as_secs_f64();

        let mut cpu = mm.clone();
        let t1 = Instant::now();
        let cpu_rank = cpu.row_reduce_cpu();
        let cpu_s = t1.elapsed().as_secs_f64();

        // Rank agreement is the cheap invariant. Materialising the full device RREF for a 645 MB
        // matrix would cost more than the measurement; `reduce_timing` does the full compare on
        // squares, and a rank mismatch is what a broken reduce actually produces.
        let ok = r == cpu_rank;

        println!(
            "  {rows:>6} {cols:>10} {aspect:>7.0}x {mb:>8.1} {dev:>9.3}s {cpu_s:>9.3}s {:>8.2}x \
             {gated:>7}  {label}{}",
            cpu_s / dev,
            if ok { "" } else { "   *** RANK MISMATCH ***" }
        );
        if !ok {
            eprintln!("rank mismatch: device {r} vs cpu {cpu_rank} at {rows}x{cols}");
            std::process::exit(1);
        }
    }
    println!(
        "\n  Read the `gated?` column against `speedup`: any row marked CPU with a speedup \
         above\n  1.00x is work the current threshold sends to the slower path."
    );
    Ok(())
}
