mod bounded_module;
mod finite_dimensional_module;
mod finite_module;
mod finitely_presented_module;
mod free_module;
mod module_trait;
mod rpn;
mod zero_module;

#[cfg(feature = "extras")]
mod bcp;
#[cfg(feature = "extras")]
mod dickson2;
#[cfg(feature = "extras")]
mod free_unstable_module;
#[cfg(feature = "extras")]
mod hom_module;
#[cfg(feature = "extras")]
mod kfpn;
#[cfg(feature = "extras")]
mod polynomial_algebra_module;
#[cfg(feature = "extras")]
mod quotient_module;
#[cfg(feature = "extras")]
mod sum_module;
#[cfg(feature = "extras")]
mod tensor_module;
#[cfg(feature = "extras")]
mod truncated_module;

pub mod block_structure;

pub mod homomorphism;

pub use bounded_module::BoundedModule;
pub use finite_dimensional_module::FiniteDimensionalModule as FDModule;
pub use finite_module::FiniteModule;
pub use finitely_presented_module::FinitelyPresentedModule as FPModule;
pub use free_module::{FreeModule, OperationGeneratorPair};
pub use module_trait::{Module, ModuleFailedRelationError};
pub use rpn::RealProjectiveSpace;
pub use zero_module::ZeroModule;

#[cfg(feature = "extras")]
pub use {
    bcp::BCp, dickson2::Dickson2, free_unstable_module::FreeUnstableModule, hom_module::HomModule,
    kfpn::KFpn, polynomial_algebra_module::PolynomialAlgebraModule,
    quotient_module::QuotientModule, sum_module::SumModule, tensor_module::TensorModule,
    truncated_module::TruncatedModule,
};

use crate::algebra::SteenrodAlgebra;

// Poor man's trait alias
pub trait SteenrodModule: Module<Algebra = SteenrodAlgebra> {}
impl<M: Module<Algebra = SteenrodAlgebra>> SteenrodModule for M {}
