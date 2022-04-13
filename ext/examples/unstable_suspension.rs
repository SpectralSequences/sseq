//! Computes the suspension map between different unstable Ext groups.
//!
//! Given an unstable Steenrod module $M$, compute the unstable Ext groups of $\Sigma^k M$ for all
//! $k$ up till the stable range. Each result is printed in the form
//! ```
//! n s k: num_gens - matrix
//! ```
//! The entries are to be interpreted as follows:
//!  - `n` is the stem, which is defined to be `t - s - min_degree`
//!  - `s` is the Adams filtration
//!  - `k` is the shift
//!  - `num_gens` is the number of generators in this Ext group
//!  - `matrix` is the matrix representing the suspension map from $\Sigma^k M$. This is omitted if
//!    the source or target of the suspension map is trivial, or if they have the same dimension
//!    and the matrix is the identity matrix.
//!
//! The output is best read after sorting with `sort -n -k 1 -k 2 -k 3`.

use std::{path::PathBuf, sync::Arc};

use algebra::module::{Module, SuspensionModule};
use ext::chain_complex::{FiniteChainComplex, FreeChainComplex};
use ext::{
    resolution::UnstableResolution, resolution_homomorphism::UnstableResolutionHomomorphism,
};
use fp::vector::FpVector;

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

    let max_n: i32 = query::raw("Max n", str::parse);
    let max_s: u32 = query::raw("Max s", str::parse);
    let min_degree = module.min_degree();

    let mut res_a;
    let mut res_b: Arc<UnstableResolution<FiniteChainComplex<_>>> =
        Arc::new(UnstableResolution::new_with_save(
            Arc::new(FiniteChainComplex::ccdz(Arc::new(SuspensionModule::new(
                Arc::clone(&module),
                0,
            )))),
            save_dir(0),
        )?);
    res_b.compute_through_stem(max_s, max_n);

    for n in min_degree..=max_n {
        for s in 0..=max_s {
            let source_num_gens = res_b.number_of_gens_in_bidegree(s, n + s as i32);
            println!("{n} {s} 0: {source_num_gens}",);
        }
    }

    for shift in 1..max_n - module.min_degree() + 3 {
        res_a = res_b;
        res_b = Arc::new(UnstableResolution::new_with_save(
            Arc::new(FiniteChainComplex::ccdz(Arc::new(SuspensionModule::new(
                Arc::clone(&module),
                shift,
            )))),
            save_dir(shift),
        )?);

        res_b.compute_through_stem(max_s, max_n + shift);

        let hom = UnstableResolutionHomomorphism::new(
            String::from("suspension"),
            Arc::clone(&res_b),
            Arc::clone(&res_a),
            0,
            1,
        );

        hom.extend_step_raw(
            0,
            min_degree + shift,
            Some(vec![FpVector::from_slice(module.prime(), &[1])]),
        );
        hom.extend_all();

        for n in 2 * (min_degree + shift - 1)..=max_n + shift {
            if n < min_degree + shift {
                continue;
            }
            for s in 0..=max_s {
                let source_num_gens = res_b.number_of_gens_in_bidegree(s, n + s as i32);
                let target_num_gens = res_a.number_of_gens_in_bidegree(s, n + s as i32 - 1);
                let m = if source_num_gens == 0 || target_num_gens == 0 {
                    String::new()
                } else {
                    let m = hom.get_map(s).hom_k(n + s as i32 - 1);
                    if source_num_gens == target_num_gens
                        && m.iter().enumerate().all(|(n, x)| {
                            x.iter()
                                .enumerate()
                                .all(|(m, &z)| if m == n { z == 1 } else { z == 0 })
                        })
                    {
                        String::new()
                    } else {
                        format!(" - {m:?}")
                    }
                };
                println!("{n} {s} {shift}: {source_num_gens}{m}", n = n - shift);
            }
        }
    }

    Ok(())
}
