use ext::utils::Config;
use ext::utils::construct_from_json;
use ext::utils::load_module_from_file;
use serde_json::Value;

#[test]
fn resolve_iterate() {
    let path = std::path::PathBuf::from("modules");
    for name in &["S_2", "S_3", "Ceta", "Calpha", "C3", "Joker"] {
        let config = Config {
            module_paths : vec![path.clone()],
            module_file_name : (*name).to_string(),
            max_degree : 0, // Doesn't matter
            algebra_name : String::from("milnor")
        };
        test_iterate(&config);

        let config = Config {
            module_paths : vec![path.clone()],
            module_file_name : (*name).to_string(),
            max_degree : 0, // Doesn't matter
            algebra_name : String::from("adem")
        };
        test_iterate(&config);
    }
}

fn test_iterate(config: &Config) {
    println!("Resolving {} with {} basis", &config.module_file_name, &config.algebra_name);

    let module_def = load_module_from_file(&config).unwrap();
    let json : Value = serde_json::from_str(&module_def).unwrap();

    let first = construct_from_json(json.clone(), config.algebra_name.clone()).unwrap();
    let second = construct_from_json(json, config.algebra_name.clone()).unwrap();

    first.resolution.read().resolve_through_degree(20);

    second.resolution.read().resolve_through_degree(0);
    second.resolution.read().resolve_through_degree(5);
    second.resolution.read().resolve_through_degree(10);
    second.resolution.read().resolve_through_degree(10);
    second.resolution.read().resolve_through_degree(18);
    second.resolution.read().resolve_through_degree(14);
    second.resolution.read().resolve_through_degree(15);
    second.resolution.read().resolve_through_degree(20);

    assert_eq!(first.resolution.read().graded_dimension_string(),
               second.resolution.read().graded_dimension_string());
}
