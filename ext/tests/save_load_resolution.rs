use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use algebra::module::homomorphism::ModuleHomomorphism;
use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    save::{SaveDirectory, SaveKind},
    secondary::{SecondaryLift, SecondaryResolution},
    utils::construct_standard,
};
use sseq::coordinates::Bidegree;

fn set_readonly(p: &Path, readonly: bool) {
    let mut perm = p.metadata().unwrap().permissions();
    perm.set_readonly(readonly);
    std::fs::set_permissions(p, perm).unwrap();
}

fn lock_tempdir(dir: &Path) {
    let mut dir: PathBuf = dir.into();
    for kind in SaveKind::resolution_data() {
        dir.push(format!("{}s", kind.name()));
        set_readonly(&dir, true);
        dir.pop();
    }
    set_readonly(&dir, true);
}

/// Should unlock after the test so that cleanup can be performed
fn unlock_tempdir(dir: &Path) {
    set_readonly(dir, false);

    let mut dir: PathBuf = dir.into();
    for kind in SaveKind::resolution_data() {
        dir.push(format!("{}s", kind.name()));
        set_readonly(&dir, false);
        dir.pop();
    }
}

#[test]
#[should_panic(expected = "Permission denied")]
fn test_tempdir_lock() {
    let tempdir = tempfile::TempDir::new().unwrap();
    let resolution1 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    resolution1.compute_through_bidegree(Bidegree::s_t(5, 5));

    lock_tempdir(tempdir.path());
    resolution1.compute_through_bidegree(Bidegree::s_t(6, 6));
}

#[test]
fn test_tempdir_unlock() {
    let tempdir = tempfile::TempDir::new().unwrap();
    let resolution1 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    resolution1.compute_through_bidegree(Bidegree::s_t(5, 5));

    lock_tempdir(tempdir.path());
    unlock_tempdir(tempdir.path());
    resolution1.compute_through_bidegree(Bidegree::s_t(6, 6));
}

#[test]
fn test_save_load() {
    let tempdir = tempfile::TempDir::new().unwrap();
    let mut resolution1 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();

    resolution1.compute_through_bidegree(Bidegree::s_t(10, 6));
    resolution1.compute_through_bidegree(Bidegree::s_t(6, 10));
    resolution1.should_save = false;

    let mut resolution2 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();

    // Check that we are not writing anything new.
    lock_tempdir(tempdir.path());
    resolution2.compute_through_bidegree(Bidegree::s_t(10, 6));
    resolution2.compute_through_bidegree(Bidegree::s_t(6, 10));

    resolution2.should_save = false;

    resolution1.compute_through_bidegree(Bidegree::s_t(20, 20));
    resolution2.compute_through_bidegree(Bidegree::s_t(20, 20));

    assert_eq!(
        resolution1.graded_dimension_string(),
        resolution2.graded_dimension_string()
    );

    assert_eq!(
        resolution1.differential(5).quasi_inverse(7),
        resolution2.differential(5).quasi_inverse(7)
    );
    unlock_tempdir(tempdir.path());
}

#[test]
#[should_panic(expected = "Invalid header: algebra was 0x20000 but expected 0x28000")]
fn wrong_algebra() {
    let tempdir = tempfile::TempDir::new().unwrap();
    let resolution1 =
        construct_standard::<false, _, _>("S_2@adem", Some(tempdir.path().into())).unwrap();
    resolution1.compute_through_bidegree(Bidegree::s_t(2, 2));

    let resolution2 =
        construct_standard::<false, _, _>("S_2@milnor", Some(tempdir.path().into())).unwrap();
    resolution2.compute_through_bidegree(Bidegree::s_t(2, 2));
}

#[test]
fn test_save_load_stem() {
    let tempdir = tempfile::TempDir::new().unwrap();

    let resolution1 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();

    resolution1.compute_through_stem(Bidegree::n_s(10, 10));

    let resolution2 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    lock_tempdir(tempdir.path());

    resolution2.compute_through_stem(Bidegree::n_s(10, 10));

    assert_eq!(
        resolution1.graded_dimension_string(),
        resolution2.graded_dimension_string()
    );

    assert_eq!(
        resolution1.differential(5).quasi_inverse(7),
        resolution2.differential(5).quasi_inverse(7)
    );
    unlock_tempdir(tempdir.path());
}

#[test]
fn test_save_load_resume() {
    let tempdir = tempfile::TempDir::new().unwrap();

    let resolution1 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    resolution1.compute_through_stem(Bidegree::n_s(14, 8));

    let resolution2 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    lock_tempdir(tempdir.path());
    resolution2.compute_through_stem(Bidegree::n_s(14, 8));
    unlock_tempdir(tempdir.path());

    resolution1.compute_through_stem(Bidegree::n_s(19, 5));
    lock_tempdir(tempdir.path());
    resolution2.compute_through_stem(Bidegree::n_s(19, 5));

    assert_eq!(
        resolution1.graded_dimension_string(),
        resolution2.graded_dimension_string()
    );
    unlock_tempdir(tempdir.path());
}

