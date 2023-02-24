//! Given an element in $\Ext(M, N)$, this computes the induced map $\Ext(N, k) \to \Ext(M, k)$
//! given by composition.
//!
//! It begins by asking for the two modules $M$, $N$ and the $\Ext$ class. Afterwards, you may
//! either supply save files for the two modules, or a range to compute the map for.
//!
//! Afterwards, the user is prompted for the Ext class. If $R_s$ is the $s$th term of the minimal
//! resolution of $M$, the Ext class is given as an element of $\Hom_A(R_s, \Sigma^t N) =
//! \Hom(\Ext^{s, *}(M, k)^\vee, \Sigma^t N)$.
//!
//! In other words, for every basis element in $\Ext^{s, *}(M, k)$, one has to specify its image in
//! $\Sigma^t N$. In the special case where $s = 0$, this is specifying the map between the
//! underlying modules on module generators under the Steenrod action.
//!
//! Our notation is as follows:
//!
//!  - `f` is the map in $\Hom_A(R_s, \Sigma^t N)$.
//!  - `F` is the induced map on Ext.
//!
//! Each prompt will be of the form `f(x_(s, n, i)) = ` and the user has to input the value of the
//! homomorphism on this basis element. For example, the following session computes the map induced
//! by the projection of spectra $C2 \to S^1$
//!
//! ```text
//!  $ cargo run --example lift_hom
//! Target module (default: S_2): C2
//! Source module (default: Cnu): S_2
//! s of Ext class (default: 0): 0
//! n of Ext class (default: 0): -1
//! Target save file (optional):
//! Max target s (default: 10): 10
//! Max target n (default: 10): 20
//!
//! Input module homomorphism to lift:
//! f(x_(0, 0, 0)): [1]
//! ```
//!
//! It is important to keep track of varaince when using this example; Both $\Ext(-, k)$ and
//! $H^*(-)$ are contravariant functors. The words "source" and "target" refer to the map between
//! Steenrod modules.

use algebra::module::Module;
use anyhow::{anyhow, Context};
use ext::chain_complex::{AugmentedChainComplex, ChainComplex, FreeChainComplex};
use ext::resolution_homomorphism::ResolutionHomomorphism;
use ext::utils;
use fp::matrix::Matrix;
use sseq::coordinates::{Bidegree, BidegreeGenerator};

use std::path::PathBuf;
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let source = Arc::new(utils::query_module_only("Source module", None, true)?);
    let b = Bidegree::n_s(
        query::with_default("Max source n", "30", str::parse),
        query::with_default("Max source s", "7", str::parse),
    );

    let source_name = source.name();
    let target = query::with_default("Target module", source_name, |s| {
        if s == source_name {
            Ok(Arc::clone(&source))
        } else if cfg!(feature = "nassau") {
            Err(anyhow!("Can only resolve S_2 with nassau"))
        } else {
            let config: utils::Config = s.try_into()?;
            let save_dir = query::optional("Target save directory", |x| {
                Result::<PathBuf, std::convert::Infallible>::Ok(PathBuf::from(x))
            });

            let mut target = utils::construct(config, save_dir)
                .context("Failed to load module from save file")
                .unwrap();

            target.set_name(s.to_owned());

            #[cfg(feature = "nassau")]
            unreachable!();

            #[cfg(not(feature = "nassau"))]
            Ok(Arc::new(target))
        }
    });

    assert_eq!(source.prime(), target.prime());
    let p = source.prime();

    let name: String = query::raw("Name of product", str::parse);

    let shift = Bidegree::n_s(
        query::with_default("n of product", "0", str::parse),
        query::with_default("s of product", "0", str::parse),
    );

    source.compute_through_stem(b);
    target.compute_through_stem(b - shift);

    let target_module = target.target().module(0);
    let hom = ResolutionHomomorphism::new(name.clone(), source, target, shift);

    eprintln!("\nInput Ext class to lift:");
    for output_t in 0..=target_module
        .max_degree()
        .expect("lift_hom requires target to be bounded")
    {
        let output = Bidegree::s_t(0, output_t);
        let input = output + shift;
        let mut matrix = Matrix::new(
            p,
            hom.source.number_of_gens_in_bidegree(input),
            target_module.dimension(output.t()),
        );

        if matrix.rows() == 0 || matrix.columns() == 0 {
            hom.extend_step(input, None);
        } else {
            for (idx, row) in matrix.iter_mut().enumerate() {
                let gen = BidegreeGenerator::new(input, idx);
                let v: Vec<u32> = query::vector(&format!("f(x_{gen}"), row.len());
                for (i, &x) in v.iter().enumerate() {
                    row.set_entry(i, x);
                }
            }
            hom.extend_step(input, Some(&matrix));
        }
    }

    hom.extend_all();

    for b2 in hom.target.iter_stem() {
        let shifted_b2 = b2 + shift;
        if shifted_b2.s() >= hom.source.next_homological_degree()
            || shifted_b2.t() > hom.source.module(shifted_b2.s()).max_computed_degree()
        {
            continue;
        }
        let matrix = hom.get_map(shifted_b2.s()).hom_k(b2.t());
        for (i, r) in matrix.iter().enumerate() {
            let gen = BidegreeGenerator::new(b2, i);
            println!("{name} x_{gen} = {r:?}");
        }
    }
    Ok(())
}
