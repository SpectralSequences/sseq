//! Computes algebraic Mahowald invariants (aka algebraic root invariants).
//!
//! Sample output (with `Max k = 7`):
//! ```
//! M({basis element}) = {mahowald_invariant}[ mod {indeterminacy}]
//! M(x_(0, 0, 0)) = x_(0, 0, 0)
//! M(x_(1, 1, 0)) = x_(1, 2, 0)
//! M(x_(2, 2, 0)) = x_(2, 4, 0)
//! M(x_(1, 2, 0)) = x_(1, 4, 0)
//! M(x_(3, 3, 0)) = x_(3, 6, 0)
//! M(x_(2, 4, 0)) = x_(2, 8, 0)
//! M(x_(1, 4, 0)) = x_(1, 8, 0)
//! M(x_(2, 5, 0)) = x_(2, 10, 0)
//! M(x_(3, 6, 0)) = x_(3, 12, 0)
//! ```
//!
//! ---
//!
//! Here is a brief overview of what this example computes.
//! For details and beyond, see for instance
//! "[The root invariant in homotopy theory][mahowald--ravenel]" or
//! "[The Bredon-Löffler conjecture][bruner--greenlees]" (where the latter also contains machine
//! computations similar to what this example does).
//! In the following, we abbreviate `Ext^{s,t}_A(-, F_2)` as `Ext^{s,t}(-)`.
//!
//! Let `M_k` be the cohomology of `RP_-k_inf`.
//! There is an isomorphism  `Ext^{s, t}(F_2) ~ lim_k Ext^{s, t-1}(M_k)`
//! induced by the (-1)-cell `S^{-1} -> RP_-k_inf` at each level.
//! Let `x` be a class in `Ext^{s, t}(F_2)`.
//! Then there is a minimal `k` such that its image in `Ext^{s, t-1}(M_k)` is non-trivial.
//! Using the long exact sequence induced by the (co)fiber sequence
//! `S^{-k} -> RP_-k_inf -> RP_{-k+1}_inf` on the level of `Ext`, that image can be lifted to a
//! class `M(x)` in `Ext^{s, t + k - 1}`, which is (a representative for) the *(algebraic) Mahowald
//! invariant of `x`*.
//!
//! This script computes these lifts (and their indeterminacy) by resolving
//! `F_2` resp. `M_k`s and constructing
//! [`ResolutionHomomorphism`][ext::resolution_homomorphism::ResolutionHomomorphism]s
//! corresponding to the bottom and (-1)-cells.
//! Given `Max k`, it will print Mahowald invariants of the `F_2`-basis elements of
//! `Ext^{*,*}(F_2)` that are detected in `Ext^{*,*}(M_k)` for the first time for some
//! `k <= Max k`.
//!
//! [mahowald--ravenel]: https://www.sciencedirect.com/science/article/pii/004093839390055Z
//! [bruner--greenlees]: https://projecteuclid.org/journals/experimental-mathematics/volume-4/issue-4/The-Bredon-L%C3%B6ffler-conjecture/em/1047674389.full

use algebra::{
    module::{homomorphism::ModuleHomomorphism, Module},
    AlgebraType, SteenrodAlgebra,
};
use ext::{
    chain_complex::{ChainComplex, FiniteChainComplex, FreeChainComplex},
    resolution::MuResolution,
    resolution_homomorphism::{MuResolutionHomomorphism, ResolutionHomomorphism},
    utils,
};
use fp::{matrix::Matrix, prime::TWO, vector::FpVector};

