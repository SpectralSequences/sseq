use ext::utils::{Config, construct, construct_from_json};
use fp::matrix::Matrix;
use ext::module::Module;
use ext::module::homomorphism::{FiniteModuleHomomorphism, IdentityHomomorphism};
use ext::resolution_homomorphism::ResolutionHomomorphism;
use fp::vector::{FpVectorT, FpVector};
use ext::chain_complex::{ChainComplex, AugmentedChainComplex};
use std::sync::Arc;

#[test]
fn extend_identity() {
    check_algebra("S_2", 30, "adem");
    check_algebra("S_3", 50, "adem");
    check_algebra("Calpha", 50, "adem");
    check_algebra("S_2", 30, "milnor");
    check_algebra("S_3", 50, "milnor");
    check_algebra("Calpha", 50, "milnor");
    check_algebra("tmf2", 40, "milnor");
    check_algebra("A-mod-Sq1-Sq2", 20, "adem");
}

fn check_algebra (module_name : &str, max_degree : i32, algebra_name: &str) {
    println!("module : {}", module_name);
    let path = std::path::PathBuf::from("steenrod_modules");
    let a = Config {
        module_paths : vec![path],
        module_file_name : module_name.to_string(),
        max_degree,
        algebra_name : String::from(algebra_name)
    };

    let bundle = construct(&a).unwrap();
    let p = bundle.chain_complex.prime();

    bundle.resolution.write().add_self_map(0, 0, &"id".to_string(), Matrix::from_vec(p, &[vec![1]]));

    let resolution = bundle.resolution.read();

    resolution.resolve_through_degree(max_degree);

    for s in 0 ..= max_degree as u32 {
        let map = resolution.self_maps[0].map.get_map(s);
        let source = resolution.module(s);
        for t in 0..= max_degree {
            let mut correct_result = FpVector::new(p, source.dimension(t));
            for idx in 0 .. source.number_of_gens_in_degree(t){
                correct_result.set_entry(source.operation_generator_to_index(0, 0, t, idx), 1);
                // Mathematically, there is no reason these should be lietrally
                // equal.
                assert_eq!(map.output(t, idx), &correct_result);
                correct_result.set_to_zero_pure();
            }
        }
    }
}

#[test]
fn extend_identity2() {
    check2(r#"{"adem_actions":[],"generic":false,"gens":{"x00":0},"milnor_actions":[],"name":"","p":2,"type":"finite dimensional module"}"#, 30, "adem");
    check2(r#"{"adem_actions":[{"input":"x10","op":[2],"output":[{"coeff":1,"gen":"x30"}]}],"generic":false,"gens":{"x00":0,"x10":1,"x30":3,"x40":4},"milnor_actions":[{"input":"x10","op":[2],"output":[{"coeff":1,"gen":"x30"}]}],"name":"","p":2,"type":"finite dimensional module"}"#, 30, "adem");
    check2(r#"{"adem_actions":[{"input":"x10","op":[0,1,0],"output":[{"coeff":1,"gen":"x90"}]}],"generic":true,"gens":{"x00":0,"x10":1,"x50":5,"x90":9},"milnor_actions":[{"input":"x10","op":[[],[1]],"output":[{"coeff":1,"gen":"x90"}]}],"name":"","p":5,"type":"finite dimensional module"}"#, 50, "milnor");
    check2(r#"{"adem_relations":[[{"coeff":1,"gen":"x10","op":[]}],[{"coeff":1,"gen":"x00","op":[2]},{"coeff":1,"gen":"x20","op":[]}]],"file_name":"1","generic":false,"gens":{"x00":0,"x10":1,"x20":2,"x40":4},"name":"","p":2,"type":"finitely presented module"}"#, 30, "adem");
}

fn check2(json: &str, max_degree: i32, algebra_name: &str) {
    println!("Module: {}", json);
    let bundle = construct_from_json(serde_json::from_str(json).unwrap(), algebra_name).unwrap();

    let resolution = bundle.resolution.read();

    resolution.resolve_through_bidegree(max_degree as u32, max_degree);
    let inner = Arc::clone(&resolution.inner);
    let module = inner.target().module(0);
    let p = module.prime();

    let id = FiniteModuleHomomorphism::identity_homomorphism(module);

    let hom = ResolutionHomomorphism::from_module_homomorphism("".to_string(), Arc::clone(&inner), Arc::clone(&inner), &id);
    hom.extend(max_degree as u32, max_degree);

    for s in 0 ..= max_degree as u32 {
        let source = inner.module(s);
        let map = hom.get_map(s);
        for t in 0 ..= max_degree {
            let mut correct_result = FpVector::new(p, source.dimension(t));
            for idx in 0 .. source.number_of_gens_in_degree(t){
                correct_result.set_entry(source.operation_generator_to_index(0, 0, t, idx), 1);
                // Mathematically, there is no reason these should be lietrally
                // equal.
                assert_eq!(map.output(t, idx), &correct_result, "Check failed at s = {}, t = {}, idx = {}", s, t, idx);
                correct_result.set_to_zero_pure();
            }

        }
    }
}
