use algebra::module::Module;
use algebra::Algebra;
use chart::{Backend, SvgBackend};
use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils::query_module;

fn main() -> anyhow::Result<()> {
    let f = std::io::stdout();
    let mut g = SvgBackend::new(f);
    let resolution = query_module(None, false)?;

    g.init(
        resolution.module(0).max_computed_degree(),
        resolution.next_homological_degree() as i32 - 1,
    )?;

    let products = resolution.algebra().default_filtration_one_products();
    for (s, n, t) in resolution.iter_stem() {
        let num_gens = resolution.number_of_gens_in_bidegree(s, t);
        g.node(n, s as i32, num_gens)?;
        if s == 0 {
            continue;
        }
        for (name, op_deg, op_idx) in &products {
            if let Some(matrix) = resolution.filtration_one_product(*op_deg, *op_idx, s, t) {
                for (source_idx, row) in matrix.iter().enumerate() {
                    for (target_idx, &entry) in row.iter().enumerate() {
                        if entry != 0 {
                            g.structline(
                                (n, s as i32, target_idx),
                                (n - *op_deg + 1, s as i32 - 1, source_idx),
                                Some(name),
                            )?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
