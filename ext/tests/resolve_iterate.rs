use algebra::AlgebraType;
use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils::construct;
use ext::utils::load_module_json;
use rstest::rstest;

#[rstest]
#[trace]
fn test_iterate(
    #[values("S_2", "S_3", "Ceta", "Calpha", "C3", "Joker")] module_name: &str,
    #[values(AlgebraType::Adem, AlgebraType::Milnor)] algebra: AlgebraType,
) {
    let json = load_module_json(module_name).unwrap();

    let first = construct((json.clone(), algebra), None).unwrap();
    let second = construct((json, algebra), None).unwrap();

    first.compute_through_bidegree(20, 20);

    second.compute_through_bidegree(0, 0);
    second.compute_through_bidegree(5, 5);
    second.compute_through_bidegree(10, 7);
    second.compute_through_bidegree(7, 10);
    second.compute_through_bidegree(18, 18);
    second.compute_through_bidegree(14, 14);
    second.compute_through_bidegree(15, 15);
    second.compute_through_bidegree(20, 20);

    assert_eq!(
        first.graded_dimension_string(),
        second.graded_dimension_string()
    );
}
