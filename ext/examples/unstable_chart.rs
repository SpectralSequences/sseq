//! Computes the suspension map between different unstable Ext groups.
//!
//! Given an unstable Steenrod module $M$, compute the unstable Ext groups of $\Sigma^k M$ for all
//! $k$ up till the stable range. Each result is printed in the form
//! ```
//! n s min_degree: num_gens - matrix
//! ```
//! The entries are to be interpreted as follows:
//!  - `n` is the stem, which is defined to be `t - s - min_degree`
//!  - `s` is the Adams filtration
//!  - `min_degree` is the minimum degre of $\Sigma^k M$.
//!  - `num_gens` is the number of generators in this Ext group
//!  - `matrix` is the matrix representing the suspension map from $\Sigma^k M$. This is omitted if
//!    the source or target of the suspension map is trivial, or if they have the same dimension
//!    and the matrix is the identity matrix.
//!
//! The output is best read after sorting with `sort -n -k 1 -k 2 -k 3`.

use std::{path::PathBuf, sync::Arc};

use algebra::module::{Module, SuspensionModule};
use algebra::Algebra;
use chart::{Backend, Orientation, TikzBackend};
use ext::chain_complex::{FiniteChainComplex, FreeChainComplex};
use ext::resolution::UnstableResolution;
use sseq::coordinates::Bidegree;

fn main() -> anyhow::Result<()> {
    let module = Arc::new(ext::utils::query_unstable_module_only()?);
    let save_dir = {
        let base = query::optional("Module save directory", |x| {
            core::result::Result::<PathBuf, std::convert::Infallible>::Ok(PathBuf::from(x))
        });
        move |shift| {
            base.as_ref().cloned().map(|mut x| {
                x.push(format!("suspension{shift}"));
                x
            })
        }
    };

    let max = Bidegree::n_s(
        query::raw("Max n", str::parse),
        query::raw("Max s", str::parse),
    );

    let disp_template: String = query::raw(
        "LaTeX name template (replace % with min degree)",
        str::parse,
    );

    let products = module.algebra().default_filtration_one_products();

    for shift_t in 0..max.n() - module.min_degree() + 3 {
        let shift = Bidegree::s_t(0, shift_t);
        let res: Arc<UnstableResolution<FiniteChainComplex<_>>> =
            Arc::new(UnstableResolution::new_with_save(
                Arc::new(FiniteChainComplex::ccdz(Arc::new(SuspensionModule::new(
                    Arc::clone(&module),
                    shift.t(),
                )))),
                save_dir(shift.t()),
            )?);

        res.compute_through_stem(max + shift);

        println!("\\begin{{figure}}[p]\\centering");

        let sseq = res.to_sseq();
        let products = products
            .iter()
            .map(|(name, op_deg, op_idx)| {
                (name.clone(), res.filtration_one_products(*op_deg, *op_idx))
            })
            .collect::<Vec<_>>();

        sseq.write_to_graph(
            TikzBackend::new(std::io::stdout()),
            2,
            false,
            products.iter(),
            |g| {
                g.text(
                    1,
                    max.s() as i32 - 1,
                    disp_template.replace('%', &format!("{shift_t}")),
                    Orientation::Right,
                )
            },
        )?;

        println!("\\end{{figure}}");
    }
    Ok(())
}
