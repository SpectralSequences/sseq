use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils::construct;
use ext::utils::Config;

#[test]
fn milnor_vs_adem() {
    compare("S_2", 30);
    compare("C2", 30);
    compare("Joker", 30);
    compare("RP4", 30);
    compare("RP_inf", 30);
    compare("RP_-4_inf", 30);
    compare("Csigma", 30);
    compare("S_3", 30);
    compare("Calpha", 30);
}

fn compare(module_name: &str, max_degree: i32) {
    println!("module: {}", module_name);
    let path = std::path::PathBuf::from("steenrod_modules");
    let a = Config {
        module_paths: vec![path.clone()],
        module_file_name: module_name.to_string(),
        algebra_name: String::from("adem"),
    };
    let b = Config {
        module_paths: vec![path],
        module_file_name: module_name.to_string(),
        algebra_name: String::from("milnor"),
    };

    let a = construct(&a).unwrap();
    let b = construct(&b).unwrap();

    a.compute_through_bidegree(max_degree as u32, max_degree);
    b.compute_through_bidegree(max_degree as u32, max_degree);

    assert_eq!(a.graded_dimension_string(), b.graded_dimension_string());
}
