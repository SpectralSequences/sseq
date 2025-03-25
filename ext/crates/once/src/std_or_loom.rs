//! This module provides a way to use the standard library or loom's types in a unified way,
//! depending on the `loom` cfg flag.

#[cfg(not(loom))]
pub(crate) use std::*;

#[cfg(loom)]
pub(crate) use loom::*;

/// A trait for getting the inner value of an atomic type when we hold a mutable reference.
///
/// We could just use `get_mut` directly, but loom's atomic types do not implement `get_mut`.
/// Instead, we just get it atomically, since performance doesn't matter so much in tests.
pub trait GetMut {
    type Inner;

    /// Use the fact that we have a mutable reference to load the value non-atomically.
    fn get_by_mut(&mut self) -> Self::Inner;
}

impl GetMut for std::sync::atomic::AtomicU8 {
    type Inner = u8;

    fn get_by_mut(&mut self) -> Self::Inner {
        *self.get_mut()
    }
}

#[cfg(loom)]
impl GetMut for loom::sync::atomic::AtomicU8 {
    type Inner = u8;

    fn get_by_mut(&mut self) -> Self::Inner {
        self.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl GetMut for std::sync::atomic::AtomicUsize {
    type Inner = usize;

    fn get_by_mut(&mut self) -> Self::Inner {
        *self.get_mut()
    }
}

#[cfg(loom)]
impl GetMut for loom::sync::atomic::AtomicUsize {
    type Inner = usize;

    fn get_by_mut(&mut self) -> Self::Inner {
        self.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl<T> GetMut for std::sync::atomic::AtomicPtr<T> {
    type Inner = *mut T;

    fn get_by_mut(&mut self) -> Self::Inner {
        *self.get_mut()
    }
}

#[cfg(loom)]
impl<T> GetMut for loom::sync::atomic::AtomicPtr<T> {
    type Inner = *mut T;

    fn get_by_mut(&mut self) -> Self::Inner {
        self.load(std::sync::atomic::Ordering::Relaxed)
    }
}
