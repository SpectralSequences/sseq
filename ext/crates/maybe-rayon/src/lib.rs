#![deny(clippy::use_self, unsafe_op_in_unsafe_fn)]

#[cfg(feature = "concurrent")]
pub mod concurrent;
#[cfg(feature = "concurrent")]
pub use concurrent::*;

#[cfg(not(feature = "concurrent"))]
pub mod sequential;
#[cfg(not(feature = "concurrent"))]
pub use sequential::*;
