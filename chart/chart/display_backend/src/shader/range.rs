#[derive(Copy, Clone, Debug)]
pub struct Range {
    pub min : usize,
    pub max : usize
}

impl Range {
    pub fn empty() -> Self {
        Self {
            min : usize::MAX,
            max : 0
        }
    }
    
    pub fn new(min : usize, max : usize) -> Self {
        if max <= min {
            Self::empty()
        } else {
            Self {
                min,
                max
            }
        }
    }

    pub fn is_empty(&mut self) -> bool {
        self.max <= self.min
    }

    pub fn include_int(&mut self, x : usize) {
        self.min = self.min.min(x);       
        self.max = self.max.max(x + 1);
    }

    pub fn include_range(&mut self, other : Range){
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }
}

use std::ops;

impl From<usize> for Range {
    fn from(a : usize) -> Self {
        Self::new(a, a+1)
    }
}

impl From<ops::Range<usize>> for Range {
    fn from(a : ops::Range<usize>) -> Self {
        Self::new(a.start, a.end + 1)
    }
}

impl From<ops::RangeFrom<usize>> for Range {
    fn from(a : ops::RangeFrom<usize>) -> Self {
        Self::new(a.start, usize::MAX)
    }
}

impl From<ops::RangeFull> for Range {
    fn from(_a : ops::RangeFull) -> Self {
        Self::new(0, usize::MAX)
    }
}

impl From<ops::RangeInclusive<usize>> for Range {
    fn from(a : ops::RangeInclusive<usize>) -> Self {
        Self::new(*a.start(), *a.end() + 1)
    }
}

impl From<ops::RangeTo<usize>> for Range {
    fn from(a : ops::RangeTo<usize>) -> Self {
        Self::new(0, a.end)
    }
}

impl From<ops::RangeToInclusive<usize>> for Range {
    fn from(a : ops::RangeToInclusive<usize>) -> Self {
        Self::new(0, a.end + 1)
    }
}