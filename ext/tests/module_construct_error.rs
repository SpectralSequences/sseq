use ext::utils::construct_from_json;

#[test]
fn module_construct_error() {
    test(r#"{"type" : "finite dimensional module", "name": "", "p": 4, "generic": true, "gens": {"x0": 0}}"#);
    test(r#"{"type" : "finite dimensional module", "name": "", "p": 2, "generic": true, "gens": {"x0": 0, "x1": 1, "x2": 2}, "actions": ["Sq1 x0 = x1", "Sq1 x1 = x2"]}"#);
}

fn test(json: &str) {
    matches!(construct_from_json(serde_json::from_str(json).unwrap(), "adem"), Err(_));
    matches!(construct_from_json(serde_json::from_str(json).unwrap(), "milnor"), Err(_));
}
