//! This saves a resolution to Bruner's format. This saves the resulting files to the current
//! working directory. It is recommended that you run this in a dedicated subdirectory.

use algebra::module::Module;
use algebra::{Algebra, AlgebraType, MilnorAlgebra};
use ext::{chain_complex::ChainComplex, utils::query_module};
use fp::vector::prelude::*;
use itertools::Itertools;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufWriter, Write as _};

fn main() -> anyhow::Result<()> {
    let resolution = query_module(Some(AlgebraType::Milnor), false)?;

    assert_eq!(*resolution.prime(), 2);
    let algebra = resolution.algebra();
    let algebra: &MilnorAlgebra = algebra.as_ref().try_into()?;

    let mut buffer = String::new();

    for s in 0..resolution.next_homological_degree() {
        let f = File::create(format!("hDiff.{s}"))?;
        let mut f = BufWriter::new(f);
        let module = resolution.module(s);
        // We don't use this when s = 0
        let dmodule = resolution.module(s.saturating_sub(1));
        let min_degree = module.min_degree();
        let max_degree = module.max_computed_degree();
        let num_gens: usize = (min_degree..=max_degree)
            .map(|t| module.number_of_gens_in_degree(t))
            .sum();

        writeln!(f, "        {num_gens}        {max_degree}\n")?;

        let d = resolution.differential(s);
        for t in min_degree..=max_degree {
            for idx in 0..module.number_of_gens_in_degree(t) {
                writeln!(f, "{t}\n")?;

                if s == 0 {
                    writeln!(f, "1\n0 0 1 i(0).\n\n\n")?;
                    continue;
                }
                let mut row_count = 0;
                buffer.clear();
                let dx = d.output(t, idx);

                let mut gen_count = 0;
                for gen_deg in min_degree..t {
                    for gen_idx in 0..dmodule.number_of_gens_in_degree(gen_deg) {
                        let op_deg = t - gen_deg;
                        let algebra_dim = algebra.dimension(op_deg);
                        let start = dmodule.generator_offset(t, gen_deg, gen_idx);
                        let slice = dx.slice(start, start + algebra_dim);
                        if slice.is_zero() {
                            gen_count += 1;
                            continue;
                        }
                        row_count += 1;
                        write!(buffer, "{gen_count} {op_deg} {algebra_dim} i").unwrap();
                        for (op_idx, _) in slice.iter_nonzero() {
                            let elt = algebra.basis_element_from_index(op_deg, op_idx);
                            write!(buffer, "({:?})", elt.p_part.iter().format(",")).unwrap();
                        }
                        writeln!(buffer, ".").unwrap();
                        gen_count += 1;
                    }
                }
                writeln!(f, "{row_count}")?;
                writeln!(f, "{buffer}")?; // buffer has one new line, writeln has one new line, add another one.
            }
        }
    }

    Ok(())
}
