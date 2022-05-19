use super::{
    ordered_bidegrees::{ClassicalBidegree, StemBidegree},
    Bidegree,
};

/// Iterates over a rectangular region in (s, t) coordinates, with lower left corner `min` and upper
/// right corner `max`, inclusively. The iteration is done first left-to-right, then bottom-to-top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClassicalIterator {
    /// Lower left corner
    min: ClassicalBidegree,
    /// Upper right corner
    max: ClassicalBidegree,
    /// Current bidegree
    current: ClassicalBidegree,
}

impl ClassicalIterator {
    pub fn new(min: Bidegree, max: Bidegree) -> Self {
        ClassicalIterator {
            min: min.into(),
            max: max.into(),
            current: min.into(),
        }
    }

    pub fn new_from_origin(max: Bidegree) -> Self {
        Self::new(Bidegree::origin(), max)
    }
}

impl From<Bidegree> for ClassicalIterator {
    fn from(deg: Bidegree) -> Self {
        Self::new_from_origin(deg)
    }
}

impl<'a> From<&'a Bidegree> for ClassicalIterator {
    fn from(deg: &'a Bidegree) -> Self {
        Self::new_from_origin(*deg)
    }
}

impl Iterator for ClassicalIterator {
    type Item = ClassicalBidegree;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current <= self.max {
            let new = if self.current.s() < self.max.s() {
                self.current + Bidegree::classical(1, 0)
            } else {
                Bidegree::classical(self.min.s(), self.current.t() + 1).into()
            };
            Some(std::mem::replace(&mut self.current, new))
        } else {
            None
        }
    }
}

/// Iterates over a rectangular region in (n, s) coordinates, with lower left corner `min` and upper
/// right corner `max`, inclusively. The iteration is done first left-to-right, then bottom-to-top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StemIterator {
    /// Lower left corner
    min: StemBidegree,
    /// Upper right corner
    max: StemBidegree,
    /// Current bidegree
    current: StemBidegree,
}

impl StemIterator {
    pub fn new(min: Bidegree, max: Bidegree) -> Self {
        StemIterator {
            min: min.into(),
            max: max.into(),
            current: min.into(),
        }
    }

    pub fn new_from_origin(max: Bidegree) -> Self {
        Self::new(Bidegree::origin(), max)
    }
}

impl From<Bidegree> for StemIterator {
    fn from(deg: Bidegree) -> Self {
        Self::new(Bidegree::origin(), deg)
    }
}

impl<'a> From<&'a Bidegree> for StemIterator {
    fn from(deg: &'a Bidegree) -> Self {
        Self::new(Bidegree::origin(), *deg)
    }
}

impl Iterator for StemIterator {
    type Item = StemBidegree;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current <= self.max {
            let new = if self.current.s() < self.max.s() {
                self.current + Bidegree::stem(1, 0)
            } else {
                Bidegree::stem(self.min.s(), self.current.n() + 1).into()
            };
            Some(std::mem::replace(&mut self.current, new))
        } else {
            None
        }
    }
}
