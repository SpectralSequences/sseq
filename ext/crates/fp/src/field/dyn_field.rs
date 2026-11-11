use std::{
    fmt,
    ops::{Add, Mul, Neg, Sub},
};

use super::{Fp, SmallFq, element::FieldElement};
use crate::prime::ValidPrime;

/// A field element whose field type has been erased to one of the two concrete kinds the public
/// API exposes: a prime field [`Fp`] or a small extension field [`SmallFq`], each over a dynamic
/// [`ValidPrime`].
///
/// This lets callers that cannot be generic over the [`Field`](super::Field) trait — most notably
/// the Python bindings, where a `pyclass` cannot carry generic parameters — manipulate field
/// elements of either kind through a single type, with the arithmetic operators implemented once
/// here rather than re-matched at every call site.
///
/// Combining two elements that do not live in the same field (a different kind, or the same kind
/// over a different prime/degree) is a genuine failure, not a value: the binary operators report it
/// as [`None`]/[`Err`] rather than silently coercing one operand or returning a default element.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DynFieldElement {
    Fp(FieldElement<Fp<ValidPrime>>),
    SmallFq(FieldElement<SmallFq<ValidPrime>>),
}

/// Why [`DynFieldElement::try_div`] could not divide.
///
/// Division has two distinct failure modes that callers may want to surface differently (the Python
/// bindings map them to `ValueError` and `ZeroDivisionError` respectively), so unlike the additive
/// operators this is reported with a dedicated enum rather than a bare [`Option`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivError {
    /// The two operands do not live in the same field.
    MismatchedField,
    /// The divisor is the zero element.
    DivisionByZero,
}

impl fmt::Display for DivError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MismatchedField => f.write_str("cannot combine elements from different fields"),
            Self::DivisionByZero => f.write_str("division by zero"),
        }
    }
}

impl std::error::Error for DivError {}

/// Implement an additive/multiplicative operator once, returning `None` when the operands are not
/// in the same field. The field-equality guard is necessary: [`FieldElement`]'s own operators use
/// the left operand's field unconditionally, so without it a mismatch would silently compute a
/// bogus result instead of signalling failure.
macro_rules! impl_binop {
    ($trait:ident, $method:ident) => {
        impl $trait for DynFieldElement {
            type Output = Option<Self>;

            fn $method(self, rhs: Self) -> Self::Output {
                match (self, rhs) {
                    (Self::Fp(a), Self::Fp(b)) if a.field() == b.field() => {
                        Some(Self::Fp(a.$method(b)))
                    }
                    (Self::SmallFq(a), Self::SmallFq(b)) if a.field() == b.field() => {
                        Some(Self::SmallFq(a.$method(b)))
                    }
                    _ => None,
                }
            }
        }
    };
}

impl_binop!(Add, add);
impl_binop!(Sub, sub);
impl_binop!(Mul, mul);

impl Neg for DynFieldElement {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Self::Fp(x) => Self::Fp(-x),
            Self::SmallFq(x) => Self::SmallFq(-x),
        }
    }
}

impl DynFieldElement {
    /// Divide, distinguishing a mismatched-field failure from division by zero. Returns
    /// [`DivError::MismatchedField`] when the operands are not in the same field and
    /// [`DivError::DivisionByZero`] when `rhs` is the zero element.
    pub fn try_div(self, rhs: Self) -> Result<Self, DivError> {
        match (self, rhs) {
            (Self::Fp(a), Self::Fp(b)) if a.field() == b.field() => {
                (a / b).map(Self::Fp).ok_or(DivError::DivisionByZero)
            }
            (Self::SmallFq(a), Self::SmallFq(b)) if a.field() == b.field() => {
                (a / b).map(Self::SmallFq).ok_or(DivError::DivisionByZero)
            }
            _ => Err(DivError::MismatchedField),
        }
    }

    /// The multiplicative inverse, or [`None`] for the zero element.
    pub fn inv(self) -> Option<Self> {
        match self {
            Self::Fp(x) => x.inv().map(Self::Fp),
            Self::SmallFq(x) => x.inv().map(Self::SmallFq),
        }
    }

    /// The Frobenius endomorphism `x -> x^p`.
    pub fn frobenius(self) -> Self {
        match self {
            Self::Fp(x) => Self::Fp(x.frobenius()),
            Self::SmallFq(x) => Self::SmallFq(x.frobenius()),
        }
    }

    /// The canonical `u32` value of a prime-field element, or [`None`] for a [`SmallFq`] element,
    /// which has no canonical integer representative.
    pub fn try_as_u32(self) -> Option<u32> {
        match self {
            Self::Fp(x) => Some(*x),
            Self::SmallFq(_) => None,
        }
    }
}

impl fmt::Display for DynFieldElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fp(x) => write!(f, "{x}"),
            Self::SmallFq(x) => write!(f, "{x}"),
        }
    }
}
