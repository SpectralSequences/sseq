use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils::{construct_nassau, construct_standard};
use rstest::rstest;
use sseq::coordinates::Bidegree;

#[rstest]
#[trace]
#[case("S_2", 30)]
#[case("C2", 30)]
#[case("Joker", 30)]
#[case("Csigma", 30)]
fn compare(#[case] module_name: &str, #[case] max_degree: i32) {
    let max = Bidegree::s_t(max_degree as u32, max_degree);
    let a = construct_standard::<false, _, _>(module_name, None).unwrap();
    let b = construct_nassau(module_name, None).unwrap();

    a.compute_through_bidegree(max);
    b.compute_through_bidegree(max);

    assert_eq!(a.graded_dimension_string(), b.graded_dimension_string());
}
