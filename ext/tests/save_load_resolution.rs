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

/// Reusing an Adem-basis save directory with the Milnor basis must fail at `bind_to_algebra`.
#[test]
#[should_panic(expected = "different algebra")]
fn test_wrong_algebra() {
    let tempdir = tempfile::TempDir::new().unwrap();
    let resolution1 =
        construct_standard::<false, _, _>("S_2@adem", Some(tempdir.path().into())).unwrap();
    resolution1.compute_through_bidegree(Bidegree::s_t(2, 2));

    construct_standard::<false, _, _>("S_2@milnor", Some(tempdir.path().into())).unwrap();
}

/// Reusing a save directory for a different module over the same algebra must fail at the recorded
/// module spec.
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

/// A save records its data as zarr arrays and identifies its complex in the root group.
#[test]
fn test_zarr_store_exists() {
    let tempdir = tempfile::TempDir::new().unwrap();

    let mut resolution =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    // The name is recorded on the store as its label.
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

    // The root group records the module spec and the label next to the algebra binding.
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

/// `construct_from_save` rebuilds a `construct`ed complex from its directory alone and resumes the
/// saved data.
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

/// A write past an array's shape grows the shape before storing, so the array's metadata covers
/// every element in it.
#[test]
fn test_shard_array_grows() {
    use ext::save::{SaveKind, ZarrSaveStore};
    use sseq::coordinates::BidegreeGenerator;

    let tempdir = tempfile::TempDir::new().unwrap();
    let store = ZarrSaveStore::create(tempdir.path()).unwrap();
    let shape = || -> Vec<u64> {
        let meta =
            std::fs::read_to_string(tempdir.path().join("secondary_composite/zarr.json")).unwrap();
        let meta: serde_json::Value = serde_json::from_str(&meta).unwrap();
        serde_json::from_value(meta["shape"].clone()).unwrap()
    };

    let near = BidegreeGenerator::n_s(3, 2, 0);
    store
        .write(SaveKind::SecondaryComposite, near, b"near")
        .unwrap();
    let initial = shape();

    let far = BidegreeGenerator::n_s(3, 2, initial[2] as usize + 5);
    store
        .write(SaveKind::SecondaryComposite, far, b"far")
        .unwrap();
    let grown = shape();
    assert!(grown[2] > initial[2] + 5, "{initial:?} -> {grown:?}");
    assert_eq!(grown[..2], initial[..2]);

    // A second handle, opened from disk, reads both elements.
    let reopened = ZarrSaveStore::create(tempdir.path()).unwrap();
    for (g, data) in [(near, &b"near"[..]), (far, &b"far"[..])] {
        assert_eq!(
            reopened
                .read(SaveKind::SecondaryComposite, g)
                .unwrap()
                .as_deref(),
            Some(data)
        );
    }
}

/// Two handles on one subgroup share its shard locks, so writes through both to elements of one
/// shard are all kept, and every write leaves no temporary file behind.
#[test]
fn test_subgroup_handles_share_locks() {
    use ext::save::{SaveKind, ZarrSaveStore};

    let tempdir = tempfile::TempDir::new().unwrap();
    let root = ZarrSaveStore::create(tempdir.path()).unwrap();
    let handles = [
        root.subgroup("products/f").unwrap(),
        root.subgroup("products/f").unwrap(),
    ];
    // Stems 0..8 and filtrations 0..8 fall in one shard.
    let cells: Vec<Bidegree> = (0..8)
        .flat_map(|n| (0..8).map(move |s| Bidegree::n_s(n, s)))
        .collect();

    std::thread::scope(|scope| {
        for (i, handle) in handles.iter().enumerate() {
            let cells = &cells;
            scope.spawn(move || {
                for (j, &b) in cells.iter().enumerate() {
                    if j % 2 == i {
                        handle
                            .write(SaveKind::ChainMap, b, format!("{b}").as_bytes())
                            .unwrap();
                    }
                }
            });
        }
    });

    for &b in &cells {
        assert_eq!(
            handles[0].read(SaveKind::ChainMap, b).unwrap(),
            Some(format!("{b}").into_bytes()),
            "lost the write at {b}"
        );
    }

    let mut stack = vec![tempdir.path().to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            assert!(
                !path.to_string_lossy().ends_with(".tmp"),
                "left a temporary file: {path:?}"
            );
            if path.is_dir() {
                stack.push(path);
            }
        }
    }
}

/// A finished quasi-inverse missing one of its chunks is an error, not rows of zeros.
#[test]
fn test_res_qi_missing_chunk() {
    use ext::save::ZarrSaveStore;
    use fp::prime::ValidPrime;

    let tempdir = tempfile::TempDir::new().unwrap();
    let resolution = construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    resolution.compute_through_stem(Bidegree::n_s(6, 4));
    drop(resolution);

    // Pick a saved quasi-inverse with at least one row.
    let qi_dir = tempdir.path().join("qi");
    let (b, chunk) = std::fs::read_dir(&qi_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .find_map(|name| {
            let chunk = qi_dir.join(&name).join("res_qi/rows/c/0/0");
            let (n, s) = name.strip_prefix('n')?.split_once("_s")?;
            chunk
                .exists()
                .then(|| (Bidegree::n_s(n.parse().unwrap(), s.parse().unwrap()), chunk))
        })
        .expect("no saved quasi-inverse has rows");

    let store = ZarrSaveStore::create(tempdir.path()).unwrap();
    let p = ValidPrime::new(2);
    assert!(
        store
            .stream_res_qi(b, p)
            .unwrap()
            .unwrap()
            .into_quasi_inverse()
            .is_ok()
    );

    std::fs::remove_file(chunk).unwrap();
    let err = store
        .stream_res_qi(b, p)
        .unwrap()
        .unwrap()
        .into_quasi_inverse()
        .unwrap_err();
    assert!(err.to_string().contains("missing"), "{err:#}");
}
