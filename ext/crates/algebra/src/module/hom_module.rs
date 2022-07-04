use std::sync::Arc;

use bivec::BiVec;

use crate::algebra::Field;
use crate::module::block_structure::BlockStructure;
use crate::module::{FreeModule, Module};
use fp::vector::{prelude::*, SliceMut};
use once::OnceBiVec;

/// Given a module N and a free module M, this is the module Hom(M, N) as a module over the ground
/// field. This requires N to be bounded, and is graded *opposite* to the usual grading so that
/// Hom(M, N) is bounded below.
pub struct HomModule<M: Module> {
    algebra: Arc<Field>,
    source: Arc<FreeModule<M::Algebra>>,
    target: Arc<M>,
    pub block_structures: OnceBiVec<BlockStructure>,
}

impl<M: Module> std::fmt::Display for HomModule<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Hom({}, {})", self.source, self.target)
    }
}

impl<M: Module> HomModule<M> {
    pub fn new(source: Arc<FreeModule<M::Algebra>>, target: Arc<M>) -> Self {
        let p = source.prime();
        let algebra = Arc::new(Field::new(p));
        let min_degree = source.min_degree()
            - target
                .max_degree()
                .expect("HomModule requires target to be bounded");
        Self {
            algebra,
            source,
            target,
            block_structures: OnceBiVec::new(min_degree), // fn_degree -> blocks
        }
    }

    pub fn source(&self) -> Arc<FreeModule<M::Algebra>> {
        Arc::clone(&self.source)
    }

    pub fn target(&self) -> Arc<M> {
        Arc::clone(&self.target)
    }
}

impl<M: Module> Module for HomModule<M> {
    type Algebra = Field;

    fn algebra(&self) -> Arc<Self::Algebra> {
        Arc::clone(&self.algebra)
    }

    fn min_degree(&self) -> i32 {
        self.block_structures.min_degree()
    }

    fn max_computed_degree(&self) -> i32 {
        self.source.max_computed_degree() - self.target.max_degree().unwrap()
    }

    fn compute_basis(&self, degree: i32) {
        self.source
            .compute_basis(degree + self.target.max_degree().unwrap());
        self.block_structures.extend(degree, |d| {
            let mut block_sizes = BiVec::new(self.target.min_degree() + d);
            block_sizes.extend_with(self.target.max_degree().unwrap() + d, |gen_deg| {
                vec![
                    self.target.dimension(gen_deg - d);
                    if self.source.max_computed_degree() >= gen_deg {
                        self.source.number_of_gens_in_degree(gen_deg)
                    } else {
                        0
                    }
                ]
            });
            BlockStructure::new(&block_sizes)
        });
    }

    fn dimension(&self, degree: i32) -> usize {
        self.block_structures[degree].total_dimension()
    }

    fn act_on_basis(
        &self,
        mut result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        _mod_degree: i32,
        mod_index: usize,
    ) {
        assert_eq!(op_degree, 0);
        assert_eq!(op_index, 0);
        result.add_basis_element(mod_index, coeff);
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        let gen_basis_elt = self.block_structures[degree].index_to_generator_basis_elt(idx);
        let gen_deg = gen_basis_elt.generator_degree;
        let gen_idx = gen_basis_elt.generator_index;
        let gen_mod_idx = self
            .source
            .operation_generator_to_index(0, 0, gen_deg, gen_idx);
        let basis_deg = gen_deg - degree;
        let basis_idx = gen_basis_elt.basis_index;
        format!(
            "{}*⊗{}",
            self.source.basis_element_to_string(gen_deg, gen_mod_idx),
            self.target.basis_element_to_string(basis_deg, basis_idx),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::FDModule;
    use crate::MilnorAlgebra;

    #[test]
    fn test_hom_dim() {
        const NUM_GENS: [usize; 3] = [1, 2, 1];
        const TARGET_DIM: [usize; 3] = [1, 3, 4];

        let algebra = Arc::new(MilnorAlgebra::new(fp::prime::TWO, false));
        let f = Arc::new(FreeModule::new(Arc::clone(&algebra), "F0".to_string(), 0));
        let m = Arc::new(
            FDModule::from_json(Arc::clone(&algebra), &crate::test::joker_json()).unwrap(),
        );

        for (deg, num_gens) in NUM_GENS.into_iter().enumerate() {
            f.add_generators(deg as i32, num_gens, None);
        }
        f.compute_basis(NUM_GENS.len() as i32 - 1);

        let hom = HomModule::new(f, m);
        assert_eq!(hom.min_degree(), -4);
        assert_eq!(hom.max_computed_degree(), -2);
        hom.compute_basis(-2);

        for (&target_dim, deg) in
            std::iter::zip(&TARGET_DIM, hom.min_degree()..=hom.max_computed_degree())
        {
            assert_eq!(hom.dimension(deg), target_dim);
        }
    }
}
