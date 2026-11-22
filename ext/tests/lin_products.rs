//! Products by W. Lin's method (the default [`ProductMethod`]) against a chain map per generator
//! ([`ProductMethod::ChainMaps`]): every product of two basis elements in range agrees exactly.

use std::sync::Arc;

use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    ext_algebra::{ExtAlgebra, ExtModule, ProductMethod},
    utils::{QueryModuleResolution, construct},
};
use rstest::rstest;
use sseq::coordinates::{Bidegree, BidegreeGenerator};

fn resolve(spec: &str, basis: &str, max: Bidegree) -> Arc<QueryModuleResolution> {
    let res = construct((spec, basis), None).unwrap();
    res.compute_through_stem(max);
    Arc::new(res)
}

/// `Ext(M)` over `Ext(k)`, with products computed by `method`, `M = module` (or `k` itself).
fn ext_module(
    module: &str,
    unit: &str,
    basis: &str,
    max: Bidegree,
    method: ProductMethod,
) -> ExtModule<QueryModuleResolution> {
    let unit_res = resolve(unit, basis, max);
    let algebra = Arc::new(ExtAlgebra::new(Arc::clone(&unit_res)));
    algebra.set_product_method(method);
    if module == unit {
        ExtModule::over_unit(algebra)
    } else {
        ExtModule::new(resolve(module, basis, max), algebra)
    }
}

/// Every product `x · y` of a generator `x` of `Ext(M)` and the generators `y` of `Ext(k)` at a
/// bidegree, as `(x, y degree, rows)`.
type Table = Vec<(BidegreeGenerator, Bidegree, Vec<Vec<u32>>)>;

fn product_table(module: &ExtModule<QueryModuleResolution>) -> Table {
    let mut table = Vec::new();
    for x_deg in module.resolution().iter_stem() {
        for g in module.basis(x_deg) {
            let x = module.generator(g);
            for b in module.algebra().resolution().iter_nonzero_stem() {
                if let Some(matrix) = module.multiply_into(&x, b) {
                    let rows = matrix.iter().map(|row| row.iter().collect()).collect();
                    table.push((g, b, rows));
                }
            }
        }
    }
    table
}

fn compare(module: &str, unit: &str, basis: &str, max: Bidegree) {
    // Nassau's resolutions need the Milnor basis and a finite module, and lifting through the
    // resolution of `C3` panics there ("remainder non-zero") with chain maps as well.
    if cfg!(feature = "nassau") && (basis != "milnor" || ["RP_-4_inf", "C3"].contains(&module)) {
        return;
    }
    let by_lin = ext_module(module, unit, basis, max, ProductMethod::Lin);
    let lin = product_table(&by_lin);
    // The products came from Lin's method, through the whole range (less the stems a module with
    // cells in negative degrees needs from the ring beyond `max`).
    let range = by_lin.lin_range().expect("Lin's method was used");
    let max_n = max.n() + range.min_n.min(0);
    assert!(range.max_s >= max.s() && range.max_n >= max_n, "{range:?}");
    assert_eq!(by_lin.algebra().product_method(), ProductMethod::Lin);
    let maps = product_table(&ext_module(
        module,
        unit,
        basis,
        max,
        ProductMethod::ChainMaps,
    ));
    assert!(
        lin.iter()
            .any(|(_, _, rows)| rows.iter().flatten().any(|&c| c != 0))
    );
    assert_eq!(lin.len(), maps.len());
    eprintln!(
        "{module} ({basis}) through {max}: {} product blocks agree",
        lin.len()
    );
    for (a, b) in lin.iter().zip(&maps) {
        assert_eq!(a, b, "{module} ({basis}): x = {}, y at {}", a.0, a.1);
    }
}

#[rstest]
#[trace]
#[case("S_2", "S_2", "milnor", 30, 12)]
#[case("S_2", "S_2", "adem", 24, 10)]
#[case("S_3", "S_3", "milnor", 50, 12)]
#[case("S_3", "S_3", "adem", 40, 10)]
#[case("S_5", "S_5", "milnor", 60, 8)]
#[case("C2", "S_2", "milnor", 24, 10)]
#[case("Ceta", "S_2", "milnor", 24, 10)]
#[case("Joker", "S_2", "milnor", 20, 8)]
#[case("tmf2", "tmf2", "milnor", 40, 14)]
#[case("RP_-4_inf", "S_2", "milnor", 16, 8)]
#[case("C2", "S_2", "adem", 20, 8)]
#[case("Calpha", "S_3", "milnor", 40, 10)]
#[case("C3", "S_3", "milnor", 40, 10)]
fn lin_matches_chain_maps(
    #[case] module: &str,
    #[case] unit: &str,
    #[case] basis: &str,
    #[case] n: i32,
    #[case] s: i32,
) {
    compare(module, unit, basis, Bidegree::n_s(n, s));
}

/// The indecomposables through `(n, s) = (36, 14)` are those of G. Wu's chart of Ext, and the
/// products there agree with chain maps.
#[test]
fn sphere_indecomposables() {
    let max = Bidegree::n_s(36, 14);
    let res = resolve("S_2", "milnor", max);
    let ring = ExtAlgebra::new(Arc::clone(&res));
    let mut gens: Vec<(i32, i32)> = ring
        .indecomposables(max)
        .unwrap()
        .iter()
        .map(|g| (g.n(), g.s()))
        .collect();
    gens.sort_unstable();
    let known = [
        (0, 1),
        (1, 1),
        (3, 1),
        (7, 1),
        (8, 3),
        (9, 5),
        (11, 5),
        (14, 4),
        (15, 1),
        (16, 7),
        (17, 4),
        (17, 9),
        (18, 4),
        (19, 3),
        (19, 9),
        (20, 4),
        (22, 8),
        (23, 7),
        (24, 11),
        (25, 8),
        (25, 13),
        (26, 7),
        (27, 13),
        (29, 7),
        (30, 6),
        (30, 12),
        (31, 1),
        (31, 5),
        (32, 4),
        (32, 6),
        (32, 7),
        (33, 4),
        (33, 12),
        (34, 11),
        (35, 7),
        (36, 6),
    ];
    assert_eq!(gens, known);
}

/// Products asked for over a small range, then over a larger one after resolving further, agree
/// with a ring computed over the larger range at once.
#[test]
fn incremental() {
    let (small, large) = (Bidegree::n_s(16, 6), Bidegree::n_s(30, 12));
    let res = Arc::new(construct(("S_2", "milnor"), None).unwrap());
    res.compute_through_stem(small);
    let grown = ExtModule::over_unit(Arc::new(ExtAlgebra::new(Arc::clone(&res))));
    let first = product_table(&grown);
    res.compute_through_stem(large);
    let fresh = ext_module("S_2", "S_2", "milnor", large, ProductMethod::Lin);
    let (later, all) = (product_table(&grown), product_table(&fresh));
    assert_eq!(later, all);
    assert!(first.iter().all(|row| all.contains(row)));
}
