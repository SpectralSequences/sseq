use ext::chain_complex::FreeChainComplex;
use ext::utils::construct;

#[test]
fn compare() {
    let a = construct("S_2", None).unwrap();
    let b = ext::nassau::Resolution::new(None);

    a.compute_through_stem(30, 7);
    b.compute_through_stem(30, 7);

    assert_eq!(a.graded_dimension_string(), b.graded_dimension_string());
}
