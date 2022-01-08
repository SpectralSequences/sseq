//! This file implements the support for [Nassau's algorithm](https://arxiv.org/abs/1910.04063).

use std::sync::{Arc, Mutex};

use crate::chain_complex::{ChainComplex, FreeChainComplex};
use algebra::combinatorics;
use algebra::milnor_algebra::{MilnorAlgebra, MilnorBasisElement, PPartEntry};
use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::{FreeModule, Module};
use algebra::Algebra;
use fp::matrix::{AugmentedMatrix, Matrix, MatrixSliceMut};
use fp::prime::ValidPrime;
use fp::vector::FpVector;
use itertools::Itertools;
use once::OnceVec;

/// This is the maximum number of new generators we expect in each bidegree. This affects how much
/// space we allocate when we are extending our resolutions. Having more than this many new
/// generators will result in a slowdown but not an error. It is relatively cheap to increment this
/// number if needs be, but up to the 140th stem we only see at most 8 new generators.
const MAX_NEW_GENS: usize = 10;

/// A Milnor subalgebra to be used in [Nassau's algorithm](https://arxiv.org/abs/1910.04063). This
/// is equipped with an ordering of the signature as in Lemma 2.4 of the paper.
///
/// To simplify implementation, we pick the ordering so that the (reverse) lexicographic ordering
/// in Lemma 2.4 is just the (reverse) lexicographic ordering of the P parts. This corresponds to
/// the ordering of $\mathcal{P}$ where $P^s_t < P^{s'}_t$ if $s < s'$).
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

/// A resolution of a chain complex.
pub struct Resolution {
    lock: Mutex<()>,
    modules: OnceVec<Arc<FreeModule<MilnorAlgebra>>>,
    zero_module: Arc<FreeModule<MilnorAlgebra>>,
    differentials: OnceVec<Arc<FreeModuleHomomorphism<FreeModule<MilnorAlgebra>>>>,
}

impl Default for Resolution {
    fn default() -> Self {
        Self::new()
    }
}

impl Resolution {
    pub const fn prime(&self) -> ValidPrime {
        ValidPrime::new(2)
    }

    pub fn new() -> Self {
        let algebra = Arc::new(MilnorAlgebra::new(ValidPrime::new(2)));

        Self {
            lock: Mutex::new(()),
            zero_module: Arc::new(FreeModule::new(algebra, "F_{-1}".to_string(), 0)),
            modules: OnceVec::new(),
            differentials: OnceVec::new(),
        }
    }

    /// This function prepares the Resolution object to perform computations up to the
    /// specified s degree. It does *not* perform any computations by itself. It simply lengthens
    /// the `OnceVec`s `modules`, `chain_maps`, etc. to the right length.
    fn extend_through_degree(&self, max_s: u32) {
        let min_degree = self.min_degree();

        for i in self.modules.len() as u32..=max_s {
            self.modules.push(Arc::new(FreeModule::new(
                Arc::clone(&self.algebra()),
                format!("F{}", i),
                min_degree,
            )));
        }

        if self.differentials.is_empty() {
            self.differentials
                .push(Arc::new(FreeModuleHomomorphism::new(
                    Arc::clone(&self.modules[0u32]),
                    Arc::clone(&self.zero_module),
                    0,
                )));
        }

        for i in self.differentials.len() as u32..=max_s {
            self.differentials
                .push(Arc::new(FreeModuleHomomorphism::new(
                    Arc::clone(&self.modules[i]),
                    Arc::clone(&self.modules[i - 1]),
                    0,
                )));
        }
    }

