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
    #[allow(clippy::redundant_clone)]
    let second = construct((json.clone(), algebra), None).unwrap();

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

    #[cfg(feature = "concurrent")]
    {
        let bucket = thread_token::TokenBucket::new(2);
        let third = construct((json, algebra), None).unwrap();

        third.compute_through_bidegree_concurrent(0, 0, &bucket);
        third.compute_through_bidegree_concurrent(5, 5, &bucket);
        third.compute_through_bidegree_concurrent(10, 7, &bucket);
        third.compute_through_bidegree_concurrent(7, 10, &bucket);
        third.compute_through_bidegree_concurrent(18, 18, &bucket);
        third.compute_through_bidegree_concurrent(14, 14, &bucket);
        third.compute_through_bidegree_concurrent(15, 15, &bucket);
        third.compute_through_bidegree_concurrent(20, 20, &bucket);

        assert_eq!(
            first.graded_dimension_string(),
            third.graded_dimension_string()
        );
    }
}
