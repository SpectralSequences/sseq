use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils::{construct, construct_standard};
use rstest::rstest;
use sseq::coordinates::Bidegree;

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
#[case("j", 30)]
#[case("j_mod_2", 30)]
#[case("ksp", 30)]
fn compare(#[case] module_name: &str, #[case] max_degree: i32) {
    let max = Bidegree::s_t(max_degree as u32, max_degree);
    let a = construct((module_name, "adem"), None).unwrap();
    let b = construct((module_name, "milnor"), None).unwrap();

    a.compute_through_bidegree(max);
    b.compute_through_bidegree(max);

    assert_eq!(a.graded_dimension_string(), b.graded_dimension_string());
}

#[rstest]
#[trace]
#[case("S_2[5]", 30)]
#[case("C2[8]", 30)]
#[case("RP4[4]", 30)]
#[case("RP_inf[7]", 30)]
#[case("Csigma[15]", 40)]
#[case("S_3[10]", 50)]
#[case("Calpha[15]", 50)]
fn compare_unstable(#[case] module_name: &str, #[case] max_degree: i32) {
    let max = Bidegree::s_t(max_degree as u32, max_degree);
    let a = construct_standard::<true, _, _>((module_name, "adem"), None).unwrap();
    let b = construct_standard::<true, _, _>((module_name, "milnor"), None).unwrap();

    a.compute_through_bidegree(max);
    b.compute_through_bidegree(max);

    assert_eq!(a.graded_dimension_string(), b.graded_dimension_string());
}
