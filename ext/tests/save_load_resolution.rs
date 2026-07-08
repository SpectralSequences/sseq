use std::sync::Arc;

use algebra::module::homomorphism::ModuleHomomorphism;
use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    secondary::{SecondaryLift, SecondaryResolution},
    utils::construct_standard,
};
use sseq::coordinates::Bidegree;

#[test]
fn test_save_load() {
    let tempdir = tempfile::TempDir::new().unwrap();
    let mut resolution1 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();

    resolution1.compute_through_bidegree(Bidegree::s_t(10, 6));
    resolution1.compute_through_bidegree(Bidegree::s_t(6, 10));
    resolution1.should_save = false;

    let resolution2 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();

    resolution2.compute_through_bidegree(Bidegree::s_t(10, 6));
    resolution2.compute_through_bidegree(Bidegree::s_t(6, 10));

    assert_eq!(
        resolution1.graded_dimension_string(),
        resolution2.graded_dimension_string()
    );

    assert_eq!(
        resolution1.differential(5).quasi_inverse(7),
        resolution2.differential(5).quasi_inverse(7)
    );
}

/// Resolving once with the Adem algebra and then attempting to reuse the same save dir with the
/// Milnor algebra must fail loudly via the `bind_to_algebra` check, not silently mix data from
/// the two bases. Resurrected from the pre-zarr `wrong_algebra` test in master.
#[test]
#[should_panic(expected = "different algebra")]
fn test_wrong_algebra() {
    let tempdir = tempfile::TempDir::new().unwrap();
    let resolution1 =
        construct_standard::<false, _, _>("S_2@adem", Some(tempdir.path().into())).unwrap();
    resolution1.compute_through_bidegree(Bidegree::s_t(2, 2));

    construct_standard::<false, _, _>("S_2@milnor", Some(tempdir.path().into())).unwrap();
}

#[test]
fn test_save_load_stem() {
    let tempdir = tempfile::TempDir::new().unwrap();

    let resolution1 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();

    resolution1.compute_through_stem(Bidegree::n_s(10, 10));

    let resolution2 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();

    resolution2.compute_through_stem(Bidegree::n_s(10, 10));

    assert_eq!(
        resolution1.graded_dimension_string(),
        resolution2.graded_dimension_string()
    );

    assert_eq!(
        resolution1.differential(5).quasi_inverse(7),
        resolution2.differential(5).quasi_inverse(7)
    );
}

#[test]
fn test_save_load_resume() {
    let tempdir = tempfile::TempDir::new().unwrap();

    let resolution1 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    resolution1.compute_through_stem(Bidegree::n_s(14, 8));

    let resolution2 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    resolution2.compute_through_stem(Bidegree::n_s(14, 8));

    resolution1.compute_through_stem(Bidegree::n_s(19, 5));
    resolution2.compute_through_stem(Bidegree::n_s(19, 5));

    assert_eq!(
        resolution1.graded_dimension_string(),
        resolution2.graded_dimension_string()
    );
}

#[test]
fn test_load_smaller() {
    let tempdir = tempfile::TempDir::new().unwrap();

    let resolution1 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    resolution1.compute_through_stem(Bidegree::n_s(14, 8));

    let resolution2 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    resolution2.compute_through_stem(Bidegree::n_s(8, 5));
}

#[test]
fn test_load_secondary() {
    let tempdir = tempfile::TempDir::new().unwrap();

    let mut resolution1 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    resolution1.load_quasi_inverse = false;
    resolution1.compute_through_stem(Bidegree::n_s(10, 4));

    let lift1 = SecondaryResolution::new(Arc::new(resolution1));
    lift1.initialize_homotopies();
    lift1.compute_composites();
    lift1.compute_intermediates();
    lift1.compute_homotopies();

    // Load the resolution and extend further
    let mut resolution2 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    resolution2.load_quasi_inverse = false;
    resolution2.compute_through_stem(Bidegree::n_s(15, 8));

    let lift2 = SecondaryResolution::new(Arc::new(resolution2));
    lift2.initialize_homotopies();
    lift2.compute_composites();
    lift2.compute_homotopies();

    // Check that we have correct result
    assert_eq!(lift2.homotopy(3).homotopies.hom_k(16), vec![vec![1]]);

    // Now try to load a smaller resolution
    let mut resolution3 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    resolution3.load_quasi_inverse = false;
    resolution3.compute_through_stem(Bidegree::n_s(12, 5));

    let lift3 = SecondaryResolution::new(Arc::new(resolution3));
    lift3.initialize_homotopies();
    lift3.compute_composites();
    lift3.compute_homotopies();
}

#[test]
fn test_zarr_store_exists() {
    let tempdir = tempfile::TempDir::new().unwrap();

    construct_standard::<false, _, _>("S_2", Some(tempdir.path().into()))
        .unwrap()
        .compute_through_bidegree(Bidegree::s_t(3, 3));

    // Verify data was stored in zarr format
    let store_path = tempdir.path();
    assert!(
        store_path.join("zarr.json").exists(),
        "zarr root group missing"
    );
    assert!(
        store_path.join("differential/zarr.json").exists(),
        "shard-tier differential array missing"
    );
}
