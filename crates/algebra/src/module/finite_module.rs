use crate::algebra::{Algebra, SteenrodAlgebra};
use crate::module::{FDModule, FPModule, Module, RealProjectiveSpace};
use fp::prime::ValidPrime;
use fp::vector::FpVector;
use serde_json::Value;
use std::sync::Arc;

#[derive(PartialEq, Eq)]
pub enum FiniteModule {
    FDModule(FDModule<SteenrodAlgebra>),
    FPModule(FPModule<SteenrodAlgebra>),
    RealProjectiveSpace(RealProjectiveSpace),
}

impl Module for FiniteModule {
    type Algebra = SteenrodAlgebra;

    fn algebra(&self) -> Arc<Self::Algebra> {
        match self {
            FiniteModule::FDModule(m) => m.algebra(),
            FiniteModule::FPModule(m) => m.algebra(),
            FiniteModule::RealProjectiveSpace(m) => m.algebra(),
        }
    }

    fn name(&self) -> String {
        match self {
            FiniteModule::FDModule(m) => m.name(),
            FiniteModule::FPModule(m) => m.name(),
            FiniteModule::RealProjectiveSpace(m) => m.name(),
        }
    }

    fn min_degree(&self) -> i32 {
        match self {
            FiniteModule::FDModule(m) => m.min_degree(),
            FiniteModule::FPModule(m) => m.min_degree(),
            FiniteModule::RealProjectiveSpace(m) => m.min_degree(),
        }
    }

    fn compute_basis(&self, degree: i32) {
        match self {
            FiniteModule::FDModule(m) => m.compute_basis(degree),
            FiniteModule::FPModule(m) => m.compute_basis(degree),
            FiniteModule::RealProjectiveSpace(m) => m.compute_basis(degree),
        }
    }

    fn max_computed_degree(&self) -> i32 {
        match self {
            FiniteModule::FDModule(m) => m.max_computed_degree(),
            FiniteModule::FPModule(m) => m.max_computed_degree(),
            FiniteModule::RealProjectiveSpace(m) => m.max_computed_degree(),
        }
    }

    fn dimension(&self, degree: i32) -> usize {
        match self {
            FiniteModule::FDModule(m) => m.dimension(degree),
            FiniteModule::FPModule(m) => m.dimension(degree),
            FiniteModule::RealProjectiveSpace(m) => m.dimension(degree),
        }
    }

    fn act_on_basis(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) {
        match self {
            FiniteModule::FDModule(m) => {
                m.act_on_basis(result, coeff, op_degree, op_index, mod_degree, mod_index)
            }
            FiniteModule::FPModule(m) => {
                m.act_on_basis(result, coeff, op_degree, op_index, mod_degree, mod_index)
            }
            FiniteModule::RealProjectiveSpace(m) => {
                m.act_on_basis(result, coeff, op_degree, op_index, mod_degree, mod_index)
            }
        }
    }

    // Dispatch these as well so that we don't have to match on the type every loop.
    // Experimentally, not doing so causes a significant performance on some runs (while having no
    // impact on the others)
    fn act(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        input_degree: i32,
        input: &FpVector,
    ) {
        match self {
            FiniteModule::FDModule(m) => {
                m.act(result, coeff, op_degree, op_index, input_degree, input)
            }
            FiniteModule::FPModule(m) => {
                m.act(result, coeff, op_degree, op_index, input_degree, input)
            }
            FiniteModule::RealProjectiveSpace(m) => {
                m.act(result, coeff, op_degree, op_index, input_degree, input)
            }
        }
    }

    fn act_by_element(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op: &FpVector,
        input_degree: i32,
        input: &FpVector,
    ) {
        match self {
            FiniteModule::FDModule(m) => {
                m.act_by_element(result, coeff, op_degree, op, input_degree, input)
            }
            FiniteModule::FPModule(m) => {
                m.act_by_element(result, coeff, op_degree, op, input_degree, input)
            }
            FiniteModule::RealProjectiveSpace(m) => {
                m.act_by_element(result, coeff, op_degree, op, input_degree, input)
            }
        }
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        match self {
            FiniteModule::FDModule(m) => m.basis_element_to_string(degree, idx),
            FiniteModule::FPModule(m) => m.basis_element_to_string(degree, idx),
            FiniteModule::RealProjectiveSpace(m) => m.basis_element_to_string(degree, idx),
        }
    }

