/// A range of bidegrees, corresponding to all `s` up to a given value, and for each of them, a
/// maximum `t`.
pub struct BidegreeRange<'a, T> {
    /// The maximal value of `s` in the range.
    s: i32,
    /// The function that gives, for a given value of `s`, the maximum value for `t`.
    t: &'a (dyn Fn(&T, i32) -> i32 + Sync),
    /// Auxillary data that `t` may depend on.
    aux: &'a T,
}

impl<'a, T> BidegreeRange<'a, T> {
    pub fn new(aux: &'a T, s: i32, t: &'a (dyn Fn(&T, i32) -> i32 + Sync)) -> Self {
        Self { s, t, aux }
    }

    pub fn s(&self) -> i32 {
        self.s
    }

    pub fn t(&self, s: i32) -> i32 {
        (self.t)(self.aux, s)
    }

    pub fn restrict(self, s: i32) -> Self {
        assert!(s <= self.s);
        Self {
            s,
            t: self.t,
            aux: self.aux,
        }
    }
}

// A `BidegreeRange<T>` is only a bundle of integers and immutable references, so it should
// implement `Clone` for all `T`, and even `Copy`. However, `#[derive(Clone)]` only implements it
// for `T: Clone`, so we do it manually instead.
impl<T> Clone for BidegreeRange<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for BidegreeRange<'_, T> {}
