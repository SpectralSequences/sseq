//! The C-motivic prime 2 Steenrod algebra and its mod-$\tau$ reduction.
//!
//! This layer implements the *deformation* view of the C-motivic Adams $E_2$: the
//! C-motivic dual Steenrod algebra $A_C$ over $\mathbb{F}_2[\tau]$, and its mod-$\tau$
//! reduction $A_C/\tau$ (a connected finite-type $\mathbb{F}_2$-algebra).
//!
//! The foundation layer is [`MotivicMilnorAlgebra`] — $A_C$, a free
//! $\mathbb{F}_2[\tau]$-module on the Milnor basis, with the Kong–Lin product
//! ([`milnor`]). This is the product **engine**; it is deliberately not an
//! [`Algebra`](crate::algebra::Algebra) (that trait is over $\mathbb{F}_p$).
//!
//! There is no representation of the coefficient ring $\mathbb{F}_2[\tau]$ here, because
//! nothing needs one: every element in sight is bidegree-homogeneous and $\tau$ has weight
//! $-1$, so a coefficient is pinned by the weights of the terms it sits between. See
//! [`milnor::Grading::tau_exponent`], which is how one is recovered when a caller wants it.
//!
//! The mod-$\tau$ reduction $A_C/\tau$ — the connected finite-type
//! $\mathbb{F}_2$-algebra that the existing resolution engine resolves to yield the
//! algebraic Novikov $E_2$ — is presented as an [`Algebra`](crate::algebra::Algebra)
//! in a follow-up on top of this engine.

pub mod milnor;
pub use milnor::MotivicMilnorAlgebra;
