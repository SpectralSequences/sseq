use crate::chain_complex::{AugmentedChainComplex, ChainComplex, FiniteAugmentedChainComplex};
use crate::CCC;
use algebra::module::homomorphism::{
    BoundedModuleHomomorphism, FiniteModuleHomomorphism, ModuleHomomorphism,
};
use algebra::module::{FiniteModule, Module, SumModule, TensorModule, ZeroModule};
use algebra::{Algebra, Bialgebra, SteenrodAlgebra};
use fp::matrix::Matrix;
use fp::vector::{FpVector, Slice, SliceMut};
use std::sync::Arc;

use bivec::BiVec;
use once::{OnceBiVec, OnceVec};

pub type STM<M, N> = SumModule<TensorModule<M, N>>;

pub type TensorSquareCC<A, C> = TensorChainComplex<A, C, C>;

pub struct TensorChainComplex<A, CC1, CC2>
where
    A: Algebra + Bialgebra,
    CC1: ChainComplex<Algebra = A>,
    CC2: ChainComplex<Algebra = A>,
{
    left_cc: Arc<CC1>,
    right_cc: Arc<CC2>,
    modules: OnceVec<Arc<STM<CC1::Module, CC2::Module>>>,
    zero_module: Arc<STM<CC1::Module, CC2::Module>>,
    differentials: OnceVec<Arc<TensorChainMap<A, CC1, CC2>>>,
}

impl<A, CC1, CC2> TensorChainComplex<A, CC1, CC2>
where
    A: Algebra + Bialgebra,
    CC1: ChainComplex<Algebra = A>,
    CC2: ChainComplex<Algebra = A>,
{
    pub fn new(left_cc: Arc<CC1>, right_cc: Arc<CC2>) -> Self {
        Self {
            modules: OnceVec::new(),
            differentials: OnceVec::new(),
            zero_module: Arc::new(SumModule::zero_module(
                left_cc.algebra(),
                left_cc.min_degree() + right_cc.min_degree(),
            )),
            left_cc,
            right_cc,
        }
    }

    fn left_cc(&self) -> Arc<CC1> {
        Arc::clone(&self.left_cc)
    }

    fn right_cc(&self) -> Arc<CC2> {
        Arc::clone(&self.right_cc)
    }
}

impl<A, CC> TensorChainComplex<A, CC, CC>
where
    A: Algebra + Bialgebra,
    CC: ChainComplex<Algebra = A>,
{
    /// This function sends a (x) b to b (x) a. This makes sense only if left_cc and right_cc are
    /// equal, but we don't check that.
    pub fn swap(&self, result: &mut FpVector, vec: &FpVector, s: u32, t: i32) {
        let s = s as usize;

        for left_s in 0..=s {
            let right_s = s - left_s;
            let module = &self.modules[s];

            let source_offset = module.offset(t, left_s);
            let target_offset = module.offset(t, right_s);

            for left_t in 0..=t {
                let right_t = t - left_t;

                let left_dim = module.modules[left_s].left.dimension(left_t);
                let right_dim = module.modules[left_s].right.dimension(right_t);

                if left_dim == 0 || right_dim == 0 {
                    continue;
                }

                let source_inner_offset = module.modules[left_s].offset(t, left_t);
                let target_inner_offset = module.modules[right_s].offset(t, right_t);

                for i in 0..left_dim {
                    for j in 0..right_dim {
                        let value =
                            vec.entry(source_offset + source_inner_offset + i * right_dim + j);
                        if value != 0 {
                            result.add_basis_element(
                                target_offset + target_inner_offset + j * left_dim + i,
                                value,
                            );
                        }
                    }
                }
            }
        }
    }
}

