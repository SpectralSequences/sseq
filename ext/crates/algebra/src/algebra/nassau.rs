//! This file implements the support for [Nassau's algorithm](https://arxiv.org/abs/1910.04063)
//! from the algebra side.
use fp::matrix::MatrixSliceMut;
use fp::prime::ValidPrime;
use fp::vector::FpVector;

use crate::algebra::combinatorics;
use crate::algebra::milnor_algebra::{MilnorAlgebra, MilnorBasisElement, PPartEntry};
use crate::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use crate::module::{FreeModule, Module};

/// A Milnor subalgebra to be used in [Nassau's algorithm](https://arxiv.org/abs/1910.04063). This
/// is equipped with an ordering of the signature as in Lemma 2.4 of the paper.
///
/// To simplify implementation, we pick the ordering so that the lexicographic ordering in Lemma
/// 2.4 is just the lexicographic ordering of the P parts. This corresponds to the ordering of
/// $\mathcal{P}$ where $P^s_t < P^{s'}_t$ if $s > s'$).
pub struct MilnorSubalgebra {
    profile: Vec<u8>,
}

impl MilnorSubalgebra {
    /// This should be used when you want an entry of the profile to be infinity
    pub const INFINITY: u8 = (std::mem::size_of::<PPartEntry>() * 4 - 1) as u8;

    pub fn new(profile: Vec<u8>) -> Self {
        Self { profile }
    }

    /// Computes the signature of an element
    pub fn has_signature(&self, elt: &MilnorBasisElement, signature: &[PPartEntry]) -> bool {
        for (i, (&profile, &signature)) in self.profile.iter().zip(signature).enumerate() {
            let ppart = elt.p_part.get(i).copied().unwrap_or(0);
            if ppart & ((1 << profile) - 1) != signature {
                return false;
            }
        }
        true
    }

    pub fn zero_signature(&self) -> Vec<PPartEntry> {
        vec![0; self.profile.len()]
    }

    /// Give a list of basis elements in degree `degree` that has signature `signature`.
    pub fn signature_mask<'a>(
        &'a self,
        module: &'a FreeModule<MilnorAlgebra>,
        degree: i32,
        signature: &'a [PPartEntry],
    ) -> impl Iterator<Item = usize> + 'a {
        let algebra = module.algebra();
        (0..module.dimension(degree)).filter(move |&i| {
            let opgen = module.index_to_op_gen(degree, i);
            self.has_signature(
                algebra.basis_element_from_index(opgen.operation_degree, opgen.operation_index),
                signature,
            )
        })
    }

    /// Get the matrix of a free module homomorphism when restricted to the subquotient given by
    /// the signature.
    pub fn signature_matrix(
        &self,
        hom: &FreeModuleHomomorphism<FreeModule<MilnorAlgebra>>,
        degree: i32,
        signature: &[PPartEntry],
        matrix: &mut MatrixSliceMut,
    ) {
        let p = hom.prime();
        let source = hom.source();
        let target = hom.target();
        let target_degree = degree - hom.degree_shift();

        let source_mask = self.signature_mask(&source, degree, signature);
        let target_mask: Vec<usize> = self
            .signature_mask(&target, degree - hom.degree_shift(), signature)
            .collect();

        let mut scratch = FpVector::new(p, target.dimension(target_degree));

        for (mut row, masked_index) in matrix.iter_mut().zip(source_mask) {
            scratch.set_to_zero();
            hom.apply_to_basis_element(scratch.as_slice_mut(), 1, degree, masked_index);
            row.add_masked(scratch.as_slice(), 1, &target_mask);
        }
    }

    /// Iterate through all signatures of this algebra that contain elements of degree at most
    /// `degree` (inclusive). This skips the initial zero signature
    pub fn iter_signatures(&self, degree: i32) -> impl Iterator<Item = Vec<PPartEntry>> + '_ {
        SignatureIterator::new(self, degree)
    }
}

struct SignatureIterator<'a> {
    subalgebra: &'a MilnorSubalgebra,
    current: Vec<PPartEntry>,
    signature_degree: i32,
    degree: i32,
}

impl<'a> SignatureIterator<'a> {
    fn new(subalgebra: &'a MilnorSubalgebra, degree: i32) -> Self {
        Self {
            current: vec![0; subalgebra.profile.len()],
            degree,
            subalgebra,
            signature_degree: 0,
        }
    }
}

impl<'a> Iterator for SignatureIterator<'a> {
    type Item = Vec<PPartEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        let xi_degrees = combinatorics::xi_degrees(ValidPrime::new(2));
        let len = self.current.len();
        for (i, current) in self.current.iter_mut().enumerate() {
            *current += 1;
            self.signature_degree += xi_degrees[i];

            if self.signature_degree > self.degree || *current == 1 << self.subalgebra.profile[i] {
                self.signature_degree -= xi_degrees[i] * *current as i32;
                *current = 0;
                if i + 1 == len {
                    return None;
                }
            } else {
                return Some(self.current.clone());
            }
        }
        // This only happens when the profile is trivial
        assert!(self.current.is_empty());
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_signature_iterator() {
        let subalgebra = MilnorSubalgebra::new(vec![2, 1]);
        assert_eq!(
            subalgebra.iter_signatures(6).collect::<Vec<_>>(),
            vec![
                vec![0, 1],
                vec![1, 0],
                vec![1, 1],
                vec![2, 0],
                vec![2, 1],
                vec![3, 0],
                vec![3, 1],
            ]
        );

        assert_eq!(
            subalgebra.iter_signatures(5).collect::<Vec<_>>(),
            vec![
                vec![0, 1],
                vec![1, 0],
                vec![1, 1],
                vec![2, 0],
                vec![2, 1],
                vec![3, 0],
            ]
        );
        assert_eq!(
            subalgebra.iter_signatures(4).collect::<Vec<_>>(),
            vec![vec![0, 1], vec![1, 0], vec![1, 1], vec![2, 0], vec![3, 0],]
        );
        assert_eq!(
            subalgebra.iter_signatures(3).collect::<Vec<_>>(),
            vec![vec![0, 1], vec![1, 0], vec![2, 0], vec![3, 0],]
        );
        assert_eq!(
            subalgebra.iter_signatures(2).collect::<Vec<_>>(),
            vec![vec![1, 0], vec![2, 0],]
        );
        assert_eq!(
            subalgebra.iter_signatures(1).collect::<Vec<_>>(),
            vec![vec![1, 0],]
        );
        assert_eq!(
            subalgebra.iter_signatures(0).collect::<Vec<_>>(),
            Vec::<Vec<PPartEntry>>::new()
        );
    }

    #[test]
    fn test_signature_iterator_large() {
        let subalgebra = MilnorSubalgebra::new(vec![
            0,
            MilnorSubalgebra::INFINITY,
            MilnorSubalgebra::INFINITY,
            MilnorSubalgebra::INFINITY,
        ]);
        assert_eq!(
            subalgebra.iter_signatures(7).collect::<Vec<_>>(),
            vec![vec![0, 0, 1, 0], vec![0, 1, 0, 0], vec![0, 2, 0, 0],]
        );
    }
}
