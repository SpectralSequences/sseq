//! This module defines wrappers around `Bidegree` that implement `PartialOrd`. We have to implement
//! these traits on the wrappers instead of `Bidegree` itself because the behavior depends or
//! whether we are reasoning about them using the classical (s, t) coordinates or by stem (n, s).

use std::{
    cmp::Ordering,
    ops::{Add, Deref, DerefMut},
};

use super::{
    bidegree::Bidegree,
    lattice::{join, meet, JoinSemilattice, MeetSemilattice},
};

macro_rules! impl_ordered_bidegree {
    ($ty:ident, $from:ident, $h:ident) => {
        #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $ty(Bidegree);

        impl Deref for $ty {
            type Target = Bidegree;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for $ty {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl From<(u32, i32)> for $ty {
            fn from(tuple: (u32, i32)) -> Self {
                Bidegree::$from(tuple.0, tuple.1).into()
            }
        }

        impl From<$ty> for (u32, i32) {
            fn from(deg: $ty) -> Self {
                (deg.s(), deg.$h())
            }
        }

        impl<'a> From<&'a $ty> for (u32, i32) {
            fn from(deg: &'a $ty) -> Self {
                (deg.s(), deg.$h())
            }
        }

        impl From<Bidegree> for $ty {
            fn from(b: Bidegree) -> Self {
                $ty(b)
            }
        }

        impl From<$ty> for Bidegree {
            fn from(b: $ty) -> Bidegree {
                b.0
            }
        }

        impl PartialOrd for $ty {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                let (s1, h1) = (self.s(), self.$h());
                let (s2, h2) = (other.s(), other.$h());
                if s1 == s2 && h1 == h2 {
                    Some(Ordering::Equal)
                } else if s1 <= s2 && h1 <= h2 {
                    Some(Ordering::Less)
                } else if s1 >= s2 && h1 >= h2 {
                    Some(Ordering::Greater)
                } else {
                    None
                }
            }
        }

        impl MeetSemilattice for $ty {
            fn meet(self, rhs: $ty) -> $ty {
                Bidegree::$from(meet(self.s(), rhs.s()), meet(self.$h(), rhs.$h())).into()
            }
        }

        impl JoinSemilattice for $ty {
            fn join(self, rhs: $ty) -> $ty {
                Bidegree::$from(join(self.s(), rhs.s()), join(self.$h(), rhs.$h())).into()
            }
        }

        impl<T: Into<Bidegree>> Add<T> for $ty {
            type Output = $ty;

            fn add(self, rhs: T) -> $ty {
                let rhs = rhs.into();
                $ty(self.0 + rhs)
            }
        }
    };
}

impl_ordered_bidegree!(ClassicalBidegree, classical, t);
impl_ordered_bidegree!(StemBidegree, stem, n);
