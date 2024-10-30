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
//! `F_2` resp. `M_k`s and constructing [`ResolutionHomomorphism`]s
//! corresponding to the bottom and (-1)-cells.
//! Given `Max k`, it will print Mahowald invariants of the `F_2`-basis elements of
//! `Ext^{*,*}(F_2)` that are detected in `Ext^{*,*}(M_k)` for the first time for some
//! `k <= Max k`.
//!
//! [mahowald--ravenel]: https://www.sciencedirect.com/science/article/pii/004093839390055Z
//! [bruner--greenlees]: https://projecteuclid.org/journals/experimental-mathematics/volume-4/issue-4/The-Bredon-L%C3%B6ffler-conjecture/em/1047674389.full

use std::{fmt, iter, num::NonZeroI32, path::PathBuf, sync::Arc};

use algebra::{
    module::{homomorphism::ModuleHomomorphism, Module},
    AlgebraType, SteenrodAlgebra,
};
use anyhow::Result;
use ext::{
    chain_complex::{ChainComplex, FiniteChainComplex, FreeChainComplex},
    resolution::MuResolution,
    resolution_homomorphism::{MuResolutionHomomorphism, ResolutionHomomorphism},
    utils,
};
use fp::{matrix::Matrix, prime::TWO, vector::FpVector};
use serde_json::json;
use sseq::coordinates::{Bidegree, BidegreeElement, BidegreeGenerator};

fn main() -> Result<()> {
    ext::utils::init_logging()?;

    let s_2_path: Option<PathBuf> = query::optional("Save directory for S_2", str::parse);
    let p_k_prefix: Option<PathBuf> = query::optional(
        "Directory containing save directories for RP_-k_inf's",
        str::parse,
    );
    // Going up to k=25 is nice because then we see an invariant that is not a basis element
    // and one that has non-trivial indeterminacy.
    let k_max = query::with_default("Max k (positive)", "25", str::parse::<NonZeroI32>).get();

    let s_2_resolution = resolve_s_2(s_2_path, k_max)?;

    println!("M({{basis element}}) = {{mahowald_invariant}}[ mod {{indeterminacy}}]");
    for k in 1..=k_max {
        let p_k = PKData::try_new(k, &p_k_prefix, &s_2_resolution)?;
        for mi in p_k.mahowald_invariants() {
            println!("{mi}")
        }
    }

    Ok(())
}

type Resolution =
    MuResolution<false, FiniteChainComplex<Box<dyn Module<Algebra = SteenrodAlgebra>>>>;

type Homomorphism = MuResolutionHomomorphism<false, Resolution, Resolution>;

struct PKData {
    k: i32,
    resolution: Arc<Resolution>,
    bottom_cell: Homomorphism,
    minus_one_cell: Homomorphism,
    s_2_resolution: Arc<Resolution>,
}

struct MahowaldInvariant {
    g: BidegreeGenerator,
    output_t: i32,
    invariant: FpVector,
    indeterminacy_basis: Vec<FpVector>,
}

fn resolve_s_2(s_2_path: Option<PathBuf>, k_max: i32) -> Result<Arc<Resolution>> {
    let s_2_resolution = Arc::new(utils::construct_standard("S_2", s_2_path)?);
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
    s_2_resolution.compute_through_stem(Bidegree::n_s(2 * k_max - 2, k_max / 2 + 1));
    Ok(s_2_resolution)
}

impl PKData {
    fn try_new(
        k: i32,
        p_k_prefix: &Option<PathBuf>,
        s_2_resolution: &Arc<Resolution>,
    ) -> Result<Self> {
        let p_k_config = json! ({
            "p": 2,
            "type": "real projective space",
            "min": -k,
        });
        let mut p_k_path = p_k_prefix.clone();
        if let Some(p) = p_k_path.as_mut() {
            p.push(PathBuf::from(&format!("RP_{minus_k}_inf", minus_k = -k)));
        };
        let resolution = Arc::new(utils::construct_standard(
            (p_k_config, AlgebraType::Milnor),
            p_k_path,
        )?);
        // As mentioned before, RP_-k_inf won't detect Mahowald invariants of any classes in the
        // k-stem and beyond or of any classes of filtration higher than k/2+1.
        resolution.compute_through_stem(Bidegree::n_s(k - 2, k / 2 + 1));

        let bottom_cell = ResolutionHomomorphism::from_class(
            String::from("bottom_cell"),
            resolution.clone(),
            s_2_resolution.clone(),
            Bidegree::s_t(0, -k),
            &[1],
        );
        bottom_cell.extend_all();

        let minus_one_cell = ResolutionHomomorphism::from_class(
            String::from("minus_one_cell"),
            resolution.clone(),
            s_2_resolution.clone(),
            Bidegree::s_t(0, -1),
            &[1],
        );
        minus_one_cell.extend_all();

        Ok(PKData {
            k,
            resolution,
            bottom_cell,
            minus_one_cell,
            s_2_resolution: s_2_resolution.clone(),
        })
    }

