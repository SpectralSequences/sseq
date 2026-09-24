//! Resolve a *non-trivial* module over $A_C/\tau$ and read its motivic Adams $E_2$:
//! the mod-2 Moore space $S/2$ (the cofiber of $2 = h_0$), whose two cells are
//! joined by $Q_0 = \mathrm{Sq}^1$.
//!
//! Demonstrates the `.json` module descriptor format over the motivic Steenrod
//! algebra ([`MotivicResolution::module_from_json`]) and
//! [`MotivicResolution::with_module`]: any module over $A_C/\tau$ flows through the
//! whole deformation pipeline — weights, the $A_C$ lift, the
//! $\mathbb{F}_2[\tau]$-module structure, products, Masseys. Because $2 = h_0$ is
//! coned off, the $h_0$-tower on the bottom cell is truncated (compare
//! `resolve_motivic`, where $h_0^n \ne 0$ for all $n$).
//!
//! The descriptor is the *standard* format: it lists only the actions of the
//! algebra generators ($Q_0$ and the $P(\xi_1^{2^k}) = \mathrm{Sq}^{2^{k+1}}$), and
//! the action of every composite operation is extended automatically (and
//! cross-checked against the Steenrod relations) — exactly as on the classical
//! side, courtesy of the `GeneratedAlgebra` implementation of $A_C/\tau$.
//!
//! Prompts for `Max n` / `Max s` (default 12 / 8). Set `MOT_SAVE=<dir>` to cache.

use ext::motivic::MotivicResolution;
use sseq::coordinates::Bidegree;

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let max = Bidegree::n_s(
        query::with_default("Max n", "12", str::parse),
        query::with_default("Max s", "8", str::parse),
    );

    // S/2 as a `.json` module descriptor: two cells x₀ (deg 0), x₁ (deg 1), joined
    // by Q₀ = Sq¹. Actions name motivic operations as the algebra prints them.
    let descriptor = serde_json::json!({
        "type": "finite dimensional module",
        "name": "S/2",
        "gens": { "x0": 0, "x1": 1 },
        "actions": ["Q_0 x0 = x1"],
    });
    let module = MotivicResolution::module_from_json(&descriptor)?;

    let save_dir = std::env::var_os("MOT_SAVE").map(std::path::PathBuf::from);
    let res = MotivicResolution::with_module(module, max, save_dir);

    println!("n,s,alg_nov,classical,tau_torsion");
    for s in 0..=(max.s() - 1) {
        for n in 0..=max.n() {
            let alg_nov = res.algebraic_novikov_rank(s, n + s);
            let m = res.tau_module(s, n + s);
            let torsion: String = m
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
            if alg_nov > 0 || m.free > 0 || !torsion.is_empty() {
                println!("{n},{s},{alg_nov},{},{torsion}", m.free);
            }
        }
    }

    Ok(())
}
