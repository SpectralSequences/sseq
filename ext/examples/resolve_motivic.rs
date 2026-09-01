//! The C-motivic Adams $E_2$ page (over $\mathbb{C}$, $p = 2$) by deformation.
//!
//! Resolves $A_C/\tau$ over $\mathbb{F}_2$, lifts to an honest $A_C$ resolution
//! over $\mathbb{F}_2[\tau]$, and reads the three pages of the one object off the
//! cohomology $H(\delta)$ (see [`ext::motivic`]):
//!
//! - **alg-Nov** — the algebraic Novikov $E_2$ (set $\tau = 0$: generator counts).
//! - **classical** — the classical Adams $E_2$ (invert $\tau$: free rank of $H(δ)$).
//! - **τ-torsion** — the $\tau$-torsion part of the motivic $E_2$ as an
//!   $\mathbb{F}_2[\tau]$-module (structure theorem): the orders of the
//!   $\mathbb{F}_2[\tau]/\tau^k$ summands, e.g. `τ` or `τ^2` or `τ+τ`. Empty when
//!   the bidegree is $\tau$-torsion-free. (A class is $\tau^k$-torsion when its own
//!   δ is $\tau^k$ times a cycle, so the $h_1$-tower torsion sits on $h_1^n$ itself
//!   — e.g. $h_1^4$ at $(4, 4)$.)
//!
//! Output is `n,s,alg_nov,classical,tau_torsion`, one line per nonzero bidegree.
//! Run with e.g. `cargo run --release --example resolve_motivic` (reads `Max n` /
//! `Max s`, defaulting to 12 / 8).

use ext::motivic::MotivicResolution;
use maybe_rayon::prelude::*;
use sseq::coordinates::Bidegree;

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let max = Bidegree::n_s(
        query::with_default("Max n", "12", str::parse),
        query::with_default("Max s", "8", str::parse),
    );

    // Set MOT_SAVE=<dir> to cache the resolution + lift to disk (and reload it on a
    // later run of the same box).
    let save_dir = std::env::var_os("MOT_SAVE").map(std::path::PathBuf::from);
    let res = MotivicResolution::with_module(MotivicResolution::trivial_module(), max, save_dir);

    let profile = std::env::var("MOT_PROFILE").is_ok();
    let t_coh = std::time::Instant::now();

    // The classical rank needs the lift one homological degree up, so the top
    // reported filtration is `max.s() - 1`.
    let top_s = max.s() - 1;

    // The cohomology at each (s, n) is independent — compute the chart in
    // parallel, then print in deterministic order.
    let cells: Vec<(i32, i32)> = (0..=top_s)
        .flat_map(|s| (0..=max.n()).map(move |n| (s, n)))
        .collect();
    let mut lines: Vec<(i32, i32, String)> = cells
        .into_maybe_par_iter()
        .filter_map(|(s, n)| {
            let t = n + s;
            let alg_nov = res.algebraic_novikov_rank(s, t);
            let module = res.tau_module(s, t);
            let classical = module.free;
            // The τ-torsion part: orders of the F₂[τ]/τ^k summands, e.g. "τ+τ^2".
            let torsion: String = module
                .torsion
                .iter()
                .map(|&k| {
                    if k == 1 {
                        "τ".to_string()
                    } else {
                        format!("τ^{k}")
                    }
                })
                .collect::<Vec<_>>()
                .join("+");
            (alg_nov > 0 || classical > 0 || !torsion.is_empty())
                .then(|| (s, n, format!("{n},{s},{alg_nov},{classical},{torsion}")))
        })
        .collect();
    lines.sort_by_key(|&(s, n, _)| (s, n));

    if profile {
        eprintln!("[profile] cohomology: {:?}", t_coh.elapsed());
    }

    println!("n,s,alg_nov,classical,tau_torsion");
    for (_, _, line) in lines {
        println!("{line}");
    }

    Ok(())
}
