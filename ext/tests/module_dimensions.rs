use algebra::module::Module;
use ext::{
    chain_complex::{AugmentedChainComplex, ChainComplex},
    utils::construct,
};

/// CP^10_1 at p=2: only even degrees between 2 and 20 have dimension 1.
#[test]
fn cp_even_degrees_only() {
    let resolution = construct(("CP10", "adem"), None).unwrap();
    let module = resolution.target().module(0);

    assert_eq!(module.min_degree(), 2);
    for d in 0..=25 {
        let expected = if d >= 2 && d <= 20 && d % 2 == 0 { 1 } else { 0 };
        assert_eq!(
            module.dimension(d),
            expected,
            "CP10 dimension({d}) should be {expected}"
        );
    }
}

/// BCp at p=3, min=1: one element per degree >= 1.
#[test]
fn bcp_one_per_degree() {
    let resolution = construct(("BCp", "adem"), None).unwrap();
    let module = resolution.target().module(0);

    assert_eq!(module.min_degree(), 1);
    for d in -5..=20 {
        let expected = if d >= 1 { 1 } else { 0 };
        assert_eq!(
            module.dimension(d),
            expected,
            "BCp dimension({d}) should be {expected}"
        );
    }
}

/// CP^∞_1 at p=2: unbounded, dimension 1 at every even degree >= 2.
#[test]
fn cp_inf_dimensions() {
    let resolution = construct(("CP_inf", "adem"), None).unwrap();
    let module = resolution.target().module(0);

    assert_eq!(module.min_degree(), 2);
    for d in 0..=50 {
        let expected = if d >= 2 && d % 2 == 0 { 1 } else { 0 };
        assert_eq!(
            module.dimension(d),
            expected,
            "CP_inf dimension({d}) should be {expected}"
        );
    }
}

/// BCp2 at p=2, min=1: same as RP_inf, one per degree >= 1.
#[test]
fn bcp2_one_per_degree() {
    let resolution = construct(("BCp2", "adem"), None).unwrap();
    let module = resolution.target().module(0);

    assert_eq!(module.min_degree(), 1);
    for d in -5..=20 {
        let expected = if d >= 1 { 1 } else { 0 };
        assert_eq!(
            module.dimension(d),
            expected,
            "BCp2 dimension({d}) should be {expected}"
        );
    }
}
