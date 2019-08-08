use rust_ext::Config;
use rust_ext::run;

#[test]
fn milnor_vs_adem() {
    compare("S_2", 30);
    compare("C2", 30);
    compare("Joker", 30);
    compare("RP4", 30);
    compare("Csigma", 30);
    compare("S_3", 30);
    compare("Calpha", 30);
    compare("C3", 60);
}

fn compare(module_name : &str, max_degree : i32) {
    println!("module : {}", module_name);
    let path = std::path::PathBuf::from("static/modules");
    let a = Config {
        module_paths : vec![path.clone()],
        module_file_name : module_name.to_string(),
        max_degree,
        algebra_name : String::from("adem")
    };
    let m = Config {
        module_paths : vec![path.clone()],
        module_file_name : module_name.to_string(),
        max_degree,
        algebra_name : String::from("milnor")
    };

    match (run(&a), run(&m)) {
        (Err(e), _)    => panic!("Failed to read file: {}", e),
        (_, Err(e))    => panic!("Failed to read file: {}", e),
        (Ok(x), Ok(y)) => assert_eq!(x, y)
    }
}
