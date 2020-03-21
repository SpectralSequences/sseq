use saveload::{Save, Load};
use std::io::{Read, Cursor, SeekFrom, Seek};
use ext::{resolution::Resolution, utils::construct_from_json};

#[test]
fn test_save_load() {
    let k = r#"{"type" : "finite dimensional module","name": "$S_2$", "file_name": "S_2", "p": 2, "generic": false, "gens": {"x0": 0}, "adem_actions": []}"#;
    let k = serde_json::from_str(k).unwrap();
    let bundle = construct_from_json(k, "adem".to_string()).unwrap();

    let resolution1 = bundle.resolution.read();
    resolution1.resolve_through_degree(10);

    let mut cursor : Cursor<Vec<u8>> = Cursor::new(Vec::new());
    resolution1.save(&mut cursor).unwrap();

    cursor.seek(SeekFrom::Start(0)).unwrap();

    let resolution2 = Resolution::load(&mut cursor, &bundle.chain_complex).unwrap();
    assert_eq!(0, cursor.bytes().count());

    assert_eq!(resolution1.graded_dimension_string(), resolution2.graded_dimension_string());

    resolution1.resolve_through_degree(20);
    resolution2.resolve_through_degree(20);

    assert_eq!(resolution1.graded_dimension_string(), resolution2.graded_dimension_string());
}
