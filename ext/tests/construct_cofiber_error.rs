use ext::utils::{construct, construct_standard};
use serde_json::json;

/// A cofiber spec with a non-integer `s` (`t`, `idx`) field must surface an
/// `Err` from `construct` rather than panicking on `.unwrap()`.
#[test]
fn cofiber_non_integer_field() {
    // S^0 over A_2: a valid, bounded base module so we reach the cofiber block.
    let base = json!({
        "p": 2,
        "type": "finite dimensional module",
        "gens": { "x0": 0 },
        "actions": [],
    });

    // Non-integer `s` -> `cofiber["s"].as_i64()` is None -> Err (was `.unwrap()`).
    let mut bad_s = base.clone();
    bad_s["cofiber"] = json!({ "s": "x", "t": 0, "idx": 0 });
    assert!(construct((bad_s.clone(), "adem"), None).is_err());
    assert!(construct((bad_s, "milnor"), None).is_err());

    // Non-integer `t` -> Err (was `.unwrap()`).
    let mut bad_t = base.clone();
    bad_t["cofiber"] = json!({ "s": 0, "t": "x", "idx": 0 });
    assert!(construct((bad_t.clone(), "adem"), None).is_err());
    assert!(construct((bad_t, "milnor"), None).is_err());

    // Non-integer `idx` -> `cofiber["idx"].as_u64()` is None -> Err (was `.unwrap()`).
    let mut bad_idx = base.clone();
    bad_idx["cofiber"] = json!({ "s": 0, "t": 0, "idx": "x" });
    assert!(construct((bad_idx.clone(), "adem"), None).is_err());
    assert!(construct((bad_idx, "milnor"), None).is_err());

    // An in-range integer `idx` that exceeds the number of generators must be an Err
    // (rather than panicking in `Matrix::row_mut`).
    let mut oor_idx = base.clone();
    oor_idx["cofiber"] = json!({ "s": 0, "t": 0, "idx": 999 });
    assert!(construct((oor_idx.clone(), "adem"), None).is_err());
    assert!(construct((oor_idx, "milnor"), None).is_err());
}

/// An unstable resolution does not support cofibers: reaching
/// `construct_standard::<true>` with a cofiber spec must return `Err`
/// (was `assert!(!U, ...)`).
#[test]
fn cofiber_unstable_unsupported() {
    let mut json = json!({
        "p": 2,
        "type": "finite dimensional module",
        "gens": { "x0": 0 },
        "actions": [],
    });
    json["cofiber"] = json!({ "s": 0, "t": 0, "idx": 0 });
    assert!(construct_standard::<true, _, _>((json, "adem"), None).is_err());
}
