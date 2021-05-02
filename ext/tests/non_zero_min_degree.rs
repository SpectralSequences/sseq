use algebra::module::Module;
use ext::chain_complex::ChainComplex;
use ext::utils::construct;

#[test]
fn negative_min_degree() {
    let resolution = construct(("S_2[-2]", "adem"), None).unwrap();
    assert_eq!(resolution.min_degree(), -2);
    assert_eq!(resolution.complex().module(0).dimension(-2), 1);
    assert_eq!(resolution.complex().module(0).dimension(0), 0);

    resolution.compute_through_bidegree(20, 20);
}

#[test]
fn positive_min_degree() {
    let resolution = construct(("S_2[2]", "adem"), None).unwrap();
    assert_eq!(resolution.min_degree(), 2);
    assert_eq!(resolution.complex().module(0).dimension(2), 1);
    assert_eq!(resolution.complex().module(0).dimension(0), 0);

    resolution.compute_through_bidegree(20, 20);
}
