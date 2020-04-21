mod finite_dimensional_module;
mod finite_module;
mod finitely_presented_module;
mod free_module;
mod free_unstable_module;
mod unstable_algebra;
mod unstable_algebra_bcp;
mod hom_module;
mod quotient_module;
mod rpn;
mod sum_module;
mod tensor_module;
mod truncated_module;
mod zero_module;
mod bounded_module;
mod module_trait;
pub mod block_structure;

pub mod homomorphism;


pub use finite_dimensional_module::FiniteDimensionalModule as FDModule;
pub use finite_module::FiniteModule;
pub use finitely_presented_module::FinitelyPresentedModule as FPModule;
pub use free_module::{FreeModule, OperationGeneratorPair};
pub use free_unstable_module::FreeUnstableModule;
pub use unstable_algebra::{UnstableAlgebra, UnstableAlgebraMonomial, UnstableAlgebraTableEntry};
pub use hom_module::HomModule;
pub use quotient_module::QuotientModule;
pub use rpn::RealProjectiveSpace;
pub use sum_module::SumModule;
pub use tensor_module::TensorModule;
pub use truncated_module::TruncatedModule;
pub use zero_module::ZeroModule;
pub use bounded_module::BoundedModule;
pub use module_trait::{Module, ModuleFailedRelationError};

use crate::algebra::SteenrodAlgebra;

// Poor man's trait alias
pub trait SteenrodModule: Module<Algebra = SteenrodAlgebra> {}
impl<M: Module<Algebra = SteenrodAlgebra>> SteenrodModule for M {}