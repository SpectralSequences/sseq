use ext::utils::construct;
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
    matches!(construct((json.clone(), "adem"), None), Err(_));
    matches!(construct((json, "milnor"), None), Err(_));
}
