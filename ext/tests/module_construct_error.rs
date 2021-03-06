use ext::utils::construct_from_json;
use serde_json::{json, Value};

#[test]
fn module_construct_error() {
    test(json!({
        "type": "finite dimensional module",
        "p": 4,
        "generic": true,
        "gens": { "x0": 0 },
    }));
    test(json!({
        "type": "finite dimensional module",
        "p": 2,
        "generic": true,
        "gens": { "x0": 0, "x1": 1, "x2": 2 },
        "actions": ["Sq1 x0 = x1", "Sq1 x1 = x2"],
    }));
}

fn test(json: Value) {
    matches!(construct_from_json(json.clone(), "adem"), Err(_));
    matches!(construct_from_json(json, "milnor"), Err(_));
}
