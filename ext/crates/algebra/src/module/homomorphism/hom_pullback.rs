use std::sync::Arc;

use crate::module::block_structure::GeneratorBasisEltPair;
use crate::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use crate::module::HomModule;
use crate::module::{FreeModule, Module};
use fp::matrix::{QuasiInverse, Subspace};
use fp::vector::{prelude::*, SliceMut};
use once::OnceBiVec;

/// Given a map $\mathtt{map}: A \to B$ and hom modules $\mathtt{source} = \Hom(B, X)$, $\mathtt{target} = \Hom(A, X)$, produce the induced pullback map $\Hom(B, X) \to \Hom(A, X)$.
pub struct HomPullback<M: Module> {
    source: Arc<HomModule<M>>,
    target: Arc<HomModule<M>>,
    map: Arc<FreeModuleHomomorphism<FreeModule<M::Algebra>>>,
    images: OnceBiVec<Subspace>,
    kernels: OnceBiVec<Subspace>,
    quasi_inverses: OnceBiVec<QuasiInverse>,
}

impl<M: Module> HomPullback<M> {
    pub fn new(
        source: Arc<HomModule<M>>,
        target: Arc<HomModule<M>>,
        map: Arc<FreeModuleHomomorphism<FreeModule<M::Algebra>>>,
    ) -> Self {
        assert!(Arc::ptr_eq(&source.source(), &map.target()));
        assert!(Arc::ptr_eq(&target.source(), &map.source()));
        assert!(Arc::ptr_eq(&source.target(), &target.target()));

        let min_degree = source.min_degree();
        Self {
            source,
            target,
            map,
            images: OnceBiVec::new(min_degree),
            kernels: OnceBiVec::new(min_degree),
            quasi_inverses: OnceBiVec::new(min_degree),
        }
    }
}

impl<M: Module> ModuleHomomorphism for HomPullback<M> {
    type Source = HomModule<M>;
    type Target = HomModule<M>;

    fn source(&self) -> Arc<Self::Source> {
        Arc::clone(&self.source)
    }

    fn target(&self) -> Arc<Self::Target> {
        Arc::clone(&self.target)
    }

    fn degree_shift(&self) -> i32 {
        -self.map.degree_shift()
    }

    fn min_degree(&self) -> i32 {
        self.source().min_degree()
    }

    fn apply_to_basis_element(
        &self,
        mut result: SliceMut,
        coeff: u32,
        fn_degree: i32,
        fn_idx: usize,
    ) {
        let GeneratorBasisEltPair {
            generator_degree,
            generator_index,
            basis_index,
        } = self.source.block_structures[fn_degree].index_to_generator_basis_elt(fn_idx);

        let target_module = self.target.target(); // == self.source.target()
        let source_free_module = self.source.source();
        let target_free_module = self.target.source();
        let degree_shift = self.map.degree_shift();

        let max_degree = fn_degree + degree_shift + target_module.max_degree().unwrap();
        let min_degree = std::cmp::max(
            *generator_degree + degree_shift,
            fn_degree + degree_shift + target_module.min_degree(),
        );

        for (target_gen_deg, target_gen_idx) in target_free_module
            .iter_gens(max_degree)
            .filter(|(t, _)| *t >= min_degree)
        {
            let target_range = self.target.block_structures[fn_degree + degree_shift]
                .generator_to_block(target_gen_deg, target_gen_idx);

            let slice = source_free_module.slice_vector(
                target_gen_deg - degree_shift,
                *generator_degree,
                *generator_index,
                self.map.output(target_gen_deg, target_gen_idx).as_slice(),
            );

            // Needed if the output is shorter due to resolve_through_stem
            if slice.is_empty() {
                continue;
            }
            target_module.act_by_element_on_basis(
                result.slice_mut(target_range.start, target_range.end),
                coeff,
                target_gen_deg - degree_shift - *generator_degree,
                slice,
                *generator_degree - fn_degree,
                *basis_index,
            );
        }
    }

    fn compute_auxiliary_data_through_degree(&self, degree: i32) {
        self.kernels.extend(degree, |i| {
            let (image, kernel, qi) = self.auxiliary_data(i);
            self.images.push_checked(image, i);
            self.quasi_inverses.push_checked(qi, i);
            kernel
        });
    }

    fn quasi_inverse(&self, degree: i32) -> Option<&QuasiInverse> {
        self.quasi_inverses.get(degree)
    }

    fn kernel(&self, degree: i32) -> Option<&Subspace> {
        self.kernels.get(degree)
    }

