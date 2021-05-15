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

use algebra::module::{BoundedModule, Module};
use ext::chain_complex::ChainComplex;
use ext::resolution_homomorphism::ResolutionHomomorphism;
use ext::utils::{construct, Config};
use fp::matrix::Matrix;
use std::convert::TryInto;
use std::{fs::File, sync::Arc};

fn main() -> error::Result {
    let target: Config = query::with_default("Target module", "S_2", |name| {
        let target: Config = name.try_into()?;
        match target.module["type"].as_str() {
            Some("finite dimensional module") => Ok(target),
            _ => Err(String::from("Target must be finite dimensional")),
        }
    });

    let source: Config = query::with_default("Source module", "Cnu", |name| name.try_into());

    let shift_s: u32 = query::with_default("s of Ext class", "0", str::parse);
    let shift_n: i32 = query::with_default("n of Ext class", "0", str::parse);
    let shift_t = shift_n + shift_s as i32;

    #[cfg(feature = "concurrent")]
    let bucket = ext::utils::query_bucket();

    #[allow(clippy::redundant_closure)]
    let target_save_file = query::optional("Target save file", |s| File::open(s));

    let (target, source) = if target == source {
        let target = match target_save_file {
            Some(f) => construct(target, Some(f))?,
            None => {
                let s: u32 = query::with_default("Max target s", "10", str::parse);
                let n: i32 = query::with_default("Max target n", "10", str::parse);
                let target = construct(target, None)?;

                #[cfg(feature = "concurrent")]
                target.compute_through_stem_concurrent(
                    s + shift_s,
                    n + std::cmp::max(0, shift_n),
                    &bucket,
                );

                #[cfg(not(feature = "concurrent"))]
                target.compute_through_stem(s + shift_s, n + std::cmp::max(0, shift_n));

                target
            }
        };
        let target = Arc::new(target);
        (Arc::clone(&target), target)
    } else {
        match target_save_file {
            Some(f) => (
                Arc::new(construct(target, Some(f))?),
                Arc::new(construct(
                    source,
                    #[allow(clippy::redundant_closure)]
                    Some(query::raw("Source save file", |s| File::open(s))),
                )?),
            ),
            None => {
                let s = query::with_default("Max target s", "10", str::parse);
                let n: i32 = query::with_default("Max target n", "10", str::parse);
                let (target, source) = (construct(target, None)?, construct(source, None)?);

                #[cfg(feature = "concurrent")]
                {
                    source.compute_through_stem_concurrent(s + shift_s, n + shift_n, &bucket);
                    target.compute_through_stem_concurrent(s, n, &bucket);
                }

                #[cfg(not(feature = "concurrent"))]
                {
                    source.compute_through_stem(s + shift_s, n + shift_n);
                    target.compute_through_stem(s, n);
                }

                (Arc::new(target), Arc::new(source))
            }
        }
    };

    assert_eq!(source.prime(), target.prime());
    let p = source.prime();

    let target_module = target.complex().module(0);
    let hom = ResolutionHomomorphism::new(String::new(), source, target, shift_s, shift_t);

    eprintln!("\nInput Ext class to lift:");
    for output_t in 0..=target_module.max_degree() {
        let input_t = output_t + shift_t;
        let mut matrix = Matrix::new(
            p,
            hom.source.number_of_gens_in_bidegree(shift_s, input_t),
            target_module.dimension(output_t),
        );

        if matrix.rows() == 0 || matrix.columns() == 0 {
            hom.extend_step(shift_s, input_t, None);
        } else {
            for (idx, row) in matrix.iter_mut().enumerate() {
                let v: Vec<u32> =
                    query::raw(&format!("f(x_({}, {}, {}))", shift_s, input_t, idx), |s| {
                        let v = s[1..s.len() - 1]
                            .split(',')
                            .map(|x| x.parse::<u32>().map_err(|e| e.to_string()))
                            .collect::<Result<Vec<_>, String>>()?;
                        if v.len() != row.len() {
                            return Err(format!(
                                "Target has dimension {} but {} coordinates supplied",
                                row.len(),
                                v.len()
                            ));
                        }
                        Ok(v)
                    });
                for (i, &x) in v.iter().enumerate() {
                    row.set_entry(i, x);
                }
            }
            hom.extend_step(shift_s, input_t, Some(&matrix));
        }
    }

    #[cfg(not(feature = "concurrent"))]
    hom.extend_all();

    #[cfg(feature = "concurrent")]
    hom.extend_all_concurrent(&bucket);

    for (s, n, t) in hom.target.iter_stem() {
        if s + shift_s >= hom.source.next_homological_degree()
            || t + shift_t > hom.source.module(s + shift_s).max_computed_degree()
        {
            continue;
        }
        let matrix = hom.get_map(s + shift_s).hom_k(t);
        for (i, r) in matrix.iter().enumerate() {
            println!("F(x_({}, {}, {})) = {:?}", n, s, i, r);
        }
    }
    Ok(())
}
