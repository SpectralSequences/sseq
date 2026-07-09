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

/// Resolving one module and then reusing the same save dir for a *different* module over the same
/// algebra must fail loudly via the recorded module-spec check, rather than silently loading the
/// first module's cached differentials for the second.
#[test]
#[should_panic(expected = "different complex")]
fn test_wrong_complex() {
    let tempdir = tempfile::TempDir::new().unwrap();
    let resolution1 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    resolution1.compute_through_bidegree(Bidegree::s_t(2, 2));

    // Same Steenrod algebra at p = 2, but a different module (the mod-2 Moore complex).
    construct_standard::<false, _, _>("C2", Some(tempdir.path().into())).unwrap();
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

    let mut resolution =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    // Naming the resolution records a human-readable label on the store, mirroring how the CLI
    // labels its resolutions.
    resolution.set_name("S_2".into());
    resolution.compute_through_bidegree(Bidegree::s_t(3, 3));

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

    // The store is self-describing: the module spec (the identity gate + reconstruction source)
    // and the human-readable label are both recorded on the root group, alongside the algebra
    // binding.
    let root_meta = std::fs::read_to_string(store_path.join("zarr.json")).unwrap();
    assert!(
        root_meta.contains("module_spec"),
        "root group should record the module spec; got: {root_meta}"
    );
    assert!(
        root_meta.contains("complex_name") && root_meta.contains("S_2"),
        "root group should record the complex name; got: {root_meta}"
    );
    assert!(
        root_meta.contains("algebra_magic"),
        "recording the spec must not drop the algebra binding; got: {root_meta}"
    );
}

/// A save created through `construct` is self-describing: `construct_from_save` can rebuild the
/// complex from the directory alone, without the caller re-supplying the `"S_2"` spec, and the
/// reconstructed resolution resumes the cached data.
#[test]
fn test_construct_from_save() {
    use ext::utils::{construct_from_save, construct_standard};

    let tempdir = tempfile::TempDir::new().unwrap();

    let resolution1 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    resolution1.compute_through_stem(Bidegree::n_s(10, 6));

    // Reopen the directory without telling the code what module it holds.
    let resolution2 = construct_from_save(tempdir.path()).unwrap();
    resolution2.compute_through_stem(Bidegree::n_s(10, 6));

    assert_eq!(
        resolution1.graded_dimension_string(),
        resolution2.graded_dimension_string()
    );

    // Extending past the saved range still works, confirming a genuine resumable resolution
    // rather than a read-only view.
    resolution2.compute_through_stem(Bidegree::n_s(14, 8));
}

/// Pointing `construct_from_save` at a directory that was never populated (no recorded spec) is a
/// clean error, not a panic.
#[test]
fn test_construct_from_save_missing_spec() {
    use ext::utils::construct_from_save;

    let tempdir = tempfile::TempDir::new().unwrap();
    assert!(construct_from_save(tempdir.path()).is_err());
}
