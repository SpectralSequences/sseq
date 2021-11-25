use algebra::module::homomorphism::ModuleHomomorphism;
use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::resolution::SaveData;
use ext::utils::construct;

use std::path::{Path, PathBuf};

fn set_readonly(p: &Path, readonly: bool) {
    let mut perm = p.metadata().unwrap().permissions();
    perm.set_readonly(readonly);
    std::fs::set_permissions(p, perm).unwrap();
}

fn lock_tempdir(dir: &Path) {
    let mut dir: PathBuf = dir.into();
    for kind in SaveData::resolution_data() {
        dir.push(format!("{}s", kind.name()));
        set_readonly(&dir, true);
        dir.pop();
    }
    set_readonly(&dir, true);
}

fn unlock_tempdir(dir: &Path) {
    set_readonly(dir, false);

    let mut dir: PathBuf = dir.into();
    for kind in SaveData::resolution_data() {
        dir.push(format!("{}s", kind.name()));
        set_readonly(&dir, false);
        dir.pop();
    }
}

#[test]
#[should_panic]
fn test_tempdir_lock() {
    let tempdir = tempfile::TempDir::new().unwrap();
    let resolution1 = construct("S_2", Some(tempdir.path().into())).unwrap();
    resolution1.compute_through_bidegree(5, 5);

    lock_tempdir(tempdir.path());
    resolution1.compute_through_bidegree(6, 6);
}

#[test]
fn test_tempdir_unlock() {
    let tempdir = tempfile::TempDir::new().unwrap();
    let resolution1 = construct("S_2", Some(tempdir.path().into())).unwrap();
    resolution1.compute_through_bidegree(5, 5);

    lock_tempdir(tempdir.path());
    unlock_tempdir(tempdir.path());
    resolution1.compute_through_bidegree(6, 6);
}

#[test]
fn test_save_load() {
    let tempdir = tempfile::TempDir::new().unwrap();
    let mut resolution1 = construct("S_2", Some(tempdir.path().into())).unwrap();
    resolution1.compute_through_bidegree(10, 6);
    resolution1.compute_through_bidegree(6, 10);
    resolution1.should_save = false;

    let mut resolution2 = construct("S_2", Some(tempdir.path().into())).unwrap();

    // Check that we are not writing anything new.
    lock_tempdir(tempdir.path());
    resolution2.compute_through_bidegree(10, 6);
    resolution2.compute_through_bidegree(6, 10);

    resolution2.should_save = false;

    resolution1.compute_through_bidegree(20, 20);
    resolution2.compute_through_bidegree(20, 20);

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
fn test_save_load_stem() {
    let tempdir = tempfile::TempDir::new().unwrap();

    let resolution1 = construct("S_2", Some(tempdir.path().into())).unwrap();
    resolution1.compute_through_stem(10, 10);

    let resolution2 = construct("S_2", Some(tempdir.path().into())).unwrap();
    lock_tempdir(tempdir.path());
    resolution2.compute_through_stem(10, 10);

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

    let resolution1 = construct("S_2", Some(tempdir.path().into())).unwrap();
    resolution1.compute_through_stem(8, 14);

    let resolution2 = construct("S_2", Some(tempdir.path().into())).unwrap();
    lock_tempdir(tempdir.path());
    resolution2.compute_through_stem(8, 14);
    unlock_tempdir(tempdir.path());

    resolution1.compute_through_stem(5, 19);
    lock_tempdir(tempdir.path());
    resolution2.compute_through_stem(5, 19);

    assert_eq!(
        resolution1.graded_dimension_string(),
        resolution2.graded_dimension_string()
    );
}
