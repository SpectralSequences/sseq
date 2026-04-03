use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    utils::construct_standard,
};
use rstest::rstest;
use sseq::coordinates::Bidegree;

/// Compare a resolution with Nassau (via save directory) against one without.
///
/// When a save directory is provided and the module is eligible, Nassau's algorithm
/// is used automatically at runtime.
#[rstest]
#[trace]
#[case("S_2", 30)]
#[case("C2", 30)]
#[case("Joker", 30)]
#[case("Csigma", 30)]
fn compare(#[case] module_name: &str, #[case] max_degree: i32) {
    let max = Bidegree::s_t(max_degree, max_degree);

    // Without save dir: classical algorithm
    let classical = construct_standard::<false, _, _>(module_name, None).unwrap();

    // With save dir: Nassau's algorithm will be used if eligible
    let save_dir = tempfile::tempdir().unwrap();
    let nassau =
        construct_standard::<false, _, _>(module_name, Some(save_dir.path().to_owned())).unwrap();

    classical.compute_through_bidegree(max);
    nassau.compute_through_bidegree(max);

    assert_eq!(
        classical.graded_dimension_string(),
        nassau.graded_dimension_string()
    );
}