use anyhow::Result;
use serde_json::json;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> Result<()> {
    let s_2_path: Option<PathBuf> = query::optional("Save directory for S_2", str::parse);
    let p_k_prefix: Option<PathBuf> = query::optional(
        "Directory containing save directories for RP_-k_inf's",
        str::parse,
    );
    // Going up to k=25 is nice because then we see an invariant that is not a basis element
    // and one that has non-trivial indeterminacy.
    let k_max = query::with_default("Max k (positive)", "25", str::parse::<NonZeroU32>).get();

    let s_2_resolution = resolve_s_2(s_2_path, k_max)?;

    println!("M({{basis element}}) = {{mahowald_invariant}}[ mod {{indeterminacy}}]");
    for k in 1..=k_max {
        let p_k = PKData::try_new(k, &p_k_prefix, &s_2_resolution)?;

        for (s, _, t) in s_2_resolution
            .iter_stem()
            .filter(|&(s, _, t)| p_k.resolution.has_computed_bidegree(s, t - 1))
        {
            let t_bottom = t + k as i32 - 1;
            let bottom_s_2_gens = s_2_resolution.number_of_gens_in_bidegree(s, t_bottom);
            let minus_one_s_2_gens = s_2_resolution.number_of_gens_in_bidegree(s, t);
            let t_p_k = t - 1;
            let p_k_gens = p_k.resolution.number_of_gens_in_bidegree(s, t_p_k);
            if bottom_s_2_gens > 0 && minus_one_s_2_gens > 0 && p_k_gens > 0 {
                let bottom_cell_map = p_k.bottom_cell.get_map(s);
                let mut matrix = vec![vec![0; p_k_gens]; bottom_s_2_gens];
                for p_k_gen in 0..p_k_gens {
                    let output = bottom_cell_map.output(t_p_k, p_k_gen);
                    for (s_2_gen, row) in matrix.iter_mut().enumerate() {
                        let index = bottom_cell_map
                            .target()
                            .operation_generator_to_index(0, 0, t_bottom, s_2_gen);
                        row[p_k_gen] = output.entry(index);
                    }
                }
                let (padded_columns, mut matrix) = Matrix::augmented_from_vec(TWO, &matrix);
                let rank = matrix.row_reduce();

                if rank > 0 {
                    let f2_vec_to_sum = |v: &FpVector| {
                        // We will only ever print non-zero vectors, so ignoring empty sums is fine.
                        v.iter()
                            .enumerate()
                            .filter_map(|(i, e)| {
                                if e == 1 {
                                    Some(format!("x_({s}, {t_bottom}, {i})"))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(" + ")
                    };

                    let kernel_subspace = matrix.compute_kernel(padded_columns);
                    let indeterminacy_info = if kernel_subspace.dimension() == 0 {
                        String::new()
                    } else {
                        format!(
                            " mod <{inner}>",
                            inner = kernel_subspace
                                .basis()
                                .iter()
                                .map(f2_vec_to_sum)
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    };
                    let image_subspace = matrix.compute_image(p_k_gens, padded_columns);
                    let quasi_inverse = matrix.compute_quasi_inverse(p_k_gens, padded_columns);

                    for i in 0..minus_one_s_2_gens {
                        let mut image = FpVector::new(TWO, p_k_gens);
                        p_k.minus_one_cell.act(image.as_slice_mut(), 1, s, t, i);
                        if !image.is_zero() && image_subspace.contains(image.as_slice()) {
                            let mut mahowald_invariant = FpVector::new(TWO, bottom_s_2_gens);
                            quasi_inverse.apply(
                                mahowald_invariant.as_slice_mut(),
                                1,
                                image.as_slice(),
                            );
                            let mahowald_invariant = f2_vec_to_sum(&mahowald_invariant);
                            println!(
                                "M(x_({s}, {t}, {i})) = {mahowald_invariant}{indeterminacy_info}",
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

type Resolution =
    MuResolution<false, FiniteChainComplex<Box<dyn Module<Algebra = SteenrodAlgebra>>>>;

type Homomorphism = MuResolutionHomomorphism<false, Resolution, Resolution>;

struct PKData {
    resolution: Arc<Resolution>,
    bottom_cell: Homomorphism,
    minus_one_cell: Homomorphism,
}

fn resolve_s_2(s_2_path: Option<PathBuf>, k_max: u32) -> Result<Arc<Resolution>> {
    let s_2_resolution = Arc::new(utils::construct("S_2", s_2_path)?);
    // Here are some bounds on the bidegrees in which we have should have resolutions available.
    //
    // A class in stem n won't be detected before RP_-{n+1}_inf, so we can only detect Mahowald
    // invariants of classes in stems <=k_max-1.
    // If an element in stem k_max-1 is detected in RP_-{k_max}_inf, then its Mahowald invariant
    // will be in stem 2*k_max-2, so we should resolve S_2 up to that stem.
    //
    // As for the filtration s, resolving up to (k/2)+1 will cover all classes in positive stems up
    // to k-1 because of the Adams vanishing line.
    // In the zero stem, the Mahowald invariant of x_(i, i, 0) (i.e. (h_0)^i) is the first element
    // of filtration i that is in a positive stem.
    // As that element appears by stem 2*i, resolving RP_-k_inf up to filtration (k/2)+1 is also
    // sufficient to detect Mahowald invariants of elements in the zero stem.
    s_2_resolution.compute_through_stem(k_max / 2 + 1, 2 * k_max as i32 - 2);
    Ok(s_2_resolution)
}

impl PKData {
    fn try_new(
        k: u32,
        p_k_prefix: &Option<PathBuf>,
        s_2_resolution: &Arc<Resolution>,
    ) -> Result<Self> {
        let p_k_config = json! ({
            "p": 2,
            "type": "real projective space",
            "min": -(k as i32),
        });
        let mut p_k_path = p_k_prefix.clone();
        if let Some(p) = p_k_path.as_mut() {
            p.push(PathBuf::from(&format!("RP_-{k}_inf")))
        };
        let resolution = Arc::new(utils::construct(
            (p_k_config, AlgebraType::Milnor),
            p_k_path,
        )?);
        // As mentioned before, RP_-k_inf won't detect Mahowald invariants of any classes in the
        // k-stem and beyond or of any classes of filtration higher than k/2+1.
        resolution.compute_through_stem(k / 2 + 1, k as i32 - 2);

        let bottom_cell = ResolutionHomomorphism::from_class(
            String::from("bottom_cell"),
            resolution.clone(),
            s_2_resolution.clone(),
            0,
            -(k as i32),
            &[1],
        );
        bottom_cell.extend_all();

        let minus_one_cell = ResolutionHomomorphism::from_class(
            String::from("minus_one_cell"),
            resolution.clone(),
            s_2_resolution.clone(),
            0,
            -1,
            &[1],
        );
        minus_one_cell.extend_all();

        Ok(PKData {
            resolution,
            bottom_cell,
            minus_one_cell,
        })
    }
}