impl<A, CC1, CC2> ChainComplex for TensorChainComplex<A, CC1, CC2>
where
    A: Algebra + Bialgebra,
    CC1: ChainComplex<Algebra = A>,
    CC2: ChainComplex<Algebra = A>,
{
    type Algebra = A;
    type Module = STM<CC1::Module, CC2::Module>;
    type Homomorphism = TensorChainMap<A, CC1, CC2>;

    fn algebra(&self) -> Arc<A> {
        self.left_cc.algebra()
    }

    fn min_degree(&self) -> i32 {
        self.left_cc.min_degree() + self.right_cc.min_degree()
    }

    fn zero_module(&self) -> Arc<Self::Module> {
        Arc::clone(&self.zero_module)
    }

    fn has_computed_bidegree(&self, s: u32, t: i32) -> bool {
        self.left_cc
            .has_computed_bidegree(s, t - self.right_cc.min_degree())
            && self
                .right_cc
                .has_computed_bidegree(s, t - self.left_cc.min_degree())
            && self.differentials.len() > s as usize
    }

    fn module(&self, s: u32) -> Arc<Self::Module> {
        Arc::clone(&self.modules[s as usize])
    }

    fn differential(&self, s: u32) -> Arc<Self::Homomorphism> {
        Arc::clone(&self.differentials[s as usize])
    }

    fn compute_through_bidegree(&self, s: u32, t: i32) {
        self.left_cc
            .compute_through_bidegree(s, t - self.right_cc.min_degree());
        self.right_cc
            .compute_through_bidegree(s, t - self.left_cc.min_degree());

        self.modules.extend(s as usize, |i| {
            let i = i as u32;
            let new_module_list: Vec<Arc<TensorModule<CC1::Module, CC2::Module>>> = (0..=i)
                .map(|j| {
                    Arc::new(TensorModule::new(
                        self.left_cc.module(j),
                        self.right_cc.module(i - j),
                    ))
                })
                .collect::<Vec<_>>();
            Arc::new(SumModule::new(
                self.algebra(),
                new_module_list,
                self.min_degree(),
            ))
        });

        for module in self.modules.iter() {
            module.compute_basis(t);
        }

        self.differentials.extend(s as usize, |s| {
            let s = s as u32;
            if s == 0 {
                Arc::new(TensorChainMap {
                    left_cc: self.left_cc(),
                    right_cc: self.right_cc(),
                    source_s: 0,
                    source: self.module(0),
                    target: self.zero_module(),
                    quasi_inverses: OnceBiVec::new(self.min_degree()),
                })
            } else {
                Arc::new(TensorChainMap {
                    left_cc: self.left_cc(),
                    right_cc: self.right_cc(),
                    source_s: s,
                    source: self.module(s),
                    target: self.module(s - 1),
                    quasi_inverses: OnceBiVec::new(self.min_degree()),
                })
            }
        });
    }

    fn set_homology_basis(
        &self,
        _homological_degree: u32,
        _internal_degree: i32,
        _homology_basis: Vec<usize>,
    ) {
        unimplemented!()
    }
    fn homology_basis(&self, _homological_degree: u32, _internal_degree: i32) -> &Vec<usize> {
        unimplemented!()
    }
    fn max_homology_degree(&self, _homological_degree: u32) -> i32 {
        unimplemented!()
    }

    fn next_homological_degree(&self) -> u32 {
        self.modules.len() as u32
    }
}

pub struct TensorChainMap<A, CC1, CC2>
where
    A: Algebra + Bialgebra,
    CC1: ChainComplex<Algebra = A>,
    CC2: ChainComplex<Algebra = A>,
{
    left_cc: Arc<CC1>,
    right_cc: Arc<CC2>,
    source_s: u32,
    source: Arc<STM<CC1::Module, CC2::Module>>,
    target: Arc<STM<CC1::Module, CC2::Module>>,
    quasi_inverses: OnceBiVec<Vec<Option<Vec<(usize, usize, FpVector)>>>>,
}

