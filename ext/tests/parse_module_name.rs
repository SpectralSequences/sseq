use ext::utils::{LoadModuleError, load_module_json, parse_module_name};

#[test]
fn valid_module_name_parses_ok() {
    // "S_2" is a bundled module (see ext/steenrod_modules/S_2.json).
    assert!(parse_module_name("S_2").is_ok());
}

#[test]
fn nonexistent_module_name_is_err_not_panic() {
    // A name with no corresponding json file must surface an error rather than panic.
    // load_module_json reports it as the distinct NotFound variant (so a caller can map it
    // to FileNotFoundError, separately from read/parse failures).
    assert!(parse_module_name("this_module_does_not_exist_xyz").is_err());
    assert!(matches!(
        load_module_json("this_module_does_not_exist_xyz"),
        Err(LoadModuleError::NotFound(_))
    ));
}

#[test]
fn non_integer_shift_suffix_is_err_not_panic() {
    // The `[shift]` suffix must parse as an integer; a non-integer suffix returns Err.
    assert!(parse_module_name("S_2[not_an_integer]").is_err());
}

#[test]
fn unterminated_shift_bracket_is_err() {
    assert!(parse_module_name("S_2[5").is_err());
}
