use algebra::Algebra;
use chart::SvgBackend;
use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    utils::query_module,
};

fn main() -> anyhow::Result<()> {
    let resolution = query_module(None, false)?;

    let sseq = resolution.to_sseq();
    let products: Vec<_> = resolution
        .algebra()
        .default_filtration_one_products()
        .into_iter()
        .map(|(name, op_deg, op_idx)| (name, resolution.filtration_one_products(op_deg, op_idx)))
        .collect();

    sseq.write_to_graph(
        SvgBackend::new(std::io::stdout()),
        2,
        false,
        products.iter(),
        |_| Ok(()),
    )?;
    Ok(())
}
