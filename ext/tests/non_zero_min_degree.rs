use ext::utils::construct_from_json;

#[test]
fn negative_min_degree() {
    let bundle = construct_from_json(serde_json::from_str(&r#"{"type" : "finite dimensional module","name": "", "file_name": "", "p": 2, "generic": true, "gens": {"x0": -2}, "adem_actions": [], "milnor_actions": [], "products":[{"hom_deg":3,"int_deg":11,"class":[1],"name":"c_0"}]}"#).unwrap(), "adem").unwrap();

    bundle.resolution.read().resolve_through_degree(20);
}

#[test]
fn positive_min_degree() {
    let bundle = construct_from_json(serde_json::from_str(&r#"{"type" : "finite dimensional module","name": "", "file_name": "", "p": 2, "generic": true, "gens": {"x0": 2}, "adem_actions": [], "milnor_actions": [], "products":[{"hom_deg":3,"int_deg":11,"class":[1],"name":"c_0"}]}"#).unwrap(), "adem").unwrap();

    bundle.resolution.read().resolve_through_degree(20);
}
