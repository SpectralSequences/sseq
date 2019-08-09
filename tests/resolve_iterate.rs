use rust_ext::Config;
use rust_ext::construct_from_json;
use rust_ext::load_module_from_file;
use serde_json::Value;

#[test]
fn resolve_iterate() {
    let path = std::path::PathBuf::from("static/modules");
    let config = Config {
        module_paths : vec![path],
        module_file_name : "S_2".to_string(),
        max_degree : 0, // Doesn't matter
        algebra_name : String::from("adem")
    };
    let module_def = load_module_from_file(&config).unwrap();
    let json : Value = serde_json::from_str(&module_def).unwrap();

    let first = construct_from_json(json.clone(), "adem".to_string()).unwrap();
    let second = construct_from_json(json, "adem".to_string()).unwrap();

    first.resolution.borrow_mut().resolve_through_degree(20);

    second.resolution.borrow_mut().resolve_through_degree(0);
    second.resolution.borrow_mut().resolve_through_degree(5);
    second.resolution.borrow_mut().resolve_through_degree(10);
    second.resolution.borrow_mut().resolve_through_degree(10);
    second.resolution.borrow_mut().resolve_through_degree(18);
    second.resolution.borrow_mut().resolve_through_degree(14);
    second.resolution.borrow_mut().resolve_through_degree(15);
    second.resolution.borrow_mut().resolve_through_degree(20);

    assert_eq!(first.resolution.borrow().graded_dimension_string(),
               second.resolution.borrow().graded_dimension_string());
}
