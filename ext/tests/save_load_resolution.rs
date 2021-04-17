use algebra::module::homomorphism::ModuleHomomorphism;
use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::{resolution::Resolution, utils::construct_from_json};
use saveload::{Load, Save};
use serde_json::json;
use std::io::{Cursor, Read, Seek, SeekFrom};

#[test]
fn test_save_load() {
    let mut json = json!({
        "type": "finite dimensional module",
        "p": 2,
        "gens": {"x0": 0},
        "actions": []
    });

    let resolution1 = construct_from_json(&mut json, "adem").unwrap();
    resolution1.compute_through_bidegree(10, 6);
    resolution1.compute_through_bidegree(6, 10);

    let mut cursor: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    resolution1.save(&mut cursor).unwrap();

    cursor.seek(SeekFrom::Start(0)).unwrap();

    let resolution2 = Resolution::load(&mut cursor, &resolution1.complex()).unwrap();
    assert_eq!(0, cursor.bytes().count());

    assert_eq!(
        resolution1.graded_dimension_string(),
        resolution2.graded_dimension_string()
    );

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
    let mut json = json!({
        "type": "finite dimensional module",
        "p": 2,
        "gens": {"x0": 0},
        "actions": []
    });

    let resolution1 = construct_from_json(&mut json, "adem").unwrap();
    resolution1.compute_through_stem(10, 10);
    resolution1.compute_through_stem(10, 10);

    let mut cursor: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    resolution1.save(&mut cursor).unwrap();

    cursor.seek(SeekFrom::Start(0)).unwrap();

    let resolution2 = Resolution::load(&mut cursor, &resolution1.complex()).unwrap();
    assert_eq!(0, cursor.bytes().count());

    assert_eq!(
        resolution1.graded_dimension_string(),
        resolution2.graded_dimension_string()
    );

    assert_eq!(
        resolution1.differential(5).quasi_inverse(7),
        resolution2.differential(5).quasi_inverse(7)
    );
}
