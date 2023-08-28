use algebra::module::Module;
use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils;
use ext::yoneda::yoneda_representative_element;
use sseq::coordinates::Bidegree;

use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let resolution = Arc::new(utils::query_module_only("Module", None, false)?);

    let b = Bidegree::n_s(
        query::raw("n of Ext class", str::parse),
        query::raw("s of Ext class", str::parse),
    );

    resolution.compute_through_stem(b);

    let class: Vec<u32> =
        query::vector("Input Ext class", resolution.number_of_gens_in_bidegree(b));

    let yoneda = Arc::new(yoneda_representative_element(
        Arc::clone(&resolution),
        b,
        &class,
    ));

    for s in 0..=b.s() {
        println!(
            "Dimension of {s}th module is {}",
            yoneda.module(s).total_dimension()
        );
    }

    Ok(())
}
