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
    ext_algebra::{ExtAlgebra, secondary::SecondaryExtAlgebra},
    utils::query_module,
};
use fp::matrix::Subquotient;
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
        let page = sec_e2.page_data(b);
        let dim = e2.dimension(b);
        if dim == 0 {
            continue;
        }

        for i in 0..dim {
            let g = BidegreeGenerator::new(b, i);
            let class_type = classify_generator(&page, i);

            match class_type {
                BZE::Z => {
                    println!("Z  x_{g}");
                }
                BZE::B => {
                    println!("B  x_{g}");
                }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BZE {
    B,
    Z,
    E,
}

fn classify_generator(page: &Subquotient, idx: usize) -> BZE {
    // B: in the quotient (boundary)
    // Z: a gen (cycle, not boundary)
    // E: in the complement (supports d2)

    // Check if this generator index is a pivot of the quotient (boundary)
    if page.zeros().pivots()[idx] >= 0 {
        return BZE::B;
    }
    // Check if this generator index is in the complement (not a cycle)
    if page.complement_pivots().any(|p| p == idx) {
        return BZE::E;
    }
    // Otherwise it's a cycle that's not a boundary
    BZE::Z
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
/// Lists the E3 = E∞ generators of S/λ² at each tridegree (n, s, bock).
/// Elements of the conical basis Xπ come in four types per the paper's Condition 3.2:
/// - x⁰_π for x ∈ B: bockstein = 0
/// - x⁰_π for x ∈ Z: bockstein = 0
/// - x¹_π for x ∈ Z: bockstein = 1
/// - x¹_π for x ∈ E: bockstein = 1
fn print_table_iii<CC>(e2: &ExtAlgebra<CC>, sec_e2: &SecondaryExtAlgebra<CC>)
where
    CC: FreeChainComplex + ext::chain_complex::AugmentedChainComplex,
    CC::Algebra: algebra::pair_algebra::PairAlgebra,
{
    for b in e2.resolution().iter_stem() {
        let [n, s] = b.coords();

        for bock in [0, 1] {
            let dim = sec_e2.lambda2_e3_dimension(n, s, bock);
            if dim > 0 {
                if let Some(pd) = sec_e2.lambda2_page_data(MultiDegree::new([n, s, bock])) {
                    for (idx, v) in pd.gens().enumerate() {
                        let coords: Vec<u32> = v.iter().collect();
                        println!("({n}, {s}, {bock})  gen {idx}  {coords:?}");
                    }
                }
            }
        }
    }
}

/// Table IV: Products in π(S/λ²) with commutators.
///
/// For each pair (α, β) of conical basis elements with both in bockstein 0,
/// computes α · β using the secondary product and the commutator α·β - β·α.
fn print_table_iv<CC>(e2: &ExtAlgebra<CC>, sec_e2: &SecondaryExtAlgebra<CC>)
where
    CC: FreeChainComplex + ext::chain_complex::AugmentedChainComplex,
    CC::Algebra: algebra::pair_algebra::PairAlgebra,
{
    // Collect all surviving generators (Z classes) at bock=0 in the bigraded page.
    let survivors: Vec<BidegreeGenerator> = e2
        .resolution()
        .iter_nonzero_stem()
        .flat_map(|b| {
            let page = sec_e2.page_data(b);
            let dim = e2.dimension(b);
            (0..dim)
                .filter(move |&i| classify_generator(&page, i) != BZE::E)
                .map(move |i| BidegreeGenerator::new(b, i))
        })
        .collect();

    for (idx_x, &x_gen) in survivors.iter().enumerate() {
        let x = e2.generator(x_gen);
        // Only compute secondary products for d2-cycles (Z generators, not B).
        let x_page = sec_e2.page_data(x_gen.degree());
        if classify_generator(&x_page, x_gen.idx()) != BZE::Z {
            continue;
        }
        for &y_gen in &survivors[idx_x..] {
            // The secondary product x · y: iterate over the secondary multiply output.
            for prod in sec_e2.secondary_multiply_into(&x, y_gen.degree()) {
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