    fn mahowald_invariants(&self) -> impl Iterator<Item = MahowaldInvariant> + '_ {
        self.s_2_resolution
            .iter_stem()
            .flat_map(|b| self.mahowald_invariants_for_bidegree(b))
    }

    fn mahowald_invariants_for_bidegree(
        &self,
        b: Bidegree,
    ) -> Box<dyn Iterator<Item = MahowaldInvariant> + '_> {
        let b_p_k = b - Bidegree::s_t(0, 1);
        if self.resolution.has_computed_bidegree(b_p_k) {
            let b_bottom = b_p_k + Bidegree::s_t(0, self.k);
            let bottom_s_2_gens = self.s_2_resolution.number_of_gens_in_bidegree(b_bottom);
            let minus_one_s_2_gens = self.s_2_resolution.number_of_gens_in_bidegree(b);
            let p_k_gens = self.resolution.number_of_gens_in_bidegree(b_p_k);
            if bottom_s_2_gens > 0 && minus_one_s_2_gens > 0 && p_k_gens > 0 {
                let bottom_cell_map = self.bottom_cell.get_map(b_bottom.s());
                let mut matrix = vec![vec![0; p_k_gens]; bottom_s_2_gens];
                for p_k_gen in 0..p_k_gens {
                    let output = bottom_cell_map.output(b_p_k.t(), p_k_gen);
                    for (s_2_gen, row) in matrix.iter_mut().enumerate() {
                        let index = bottom_cell_map.target().operation_generator_to_index(
                            0,
                            0,
                            b_bottom.t(),
                            s_2_gen,
                        );
                        row[p_k_gen] = output.entry(index);
                    }
                }
                let (padded_columns, mut matrix) = Matrix::augmented_from_vec(TWO, &matrix);
                let rank = matrix.row_reduce();

                if rank > 0 {
                    let kernel_subspace = matrix.compute_kernel(padded_columns);
                    let indeterminacy_basis = kernel_subspace.basis().to_vec();
                    let image_subspace = matrix.compute_image(p_k_gens, padded_columns);
                    let quasi_inverse = matrix.compute_quasi_inverse(p_k_gens, padded_columns);

                    let it = (0..minus_one_s_2_gens).filter_map(move |i| {
                        let mut image = FpVector::new(TWO, p_k_gens);
                        let g = BidegreeGenerator::new(b, i);
                        self.minus_one_cell.act(image.as_slice_mut(), 1, g);
                        if !image.is_zero() && image_subspace.contains(image.as_slice()) {
                            let mut invariant = FpVector::new(TWO, bottom_s_2_gens);
                            quasi_inverse.apply(invariant.as_slice_mut(), 1, image.as_slice());
                            Some(MahowaldInvariant {
                                g,
                                output_t: b_bottom.t(),
                                invariant,
                                indeterminacy_basis: indeterminacy_basis.clone(),
                            })
                        } else {
                            None
                        }
                    });
                    return Box::new(it);
                }
            }
        }

        Box::new(iter::empty())
    }
}

impl fmt::Display for MahowaldInvariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        let output_t = self.output_t;
        let f2_vec_to_sum = |v: &FpVector| {
            let elt = BidegreeElement::new(Bidegree::s_t(self.g.s(), output_t), v.clone());
            elt.to_basis_string()
        };
        let indeterminacy_info = if self.indeterminacy_basis.is_empty() {
            String::new()
        } else {
            format!(
                " mod <{inner}>",
                inner = self
                    .indeterminacy_basis
                    .iter()
                    .map(f2_vec_to_sum)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let invariant = f2_vec_to_sum(&self.invariant);
        write!(f, "M(x_{g}) = {invariant}{indeterminacy_info}", g = self.g)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(1, 0, 0, 0, 0, vec![1], 0)]
    #[case(5, 1, 4, 0, 8, vec![1], 0)]
    #[case(18, 3, 17, 0, 34, vec![0, 1], 0)]
    #[case(25, 6, 20, 0, 44, vec![1, 0], 1)]
    fn test_mahowald_invariants(
        #[case] k: i32,
        #[case] s: i32,
        #[case] input_t: i32,
        #[case] input_i: usize,
        #[case] output_t: i32,
        #[case] invariant: Vec<u32>,
        #[case] indeterminacy_dim: usize,
    ) {
        let g = BidegreeGenerator::new(Bidegree::s_t(s, input_t), input_i);
        let s_2_resolution = resolve_s_2(None, k).unwrap();
        let p_k = PKData::try_new(k, &None, &s_2_resolution).unwrap();
        for mi in p_k.mahowald_invariants_for_bidegree(g.degree()) {
            if mi.g.idx() == g.idx() {
                assert_eq!(mi.output_t, output_t);
                assert_eq!(Vec::from(&mi.invariant), invariant);
                assert_eq!(mi.indeterminacy_basis.len(), indeterminacy_dim);
                return;
            }
        }
        panic!("could not find Mahowald invariant")
    }
}
