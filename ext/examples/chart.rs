use algebra::Algebra;
use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    utils::query_module,
};
use sseq::charting::{SeqSeeBackend, SvgBackend, TikzBackend};

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let resolution = query_module(None, false)?;

    let format = query::with_default("Output format (svg/tikz/seqsee)", "svg", |x| match x {
        "svg" | "tikz" | "seqsee" => Ok(x.to_string()),
        _ => Err(format!(
            "unknown format '{x}'; expected one of svg, tikz, seqsee"
        )),
    });

    let sseq = resolution.to_sseq();
    let products: Vec<_> = resolution
        .algebra()
        .default_filtration_one_products()
        .into_iter()
        .map(|(name, op_deg, op_idx)| (name, resolution.filtration_one_products(op_deg, op_idx)))
        .collect();

    let out = std::io::stdout();
    match format.as_str() {
        "svg" => {
            sseq.write_to_graph(SvgBackend::new(out), 2, false, products.iter(), |_| Ok(()))?
        }
        "tikz" => {
            sseq.write_to_graph(TikzBackend::new(out), 2, false, products.iter(), |_| Ok(()))?
        }
        "seqsee" => {
            sseq.write_to_graph(SeqSeeBackend::new(out), 2, false, products.iter(), |_| {
                Ok(())
            })?
        }
        _ => unreachable!(),
    }
    Ok(())
}
