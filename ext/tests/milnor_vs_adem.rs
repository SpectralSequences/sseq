use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils::construct;
use rstest::rstest;

#[rstest]
#[trace]
#[case("S_2", 30)]
#[case("C2", 30)]
#[case("Joker", 30)]
#[case("RP4", 30)]
#[case("RP_inf", 30)]
#[case("RP_-4_inf", 30)]
#[case("Csigma", 30)]
#[case("S_3", 30)]
#[case("Calpha", 30)]
fn compare(#[case] module_name: &str, #[case] max_degree: i32) {
    let a = construct((module_name, "adem"), None).unwrap();
    let b = construct((module_name, "milnor"), None).unwrap();

    #[cfg(feature = "concurrent")]
    {
        let bucket = thread_token::TokenBucket::new(2.try_into().unwrap());
        a.compute_through_bidegree_concurrent(max_degree as u32, max_degree, &bucket);
        b.compute_through_bidegree_concurrent(max_degree as u32, max_degree, &bucket);
    }

    #[cfg(not(feature = "concurrent"))]
    {
        a.compute_through_bidegree(max_degree as u32, max_degree);
        b.compute_through_bidegree(max_degree as u32, max_degree);
    }

    assert_eq!(a.graded_dimension_string(), b.graded_dimension_string());
}
