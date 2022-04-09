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

use algebra::module::{FDModule, Module};
use algebra::Algebra;
use chart::{Backend, Orientation, TikzBackend};
use ext::chain_complex::{ChainComplex, FiniteChainComplex, FreeChainComplex};
use ext::resolution::UnstableResolution;

fn main() -> anyhow::Result<()> {
    let mut module = ext::utils::query_unstable_module_only()?;
    let save_dir = {
        let base = query::optional("Module save directory", |x| {
            core::result::Result::<PathBuf, std::convert::Infallible>::Ok(PathBuf::from(x))
        });
        move |module: &FDModule<_>| {
            base.as_ref().cloned().map(|mut x| {
                x.push(format!("suspension{}", module.min_degree()));
                x
            })
        }
    };

    let max_n: i32 = query::raw("Max n", str::parse);
    let max_s: u32 = query::raw("Max s", str::parse);

    let disp_template: String = query::raw(
        "LaTeX name template (replace % with min degree)",
        str::parse,
    );

    let products = module.algebra().default_filtration_one_products();

    while module.min_degree() - 2 <= max_n {
        let res: Arc<UnstableResolution<FiniteChainComplex<_>>> =
            Arc::new(UnstableResolution::new_with_save(
                Arc::new(FiniteChainComplex::ccdz(Arc::new(module.clone()))),
                save_dir(&module),
            )?);

        res.compute_through_stem(max_s, max_n + module.min_degree());

        let min_degree = module.min_degree();

        println!("\\begin{{figure}}[p]\\centering");
        let mut g = TikzBackend::new(std::io::stdout());
        g.init(max_n, max_s as i32)?;
        g.text(
            1,
            max_s as i32 - 1,
            disp_template.replace('%', &format!("{}", min_degree)),
            Orientation::Right,
        )?;

        for (s, n, t) in res.iter_stem() {
            let num_gens = res.number_of_gens_in_bidegree(s, t);
            g.node(n - min_degree, s as i32, num_gens)?;
            if s == 0 {
                continue;
            }
            for &(_, op_deg, op_idx) in &products {
                if let Some(matrix) = res.filtration_one_product(op_deg, op_idx, s, t) {
                    for (source_idx, row) in matrix.iter().enumerate() {
                        for (target_idx, &entry) in row.iter().enumerate() {
                            if entry != 0 {
                                g.structline(
                                    (n - min_degree, s as i32, target_idx),
                                    (n - min_degree - op_deg + 1, s as i32 - 1, source_idx),
                                    None,
                                )?;
                            }
                        }
                    }
                }
            }
        }
        drop(g);
        println!("\\end{{figure}}");

        module.suspend(1);
    }
    Ok(())
}
