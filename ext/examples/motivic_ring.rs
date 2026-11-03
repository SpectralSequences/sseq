//! Print the C-motivic Adams $E_2$ of the sphere as an $\mathbb{F}_2[\tau]$-algebra.

use std::sync::Arc;

use ext::motivic::MotivicResolution;
use sseq::coordinates::{Bidegree, BidegreeGenerator};

/// Print the motivic Adams $E_2$ of `Ctau` as an $\mathbb{F}_2[\tau]$-module in each bidegree,
/// the hidden $\tau$-extensions in products by the Hopf elements, and two triple Massey products.
fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let max = Bidegree::n_s(
        query::with_default("Max n", "20", str::parse),
        query::with_default("Max s", "12", str::parse),
    );
    let resolution = ext::utils::construct_standard::<false, _, _>("Ctau", None)?;
    let mut res = MotivicResolution::new(Arc::new(resolution), max)?;

    println!("== the motivic Adams E₂ as an F₂[τ]-module (n, s: module) ==");
    // Ext at the top filtration would read generators one filtration higher.
    for s in 0..max.s() {
        for n in 0..=max.n() {
            let m = res.tau_module(s, n + s);
            if m.free > 0 || !m.torsion.is_empty() {
                println!("  ({n:>2}, {s:>2}): {m}");
            }
        }
    }

    let hopf: Vec<(u32, BidegreeGenerator)> = (0..)
        .map(|i| (i, BidegreeGenerator::n_s((1 << i) - 1, 1, 0)))
        .take_while(|(_, g)| g.n() <= max.n())
        .filter(|(_, g)| res.algebraic_novikov_rank(g.s(), g.t()) > 0)
        .collect();
    let name = |g: BidegreeGenerator| -> String {
        hopf.iter()
            .find(|(_, h)| *h == g)
            .map(|(i, _)| format!("h{i}"))
            .unwrap_or_else(|| format!("x({},{})", g.n(), g.s()))
    };
    let tau_power = |p: u32| match p {
        0 => String::new(),
        1 => "τ·".to_string(),
        p => format!("τ^{p}·"),
    };

    println!("\n== hidden τ-extensions in products by hᵢ ==");
    let mut found = false;
    for &(_, a) in &hopf {
        for s in 1..=max.s() - a.s() {
            for n in 0..=max.n() - a.n() {
                for idx in 0..res.algebraic_novikov_rank(s, n + s) {
                    let b = BidegreeGenerator::n_s(n, s, idx);
                    for (g, power) in res.motivic_product(a, b) {
                        if power > 0 {
                            println!(
                                "  {}·{} = {}{}",
                                name(a),
                                name(b),
                                tau_power(power),
                                name(g)
                            );
                            found = true;
                        }
                    }
                }
            }
        }
    }
    if !found {
        println!("  (none in this range)");
    }

    println!("\n== triple Massey products ==");
    let (h0, h1) = (hopf[0].1, hopf[1].1);
    for (a, b, c) in [(h1, h0, h1), (h0, h1, h0)] {
        let coset = res.motivic_massey_coset(a, b, c);
        let terms: Vec<String> = coset
            .representative
            .iter()
            .map(|&(g, p)| format!("{}{}", tau_power(p), name(g)))
            .collect();
        let value = if terms.is_empty() {
            "0".to_string()
        } else {
            terms.join(" + ")
        };
        println!(
            "  ⟨{}, {}, {}⟩ = {value}   (indeterminacy: {} generator(s){})",
            name(a),
            name(b),
            name(c),
            coset.indeterminacy.len(),
            if coset.is_zero { ", contains 0" } else { "" },
        );
    }

    Ok(())
}
