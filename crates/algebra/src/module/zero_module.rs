use std::sync::Arc;

use crate::SteenrodAlgebra;
use crate::module::Module;
use crate::module::{FiniteModule, FDModule};

pub trait ZeroModule: Module {
    fn zero_module(algebra: Arc<Self::Algebra>, min_degree: i32) -> Self;
}

impl ZeroModule for FiniteModule {
    fn zero_module(algebra: Arc<SteenrodAlgebra>, min_degree: i32) -> Self {
        FiniteModule::FDModule(FDModule::zero_module(algebra, min_degree))
    }
}
