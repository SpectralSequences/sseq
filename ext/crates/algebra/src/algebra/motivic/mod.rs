//! The C-motivic prime 2 Steenrod algebra and its mod-$\tau$ reduction.

pub mod milnor;
pub use milnor::MotivicMilnorAlgebra;

pub mod ctau;
pub use ctau::CTauAlgebra;