    fn step_resolution_with_subalgebra(&self, s: u32, t: i32, subalgebra: MilnorSubalgebra) {
        let p = self.prime();

        let source = &*self.modules[s];
        let target = &*self.modules[s - 1];

        source.extend_table_entries(t);
        target.extend_table_entries(t);

        let zero_sig = subalgebra.zero_signature();
        let source_masked_dim = subalgebra.signature_mask(source, t, &zero_sig).count();
        let target_masked_dim = subalgebra.signature_mask(target, t, &zero_sig).count();

        // Compute kernel
        let ker = if s > 1 {
            self.modules[s - 2].extend_table_entries(t);

            let next_masked_dim = subalgebra
                .signature_mask(&self.modules[s - 2], t, &zero_sig)
                .count();
            let mut m =
                AugmentedMatrix::new(p, target_masked_dim, [next_masked_dim, target_masked_dim]);
            subalgebra.signature_matrix(
                &self.differential(s - 1),
                t,
                &zero_sig,
                &mut m.segment(0, 0),
            );
            m.segment(1, 1).add_identity();
            m.row_reduce();
            Some(m.compute_kernel())
        } else {
            None
        };

        let mut n =
            Matrix::new_with_capacity(p, source_masked_dim, target_masked_dim, MAX_NEW_GENS, 0);
        subalgebra.signature_matrix(&self.differential(s), t, &zero_sig, &mut n.as_slice_mut());
        n.row_reduce();

        let num_new_gens = if let Some(ker) = ker {
            n.extend_image(0, n.columns(), &ker, 0).len()
        } else {
            n.extend_to_surjection(0, n.columns(), 0).len()
        };

        if t < s as i32 {
            assert_eq!(num_new_gens, 0, "Adding generators at t = {t}, s = {s}");
        }

        source.add_generators(t, num_new_gens, None);

        if num_new_gens == 0 {
            self.differential(s).extend_by_zero(t);
            return;
        }

        let mut xs = vec![FpVector::new(p, target.dimension(t)); num_new_gens];
        let target_mask: Vec<usize> = subalgebra.signature_mask(target, t, &zero_sig).collect();

        for (x, x_masked) in xs.iter_mut().zip_eq(&n[source_masked_dim..]) {
            x.as_slice_mut()
                .add_unmasked(x_masked.as_slice(), 1, &target_mask)
        }
        if s == 1 {
            self.differential(s).add_generators_from_rows(t, xs);
            return;
        }

        let next = &self.modules[s - 2];

        let mut dxs = vec![FpVector::new(p, next.dimension(t)); num_new_gens];
        for (x, dx) in xs.iter().zip(&mut dxs) {
            self.differential(s - 1)
                .apply(dx.as_slice_mut(), 1, t, x.as_slice());
        }
        for signature in subalgebra.iter_signatures(t) {
            let target_mask: Vec<usize> =
                subalgebra.signature_mask(target, t, &signature).collect();
            let next_mask: Vec<usize> = subalgebra.signature_mask(next, t, &signature).collect();

            let mut m =
                AugmentedMatrix::new(p, target_mask.len(), [next_mask.len(), target_mask.len()]);
            subalgebra.signature_matrix(
                &self.differential(s - 1),
                t,
                &signature,
                &mut m.segment(0, 0),
            );
            m.segment(1, 1).add_identity();
            m.row_reduce();
            let qi = m.compute_quasi_inverse();
            let pivots = qi.pivots().unwrap();
            let preimage = qi.preimage();

            let mut scratch = FpVector::new(p, target_mask.len());

            for (x, dx) in xs.iter_mut().zip(&mut dxs) {
                scratch.set_to_zero();
                let mut row = 0;
                for (i, &v) in next_mask.iter().enumerate() {
                    if pivots[i] < 0 {
                        continue;
                    }
                    if dx.entry(v) != 0 {
                        scratch.add(&preimage[row], 1);
                    }
                    row += 1;
                }
                for (i, _) in scratch.iter_nonzero() {
                    x.add_basis_element(target_mask[i], 1);
                    self.differential(s - 1).apply_to_basis_element(
                        dx.as_slice_mut(),
                        1,
                        t,
                        target_mask[i],
                    );
                }
            }
        }
        for dx in &dxs {
            assert!(dx.is_zero(), "dx non-zero at t = {t}, s = {s}");
        }
        self.differential(s).add_generators_from_rows(t, xs);
    }

    fn step_resolution(&self, s: u32, t: i32) {
        if s == 0 {
            self.zero_module.extend_by_zero(t);

            if t == 0 {
                self.modules[0usize].add_generators(t, 1, None);
            } else {
                self.modules[0usize].extend_by_zero(t);
            }
            self.differentials[0usize].extend_by_zero(t);
            self.modules[0usize].extend_table_entries(t);

            return;
        }
        if s == 1 && t == 0 {
            // We special case this because we don't add any new generators
            self.modules[1usize].extend_by_zero(0);
            self.differentials[1usize].extend_by_zero(0);
            return;
        }

        if t <= s as i32 {
            self.step_resolution_with_subalgebra(s, t, MilnorSubalgebra::new(vec![]));
        } else if t > 3 * (s as i32 + 1) + 6 {
            self.step_resolution_with_subalgebra(s, t, MilnorSubalgebra::new(vec![2, 1]));
        } else {
            self.step_resolution_with_subalgebra(s, t, MilnorSubalgebra::new(vec![1]));
        }
    }

