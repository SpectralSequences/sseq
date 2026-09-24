//! The motivic Adams $E_2$ of the sphere as an $\mathbb{F}_2[\tau]$-algebra: the
//! full module structure, the hidden τ-extensions in products, and some triple
//! Massey products — everything the deformation computes over $\mathbb{F}_2[\tau]$.
//!
//! Prompts for `Max n` / `Max s` (default 20 / 12).

use ext::motivic::{Gen, MotivicResolution};
use sseq::coordinates::Bidegree;

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let max = Bidegree::n_s(
        query::with_default("Max n", "20", str::parse),
        query::with_default("Max s", "12", str::parse),
    );
    let res = MotivicResolution::new(max);
    let top_s = max.s() - 1;

    // 1. The motivic E₂ as an F₂[τ]-module at each bidegree (structure theorem):
    //    `free ⊕ ⊕ F₂[τ]/τ^k`, printed compactly (e.g. `2+τ` = F₂[τ]² ⊕ F₂[τ]/τ).
    println!("== motivic Adams E₂ as an F₂[τ]-module (n, s: module) ==");
    for s in 0..=top_s {
        for n in 0..=max.n() {
            let m = res.tau_module(s, n + s);
            if m.free > 0 || !m.torsion.is_empty() {
                println!("  ({n:>2}, {s:>2}): {m}");
            }
        }
    }

    // The filtration-1 Hopf generators hᵢ live at (2ⁱ−1, 1).
    let hopf: Vec<(i32, Gen)> = (0..)
        .map(|i| (1 << i) - 1)
        .take_while(|&n| n <= max.n())
        .map(|n| {
            (
                n,
                Gen {
                    s: 1,
                    t: n + 1,
                    idx: 0,
                },
            )
        })
        .filter(|(_, g)| res.algebraic_novikov_rank(g.s, g.t) > 0)
        .collect();
    let name = |g: Gen| -> String {
        hopf.iter()
            .find(|(_, h)| *h == g)
            .map(|(i, _)| format!("h{}", (*i + 1).trailing_zeros()))
            .unwrap_or_else(|| format!("x({},{})", g.t - g.s, g.s))
    };

    // 2. Hidden τ-extensions: products hᵢ·b that vanish mod τ but carry a τ-power.
    println!("\n== hidden τ-extensions in products hᵢ·(–) ==");
    let mut found = false;
    for (_, a) in &hopf {
        for s in 1..=top_s {
            for t in 0..=(max.n() + s) {
                for idx in 0..res.algebraic_novikov_rank(s, t) {
                    let b = Gen { s, t, idx };
                    for (g, power) in res.motivic_product(*a, b) {
                        if power > 0 {
                            println!(
                                "  {}·{} = τ{}·{}",
                                name(*a),
                                name(b),
                                if power == 1 {
                                    String::new()
                                } else {
                                    format!("^{power}")
                                },
                                name(g),
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

    // 3. A few triple Massey products ⟨a, b, c⟩ over F₂[τ] (with indeterminacy).
    println!("\n== triple Massey products ==");
    let h0 = Gen { s: 1, t: 1, idx: 0 };
    let h1 = Gen { s: 1, t: 2, idx: 0 };
    for (a, b, c) in [(h1, h0, h1), (h0, h1, h0)] {
        let coset = res.motivic_massey_coset(a, b, c);
        let terms: Vec<String> = coset
            .representative
            .iter()
            .map(|&(g, p)| {
                format!(
                    "τ{}·{}",
                    if p == 0 {
                        "⁰".into()
                    } else if p == 1 {
                        String::new()
                    } else {
                        format!("^{p}")
                    },
                    name(g)
                )
            })
            .collect();
        let val = if terms.is_empty() {
            "0".to_string()
        } else {
            terms.join(" + ")
        };
        println!(
            "  ⟨{}, {}, {}⟩ = {val}   (indeterminacy: {} gen(s){})",
            name(a),
            name(b),
            name(c),
            coset.indeterminacy.len(),
            if coset.is_zero { ", contains 0" } else { "" },
        );
    }

    Ok(())
}
