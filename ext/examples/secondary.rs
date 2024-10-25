//! This computes $d_2$ differentials in the Adams spectral sequence. This only works for fairly
//! specific modules, but tends to cover most cases of interest.
//!
//! # Usage
//! This asks for a module in the usual way, and verifies that the module satisfies the conditions
//! necessary for the algorithm the work. It only works with the Milnor basis.
//!
//! # Output
//! We omit differentials if the target bidegree is zero.
//!
//! # Sharding
//! *This section applies to all of the secondary scripts, namely `secondary`,
//! [`secondary_product`](../secondary_product/index.html)
//! and [`secondary_massey`](../secondary_massey/index.html).*
//!
//! Most of the computation can be fully distributed. Rudimentary sharding over multiple machines
//! is currently supported, where each machine works on a single `s`.
//!
//! These machines should share the same save directory (e.g. over a network-mounted drive), and
//! all prerequisites for the computation must have been computed. This includes the "primary" data
//! (resolutions, lifts, etc.) as well as the secondary prerequisites (the secondary resolution for
//! secondary products, secondary products for secondary Massey product). Otherwise, conflicts may
//! arise.
//!
//! To compute data for a single `s`, run the script with the environment variable
//! `SECONDARY_JOB=s`. The minimum value of `s` is the cohomological degree shift of the secondary
//! homotopy (i.e. the difference in degrees between the input class and the λ part of the answer;
//! 2 in the case of `secondary`), and the maximum value of `s` is the `max_s` of the resolution.
//!
//! After running this script for all `s` in the range, run it as usual to produce the final
//! output. An example script is as follows:
//!
//! ```shell
//! #!/bin/sh
//!
//! cargo run --example resolve_through_stem S_2 /tmp/save 40 20;
//!
//! cargo build --example secondary
//! for n in `seq 20 -1 2`; do
//!     SECONDARY_JOB=$n target/debug/examples/secondary S_2 /tmp/save 40 20 &
//! done
//!
//! wait
//!
//! target/debug/examples/secondary S_2 /tmp/save 40 20;
//! ```

use std::sync::Arc;

use algebra::module::Module;
use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    secondary::*,
    utils::query_module,
};
use sseq::coordinates::{Bidegree, BidegreeGenerator};

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging();

    let resolution = Arc::new(query_module(Some(algebra::AlgebraType::Milnor), true)?);

    let lift = SecondaryResolution::new(Arc::clone(&resolution));
    if let Some(s) = ext::utils::secondary_job() {
        lift.compute_partial(s);
        return Ok(());
    }

    lift.extend_all();

    let d2_shift = Bidegree::n_s(-1, 2);

    // Iterate through target of the d2
    for b in lift.underlying().iter_nonzero_stem() {
        if b.s() < 3 {
            continue;
        }

        if b.t() - 1 > resolution.module(b.s() - 2).max_computed_degree() {
            continue;
        }
        let homotopy = lift.homotopy(b.s());
        let m = homotopy.homotopies.hom_k(b.t() - 1);

        for (i, entry) in m.into_iter().enumerate() {
            let source_gen = BidegreeGenerator::new(b - d2_shift, i);
            println!("d_2 x_{source_gen} = {entry:?}");
        }
    }

    Ok(())
}
