use algebra::module::Module;
use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils;
use ext::yoneda::yoneda_representative_element;

use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let resolution = Arc::new(utils::query_module_only("Module", None, false)?);

    let n: i32 = query::raw("n of Ext class", str::parse);
    let s: u32 = query::raw("s of Ext class", str::parse);
    let t = n + s as i32;

    resolution.compute_through_stem(s, n);

    let class: Vec<u32> = query::vector(
        "Input Ext class",
        resolution.number_of_gens_in_bidegree(s, t),
    );

    let yoneda = Arc::new(yoneda_representative_element(
        Arc::clone(&resolution),
        s,
        t,
        &class,
    ));

    for s in 0..=s {
        println!(
            "Dimension of {s}th module is {}",
            yoneda.module(s).total_dimension()
        );
    }

    Ok(())
}
