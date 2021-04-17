use algebra::module::homomorphism::{FiniteModuleHomomorphism, IdentityHomomorphism};
use algebra::module::Module;
use ext::chain_complex::{AugmentedChainComplex, ChainComplex};
use ext::resolution_homomorphism::ResolutionHomomorphism;
use ext::utils::{construct_from_json, load_module_from_file, Config};
use fp::vector::FpVector;
use serde_json::{json, Value};
use std::sync::Arc;

#[test]
fn extend_identity() {
    check_file("S_2", 30, "adem");
    check_file("S_3", 50, "adem");
    check_file("Calpha", 50, "adem");
    check_file("S_2", 30, "milnor");
    check_file("S_3", 50, "milnor");
    check_file("Calpha", 50, "milnor");
    check_file("tmf2", 40, "milnor");
    check_file("A-mod-Sq1-Sq2", 20, "adem");

    check(
        json!({
            "type":"finite dimensional module",
            "p":2,
            "actions": ["Sq2 x1 = x3"],
            "gens":{ "x0":0, "x1":1, "x3":3, "x4":4 },
        }),
        30,
        "adem",
    );

    check(
        json!({
            "type":"finite dimensional module",
            "p":5,
            "gens":{"x0":0,"x1":1,"x5":5,"x9":9},
            "adem_actions":[{"input":"x1","op":[0,1,0],"output":[{"coeff":1,"gen":"x9"}]}],
            "milnor_actions":[{"input":"x1","op":[[],[1]],"output":[{"coeff":1,"gen":"x9"}]}],
        }),
        50,
        "milnor",
    );

    check(
        json!({
            "type":"finitely presented module",
            "p":2,
            "gens": { "x0":0, "x1":1, "x2":2, "x4":4 },
            "adem_relations":[[{"coeff":1,"gen":"x1","op":[]}],[{"coeff":1,"gen":"x0","op":[2]},{"coeff":1,"gen":"x2","op":[]}]],
        }),
        30,
        "adem",
    );
}

fn check_file(module_name: &str, max_degree: i32, algebra_name: &str) {
    println!("module: {}", module_name);
    let path = std::path::PathBuf::from("steenrod_modules");
    let config = Config {
        module_paths: vec![path],
        module_file_name: module_name.to_string(),
        algebra_name: String::from(algebra_name),
    };

    let json = load_module_from_file(&config).unwrap();

    check(json, max_degree, algebra_name);
}

fn check(mut json: Value, max_degree: i32, algebra_name: &str) {
    println!("Module: {}", json);
    let resolution = Arc::new(construct_from_json(&mut json, algebra_name).unwrap());
    resolution.compute_through_bidegree(max_degree as u32, max_degree);

    let module = resolution.target().module(0);
    let p = module.prime();

    let id = FiniteModuleHomomorphism::identity_homomorphism(module);

    let hom = ResolutionHomomorphism::from_module_homomorphism(
        "".to_string(),
        Arc::clone(&resolution),
        Arc::clone(&resolution),
        &id,
    );
    hom.extend(max_degree as u32, max_degree);

    for s in 0..=max_degree as u32 {
        let source = resolution.module(s);
        let map = hom.get_map(s);
        for t in 0..=max_degree {
            let mut correct_result = FpVector::new(p, source.dimension(t));
            for idx in 0..source.number_of_gens_in_degree(t) {
                correct_result.set_entry(source.operation_generator_to_index(0, 0, t, idx), 1);
                // Mathematically, there is no reason these should be lietrally
                // equal.
                assert_eq!(
                    map.output(t, idx),
                    &correct_result,
                    "Check failed at s = {}, t = {}, idx = {}",
                    s,
                    t,
                    idx
                );
                correct_result.set_to_zero();
            }
        }
    }
}
