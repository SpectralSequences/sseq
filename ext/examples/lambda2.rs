//! Computes tables for $S/\lambda^2$ (the cofiber of the Adams $d_2$ differential).
//!
//! This implements the four tables from "The Cofiber of the Adams d2 Differential":
//!
//! - **Table I**: B/Z/E decomposition of Ext, adapted to $d_2$.
//! - **Table II**: Products in Ext expressed in the standard basis.
//! - **Table III**: Conical basis for $\pi(S/\lambda^2)$.
//! - **Table IV**: Products in $\pi(S/\lambda^2)$ with commutators.
//!
//! # Usage
//! ```shell
//! cargo run --example lambda2 S_2 /tmp/save 40 20
//! ```
//!
//! Set `TABLE=I`, `TABLE=II`, `TABLE=III`, or `TABLE=IV` to print a single table; otherwise all
//! tables are printed. Supports sharding via `SECONDARY_JOB` (see [`secondary`](../secondary)).

use std::sync::Arc;

use algebra::module::Module;
use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    ext_algebra::{
        BZE, ExtAlgebra,
        secondary::SecondaryExtAlgebra,
    },
    secondary::LAMBDA_BIDEGREE,
    utils::query_module,
};
use sseq::coordinates::{Bidegree, BidegreeGenerator, MultiDegree};

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let table_var = std::env::var("TABLE").ok();
    let table = table_var.as_deref().unwrap_or("all");

    let resolution = Arc::new(query_module(Some(algebra::AlgebraType::Milnor), true)?);
    let e2 = Arc::new(ExtAlgebra::from_resolution(Arc::clone(&resolution))?);

    if !e2.is_unit() {
        let max = Bidegree::n_s(
            resolution.module(0).max_computed_degree(),
            resolution.next_homological_degree() - 1,
        );
        e2.unit().compute_through_stem(max);
    }

    let sec_e2 = Arc::new(SecondaryExtAlgebra::new(Arc::clone(&e2)));

    if let Some(s) = ext::utils::secondary_job() {
        sec_e2.compute_partial(s);
        return Ok(());
    }

    sec_e2.extend_all();

    match table {
        "I" => print_table_i(&e2, &sec_e2),
        "II" => print_table_ii(&e2, &sec_e2),
        "III" => print_table_iii(&e2, &sec_e2),
        "IV" => print_table_iv(&e2, &sec_e2),
        "all" => {
            println!("=== TABLE I: B/Z/E decomposition ===");
            print_table_i(&e2, &sec_e2);
            println!("\n=== TABLE II: Ext products ===");
            print_table_ii(&e2, &sec_e2);
            println!("\n=== TABLE III: Conical basis for π(S/λ²) ===");
            print_table_iii(&e2, &sec_e2);
            println!("\n=== TABLE IV: Products in π(S/λ²) ===");
            print_table_iv(&e2, &sec_e2);
        }
        _ => anyhow::bail!("unknown table: {table}; expected I, II, III, IV, or all"),
    }

    Ok(())
}

/// Table I: B/Z/E decomposition of Ext adapted to d2.
///
/// For each bidegree (n, s), classifies each Ext generator as:
/// - **Z** (d2-cycle, not a boundary) — survives to E3.
/// - **B** (boundary) — in the image of d2.
/// - **E** (supports d2) — d2(x) ≠ 0; prints the d2 value.
fn print_table_i<CC>(e2: &ExtAlgebra<CC>, sec_e2: &SecondaryExtAlgebra<CC>)
where
    CC: FreeChainComplex + ext::chain_complex::AugmentedChainComplex,
    CC::Algebra: algebra::pair_algebra::PairAlgebra,
{
    for b in e2.resolution().iter_nonzero_stem() {
        let dim = e2.dimension(b);
        for i in 0..dim {
            let g = BidegreeGenerator::new(b, i);
            match sec_e2.classify(g) {
                BZE::Z => println!("Z  x_{g}"),
                BZE::B => println!("B  x_{g}"),
                BZE::E => {
                    let elem = e2.generator(g);
                    if let Some(d2) = sec_e2.d2(&elem) {
                        let target: Vec<u32> = d2.vec().iter().collect();
                        println!("E  x_{g}  d2 = {target:?}");
                    } else {
                        println!("E  x_{g}  d2 = (not computed)");
                    }
                }
            }
        }
    }
}

