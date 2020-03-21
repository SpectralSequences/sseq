#![allow(unused_variables)]

use std::sync::Arc;

use ext::chain_complex::ChainComplex;
use ext::utils::construct;
use ext::utils::Config;

// use algebra::AdemAlgebra;
// use module::TensorModule;
// use module::homomorphism::BoundedModuleHomomorphism;
// use serde_json::json;
// use chain_complex::ChainComplex;
#[allow(unreachable_code)]
pub fn test(config: &Config) -> error::Result<()> {
    let bundle = construct(config)?;
    let res = bundle.resolution;
    let max_degree = 20; //config.max_degree;
    res.read().resolve_through_degree(max_degree);
    res.write().add_structline = Some(Box::new(
        |name: &str, s1: u32, t1: i32, s2: u32, t2: i32, _ignored: bool, coeffs: Vec<Vec<u32>>| {
            println!("{} coeffs : {:?}", name, coeffs);
        },
    ));
    res.write().set_unit_resolution(Arc::downgrade(&res));
    // res.compute_through_bidegree(5,5);
    for t in 1..max_degree {
        for s in 1..=t {
            let num_gens = res
                .read()
                .inner
                .module(s as u32)
                .number_of_gens_in_degree(t);
            for i in 0..num_gens {
                let mut vec = vec![0; num_gens];
                vec[i] = 1;
                res.write()
                    .add_product(s as u32, t, vec, &format!("({},{},{})", s, t, i));
                res.read().catch_up_products();
            }
        }
    }

    // res.
    Ok(())

    // // let algebra = Arc::new(AlgebraAny::from(AdemAlgebra::new(p, false, false)));
    // // let y_json = r#"{"type" : "finite dimensional module", "p": 2, "generic": false, "gens": {"x0": 0, "x1": 1, "x2": 2, "x3": 3}, "actions": ["Sq1 x0 = x1", "Sq1 x2 = x3", "Sq2 x0 = x2", "Sq2 x1 = x3"]}"#;

    // // let y_module = Arc::new(FiniteModule::from_json(Arc::clone(&algebra), &mut serde_json::from_str(y_json)?)?);
    // // let y_chain_complex: Arc<CCC> = Arc::new(FiniteChainComplex::ccdz(Arc::clone(&y_module)));
    // // let y_resolution = Resolution::new(y_chain_complex, None, None);

    // // let yy_module = Arc::new(FiniteModule::from(TensorModule::new(Arc::clone(&y_module), Arc::clone(&y_module)).to_fd_module()));
    // // let yy_chain_complex: Arc<CCC> = Arc::new(FiniteChainComplex::ccdz(Arc::clone(&yy_module)));
    // // let yy_resolution = Resolution::new(yy_chain_complex, None, None);

    // // y_resolution.resolve_through_bidegree(1, 6);
    // // yy_resolution.resolve_through_bidegree(1, 6);

    // if let FiniteModule::FDModule(m) = &*yy_module {
    //     let output_json = json!({
    //         "p" : 2,
    //         "generic" : false,
    //         "type": "finite dimensional module",
    //         "adem_actions": m.actions_to_json(),
    //         "gens": m.gens_to_json(),
    //     });
    //     println!("{}", output_json.to_string());
    // }

    // let hom = BoundedModuleHomomorphism::from_matrices(
    //     yy_module,
    //     y_module,
    //     0,
    //     BiVec::from_vec(0, vec![
    //         Matrix::from_vec(p, &[vec![1]]),
    //         Matrix::from_vec(p, &[vec![0], vec![1]]),
    //         Matrix::from_vec(p, &[vec![0], vec![0], vec![1]]),
    //         Matrix::from_vec(p, &[vec![0], vec![0], vec![0], vec![1]]),
    //     ])
    // );
    // let res_hom = ResolutionHomomorphism::from_module_homomorphism(
    //     "".to_string(),
    //     Arc::clone(&yy_resolution.inner),
    //     Arc::clone(&y_resolution.inner),
    //     &FiniteModuleHomomorphism::from(hom)
    // );

    // res_hom.extend(1, 6);
    // let mut result = FpVector::new(p, 3);
    // res_hom.act(&mut result, 1, 6, 0);
    // println!("{}", result);

    // Ok(())
}