impl<A, CC1, CC2> ModuleHomomorphism for TensorChainMap<A, CC1, CC2>
where
    A: Algebra + Bialgebra,
    CC1: ChainComplex<Algebra = A>,
    CC2: ChainComplex<Algebra = A>,
{
    type Source = STM<CC1::Module, CC2::Module>;
    type Target = STM<CC1::Module, CC2::Module>;

    fn source(&self) -> Arc<Self::Source> {
        Arc::clone(&self.source)
    }
    fn target(&self) -> Arc<Self::Target> {
        Arc::clone(&self.target)
    }
    fn degree_shift(&self) -> i32 {
        0
    }

    /// At the moment, this is off by a sign. However, we only use this for p = 2
    fn apply_to_basis_element(
        &self,
        mut result: SliceMut,
        coeff: u32,
        degree: i32,
        input_idx: usize,
    ) {
        // Source is of the form ⊕_i L_i ⊗ R_(s - i). This i indexes the s degree. First figure out
        // which i this belongs to.
        let left_s = self.source.get_module_num(degree, input_idx);
        let right_s = self.source_s as usize - left_s;

        let source_module = &self.source.modules[left_s];

        let first_offset = self.source.offset(degree, left_s);
        let inner_index = input_idx - first_offset;

        // Now redefine L = L_i, R = R_(degree - i). Then L ⊗ R is itself a sum of terms of
        // the form L_i ⊗ R_(degree - i), where we are now summing over the t degree.
        let left_t = source_module.seek_module_num(degree, inner_index);
        let right_t = degree - left_t;

        let inner_index = inner_index - source_module.offset(degree, left_t);

        let source_right_dim = source_module.right.dimension(right_t);
        let right_index = inner_index % source_right_dim;
        let left_index = inner_index / source_right_dim;

        // Now calculate 1 (x) d
        if right_s > 0 {
            let target_module = &self.target.modules[left_s];
            let target_offset = self.target.offset(degree, left_s)
                + self.target.modules[left_s].offset(degree, left_t);
            let target_right_dim = target_module.right.dimension(right_t);

            let result = result.slice_mut(
                target_offset + left_index * target_right_dim,
                target_offset + (left_index + 1) * target_right_dim,
            );
            self.right_cc
                .differential(right_s as u32)
                .apply_to_basis_element(result, coeff, right_t, right_index);
        }

        // Now calculate d (x) 1
        if left_s > 0 {
            let target_module = &self.target.modules[left_s - 1];
            let target_offset = self.target.offset(degree, left_s - 1)
                + self.target.modules[left_s - 1].offset(degree, left_t);
            let target_right_dim = target_module.right.dimension(right_t);

            let mut dl = FpVector::new(self.prime(), target_module.left.dimension(left_t));
            self.left_cc
                .differential(left_s as u32)
                .apply_to_basis_element(dl.as_slice_mut(), coeff, left_t, left_index);
            for i in 0..dl.len() {
                result.add_basis_element(
                    target_offset + i * target_right_dim + right_index,
                    dl.entry(i),
                );
            }
        }
    }

    fn compute_auxiliary_data_through_degree(&self, degree: i32) {
        self.quasi_inverses
            .extend(degree, |i| self.calculate_quasi_inverse(i));
    }

    fn apply_quasi_inverse(&self, mut result: SliceMut, degree: i32, input: Slice) -> bool {
        let qis = &self.quasi_inverses[degree];
        assert_eq!(input.len(), qis.len());

        for (i, x) in input.iter_nonzero() {
            if let Some(qi) = &qis[i] {
                for (offset_start, offset_end, data) in qi.iter() {
                    result
                        .slice_mut(*offset_start, *offset_end)
                        .add(data.as_slice(), x);
                }
            }
        }
        true
    }
}

