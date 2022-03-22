use algebra::module::homomorphism::{FullModuleHomomorphism, IdentityHomomorphism};
use algebra::module::Module;
use ext::chain_complex::{
    AugmentedChainComplex, BoundedChainComplex, ChainComplex, FreeChainComplex,
};
use ext::resolution_homomorphism::ResolutionHomomorphism;
use ext::utils;
use ext::yoneda::yoneda_representative_element;

use std::sync::Arc;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let resolution = Arc::new(utils::query_module_only("Module", None, false)?);

    let module = resolution.target().module(0);
    let min_degree = resolution.min_degree();

    let n: i32 = query::raw("n of Ext class", str::parse);
    let s: u32 = query::raw("s of Ext class", str::parse);
    let t = n + s as i32;

    resolution.compute_through_stem(s, n);

    let class: Vec<u32> = query::vector(
        "Input Ext class",
        resolution.number_of_gens_in_bidegree(s, t),
    );

    let start = Instant::now();
    let yoneda = Arc::new(yoneda_representative_element(
        Arc::clone(&resolution),
        s,
        t,
        &class,
    ));
    utils::log_time(start.elapsed(), format_args!("Found yoneda representative"));

    // Lift the identity and check that it gives the right class
    let f = ResolutionHomomorphism::from_module_homomorphism(
        "".to_string(),
        Arc::clone(&resolution),
        Arc::clone(&yoneda),
        &FullModuleHomomorphism::identity_homomorphism(Arc::clone(&module)),
    );

    f.extend_through_stem(s, n);
    let final_map = f.get_map(s);
    for (i, &v) in class.iter().enumerate() {
        assert_eq!(final_map.output(t, i).len(), 1);
        assert_eq!(final_map.output(t, i).entry(0), v);
    }

    for t in min_degree..=t {
        assert_eq!(
            yoneda.euler_characteristic(t),
            module.dimension(t) as isize,
            "Incorrect Euler characteristic at t = {t}",
        );
    }

    for s in 0..=s {
        println!(
            "Dimension of {s}th module is {}",
            yoneda.module(s).total_dimension()
        );
    }

    Ok(())
}
