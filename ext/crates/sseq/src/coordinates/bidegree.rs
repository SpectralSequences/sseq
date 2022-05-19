use std::{
    fmt::{self, Display, Formatter},
    ops::{Add, Sub},
};

use super::iter::{ClassicalIterator, StemIterator};

/// Type synonym for (s, t) bidegrees.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bidegree {
    /// Homological degree
    s: u32,
    /// Internal degree
    t: i32,
}

impl Bidegree {
    pub fn classical(s: u32, t: i32) -> Bidegree {
        Self { s, t }
    }
    pub fn stem(s: u32, n: i32) -> Bidegree {
        Self { s, t: n + s as i32 }
    }

    pub const fn origin() -> Bidegree {
        Bidegree { s: 0, t: 0 }
    }

    pub fn s(&self) -> u32 {
        self.s
    }

    pub fn t(&self) -> i32 {
        self.t
    }

    pub fn n(&self) -> i32 {
        self.t - self.s as i32
    }

    pub fn step_t(&self, step: i32) -> Bidegree {
        Bidegree::classical(self.s(), self.t() + step)
    }

    pub fn step_n(&self, step: i32) -> Bidegree {
        Bidegree::stem(self.s(), self.n() + step)
    }

    /// Iterate over the rectangular region with lower left corner the origin and upper right corner
    /// `self`, in classical (s, t) coordinates. Bounds are inclusive.
    pub fn iter_classical(&self) -> ClassicalIterator {
        Bidegree::origin().iter_classical_to(*self)
    }

    /// Iterate over a rectangular region with *lower left* corner `self` and upper right corner
    /// `end`, in classical (s, t) coordinates. Note this is the opposite behavior as
    /// [`iter_classical`], in the sense that `self` is now the opposite corner.
    pub fn iter_classical_to(&self, end: Bidegree) -> ClassicalIterator {
        ClassicalIterator::new(*self, end)
    }

    /// Iterate over the rectangular region with lower left corner the origin and upper right corner
    /// `self`, in stem (s, n) coordinates. Bounds are inclusive.
    pub fn iter_stem(&self) -> StemIterator {
        Bidegree::origin().iter_stem_to(*self)
    }

    /// Iterate over a rectangular region with *lower left* corner `self` and upper right corner
    /// `end`, in stem (s, n) coordinates. Note this is the opposite behavior as [`iter_stem`], in
    /// the sense that `self` is now the opposite corner.
    pub fn iter_stem_to(&self, end: Bidegree) -> StemIterator {
        StemIterator::new(*self, end)
    }

    /// Returns difference as a bidegree if the difference in homological degrees is nonnegative,
    /// otherwise returns None.
    pub fn try_subtract<T: Into<Bidegree>>(&self, smaller: T) -> Option<Bidegree> {
        let smaller = smaller.into();
        if self.s >= smaller.s {
            Some(Bidegree {
                s: self.s - smaller.s,
                t: self.t - smaller.t,
            })
        } else {
            None
        }
    }

    /// Computes the bidegree containing the Massey product of elements in the given bidegrees.
    ///
    /// # Panics
    /// Panics if every element is in homological degree 0. This is the only case that would result
    /// in a bidegree in negative homological degree.
    pub fn massey_bidegree(a: Bidegree, b: Bidegree, c: Bidegree) -> Bidegree {
        (a + b + c)
            .try_subtract(Bidegree::classical(1, 0))
            .expect("Trying to compute Massey product of elements in s = 0")
    }
}

impl Display for Bidegree {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.n(), self.s())
    }
}

impl<T: Into<Bidegree>> Add<T> for Bidegree {
    type Output = Self;

    fn add(self, other: T) -> Self {
        let other = other.into();
        Self {
            s: self.s + other.s,
            t: self.t + other.t,
        }
    }
}

impl<T: Into<Bidegree>> Sub<T> for Bidegree {
    type Output = Self;

    fn sub(self, other: T) -> Self {
        let other = other.into();
        Self {
            s: self.s - other.s,
            t: self.t - other.t,
        }
    }
}