impl<A, CC1, CC2> TensorChainMap<A, CC1, CC2>
where
    A: Algebra + Bialgebra,
    CC1: ChainComplex<Algebra = A>,
    CC2: ChainComplex<Algebra = A>,
{
    #[allow(clippy::range_minus_one)]
    fn calculate_quasi_inverse(&self, degree: i32) -> Vec<Option<Vec<(usize, usize, FpVector)>>> {
        let p = self.prime();
        // start, end, preimage
        let mut quasi_inverse_list: Vec<Option<Vec<(usize, usize, FpVector)>>> =
            vec![None; self.target.dimension(degree)];

        for left_t in self.left_cc.min_degree()..=degree - self.right_cc.min_degree() {
            let right_t = degree - left_t;

            let source_dim = self
                .source
                .modules
                .iter()
                .map(|m| m.left.dimension(left_t) * m.right.dimension(right_t))
                .sum();
            let target_dim = self
                .target
                .modules
                .iter()
                .map(|m| m.left.dimension(left_t) * m.right.dimension(right_t))
                .sum();

            if source_dim == 0 || target_dim == 0 {
                continue;
            }

            let padded_target_dim = FpVector::padded_len(p, target_dim);

            let mut matrix = Matrix::new(p, source_dim, padded_target_dim + source_dim);

            // Compute 1 (x) d
            let mut target_offset = 0;
            let mut row_count = 0;
            for s in 0..=self.source_s - 1 {
                let source_module = &self.source.modules[s as usize]; // C_s (x) D_{source_s - s}
                let target_module = &self.target.modules[s as usize]; // C_s (x) D_{source_s - s - 1}

                let source_right_dim = source_module.right.dimension(right_t);
                let source_left_dim = source_module.left.dimension(left_t);
                let target_right_dim = target_module.right.dimension(right_t);
                let target_left_dim = target_module.left.dimension(left_t);
                assert_eq!(target_left_dim, source_left_dim);

                let mut result = FpVector::new(p, target_right_dim);
                for ri in 0..source_right_dim {
                    self.right_cc
                        .differential(self.source_s - s)
                        .apply_to_basis_element(result.as_slice_mut(), 1, right_t, ri);
                    for li in 0..source_left_dim {
                        let row = &mut matrix[row_count + li * source_right_dim + ri];
                        row.slice_mut(
                            target_offset + li * target_right_dim,
                            target_offset + (li + 1) * target_right_dim,
                        )
                        .assign(result.as_slice());
                    }
                    result.set_to_zero();
                }
                target_offset += target_right_dim * target_left_dim;
                row_count += source_right_dim * source_left_dim;
            }

            // Compute d (x) 1
            let mut target_offset = 0;
            let mut row_count = {
                let m = &self.source.modules[0usize];
                m.left.dimension(left_t) * m.right.dimension(right_t)
            };
            for s in 1..=self.source_s {
                let source_module = &self.source.modules[s as usize]; // C_s (x) D_{source_s - s}
                let target_module = &self.target.modules[s as usize - 1]; // C_{s - 1} (x) D_{source_s - s}

                let source_right_dim = source_module.right.dimension(right_t);
                let source_left_dim = source_module.left.dimension(left_t);
                let target_right_dim = target_module.right.dimension(right_t);
                let target_left_dim = target_module.left.dimension(left_t);
                assert_eq!(target_right_dim, source_right_dim);

                let mut result = FpVector::new(p, target_left_dim);
                for li in 0..source_left_dim {
                    self.left_cc.differential(s).apply_to_basis_element(
                        result.as_slice_mut(),
                        1,
                        left_t,
                        li,
                    );
                    for ri in 0..source_right_dim {
                        let row = &mut matrix[row_count];
                        for (i, x) in result.iter_nonzero() {
                            row.add_basis_element(target_offset + i * target_right_dim + ri, x);
                        }
                        row_count += 1;
                    }
                    result.set_to_zero();
                }
                target_offset += target_right_dim * target_left_dim;
            }

            for i in 0..source_dim {
                matrix[i].set_entry(padded_target_dim + i, 1);
            }

            matrix.row_reduce();

            let mut index = 0;
            let mut row = 0;
            for s in 0..self.source_s as usize {
                let target_module = &self.target.modules[s as usize]; // C_s (x) D_{source_s - s - 1}

                let target_right_dim = target_module.right.dimension(right_t);
                let target_left_dim = target_module.left.dimension(left_t);

                for li in 0..target_left_dim {
                    for ri in 0..target_right_dim {
                        if matrix.pivots()[index] >= 0 {
                            let true_index = self.target.offset(degree, s)
                                + self.target.modules[s].offset(degree, left_t)
                                + li * target_right_dim
                                + ri;
                            let mut entries = Vec::new();
                            let mut offset = 0;
                            for s_ in 0..=self.source_s as usize {
                                let dim = {
                                    let m = &self.source.modules[s_];
                                    m.left.dimension(left_t) * m.right.dimension(right_t)
                                };
                                if dim == 0 {
                                    continue;
                                }

                                let mut entry = FpVector::new(p, dim);
                                entry.as_slice_mut().assign(matrix[row].slice(
                                    padded_target_dim + offset,
                                    padded_target_dim + offset + dim,
                                ));

                                if !entry.is_zero() {
                                    let true_slice_start = self.source.offset(degree, s_)
                                        + self.source.modules[s_].offset(degree, left_t);
                                    let true_slice_end = true_slice_start + dim;
                                    entries.push((true_slice_start, true_slice_end, entry));
                                }

                                offset += dim;
                            }
                            assert!(quasi_inverse_list[true_index].is_none());
                            assert!(!entries.is_empty());
                            quasi_inverse_list[true_index] = Some(entries);
                            row += 1;
                        }
                        index += 1;
                    }
                }
            }
        }
        quasi_inverse_list
    }
}