/// Table II: Products in Ext expressed in the standard basis.
///
/// For each pair of generators (x, y) with x ≤ y (by bidegree then index), computes x · y.
fn print_table_ii<CC>(e2: &ExtAlgebra<CC>, _sec_e2: &SecondaryExtAlgebra<CC>)
where
    CC: FreeChainComplex + ext::chain_complex::AugmentedChainComplex,
    CC::Algebra: algebra::pair_algebra::PairAlgebra,
{
    let gens: Vec<BidegreeGenerator> = e2
        .resolution()
        .iter_nonzero_stem()
        .flat_map(|b| (0..e2.dimension(b)).map(move |i| BidegreeGenerator::new(b, i)))
        .collect();

    for (idx_x, &x_gen) in gens.iter().enumerate() {
        let x = e2.generator(x_gen);
        for &y_gen in &gens[idx_x..] {
            let y = e2.unit_generator(y_gen);
            if let Some(prod) = e2.try_multiply(&x, &y) {
                let coords: Vec<u32> = prod.vec().iter().collect();
                if coords.iter().any(|&c| c != 0) {
                    println!("x_{x_gen} · x_{y_gen} = {coords:?}");
                }
            }
        }
    }
}

/// Table III: Conical basis for π(S/λ²).
///
/// Lists the E3 = E∞ generators of S/λ² at each tridegree (n, s, bock), annotated with their
/// B/Z/E type. The conical basis Xπ has four element types per Condition 3.2 of the paper:
/// - x⁰_π for x ∈ B: (n, s, 0)
/// - x⁰_π for x ∈ Z: (n, s, 0)
/// - x¹_π for x ∈ Z: (n, s, 1)
/// - x¹_π for x ∈ E: (n, s, 1)
fn print_table_iii<CC>(e2: &ExtAlgebra<CC>, sec_e2: &SecondaryExtAlgebra<CC>)
where
    CC: FreeChainComplex + ext::chain_complex::AugmentedChainComplex,
    CC::Algebra: algebra::pair_algebra::PairAlgebra,
{
    for b in e2.resolution().iter_stem() {
        let [n, s] = b.coords();
        let dim = e2.dimension(b);
        if dim == 0 {
            continue;
        }

        // bock=0: surviving classes are B ⨿ Z (d2-cycles).
        // bock=1: surviving classes are Z ⨿ E (cokernel of d2).
        for bock in [0, 1] {
            let e3_dim = sec_e2.lambda2_e3_dimension(n, s, bock);
            if e3_dim == 0 {
                continue;
            }

            if let Some(pd) = sec_e2.lambda2_page_data(MultiDegree::new([n, s, bock])) {
                for (idx, v) in pd.gens().enumerate() {
                    let coords: Vec<u32> = v.iter().collect();
                    println!("({n}, {s}, {bock})  gen {idx}  {coords:?}");
                }
            }
        }
    }
}

/// Table IV: Products in π(S/λ²) with commutators.
///
/// For each pair (x, y) where x ∈ Z (surviving cycle) and y runs over E3-surviving classes of
/// the unit at each bidegree, computes the secondary product x · y. The secondary product lives
/// in Mod_{Cλ²}: it has an Ext part and a λ part.
///
/// The commutator x·y − y·x = 2·x·y when both stems are odd (Proposition 6.4).
fn print_table_iv<CC>(e2: &ExtAlgebra<CC>, sec_e2: &SecondaryExtAlgebra<CC>)
where
    CC: FreeChainComplex + ext::chain_complex::AugmentedChainComplex,
    CC::Algebra: algebra::pair_algebra::PairAlgebra,
{
    // Secondary products need s ≥ 1 for the multiplier (the secondary lift's shift is s+1,
    // and secondary homotopies start at s=2).
    let z_gens: Vec<BidegreeGenerator> = e2
        .resolution()
        .iter_nonzero_stem()
        .filter(|b| b.s() >= 1)
        .flat_map(|b| {
            let dim = e2.dimension(b);
            (0..dim)
                .filter(move |&i| sec_e2.classify(BidegreeGenerator::new(b, i)) == BZE::Z)
                .map(move |i| BidegreeGenerator::new(b, i))
        })
        .collect();

    for &x_gen in &z_gens {
        let x = e2.generator(x_gen);
        let shift = x.degree();

        for b in e2.unit().iter_nonzero_stem() {
            if !e2.resolution().has_computed_bidegree(b + shift + LAMBDA_BIDEGREE) {
                continue;
            }
            if !e2.resolution().has_computed_bidegree(b + shift - Bidegree::s_t(1, 0)) {
                continue;
            }

            let target_dim = e2.dimension(b + shift);
            let lambda_dim = e2.dimension(b + shift + LAMBDA_BIDEGREE);
            if target_dim == 0 && lambda_dim == 0 {
                continue;
            }

            for prod in sec_e2.secondary_multiply_into(&x, b) {
                let ext: Vec<u32> = prod.ext_part.iter().collect();
                let lambda: Vec<u32> = prod.lambda_part.iter().collect();
                if ext.iter().any(|&c| c != 0) || lambda.iter().any(|&c| c != 0) {
                    println!(
                        "x_{x_gen} · [{src}] = {ext:?} + λ {lambda:?}",
                        src = prod.source.to_basis_string(),
                    );
                }
            }
        }
    }
}
