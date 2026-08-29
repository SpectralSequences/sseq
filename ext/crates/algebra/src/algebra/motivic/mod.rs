//! The C-motivic prime 2 Steenrod algebra and its mod-$\tau$ reduction.
//!
//! This layer implements the *deformation* view of the C-motivic Adams $E_2$: the
//! C-motivic dual Steenrod algebra $A_C$ over $\mathbb{F}_2[\tau]$, its mod-$\tau$
//! reduction $A_C/\tau$ (a connected finite-type $\mathbb{F}_2$-algebra), and the
//! coefficient ring $\mathbb{F}_2[\tau]$ itself.
//!
//! This module provides the foundation layer for that computation:
//!
//! - [`Tau`] — the coefficient ring $\mathbb{F}_2[\tau]$ as a small homogeneous
//!   scalar ([`tau`]); the whole $\tau$-tower is carried here rather than threaded
//!   through the resolution engine.
//! - [`MotivicMilnorAlgebra`] — $A_C$, a free $\mathbb{F}_2[\tau]$-module on the
//!   Milnor basis, with the Kong–Lin product ([`milnor`]). This is the product
//!   **engine**; it is deliberately not an [`Algebra`](crate::algebra::Algebra)
//!   (that trait is over $\mathbb{F}_p$).
//!
//! The mod-$\tau$ reduction $A_C/\tau$ — the connected finite-type
//! $\mathbb{F}_2$-algebra that the existing resolution engine resolves to yield the
//! algebraic Novikov $E_2$ — is presented as an [`Algebra`](crate::algebra::Algebra)
//! in a follow-up on top of this engine.

pub mod tau;
pub use tau::Tau;

pub mod milnor;
pub use milnor::MotivicMilnorAlgebra;
