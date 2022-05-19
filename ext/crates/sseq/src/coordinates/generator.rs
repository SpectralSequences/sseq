use super::{Bidegree, BidegreeElement};

use std::fmt::{self, Display, Formatter};

/// A *basis* element of a bigraded vector space. Most commonly used to index elements of spectral
/// sequences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BidegreeGenerator {
    /// Bidegree of the element
    degree: Bidegree,
    /// Position in the canonical basis for this bidegree
    idx: usize,
}

impl BidegreeGenerator {
    pub fn s(&self) -> u32 {
        self.degree.s()
    }

    pub fn t(&self) -> i32 {
        self.degree.t()
    }

    pub fn degree(&self) -> Bidegree {
        self.degree
    }

    pub fn n(&self) -> i32 {
        self.degree.n()
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn new<T: Into<Bidegree>>(degree: T, idx: usize) -> BidegreeGenerator {
        BidegreeGenerator {
            degree: degree.into(),
            idx,
        }
    }
}

impl Display for BidegreeGenerator {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.n(), self.s(), self.idx())
    }
}

impl From<(Bidegree, usize)> for BidegreeGenerator {
    fn from(tuple: (Bidegree, usize)) -> Self {
        Self::new(tuple.0, tuple.1)
    }
}

impl TryFrom<BidegreeElement> for BidegreeGenerator {
    type Error = ();

    fn try_from(value: BidegreeElement) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl<'a> TryFrom<&'a BidegreeElement> for BidegreeGenerator {
    type Error = ();

    fn try_from(value: &'a BidegreeElement) -> Result<Self, Self::Error> {
        let (degree, v) = value.into();
        if v.iter().sum::<u32>() == 1 {
            let (idx, _) = v.first_nonzero().unwrap();
            Ok((degree, idx).into())
        } else {
            Err(())
        }
    }
}
