//! The base ring an algebra is linear over.
//!
//! An [`Algebra`] declares its coefficient ring as `type BaseRing: Ring`. [`Ring`] is that ring: the
//! scalar type, its arithmetic, and the representation of finite free modules over it (vectors and
//! their slices) — everything needed to define an algebra and act on its modules. A coefficient ring
//! is itself an [`Algebra`] (over itself), so [`Ring`] is a *sub-trait* of [`Algebra`]: `Field` is
//! the field $\mathbb{F}_p$ as a one-dimensional algebra over itself, and $\mathbb{F}_2[\tau]$ is the
//! polynomial algebra on $\tau$ over itself. This fold is what makes `Algebra::BaseRing: Algebra`
//! hold, so that each graded piece of an algebra can be viewed as a module over the base ring.
//!
//! The *harder* linear algebra that a free resolution needs — kernels and quasi-inverses, which
//! require the ring to be a graded DVR — is the [`GradedDvr`](crate::linear_algebra::GradedDvr)
//! subtrait, in [`linear_algebra`](crate::linear_algebra). Keeping that separate from [`Ring`] lets
//! an algebra be defined over a ring whose solving linear algebra is not yet implemented.
//!
//! Every classical algebra has `BaseRing = Field` (the field $\mathbb{F}_p$); the C-motivic Steenrod
//! algebra will use $\mathbb{F}_2[\tau]$.

use fp::{
    prime::Prime,
    vector::{FpSlice, FpSliceMut, FpVector},
};

use crate::{
    algebra::{Algebra, Field},
    linear_algebra::{BaseSlice, BaseSliceMut},
};

/// The base ring of an algebra `A`, i.e. `<A as Algebra>::BaseRing`.
///
/// The base ring is a `Copy` handle ([`Ring`] requires `Copy`), so it is threaded *by value*;
/// coefficient code reaches it via [`Module::base_ring`](crate::module::Module::base_ring) rather
/// than through the algebra's `Arc`.
pub type BaseRingOf<A> = <A as Algebra>::BaseRing;

/// The base-ring scalar type of an algebra `A`, i.e. `<A::BaseRing as Ring>::Element`.
///
/// This is the type of coefficients in `A`'s products and its modules' actions. For every classical
/// algebra it is `u32` (the field $\mathbb{F}_p$), so threading it through a signature is a no-op
/// there; the C-motivic algebra will make it $\mathbb{F}_2[\tau]$.
pub type Scalar<A> = <BaseRingOf<A> as Ring>::Element;

/// A graded coefficient ring over which algebras and their modules are linear — concretely
/// $\mathbb{F}_p$ (a field, via [`Field`]) or $\mathbb{F}_2[\tau]$.
///
/// A coefficient ring is itself an [`Algebra`] (over itself, i.e. `BaseRing = Self`), so this trait
/// is a *sub-trait* of [`Algebra`]. On top of the algebra structure it adds what is needed to serve
/// as coefficients: the scalar type and its arithmetic, together with a representation of finite free
/// modules over the ring (vectors and their slices). This is enough to *define* an algebra and its
/// modules — to multiply basis elements and act on modules, accumulating ring-coefficient results
/// into a vector. It is deliberately *not* enough to resolve: the linear algebra of solving (kernels,
/// quasi-inverses, minimal generators) requires the ring to be a graded DVR, and lives in the
/// [`GradedDvr`](crate::linear_algebra::GradedDvr) subtrait. Splitting them this way lets us define
/// algebras over rings whose solving linear algebra we have not yet implemented (e.g.
/// $\mathbb{R}$-motivic).
///
/// Confining the coefficient capacity (scalar, vector, arithmetic) to this sub-trait — rather than
/// putting it on [`Algebra`] itself — keeps [`Algebra`] lean: the Steenrod, Milnor, Adem, and
/// module-algebras, which are never coefficient rings, carry no dead scalar/vector members.
///
/// The ring is a `Copy` handle (like `fp`'s `Field`): it may carry a small amount of runtime data
/// (e.g. the prime of $\mathbb{F}_p$), so its operations take `self`.
pub trait Ring: Algebra + Copy + Send + Sync + 'static {
    /// A scalar of the ring.
    type Element: Copy + PartialEq + Send + Sync;

    fn zero(self) -> Self::Element;
    fn one(self) -> Self::Element;
    fn add(self, a: Self::Element, b: Self::Element) -> Self::Element;
    fn mul(self, a: Self::Element, b: Self::Element) -> Self::Element;
    fn is_zero(self, a: Self::Element) -> bool;

    /// Embed a prime-field coefficient — a canonical $\mathbb{F}_p$ representative, e.g. an entry of
    /// an $\mathbb{F}_p$-valued vector — into the ring.
    ///
    /// Every base ring is an (augmented) $\mathbb{F}_p$-algebra, so this is the inclusion
    /// $\mathbb{F}_p \hookrightarrow R$. It is used to lift the $\mathbb{F}_p$ multiplicities that
    /// appear when expanding a general element into basis elements into ring coefficients.
    fn embed_field(self, c: u32) -> Self::Element;

    /// An owned vector of a finite free module over the ring. For [`Field`] this is `fp`'s
    /// `FpVector`.
    type Vector: Send + Sync;

    /// An immutable slice of a [`Vector`](Ring::Vector). For [`Field`] this is `fp`'s `FpSlice`.
    type Slice<'a>: BaseSlice<'a, Self>;

    /// A mutable slice of a [`Vector`](Ring::Vector). For [`Field`] this is `fp`'s `FpSliceMut`.
    type SliceMut<'a>: BaseSliceMut<'a, Self>;

    /// Allocate a zero vector of length `len`.
    fn new_vector(self, len: usize) -> Self::Vector;

    /// The length of a vector.
    fn vector_len(self, vector: &Self::Vector) -> usize;

    /// Borrow a vector immutably.
    fn as_slice(self, vector: &Self::Vector) -> Self::Slice<'_>;

    /// Borrow a vector mutably.
    fn as_slice_mut(self, vector: &mut Self::Vector) -> Self::SliceMut<'_>;
}

impl Ring for Field {
    type Element = u32;
    type Slice<'a> = FpSlice<'a>;
    type SliceMut<'a> = FpSliceMut<'a>;
    type Vector = FpVector;

    fn zero(self) -> u32 {
        0
    }

    fn one(self) -> u32 {
        1
    }

    fn add(self, a: u32, b: u32) -> u32 {
        Algebra::prime(&self).sum(a, b)
    }

    fn mul(self, a: u32, b: u32) -> u32 {
        Algebra::prime(&self).product(a, b)
    }

    fn is_zero(self, a: u32) -> bool {
        a.is_multiple_of(Algebra::prime(&self).as_u32())
    }

    fn embed_field(self, c: u32) -> u32 {
        c
    }

    fn new_vector(self, len: usize) -> FpVector {
        FpVector::new(Algebra::prime(&self), len)
    }

    fn vector_len(self, vector: &FpVector) -> usize {
        vector.len()
    }

    fn as_slice(self, vector: &FpVector) -> FpSlice<'_> {
        vector.as_slice()
    }

    fn as_slice_mut(self, vector: &mut FpVector) -> FpSliceMut<'_> {
        vector.as_slice_mut()
    }
}
