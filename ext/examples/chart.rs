use algebra::module::{Module, OperationGeneratorPair};
use chart::{Backend, SvgBackend};
use ext::chain_complex::ChainComplex;
use ext::utils::query_module;

fn main() -> anyhow::Result<()> {
    let f = std::io::stdout();
    let mut g = SvgBackend::new(f);
    let resolution = query_module(None, false)?.resolution;

    g.init(
        resolution.module(0).max_computed_degree(),
        resolution.next_homological_degree() as i32 - 1,
    )?;

    if *resolution.prime() == 2 {
        for (s, n, t) in resolution.iter_stem() {
            let num_gens = resolution.module(s).number_of_gens_in_degree(t);
            g.node(n as i32, s as i32, num_gens)?;
            if s == 0 {
                continue;
            }
            let target = resolution.module(s - 1);
            for k in 0..num_gens {
                let d = resolution.differential(s);
                let dg = d.output(t, k);
                for i in 0..3 {
                    for l in 0..target.number_of_gens_in_degree(t - (1 << i)) {
                        let elt = target.operation_generator_pair_to_idx(&OperationGeneratorPair {
                            operation_index: 0,
                            operation_degree: 1 << i,
                            generator_index: l,
                            generator_degree: t - (1 << i),
                        });
                        if dg.entry(elt) != 0 {
                            g.structline(
                                (n, s as i32, k),
                                (n - (1 << i) + 1, s as i32 - 1, l),
                                None,
                            )?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
