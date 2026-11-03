//! Print the C-motivic Adams $E_2$ of a module over $A_C/\tau$, one bidegree per line.

use std::sync::Arc;

use ext::motivic::MotivicResolution;
use sseq::coordinates::Bidegree;

/// Print `n,s,alg_nov,classical,tau_torsion` for every nonzero bidegree.
///
/// The three columns are three readings of one $\mathbb{F}_2[\tau]$-module: setting $\tau = 0$
/// gives the algebraic Novikov $E_2$, inverting it the classical Adams $E_2$, and keeping it adds
/// the $\tau$-torsion summands, written `τ` or `τ^k` and joined by `+`.
fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let module: String = query::with_default("Module", "Ctau", |s| {
        Ok::<_, std::convert::Infallible>(s.to_owned())
    });
    let max = Bidegree::n_s(
        query::with_default("Max n", "12", str::parse),
        query::with_default("Max s", "8", str::parse),
    );

    let resolution = ext::utils::construct_standard::<false, _, _>(module.as_str(), None)?;
    let res = MotivicResolution::new(Arc::new(resolution), max)?;

    println!("n,s,alg_nov,classical,tau_torsion");
    // Ext at the top filtration would read generators one filtration higher.
    for s in 0..max.s() {
        for n in 0..=max.n() {
            let t = n + s;
            let alg_nov = res.algebraic_novikov_rank(s, t);
            let module = res.tau_module(s, t);
            let torsion = module
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
            if alg_nov > 0 || module.free > 0 || !torsion.is_empty() {
                println!("{n},{s},{alg_nov},{},{torsion}", module.free);
            }
        }
    }
    Ok(())
}