    fn is_unit(&self) -> bool {
        match self {
            FiniteModule::FDModule(m) => m.is_unit(),
            _ => false,
        }
    }

    fn prime(&self) -> ValidPrime {
        self.algebra().prime()
    }

    /// Whether act_on_basis_borrow is available.
    fn borrow_output(&self) -> bool {
        match self {
            FiniteModule::FDModule(_) => true,
            _ => false,
        }
    }

    fn act_on_basis_borrow(
        &self,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) -> &FpVector {
        match self {
            FiniteModule::FDModule(m) => {
                m.act_on_basis_borrow(op_degree, op_index, mod_degree, mod_index)
            }
            _ => unimplemented!(),
        }
    }
}

impl From<FPModule<SteenrodAlgebra>> for FiniteModule {
    fn from(m: FPModule<SteenrodAlgebra>) -> Self {
        Self::FPModule(m)
    }
}
impl From<FDModule<SteenrodAlgebra>> for FiniteModule {
    fn from(m: FDModule<SteenrodAlgebra>) -> Self {
        Self::FDModule(m)
    }
}
impl From<RealProjectiveSpace> for FiniteModule {
    fn from(m: RealProjectiveSpace) -> Self {
        Self::RealProjectiveSpace(m)
    }
}

impl FiniteModule {
    pub fn from_json(
        algebra: Arc<SteenrodAlgebra>,
        json: &mut serde_json::Value,
    ) -> error::Result<Self> {
        match json["type"].as_str() {
            Some("real projective space") => Ok(FiniteModule::from(RealProjectiveSpace::from_json(
                algebra, json,
            )?)),
            Some("finite dimensional module") => {
                Ok(FiniteModule::from(FDModule::from_json(algebra, json)?))
            }
            Some("finitely presented module") => {
                Ok(FiniteModule::from(FPModule::from_json(algebra, json)?))
            }
            x => Err(UnknownModuleTypeError {
                module_type: x.map(str::to_string),
            }.into()),
        }
    }

    pub fn to_json(&self, json: &mut Value) {
        match self {
            Self::FDModule(m) => m.to_json(json),
            Self::FPModule(m) => m.to_json(json),
            Self::RealProjectiveSpace(m) => m.to_json(json),
        }
    }

    pub fn type_(&self) -> &str {
        match self {
            Self::FDModule(_) => "finite dimensional module",
            Self::FPModule(_) => "finitely presented module",
            Self::RealProjectiveSpace(_) => "real projective space",
        }
    }

    pub fn is_real_projective_space(&self) -> bool {
        match self {
            FiniteModule::RealProjectiveSpace(_) => true,
            _ => false,
        }
    }

    pub fn is_fp_module(&self) -> bool {
        match self {
            FiniteModule::FPModule(_) => true,
            _ => false,
        }
    }

    pub fn is_fd_module(&self) -> bool {
        match self {
            FiniteModule::FDModule(_) => true,
            _ => false,
        }
    }

    pub fn into_real_projective_space(self) -> Option<RealProjectiveSpace> {
        match self {
            FiniteModule::RealProjectiveSpace(m) => Some(m),
            _ => None,
        }
    }

    pub fn into_fp_module(self) -> Option<FPModule<SteenrodAlgebra>> {
        match self {
            FiniteModule::FPModule(m) => Some(m),
            _ => None,
        }
    }

    pub fn into_fd_module(self) -> Option<FDModule<SteenrodAlgebra>> {
        match self {
            FiniteModule::FDModule(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_real_projective_space(&self) -> Option<&RealProjectiveSpace> {
        match self {
            FiniteModule::RealProjectiveSpace(m) => Some(&m),
            _ => None,
        }
    }

    pub fn as_fp_module(&self) -> Option<&FPModule<SteenrodAlgebra>> {
        match self {
            FiniteModule::FPModule(m) => Some(&m),
            _ => None,
        }
    }

    pub fn as_fd_module(&self) -> Option<&FDModule<SteenrodAlgebra>> {
        match self {
            FiniteModule::FDModule(m) => Some(&m),
            _ => None,
        }
    }

}

#[derive(Debug)]
pub struct UnknownModuleTypeError {
    pub module_type: Option<String>,
}

impl std::fmt::Display for UnknownModuleTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.module_type {
            Some(s) => write!(f, "Unknown module type: {}", s),
            None => write!(f, "Missing module type"),
        }
    }
}

impl std::error::Error for UnknownModuleTypeError {}
