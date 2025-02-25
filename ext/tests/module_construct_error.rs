use ext::utils::construct;
use serde_json::{Value, json};

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
    assert!(construct((json.clone(), "adem"), None).is_err());
    assert!(construct((json, "milnor"), None).is_err());
}
