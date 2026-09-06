use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    utils::{construct_nassau, construct_standard},
};
use rstest::rstest;
use sseq::coordinates::Bidegree;

#[rstest]
#[trace]
#[case("S_2", 30)]
#[case("C2", 30)]
#[case("Joker", 30)]
#[case("Csigma", 30)]
fn compare(#[case] module_name: &str, #[case] max_degree: i32) {
    let max = Bidegree::s_t(max_degree, max_degree);
    let a = construct_standard::<false, _, _>(module_name, None).unwrap();
    let b = construct_nassau(module_name, None).unwrap();

    a.compute_through_bidegree(max);
    b.compute_through_bidegree(max);

    assert_eq!(a.graded_dimension_string(), b.graded_dimension_string());
}

/// Cross-check the parallel [`compute_through_stem`](ext::nassau::Resolution::compute_through_stem)
/// against the standard resolution over a wide stem range. This exercises the relaxed
/// (column-parallel) dependency graph, in which a whole column of internal degree is computed
/// concurrently.
#[rstest]
#[trace]
#[case("S_2", 40, 30)]
#[case("C2", 40, 30)]
#[case("Joker", 30, 20)]
// `max_n = 2^i - 1` puts a nonzero class (`h_i`, i.e. `Sq^{2^i}`) exactly on the `s = 1` stem edge
// `(1, max_n + 1)`, exercising the row-1 boundary of the wavefront where the diagonal predecessor
// `(0, max_n + 1)` lies outside the computed region.
#[case("S_2", 7, 6)]
#[case("S_2", 15, 8)]
fn compare_stem(#[case] module_name: &str, #[case] n: i32, #[case] s: i32) {
    let max = Bidegree::n_s(n, s);
    let a = construct_standard::<false, _, _>(module_name, None).unwrap();
    let b = construct_nassau(module_name, None).unwrap();

    a.compute_through_stem(max);
    b.compute_through_stem(max);

    assert_eq!(a.graded_dimension_string(), b.graded_dimension_string());
}
