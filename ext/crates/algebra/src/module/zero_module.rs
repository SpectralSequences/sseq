use std::sync::Arc;

use crate::{
    SteenrodAlgebra,
    module::{FDModule, Module, SteenrodModule},
};

pub trait ZeroModule: Module {
    fn zero_module(algebra: Arc<Self::Algebra>, min_degree: i32) -> Self;
}

impl ZeroModule for SteenrodModule {
    fn zero_module(algebra: Arc<SteenrodAlgebra>, min_degree: i32) -> Self {
        Box::new(FDModule::zero_module(algebra, min_degree))
    }
}
