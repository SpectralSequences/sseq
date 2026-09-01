//! Resolve $A_C/\tau$ over $\mathbb{F}_2$ — the algebraic Novikov $E_2$.
//!
//! $A_C/\tau$ is the mod-$\tau$ reduction of the C-motivic Steenrod algebra (over
//! $\mathbb{C}$, $p = 2$): a connected, finite-type $\mathbb{F}_2$-algebra, and an
//! ordinary [`algebra::Algebra`] ([`CTauAlgebra`]). We resolve
//! the trivial module $\mathbb{F}_2$ over it with the **unmodified** production
//! resolution engine; the generator counts in $\mathrm{Ext}^{s,t}_{A_C/\tau}$ are
//! the **algebraic Novikov $E_2$** ranks, equivalently the motivic Adams $E_2$ of
//! $C\tau$ (Gheorghe–Wang–Xu).
//!
//! This is Phase 1 of `MOTIVIC_PLAN.md`: the first end-to-end motivic numbers,
//! carrying essentially zero engine risk (nothing about the engine is motivic —
//! it just resolves over an $\mathbb{F}_2$-algebra it has never seen), and it
//! validates the $A_C/\tau$ algebra structure before the harder $\tau$-lift.
//!
//! Run with e.g. `cargo run --example resolve_motivic_ctau` (reads `Max n` / `Max
//! s`, defaulting to 30 / 15).

use std::sync::Arc;

use algebra::{CTauAlgebra, module::FDModule};
use bivec::BiVec;
use ext::{
    chain_complex::{ChainComplex, FiniteChainComplex, FreeChainComplex},
    resolution::Resolution,
};
use sseq::coordinates::Bidegree;

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging()?;

    let max = Bidegree::n_s(
        query::with_default("Max n", "30", str::parse),
        query::with_default("Max s", "15", str::parse),
    );

    // The trivial module F_2, concentrated in degree 0 (one generator, no
    // positive-degree action), over A_C/τ.
    let algebra = Arc::new(CTauAlgebra::new());
    let trivial = Arc::new(FDModule::new(
        algebra,
        "F2".to_string(),
        BiVec::from_vec(0, vec![1]),
    ));
    let cc: Arc<FiniteChainComplex<FDModule<CTauAlgebra>>> =
        Arc::new(FiniteChainComplex::ccdz(trivial));

    let res = Resolution::new(cc);
    res.compute_through_stem(max);

    println!("{}", res.graded_dimension_string());
    println!("n,s,num_gens");
    for b in res.iter_stem() {
        let num = res.number_of_gens_in_bidegree(b);
        if num > 0 {
            println!("{},{},{}", b.n(), b.s(), num);
        }
    }

    Ok(())
}
