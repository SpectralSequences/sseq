#![feature(const_panic)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::unreadable_literal)]

pub mod matrix;
pub mod prime;
pub mod vector;
mod vector_inner;

/// This is a version of [`std::convert::TryInto`] that deals with mutable references instead. We
/// cannot condition on `TryInto` for mutable references due to lifetime issues. This is used for
/// the add_carry implementation
pub(crate) trait TryInto<T> {
    fn try_into(&mut self) -> Option<&mut T>;
}

impl<T> TryInto<T> for T {
    fn try_into(&mut self) -> Option<&mut T> {
        Some(self)
    }
}