#[test]
fn test_save_load_split() {
    let tempdir_read = tempfile::TempDir::new().unwrap();
    let tempdir_write = tempfile::TempDir::new().unwrap();

    let resolution = construct_standard::<false, _, _>(
        "S_2",
        SaveDirectory::Combined(tempdir_read.path().into()),
    )
    .unwrap();
    resolution.compute_through_stem(Bidegree::n_s(14, 8));
    lock_tempdir(tempdir_read.path());

    let resolution = construct_standard::<false, _, _>(
        "S_2",
        SaveDirectory::Split {
            read: tempdir_read.path().into(),
            write: tempdir_write.path().into(),
        },
    )
    .unwrap();
    resolution.compute_through_stem(Bidegree::n_s(14, 8));

    let contains_only_dirs = |p: &Path| {
        p.read_dir().unwrap().all(|dir| {
            let dir = dir.unwrap();
            dir.file_type().unwrap().is_dir() && dir.path().read_dir().unwrap().next().is_none()
        })
    };

    assert!(contains_only_dirs(tempdir_write.path()));

    resolution.compute_through_stem(Bidegree::n_s(19, 5));

    assert!(!contains_only_dirs(tempdir_write.path()));
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

    let mut dir = tempdir.path().to_owned();
    let mut is_empty = |d| {
        dir.push(d);
        let result = dir.read_dir().unwrap().next().is_none();
        dir.pop();
        result
    };

    // Check that intermediates is non-empty
    assert!(!is_empty("secondary_intermediates"));

    lift1.compute_homotopies();

    assert!(is_empty("secondary_intermediates"));
    assert!(!is_empty("secondary_homotopys"));
    assert!(!is_empty("secondary_composites"));

    // Load the resolution and extend further
    let mut resolution2 =
        construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
    resolution2.load_quasi_inverse = false;
    resolution2.compute_through_stem(Bidegree::n_s(15, 8));

    let lift2 = SecondaryResolution::new(Arc::new(resolution2));
    lift2.initialize_homotopies();
    lift2.compute_composites();
    lift2.compute_homotopies();

    // Check that all intermediates are consumed
    assert!(is_empty("secondary_intermediates"));

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
fn test_checksum_early() {
    use std::{
        fs::OpenOptions,
        io::{Seek, SeekFrom, Write},
    };

    let tempdir = tempfile::TempDir::new().unwrap();

    construct_standard::<false, _, _>("S_2", Some(tempdir.path().into()))
        .unwrap()
        .compute_through_bidegree(Bidegree::s_t(2, 2));

    let mut path = tempdir.path().to_owned();
    path.push("differentials/2_2_differential");

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();

    file.seek(SeekFrom::Start(41)).unwrap();
    file.write_all(&[1]).unwrap();

    // Differentials are checked early for integrity, and silently replaced if they are malformed
    construct_standard::<false, _, _>("S_2", Some(tempdir.path().into()))
        .unwrap()
        .compute_through_bidegree(Bidegree::s_t(2, 2));
}

#[test]
#[should_panic(expected = "Error when deleting")]
fn test_checksum_early_locked() {
    use std::{
        fs::OpenOptions,
        io::{Seek, SeekFrom, Write},
    };

    let tempdir = tempfile::TempDir::new().unwrap();

    construct_standard::<false, _, _>("S_2", Some(tempdir.path().into()))
        .unwrap()
        .compute_through_bidegree(Bidegree::s_t(2, 2));

    let mut path = tempdir.path().to_owned();
    path.push("differentials/2_2_differential");

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();

    file.seek(SeekFrom::Start(41)).unwrap();
    file.write_all(&[1]).unwrap();

    lock_tempdir(tempdir.path());

    // This should try to delete the file and panic
    construct_standard::<false, _, _>("S_2", Some(tempdir.path().into()))
        .unwrap()
        .compute_through_bidegree(Bidegree::s_t(2, 2));
}

#[test]
#[should_panic(expected = "Invalid file checksum")]
fn test_checksum_late() {
    use std::{
        fs::OpenOptions,
        io::{Seek, SeekFrom, Write},
    };

    let tempdir = tempfile::TempDir::new().unwrap();

    construct_standard::<false, _, _>("S_2", Some(tempdir.path().into()))
        .unwrap()
        .compute_through_bidegree(Bidegree::s_t(2, 2));

    let mut path = tempdir.path().to_owned();
    path.push("res_qis/1_2_res_qi");

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();

    file.seek(SeekFrom::Start(41)).unwrap();
    file.write_all(&[1]).unwrap();

    // Quasi-inverses are checked after using them, and we panic if the check fails
    construct_standard::<false, _, _>("S_2", Some(tempdir.path().into()))
        .unwrap()
        .compute_through_bidegree(Bidegree::s_t(2, 2));
}
