use crate::algebra::{Algebra, SteenrodAlgebra};
use crate::module::{FDModule, FPModule, Module, RealProjectiveSpace};
use fp::prime::ValidPrime;
use fp::vector::{FpVector, Slice, SliceMut};
use std::sync::Arc;

#[cfg(feature = "json")]
use serde_json::Value;

#[derive(PartialEq, Eq)]
pub enum FiniteModule {
    FDModule(FDModule<SteenrodAlgebra>),
    FPModule(FPModule<SteenrodAlgebra>),
    RealProjectiveSpace(RealProjectiveSpace<SteenrodAlgebra>),
}

macro_rules! dispatch {
    () => {};
    ($vis:vis fn $method:ident(&self$(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?; $($tail:tt)*) => {
        $vis fn $method(&self, $($arg: $ty),* ) $(-> $ret)* {
            match self {
                FiniteModule::FDModule(m) => m.$method($($arg),*),
                FiniteModule::FPModule(m) => m.$method($($arg),*),
                FiniteModule::RealProjectiveSpace(m) => m.$method($($arg),*),
            }
        }
        dispatch!{$($tail)*}
    };
}

impl std::fmt::Display for FiniteModule {
    dispatch! {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result;
    }
}

// We dispatch the act/act_by_element functions to avoid matching in every loop.
impl Module for FiniteModule {
    type Algebra = SteenrodAlgebra;

    dispatch! {
        fn algebra(&self) -> Arc<Self::Algebra>;
        fn min_degree(&self) -> i32;
        fn compute_basis(&self, degree: i32);
        fn max_computed_degree(&self) -> i32;
        fn dimension(&self, degree: i32) -> usize;
        fn act_on_basis(&self, result: SliceMut, coeff: u32, op_degree: i32, op_index: usize, mod_degree: i32, mod_index: usize);
        fn act(&self, result: SliceMut, coeff: u32, op_degree: i32, op_index: usize, input_degree: i32, input: Slice);
        fn act_by_element(&self, result: SliceMut, coeff: u32, op_degree: i32, op: Slice, input_degree: i32, input: Slice);
        fn basis_element_to_string(&self, degree: i32, idx: usize) -> String;
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
        self.is_fd_module()
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
impl From<RealProjectiveSpace<SteenrodAlgebra>> for FiniteModule {
    fn from(m: RealProjectiveSpace<SteenrodAlgebra>) -> Self {
        Self::RealProjectiveSpace(m)
    }
}

#[cfg(feature = "json")]
impl FiniteModule {
    pub fn from_json(
        algebra: Arc<SteenrodAlgebra>,
        json: &serde_json::Value,
    ) -> error::Result<Self> {
        match json["type"].as_str() {
            Some("real projective space") => Ok(FiniteModule::from(
                RealProjectiveSpace::from_json(algebra, json)?,
            )),
            Some("finite dimensional module") => {
                Ok(FiniteModule::from(FDModule::from_json(algebra, json)?))
            }
            Some("finitely presented module") => {
                Ok(FiniteModule::from(FPModule::from_json(algebra, json)?))
            }
            x => Err(UnknownModuleTypeError {
                module_type: x.map(str::to_string),
            }
            .into()),
        }
    }

    dispatch! { pub fn to_json(&self, json: &mut Value); }
}

impl FiniteModule {
    pub fn type_(&self) -> &str {
        match self {
            Self::FDModule(_) => "finite dimensional module",
            Self::FPModule(_) => "finitely presented module",
            Self::RealProjectiveSpace(_) => "real projective space",
        }
    }

    pub fn is_real_projective_space(&self) -> bool {
        matches!(self, FiniteModule::RealProjectiveSpace(_))
    }

    pub fn is_fp_module(&self) -> bool {
        matches!(self, FiniteModule::FPModule(_))
    }

    pub fn is_fd_module(&self) -> bool {
        matches!(self, FiniteModule::FDModule(_))
    }

    pub fn into_real_projective_space(self) -> Option<RealProjectiveSpace<SteenrodAlgebra>> {
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

    pub fn as_real_projective_space(&self) -> Option<&RealProjectiveSpace<SteenrodAlgebra>> {
        match self {
            FiniteModule::RealProjectiveSpace(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_fp_module(&self) -> Option<&FPModule<SteenrodAlgebra>> {
        match self {
            FiniteModule::FPModule(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_fd_module(&self) -> Option<&FDModule<SteenrodAlgebra>> {
        match self {
            FiniteModule::FDModule(m) => Some(m),
            _ => None,
        }
    }
}

impl crate::module::BoundedModule for FiniteModule {
    fn max_degree(&self) -> i32 {
        match self {
            FiniteModule::FDModule(m) => m.max_degree(),
            FiniteModule::RealProjectiveSpace(m) => m.max_degree(),
            FiniteModule::FPModule(_) => i32::MAX,
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
