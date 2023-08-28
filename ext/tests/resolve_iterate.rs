use algebra::AlgebraType;
use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils::construct;
use ext::utils::load_module_json;
use rstest::rstest;
use sseq::coordinates::Bidegree;

#[rstest]
#[trace]
fn test_iterate(
    #[values("S_2", "S_3", "Ceta", "Calpha", "C3", "Joker")] module_name: &str,
    #[values(AlgebraType::Adem, AlgebraType::Milnor)] algebra: AlgebraType,
) {
    let json = load_module_json(module_name).unwrap();

    let first = construct((json.clone(), algebra), None).unwrap();
    let second = construct((json, algebra), None).unwrap();

    first.compute_through_bidegree(Bidegree::s_t(20, 20));

    second.compute_through_bidegree(Bidegree::s_t(0, 0));
    second.compute_through_bidegree(Bidegree::s_t(5, 5));
    second.compute_through_bidegree(Bidegree::s_t(10, 7));
    second.compute_through_bidegree(Bidegree::s_t(7, 10));
    second.compute_through_bidegree(Bidegree::s_t(18, 18));
    second.compute_through_bidegree(Bidegree::s_t(14, 14));
    second.compute_through_bidegree(Bidegree::s_t(15, 15));
    second.compute_through_bidegree(Bidegree::s_t(20, 20));

    assert_eq!(
        first.graded_dimension_string(),
        second.graded_dimension_string()
    );
}
