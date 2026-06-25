//! This module exports the [`Resolution`] object, which is a chain complex resolving a module. In
//! particular, this contains the core logic that compute minimal resolutions.

use std::sync::Arc;

use algebra::{
    MilnorAlgebra, MuAlgebra, SteenrodAlgebra,
    module::{FDModule, MuFreeModule, homomorphism::MuFreeModuleHomomorphism},
};
use sseq::coordinates::Bidegree;

use crate::{
    chain_complex::{AugmentedChainComplex, ChainComplex},
    save::SaveDirectory,
};

mod classical;
mod nassau;

use classical::MuClassicalResolution;
use nassau::NassauResolution;

pub type Resolution<CC> = MuResolution<false, CC>;
pub type UnstableResolution<CC> = MuResolution<true, CC>;

pub enum MuResolution<const U: bool, CC: ChainComplex>
where
    CC::Algebra: MuAlgebra<U>,
{
    Classical(MuClassicalResolution<U, CC>),
    Nassau(NassauResolution),
}

impl<const U: bool, CC: ChainComplex> MuResolution<U, CC>
where
    CC::Algebra: MuAlgebra<U>,
{
    pub fn new(complex: Arc<CC>) -> Self {
        // It doesn't error if the save file is None
        Self::new_with_save(complex, None).unwrap()
    }

    pub fn new_with_save(
        complex: Arc<CC>,
        save_dir: impl Into<SaveDirectory>,
    ) -> anyhow::Result<Self> {
        Ok(match Self::try_into_nassau(complex) {
            Ok(module) => Self::Nassau(NassauResolution::new_with_save(module, save_dir)?),
            Err(complex) => {
                Self::Classical(MuClassicalResolution::new_with_save(complex, save_dir)?)
            }
        })
    }

    fn try_into_nassau(complex: Arc<CC>) -> Result<Arc<FDModule<MilnorAlgebra>>, Arc<CC>> {
        todo!()
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Classical(classical) => classical.name(),
            Self::Nassau(nassau) => nassau.name(),
        }
    }

    pub fn compute_through_stem(&self, max: Bidegree) {
        match self {
            Self::Classical(classical) => classical.compute_through_stem(max),
            Self::Nassau(nassau) => nassau.compute_through_stem(max),
        }
    }

    pub fn set_load_quasi_inverse(&mut self, load_quasi_inverse: bool) {
        match self {
            Self::Classical(classical) => classical.load_quasi_inverse = load_quasi_inverse,
            Self::Nassau(nassau) => assert!(
                !load_quasi_inverse || nassau.save_dir().is_some(),
                "Quasi inverse loading not supported with Nassau. Please use a save directory \
                 instead"
            ),
        }
    }

    pub fn set_name(&mut self, name: String) {
        match self {
            Self::Classical(classical) => classical.set_name(name),
            Self::Nassau(nassau) => nassau.set_name(name),
        }
    }
}

impl<const U: bool, CC: ChainComplex> ChainComplex for MuResolution<U, CC>
where
    CC::Algebra: MuAlgebra<U>,
{
    type Algebra = CC::Algebra;
    type Homomorphism = MuFreeModuleHomomorphism<U, MuFreeModule<U, Self::Algebra>>;
    type Module = MuFreeModule<U, Self::Algebra>;

    fn algebra(&self) -> Arc<Self::Algebra> {
        match self {
            Self::Classical(classical) => classical.algebra(),
            Self::Nassau(nassau) => todo!(),
        }
    }

    fn min_degree(&self) -> i32 {
        todo!()
    }

    fn zero_module(&self) -> Arc<Self::Module> {
        todo!()
    }

    fn module(&self, homological_degree: i32) -> Arc<Self::Module> {
        todo!()
    }

    fn differential(&self, s: i32) -> Arc<Self::Homomorphism> {
        todo!()
    }

    fn has_computed_bidegree(&self, b: sseq::coordinates::Bidegree) -> bool {
        todo!()
    }

    fn compute_through_bidegree(&self, b: sseq::coordinates::Bidegree) {
        todo!()
    }

    fn next_homological_degree(&self) -> i32 {
        todo!()
    }
}

impl<const U: bool, CC: ChainComplex> AugmentedChainComplex for MuResolution<U, CC>
where
    CC::Algebra: MuAlgebra<U>,
{
    type ChainMap = <MuClassicalResolution<U, CC> as AugmentedChainComplex>::ChainMap;
    type TargetComplex = <MuClassicalResolution<U, CC> as AugmentedChainComplex>::TargetComplex;

    fn target(&self) -> Arc<Self::TargetComplex> {
        todo!()
    }

    fn chain_map(&self, s: i32) -> Arc<Self::ChainMap> {
        todo!()
    }
}
