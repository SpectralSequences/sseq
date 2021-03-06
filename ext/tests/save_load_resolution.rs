use algebra::module::homomorphism::ModuleHomomorphism;
use ext::{resolution::Resolution, utils::construct_from_json};
use saveload::{Load, Save};
use serde_json::json;
use std::io::{Cursor, Read, Seek, SeekFrom};

#[test]
fn test_save_load() {
    let json = json!({
        "type": "finite dimensional module",
        "p": 2,
        "gens": {"x0": 0},
        "actions": []
    });

    let bundle = construct_from_json(json, "adem").unwrap();

    let resolution1 = bundle.resolution.read();
    resolution1.resolve_through_degree(10);

    let mut cursor: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    resolution1.save(&mut cursor).unwrap();

    cursor.seek(SeekFrom::Start(0)).unwrap();

    let resolution2 = Resolution::load(&mut cursor, &bundle.chain_complex).unwrap();
    assert_eq!(0, cursor.bytes().count());

    assert_eq!(
        resolution1.graded_dimension_string(),
        resolution2.graded_dimension_string()
    );

    resolution1.resolve_through_degree(20);
    resolution2.resolve_through_degree(20);

    assert_eq!(
        resolution1.graded_dimension_string(),
        resolution2.graded_dimension_string()
    );

    assert_eq!(
        resolution1.differential(5).quasi_inverse(7),
        resolution2.differential(5).quasi_inverse(7)
    );
}
