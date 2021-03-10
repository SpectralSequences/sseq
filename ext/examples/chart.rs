use algebra::module::OperationGeneratorPair;
use chart::Graph;
use ext::chain_complex::ChainComplex;
use ext::load_s_2;
use fp::vector::FpVectorT;
use std::io::Result;

fn main() -> Result<()> {
    let f = std::io::stdout();
    let mut g = Graph::new(f, 20, 8)?;

    load_s_2!(resolution, "milnor", "resolution.save");
    resolution.resolve_through_bidegree(8, 28);
    let resolution = &*resolution.inner;

    for f in 0..=20 {
        for s in 0..=8 {
            let t = s as i32 + f;
            let num_gens = resolution.module(s).number_of_gens_in_degree(t);
            g.node(f as i32, s as i32, num_gens)?;
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
                                (f, s as i32, k),
                                (f - (1 << i) + 1, s as i32 - 1, l),
                                None,
                            )?;
                        }
                    }
                }
            }
        }
    }

    g.structline((15, 1, 0), (14, 3, 0), Some("d2"))?;
    g.structline((17, 4, 0), (16, 6, 0), Some("d2"))?;
    g.structline((18, 5, 0), (17, 7, 0), Some("d2"))?;
    g.structline((18, 4, 1), (17, 6, 0), Some("d2"))?;

    Ok(())
}
