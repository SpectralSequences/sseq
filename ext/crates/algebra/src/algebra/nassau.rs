//! This file implements the support for [Nassau's algorithm](https://arxiv.org/abs/1910.04063)
//! from the algebra side.
use fp::matrix::Matrix;
use fp::vector::FpVector;

use crate::algebra::milnor_algebra::{MilnorAlgebra, MilnorBasisElement};
use crate::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use crate::module::{FreeModule, Module};

/// A Milnor subalgebra to be used in [Nassau's algorithm](https://arxiv.org/abs/1910.04063). This
/// is equipped with an ordering of the signature as in Lemma 2.4 of the paper.
///
/// To implement the ordering, we seek to define an order isomorphism from the set of signatures
/// to `0..dimension`, which we store as a `u32`. Since the algorithm involes a loop whose length
/// is the dimension of the subalgebra, we are not going to run out of space.
///
/// Noting that the lexicographic ordering of a bit vector is just the usual ordering when we view
/// them as the binary digits (most-significant-bit first), the order isomorphism is given by
/// simply concatenating the p part of the basis element (this corresponds to ordering of
/// $\mathcal{P}$ where $P^s_t < P^{s'}_t$ if $s > s'$).
pub struct MilnorSubalgebra {
    profile: Vec<u32>,
}

impl MilnorSubalgebra {
    pub fn new(profile: Vec<u32>) -> Self {
        Self { profile }
    }

    /// Computes the signature of an element
    pub fn signature(&self, elt: &MilnorBasisElement) -> u32 {
        let mut signature = 0;
        for (&profile, &entry) in self.profile.iter().zip(&elt.p_part) {
            signature <<= profile;
            signature += entry as u32 & ((1 << profile) - 1);
        }
        signature
    }

    /// The dimension of the subalgebra is `1 << self.log_dim()`.
    pub fn log_dim(&self) -> u32 {
        self.profile.iter().copied().sum()
    }

    /// Give a list of basis elements in degree `degree` that has signature `signature`.
    pub fn signature_mask(
        &self,
        module: &FreeModule<MilnorAlgebra>,
        degree: i32,
        signature: u32,
    ) -> Vec<usize> {
        let algebra = module.algebra();
        (0..module.dimension(degree))
            .filter(move |&i| {
                let opgen = module.index_to_op_gen(degree, i);
                self.signature(
                    algebra.basis_element_from_index(opgen.operation_degree, opgen.operation_index),
                ) == signature
            })
            .collect()
    }

    /// Get the matrix of a free module homomorphism when restricted to the subquotient given by
    /// the signature.
    pub fn signature_matrix(
        &self,
        hom: &FreeModuleHomomorphism<FreeModule<MilnorAlgebra>>,
        degree: i32,
        signature: u32,
    ) -> Matrix {
        let p = hom.prime();
        let source = hom.source();
        let target = hom.target();
        let target_degree = degree - hom.degree_shift();

        let source_mask = self.signature_mask(&source, degree, signature);
        let target_mask = self.signature_mask(&target, degree - hom.degree_shift(), signature);

        let mut matrix = Matrix::new(p, source_mask.len(), target_mask.len());
        let mut scratch = FpVector::new(p, target.dimension(target_degree));

        for (i, x) in source_mask.into_iter().enumerate() {
            scratch.set_to_zero();
            hom.apply_to_basis_element(scratch.as_slice_mut(), 1, degree, x);
            matrix[i]
                .as_slice_mut()
                .add_masked(scratch.as_slice(), 1, &target_mask);
        }
        matrix
    }
}