/// This implementation assumes the target of the augmentation is k, which is the only case we need
/// for Steenrod operations.
impl AugmentedChainComplex
    for TensorSquareCC<
        SteenrodAlgebra,
        FiniteAugmentedChainComplex<
            FiniteModule,
            FiniteModuleHomomorphism<FiniteModule>,
            FiniteModuleHomomorphism<FiniteModule>,
            CCC,
        >,
    >
{
    type TargetComplex = CCC;
    type ChainMap = BoundedModuleHomomorphism<STM<FiniteModule, FiniteModule>, FiniteModule>;

    fn target(&self) -> Arc<Self::TargetComplex> {
        self.left_cc.target()
    }

    // Once this is implemented correctly, make the fields in BoundedModuleHomomoprhism private
    // again
    fn chain_map(&self, s: u32) -> Arc<Self::ChainMap> {
        assert_eq!(s, 0);
        Arc::new(BoundedModuleHomomorphism {
            source: self.module(0),
            target: self.left_cc.target().module(0),
            degree_shift: 0,
            matrices: BiVec::from_vec(0, vec![Matrix::from_vec(self.prime(), &[vec![1]])]),
            quasi_inverses: OnceBiVec::new(0),
            kernels: OnceBiVec::new(0),
            images: OnceBiVec::new(0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resolution_homomorphism::ResolutionHomomorphism;
    use crate::utils::construct;
    use crate::yoneda::yoneda_representative_element;
    use fp::prime::ValidPrime;

    #[test]
    fn test_square_ccs() {
        test_square_cc(1, 1, 0, 0);
        test_square_cc(2, 2, 0, 0);
        test_square_cc(1, 2, 0, 0);
        test_square_cc(1, 4, 0, 0);
        test_square_cc(4, 18, 0, 0);
    }

    fn test_square_cc(s: u32, t: i32, i: usize, fi: usize) {
        let k = r#"{"type" : "finite dimensional module","name": "$S_2$", "file_name": "S_2", "p": 2, "generic": false, "gens": {"x0": 0}, "adem_actions": []}"#;
        let p = ValidPrime::new(2);

        let k: serde_json::Value = serde_json::from_str(k).unwrap();
        let resolution = Arc::new(construct((k, "adem"), None).unwrap());
        resolution.compute_through_bidegree(2 * s, 2 * t);

        let yoneda = Arc::new(yoneda_representative_element(
            Arc::clone(&resolution),
            s,
            t,
            i,
        ));

        let square = Arc::new(TensorChainComplex::new(
            Arc::clone(&yoneda),
            Arc::clone(&yoneda),
        ));
        square.compute_through_bidegree(2 * s, 2 * t);

        let f = ResolutionHomomorphism::new("".to_string(), Arc::clone(&resolution), square, 0, 0);
        let mut mat = Matrix::new(p, 1, 1);
        mat[0].set_entry(0, 1);
        f.extend_step(0, 0, Some(&mat));

        f.extend(2 * s, 2 * t);
        let final_map = f.get_map(2 * s);

        let num_gens = resolution.number_of_gens_in_bidegree(2 * s, 2 * t);
        for i_ in 0..num_gens {
            assert_eq!(final_map.output(2 * t, i_).len(), 1);
            if i_ == fi {
                assert_eq!(final_map.output(2 * t, i_).entry(0), 1);
            } else {
                assert_eq!(final_map.output(2 * t, i_).entry(0), 0);
            }
        }
    }
}
