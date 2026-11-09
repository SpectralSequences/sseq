//! Compares $\mathrm{Ext}_{A/\tau}(\mathbb{F}_2)$ computed from classical data with a direct resolution.
//!
//! Usage: `ctau_from_classical <max_n> <max_s> [xi|conjugate] [direct]`. Prints each (stem, s)
//! where the two disagree, then `n,s,w,dim` for every class.
use std::time::Instant;

use ext::algebraic_novikov::{Convention, ctau_e2, ctau_e2_direct};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let max_n: i32 = args[1].parse().unwrap();
    let max_s: i32 = args[2].parse().unwrap();
    let convention = match args.get(3).map(String::as_str) {
        Some("conjugate") => Convention::Conjugate,
        _ => Convention::Xi,
    };
    let start = Instant::now();
    let tri = ctau_e2(max_n, max_s, convention);
    eprintln!("Ext_P(Q): {:?}", start.elapsed());
    if args.get(4).map(String::as_str) == Some("direct") {
        let start = Instant::now();
        let direct = ctau_e2_direct(max_n, max_s);
        eprintln!("direct A/tau: {:?}", start.elapsed());
        let ours = tri.bigraded();
        let mut keys: Vec<_> = ours.keys().chain(direct.keys()).copied().collect();
        keys.sort();
        keys.dedup();
        let mut bad = 0;
        for k in keys {
            let (a, b) = (
                ours.get(&k).copied().unwrap_or(0),
                direct.get(&k).copied().unwrap_or(0),
            );
            if a != b {
                bad += 1;
                eprintln!("MISMATCH (n, s) = {k:?}: Ext_P(Q) {a}, direct {b}");
            }
        }
        eprintln!(
            "{bad} mismatches over {} classes",
            direct.values().sum::<usize>()
        );
    }
    for ((n, s, w), d) in &tri.0 {
        println!("{n},{s},{w},{d}");
    }
}