    /// This function resolves up till a fixed stem instead of a fixed t.
    pub fn compute_through_stem(&self, max_s: u32, max_n: i32) {
        let _lock = self.lock.lock();
        let max_t = max_s as i32 + max_n;

        self.extend_through_degree(max_s);
        self.algebra().compute_basis(max_t);

        for t in 0..=max_t {
            let start_s = std::cmp::max(0, t - max_n) as u32;
            for s in start_s..=max_s {
                if self.has_computed_bidegree(s, t) {
                    continue;
                }
                self.step_resolution(s, t);
            }
        }
    }
}

impl ChainComplex for Resolution {
    type Algebra = MilnorAlgebra;
    type Module = FreeModule<Self::Algebra>;
    type Homomorphism = FreeModuleHomomorphism<FreeModule<Self::Algebra>>;

    fn algebra(&self) -> Arc<Self::Algebra> {
        self.zero_module.algebra()
    }

    fn module(&self, s: u32) -> Arc<Self::Module> {
        Arc::clone(&self.modules[s as usize])
    }

    fn zero_module(&self) -> Arc<Self::Module> {
        Arc::clone(&self.zero_module)
    }

    fn min_degree(&self) -> i32 {
        0
    }

    fn has_computed_bidegree(&self, s: u32, t: i32) -> bool {
        self.differentials.len() > s as usize && self.differential(s).next_degree() > t
    }

    fn set_homology_basis(&self, _s: u32, _t: i32, _homology_basis: Vec<usize>) {
        unimplemented!()
    }

    fn homology_basis(&self, _s: u32, _t: i32) -> &Vec<usize> {
        unimplemented!()
    }

    fn homology_dimension(&self, s: u32, t: i32) -> usize {
        self.number_of_gens_in_bidegree(s, t)
    }

    fn max_homology_degree(&self, _s: u32) -> i32 {
        unimplemented!()
    }

    fn differential(&self, s: u32) -> Arc<Self::Homomorphism> {
        Arc::clone(&self.differentials[s as usize])
    }

    fn compute_through_bidegree(&self, max_s: u32, max_t: i32) {
        let _lock = self.lock.lock();

        self.extend_through_degree(max_s);
        self.algebra().compute_basis(max_t);

        for t in 0..=max_t {
            for s in 0..=max_s {
                if self.has_computed_bidegree(s, t) {
                    continue;
                }
                self.step_resolution(s, t);
            }
        }
    }

    fn next_homological_degree(&self) -> u32 {
        self.modules.len() as u32
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::chain_complex::FreeChainComplex;
    use expect_test::expect;

    #[test]
    fn test_restart_stem() {
        let res = Resolution::new();
        res.compute_through_stem(8, 14);
        res.compute_through_bidegree(5, 19);

        expect![[r#"
            ·                             
            ·                     ·       
            ·                   · ·     · 
            ·                 ·   ·     · 
            ·             ·   ·         · · 
            ·     ·       · · ·         · ·   
            ·   · ·     · · ·           · · ·   
            · ·   ·       ·               ·       
            ·                                       
        "#]]
        .assert_eq(&res.graded_dimension_string());
    }

    #[test]
    fn test_signature_iterator() {
        let subalgebra = MilnorSubalgebra::new(vec![2, 1]);
        assert_eq!(
            subalgebra.iter_signatures(6).collect::<Vec<_>>(),
            vec![
                vec![1, 0],
                vec![2, 0],
                vec![3, 0],
                vec![0, 1],
                vec![1, 1],
                vec![2, 1],
                vec![3, 1],
            ]
        );

        assert_eq!(
            subalgebra.iter_signatures(5).collect::<Vec<_>>(),
            vec![
                vec![1, 0],
                vec![2, 0],
                vec![3, 0],
                vec![0, 1],
                vec![1, 1],
                vec![2, 1],
            ]
        );
        assert_eq!(
            subalgebra.iter_signatures(4).collect::<Vec<_>>(),
            vec![vec![1, 0], vec![2, 0], vec![3, 0], vec![0, 1], vec![1, 1],]
        );
        assert_eq!(
            subalgebra.iter_signatures(3).collect::<Vec<_>>(),
            vec![vec![1, 0], vec![2, 0], vec![3, 0], vec![0, 1],]
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
            vec![vec![0, 1, 0, 0], vec![0, 2, 0, 0], vec![0, 0, 1, 0],]
        );
    }
}
