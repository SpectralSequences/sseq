use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    utils::construct,
};
use sseq::coordinates::Bidegree;

#[test]
fn filtration_one_product() {
    let resolution = construct(("S_2", "milnor"), None).unwrap();
    resolution.compute_through_bidegree(Bidegree::s_t(8, 8));

    // op_deg 1 is Sq^1 at p=2; the algebra dimension in degree 1 is 1, so op_idx 0 is
    // valid and op_idx 999 is far out of range.

    // Stable out-of-range op_idx on a computed source is a caller error -> Err.
    let source = Bidegree::s_t(0, 0);
    assert!(resolution.has_computed_bidegree(source + Bidegree::s_t(1, 1)));
    assert!(
        resolution
            .try_filtration_one_product(1, 999, source)
            .is_err()
    );

    // A valid call succeeds.
    assert!(resolution.try_filtration_one_product(1, 0, source).is_ok());

    // Negative op_deg or source coordinates are rejected (rather than indexing modules/algebra
    // with a negative degree and panicking).
    assert!(
        resolution
            .try_filtration_one_product(-1, 0, source)
            .is_err()
    );
    assert!(
        resolution
            .try_filtration_one_product(1, 0, Bidegree::s_t(-1, 0))
            .is_err()
    );

    // The not-computed branch: a source whose target bidegree is far beyond what was
    // resolved is unavailable -> Err.
    let far = Bidegree::s_t(100, 100);
    assert!(!resolution.has_computed_bidegree(far + Bidegree::s_t(1, 1)));
    assert!(resolution.try_filtration_one_product(1, 0, far).is_err());

    // filtration_one_product discards the error: out-of-range and not-computed both give
    // None, while a valid call gives Some.
    assert_eq!(resolution.filtration_one_product(1, 999, source), None);
    assert_eq!(resolution.filtration_one_product(1, 0, far), None);
    assert!(resolution.filtration_one_product(1, 0, source).is_some());
}
