use ext::chain_complex::FreeChainComplex;
use ext::utils::construct_from_json;
use ext::utils::load_module_from_file;
use ext::utils::Config;

#[test]
fn resolve_iterate() {
    let path = std::path::PathBuf::from("steenrod_modules");
    for name in &["S_2", "S_3", "Ceta", "Calpha", "C3", "Joker"] {
        let config = Config {
            module_paths: vec![path.clone()],
            module_file_name: (*name).to_string(),
            max_degree: 0, // Doesn't matter
            algebra_name: String::from("milnor"),
        };
        test_iterate(&config);

        let config = Config {
            module_paths: vec![path.clone()],
            module_file_name: (*name).to_string(),
            max_degree: 0, // Doesn't matter
            algebra_name: String::from("adem"),
        };
        test_iterate(&config);
    }
}

fn test_iterate(config: &Config) {
    println!(
        "Resolving {} with {} basis",
        &config.module_file_name, &config.algebra_name
    );

    let mut json = load_module_from_file(&config).unwrap();

    let first = construct_from_json(&mut json.clone(), &config.algebra_name).unwrap();
    let second = construct_from_json(&mut json, &config.algebra_name).unwrap();

    first.resolve_through_bidegree(20, 20);

    second.resolve_through_bidegree(0, 0);
    second.resolve_through_bidegree(5, 5);
    second.resolve_through_bidegree(10, 10);
    second.resolve_through_bidegree(10, 10);
    second.resolve_through_bidegree(18, 18);
    second.resolve_through_bidegree(14, 14);
    second.resolve_through_bidegree(15, 15);
    second.resolve_through_bidegree(20, 20);

    assert_eq!(
        first.graded_dimension_string(20, 20),
        second.graded_dimension_string(20, 20)
    );
}
