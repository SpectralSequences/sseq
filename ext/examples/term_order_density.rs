//! Experimental: resolve the sphere with the term order given by `SSEQ_TERM_ORDER` and report
//! the number of terms in the differentials, before and after a greedy reduction by boundaries.
//! Also checks that d^2 = 0. Range is set by the environment variables `S` and `T`.

use algebra::module::{Module, homomorphism::ModuleHomomorphism};
use ext::chain_complex::ChainComplex;
use fp::{matrix::Subspace, vector::FpVector};
use sseq::coordinates::Bidegree;

fn main() {
    let max_s: i32 = std::env::var("S").unwrap().parse().unwrap();
    let max_t: i32 = std::env::var("T").unwrap().parse().unwrap();
    let res = ext::utils::construct("S_2@milnor", None).unwrap();
    let start = std::time::Instant::now();
    res.compute_through_bidegree(Bidegree::s_t(max_s, max_t));
    let elapsed = start.elapsed();
    let (mut terms, mut greedy, mut gens, mut sig) = (0usize, 0usize, 0usize, 0u64);
    for s in 1..=max_s {
        let d = res.differential(s);
        for t in 0..=max_t {
            let src = res.module(s);
            let ng = src.number_of_gens_in_degree(t);
            sig = sig.wrapping_mul(31).wrapping_add(ng as u64);
            if ng == 0 {
                continue;
            }
            let old: Vec<usize> = (0..src.dimension(t) - ng).collect();
            let img = Subspace::from_matrix(d.get_partial_matrix(t, &old));
            let basis: Vec<FpVector> = img.basis().map(|x| x.to_owned()).collect();
            for i in 0..ng {
                let mut v = d.output(t, i).clone();
                // d∘d = 0
                if s > 1 {
                    let dd = res.differential(s - 1);
                    let mut w = FpVector::new(fp::prime::TWO, res.module(s - 2).dimension(t));
                    dd.apply(w.as_slice_mut(), 1, t, v.as_slice());
                    assert!(w.is_zero(), "d^2 != 0 at s={s} t={t}");
                }
                terms += v.iter_nonzero().count();
                loop {
                    let mut improved = false;
                    for b in &basis {
                        let mut w = v.clone();
                        w.add(b, 1);
                        if w.iter_nonzero().count() < v.iter_nonzero().count() {
                            v = w;
                            improved = true;
                        }
                    }
                    if !improved {
                        break;
                    }
                }
                greedy += v.iter_nonzero().count();
                gens += 1;
            }
        }
    }
    println!(
        "terms {terms} greedy {greedy} gens {gens} sig {sig:x} time {:.1}s",
        elapsed.as_secs_f64()
    );
}
