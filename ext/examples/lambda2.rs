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

use ext::{
    chain_complex::FreeChainComplex,
    ext_algebra::{ExtAlgebra, secondary::SecondaryExtAlgebra},
    secondary::LAMBDA_BIDEGREE,
    utils::query_module,
};
use fp::vector::FpVector;
use sseq::coordinates::{Bidegree, BidegreeElement, BidegreeGenerator};

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let table_var = std::env::var("TABLE").ok();
    let table = table_var.as_deref().unwrap_or("all");

    let resolution = Arc::new(query_module(Some(algebra::AlgebraType::Milnor), true)?);
    let e2 = Arc::new(ExtAlgebra::from_resolution(Arc::clone(&resolution))?);

    // The tables are defined for the sphere: Table II reads multiplicands from `e2.resolution()`
    // and feeds them to `e2.unit_generator(..)`, which is only the same basis when `M == k`. Bail
    // early on non-unit modules rather than print products from a mismatched basis.
    anyhow::ensure!(
        e2.is_unit(),
        "lambda2 only supports the unit module (e.g. S_2); got a non-unit module"
    );

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
/// For each bidegree (n, s), lists the classes of the decomposition — each expressed in the
/// original generator basis (a class that is a non-trivial combination is written out in full, not
/// named by a leading generator):
/// - **B** (boundary) — in the image of d2.
/// - **Z** (d2-cycle, not a boundary) — survives to E3.
/// - **E** (supports d2) — d2(x) ≠ 0; prints the d2 value, also in the original basis.
fn print_table_i<CC>(e2: &ExtAlgebra<CC>, sec_e2: &SecondaryExtAlgebra<CC>)
where
    CC: FreeChainComplex + ext::chain_complex::AugmentedChainComplex,
    CC::Algebra: algebra::pair_algebra::PairAlgebra,
{
    for b in e2.resolution().iter_nonzero_stem() {
        if e2.dimension(b) == 0 {
            continue;
        }
        for v in sec_e2.b_boundaries(b) {
            println!("B  {}", BidegreeElement::new(b, v).to_basis_string());
        }
        for (_, v) in sec_e2.z_cycles(b) {
            println!("Z  {}", BidegreeElement::new(b, v).to_basis_string());
        }
        for (_, v) in sec_e2.e_supports(b) {
            let elem = BidegreeElement::new(b, v);
            let d2str = match sec_e2.d2(&elem) {
                Some(d2) if !d2.vec().is_zero() => d2.to_basis_string(),
                Some(_) => "0".to_string(),
                None => "(not computed)".to_string(),
            };
            println!("E  {}  d2 = {}", elem.to_basis_string(), d2str);
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
        if e2.dimension(b) == 0 {
            continue;
        }
        for g in sec_e2.pi_basis(b) {
            let [n, s] = g.bidegree().coords();
            let bock = g.weight().as_i32();
            // `g` displays as "BZE  <combination in the original basis>^weight".
            println!("{g}  ({n}, {s}, {bock})");
        }
    }
}

/// Table IV: Products in π(S/λ²) with commutators.
///
/// For each pair (x, y) where x ∈ Z (surviving cycle) and y runs over E3-surviving classes of
/// the unit at each bidegree, computes the secondary product x · y and projects to π(S/λ²) via
/// `to_pi`. The result is expressed in the E3 subquotient at each weight.
///
/// The commutator [x, y] = x·y − y·x equals 2·(x·y) when both stems are odd (Proposition 6.4),
/// and vanishes when at least one stem is even.
fn print_table_iv<CC>(e2: &ExtAlgebra<CC>, sec_e2: &SecondaryExtAlgebra<CC>)
where
    CC: FreeChainComplex + ext::chain_complex::AugmentedChainComplex,
    CC::Algebra: algebra::pair_algebra::PairAlgebra,
{
    // The multiplier must be an actual d_2-cycle, not the generator `e_i` sharing its index: the
    // secondary lift only exists for cycles. See `SecondaryExtAlgebra::z_cycles`.
    let z_classes: Vec<(BidegreeGenerator, FpVector)> = e2
        .resolution()
        .iter_nonzero_stem()
        .filter(|b| b.s() >= 1)
        .flat_map(|b| sec_e2.z_cycles(b))
        .collect();

    for (x_gen, x_vec) in &z_classes {
        let x = BidegreeElement::new(x_gen.degree(), x_vec.clone());
        let shift = x.degree();

        for b in e2.unit().iter_nonzero_stem() {
            if !e2
                .resolution()
                .has_computed_bidegree(b + shift + LAMBDA_BIDEGREE)
            {
                continue;
            }
            if !e2
                .resolution()
                .has_computed_bidegree(b + shift - Bidegree::s_t(1, 0))
            {
                continue;
            }

            let target_dim = e2.dimension(b + shift);
            let lambda_dim = e2.dimension(b + shift + LAMBDA_BIDEGREE);
            if target_dim == 0 && lambda_dim == 0 {
                continue;
            }

            let both_odd = shift.n() % 2 != 0 && b.n() % 2 != 0;

            for prod in sec_e2.secondary_multiply_into(&x, b) {
                let (pi_ext, pi_lambda) = sec_e2.to_pi(&prod.value);
                if pi_ext.is_zero() && pi_lambda.is_zero() {
                    continue;
                }

                let comm = if both_odd {
                    "[x,y] = 2(x·y)"
                } else {
                    "[x,y] = 0"
                };

                let mut parts = Vec::new();
                if !pi_ext.is_zero() {
                    parts.push(format!("[{pi_ext}]"));
                }
                if !pi_lambda.is_zero() {
                    parts.push(format!("{pi_lambda}"));
                }
                let value_str = parts.join(" + ");
                println!(
                    "x_{x_gen} · [{src}] = {value_str}  {comm}",
                    src = prod.source.to_basis_string(),
                );
            }
        }
    }
}
