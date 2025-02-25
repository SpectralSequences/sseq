use std::{
    hash::Hash,
    ops::{Add, AddAssign, Deref, Div, Mul, MulAssign, Neg, Sub, SubAssign},
};

use super::{Field, field_internal::FieldInternal};

/// This just ensures that the containers are "nice enough", in the sense that they are cloneable,
/// hashable, etc. We may add more custom methods in the future.
pub trait FieldElementContainer:
    std::fmt::Debug + std::fmt::Display + Clone + PartialEq + Eq + Hash
{
}

/// An element of a field.
///
/// This contains the field itself so that it knows how to do arithmetic operations. We want this to
/// be a _struct_ rather than a trait, which means that we want the _actual_ storage of the value to
/// be managed by the field itself. Therefore, we have an internal field trait that knows about
/// arithmetic operations and other implementation details, but these operations are only accessible
/// from outside the crate using this struct.
///
/// It might seem wasteful to handle, say, `FieldElement<Fp<P>>`s rather than `u32` in the API for
/// `FqVector<Fp<P>>`. However, this gives us type-level guarantees that the invariants of the
/// elements hold, i.e. in this case that its value is in the range `0..P`. Moreover, this is bigger
/// than a bare `F::ElementContainer` only when the field has a positive memory footprint. The cases
/// we care most about, `Fp<P2>`, `Fp<P3>`, `Fp<P5>`, and `Fp<P7>`, are all ZSTs and therefore don't
/// cause any overhead.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct FieldElement<F: FieldInternal> {
    field: F,
    pub(super) value: F::ElementContainer,
}

impl<F: FieldInternal> FieldElement<F> {
    /// Create a new field element. This is only visible to the `field` module, because the caller
    /// is responsible for ensuring that the invariants of `value` hold.
    ///
    /// Handling `FieldElement`s in the API rather than the containers directly has the advantage of
    /// being sure at compile-time that the invariants hold.
    pub(super) fn new(field: F, value: F::ElementContainer) -> Self {
        Self { field, value }
    }

    pub fn field(&self) -> F {
        self.field
    }

    pub(crate) fn val(self) -> F::ElementContainer {
        self.value
    }

    pub fn inv(self) -> Option<Self> {
        self.field.inv(self)
    }

    pub fn frobenius(self) -> Self {
        self.field.frobenius(self)
    }
}

// Allows us to access methods on `F::Element` directly
impl<F: FieldInternal> Deref for FieldElement<F> {
    type Target = F::ElementContainer;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

macro_rules! impl_arith {
    ($trait:ident, $trait_assign:ident, $method:ident, $method_assign:ident) => {
        impl<F: Field> $trait for FieldElement<F> {
            type Output = Self;

            fn $method(self, rhs: Self) -> Self::Output {
                self.field.$method(self, rhs)
            }
        }

        impl<F: Field> $trait_assign for FieldElement<F> {
            fn $method_assign(&mut self, rhs: Self) {
                self.field.$method_assign(self, rhs);
            }
        }
    };
}

impl_arith!(Add, AddAssign, add, add_assign);
impl_arith!(Sub, SubAssign, sub, sub_assign);
impl_arith!(Mul, MulAssign, mul, mul_assign);

impl<F: Field> Div for FieldElement<F> {
    type Output = Option<Self>;

    fn div(self, rhs: Self) -> Self::Output {
        self.field.div(self, rhs)
    }
}

impl<F: Field> Neg for FieldElement<F> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        self.field.neg(self)
    }
}

impl<F: Field> std::fmt::Display for FieldElement<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}
