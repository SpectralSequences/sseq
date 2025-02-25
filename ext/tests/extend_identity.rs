use std::sync::Arc;

use algebra::module::{
    Module,
    homomorphism::{FullModuleHomomorphism, IdentityHomomorphism},
};
use ext::{
    chain_complex::{AugmentedChainComplex, ChainComplex},
    resolution_homomorphism::ResolutionHomomorphism,
    utils::{Config, construct},
};
use fp::vector::FpVector;
use rstest::rstest;
use serde_json::json;
use sseq::coordinates::Bidegree;

#[rstest]
#[trace]
#[case(("S_2", "adem"), 30)]
#[case(("S_3", "adem"), 50)]
#[case(("Calpha", "adem"), 50)]
#[case(("S_2", "milnor"), 30)]
#[case(("S_3", "milnor"), 50)]
#[case(("Calpha", "milnor"), 50)]
#[case(("tmf2", "milnor"), 40)]
#[case((json!({
    "type":"finite dimensional module",
        "p":2,
        "gens":{ "x0":0, "x1":1, "x3":3, "x4":4 },
        "actions": ["Sq2 x1 = x3"]
    }),
    "adem"),
    30,
)]
#[case((json!({
    "type":"finite dimensional module",
        "p":5,
        "gens":{"x0":0,"x1":1,"x5":5,"x9":9},
        "actions": ["P1 x1 = x9"]
    }),
    "milnor"),
    50,
)]
fn extend_identity<T: TryInto<Config>>(#[case] config: T, #[case] max_degree: i32) {
    let config: Config = config.try_into().ok().unwrap();
    let resolution = Arc::new(construct(config, None).unwrap());
    let max = Bidegree::s_t(max_degree, max_degree);
    resolution.compute_through_bidegree(max);

    let module = resolution.target().module(0);
    let p = module.prime();

    let id = FullModuleHomomorphism::identity_homomorphism(module);

    let hom = ResolutionHomomorphism::from_module_homomorphism(
        "".to_string(),
        Arc::clone(&resolution),
        Arc::clone(&resolution),
        &id,
    );
    hom.extend(max);

    for s in 0..=max_degree {
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
                    "Check failed at s = {s}, t = {t}, idx = {idx}"
                );
                correct_result.set_to_zero();
            }
        }
    }
}
