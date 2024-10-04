use std::fmt::{self, Display, Formatter};

use fp::{prime::ValidPrime, vector::FpVector};
use serde::{Deserialize, Serialize};

use super::{Bidegree, BidegreeElement};

/// A *basis* element of a bigraded vector space. Most commonly used to index elements of spectral
/// sequences.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BidegreeGenerator {
    /// Bidegree of the element
    degree: Bidegree,
    /// Position in the canonical basis for this bidegree
    idx: usize,
}

impl BidegreeGenerator {
    pub fn new<T: Into<Bidegree>>(degree: T, idx: usize) -> Self {
        Self {
            degree: degree.into(),
            idx,
        }
    }

    pub fn s_t(s: u32, t: i32, idx: usize) -> Self {
        Self::new(Bidegree::s_t(s, t), idx)
    }

    pub fn n_s(n: i32, s: u32, idx: usize) -> Self {
        Self::new(Bidegree::n_s(n, s), idx)
    }

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

    pub fn into_element(self, p: ValidPrime, ambient: usize) -> BidegreeElement {
        let mut vec = FpVector::new(p, ambient);
        vec.set_entry(self.idx, 1);
        BidegreeElement::new(self.degree, vec)
    }
}

impl Display for BidegreeGenerator {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if f.alternate() {
            // Compact representation
            write!(f, "({},{},{})", self.n(), self.s(), self.idx())
        } else {
            write!(f, "({}, {}, {})", self.n(), self.s(), self.idx())
        }
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
        if value.vec().iter().sum::<u32>() == 1 {
            let (idx, _) = value.vec().iter_nonzero().next().unwrap();
            Ok((value.degree(), idx).into())
        } else {
            Err(())
        }
    }
}
