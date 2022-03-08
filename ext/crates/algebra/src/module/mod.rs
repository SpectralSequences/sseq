mod finite_dimensional_module;
mod finitely_presented_module;
mod free_module;
mod module_trait;
mod rpn;
pub mod steenrod_module;
mod zero_module;

mod hom_module;
mod quotient_module;
mod sum_module;
mod tensor_module;
mod truncated_module;

pub mod block_structure;

pub mod homomorphism;

pub use finite_dimensional_module::FiniteDimensionalModule as FDModule;
pub use finitely_presented_module::FinitelyPresentedModule as FPModule;
pub use free_module::{FreeModule, GeneratorData, OperationGeneratorPair};
pub use module_trait::{Module, ModuleFailedRelationError};
pub use rpn::RealProjectiveSpace;
pub use steenrod_module::SteenrodModule;
pub use zero_module::ZeroModule;

pub use {
    hom_module::HomModule, quotient_module::QuotientModule, sum_module::SumModule,
    tensor_module::TensorModule, truncated_module::TruncatedModule,
};