    fn image(&self, degree: i32) -> Option<&Subspace> {
        self.images.get(degree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::FDModule;
    use crate::MilnorAlgebra;
    use bivec::BiVec;
    use fp::matrix::Matrix;
    use fp::vector::FpVector;

    #[test]
    fn test_pullback_id() {
        const SHIFT: i32 = 2;
        const NUM_GENS: [usize; 3] = [1, 2, 1];

        let p = fp::prime::TWO;

        let algebra = Arc::new(MilnorAlgebra::new(p, false));
        let f0 = Arc::new(FreeModule::new(Arc::clone(&algebra), "F0".to_string(), 0));
        let f1 = Arc::new(FreeModule::new(
            Arc::clone(&algebra),
            "F1".to_string(),
            SHIFT,
        ));
        let m = Arc::new(
            FDModule::from_json(Arc::clone(&algebra), &crate::test::joker_json()).unwrap(),
        );

        let d = Arc::new(FreeModuleHomomorphism::new(
            Arc::clone(&f1),
            Arc::clone(&f0),
            SHIFT,
        ));

        f0.compute_basis(NUM_GENS.len() as i32);
        f1.compute_basis(NUM_GENS.len() as i32 + SHIFT);

        for (deg, num_gens) in NUM_GENS.into_iter().enumerate() {
            f0.add_generators(deg as i32, num_gens, None);
            f1.add_generators(deg as i32 + SHIFT, num_gens, None);
            let mut rows = vec![FpVector::new(p, f0.dimension(deg as i32)); num_gens];
            for (i, row) in rows.iter_mut().enumerate() {
                row.add_basis_element(row.len() - num_gens + i, 1);
            }
            d.add_generators_from_rows(deg as i32 + SHIFT, rows);
        }

        let pb = HomPullback::new(
            Arc::new(HomModule::new(f0, Arc::clone(&m))),
            Arc::new(HomModule::new(f1, Arc::clone(&m))),
            d,
        );

        pb.source.compute_basis(-2);
        pb.target.compute_basis(-2 + SHIFT);

        for deg in pb.source.min_degree()..=pb.source.max_computed_degree() {
            let dim = pb.source.dimension(deg);
            let mut matrix = Matrix::new(p, dim, dim);
            pb.get_matrix(matrix.as_slice_mut(), deg);
            assert_eq!(matrix, Matrix::identity(p, dim));
        }
    }

    #[test]
    fn test_pullback() {
        const SHIFT: i32 = 3;
        const NUM_GENS: [usize; 3] = [1, 1, 1];

        let p = fp::prime::TWO;

        let algebra = Arc::new(MilnorAlgebra::new(p, false));
        let f0 = Arc::new(FreeModule::new(Arc::clone(&algebra), "F0".to_string(), 0));
        let f1 = Arc::new(FreeModule::new(
            Arc::clone(&algebra),
            "F1".to_string(),
            SHIFT,
        ));
        let m = Arc::new(
            FDModule::from_json(Arc::clone(&algebra), &crate::test::joker_json()).unwrap(),
        );

        let d = Arc::new(FreeModuleHomomorphism::new(
            Arc::clone(&f1),
            Arc::clone(&f0),
            SHIFT,
        ));

        f0.compute_basis(NUM_GENS.len() as i32);
        f1.compute_basis(NUM_GENS.len() as i32 + SHIFT);

        for (deg, num_gens) in NUM_GENS.into_iter().enumerate() {
            f0.add_generators(deg as i32, num_gens, None);
            f1.add_generators(deg as i32 + SHIFT, num_gens, None);
        }

        d.add_generators_from_rows(SHIFT, vec![FpVector::from_slice(p, &[1])]);
        d.add_generators_from_rows(SHIFT + 1, vec![FpVector::from_slice(p, &[0, 1])]);
        d.add_generators_from_rows(SHIFT + 2, vec![FpVector::from_slice(p, &[1, 1, 1])]);

        let pb = HomPullback::new(
            Arc::new(HomModule::new(f0, Arc::clone(&m))),
            Arc::new(HomModule::new(f1, Arc::clone(&m))),
            d,
        );

        pb.source.compute_basis(-2);
        pb.target.compute_basis(-2 + SHIFT);

        let outputs = BiVec::from_vec(
            -4,
            vec![
                Matrix::from_vec(p, &[vec![1]]),
                Matrix::from_vec(p, &[vec![1, 0], vec![0, 1]]),
                Matrix::from_vec(p, &[vec![1, 0, 1], vec![0, 1, 1], vec![0, 0, 1]]),
            ],
        );

        for deg in pb.source.min_degree()..=pb.source.max_computed_degree() {
            let dim = pb.source.dimension(deg);
            let mut matrix = Matrix::new(p, dim, dim);
            pb.get_matrix(matrix.as_slice_mut(), deg);
            assert_eq!(matrix, outputs[deg]);
        }
    }
}
