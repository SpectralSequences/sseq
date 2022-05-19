use super::Bidegree;

use std::fmt::{self, Display, Formatter};

use fp::vector::FpVector;

/// An element of a bigraded vector space. Most commonly used to index elements of spectral
/// sequences.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BidegreeElement {
    /// Bidegree of the element
    degree: Bidegree,
    /// Representing vector
    vec: FpVector,
}

impl BidegreeElement {
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

    pub fn vec(&self) -> &FpVector {
        &self.vec
    }

    pub fn new<T: Into<Bidegree>>(degree: T, vec: FpVector) -> BidegreeElement {
        BidegreeElement {
            degree: degree.into(),
            vec,
        }
    }
}

impl Display for BidegreeElement {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.n(), self.s(), self.vec())
    }
}

impl From<(Bidegree, FpVector)> for BidegreeElement {
    fn from(tuple: (Bidegree, FpVector)) -> Self {
        Self::new(tuple.0, tuple.1)
    }
}

impl From<BidegreeElement> for (Bidegree, FpVector) {
    fn from(elt: BidegreeElement) -> Self {
        (elt.degree, elt.vec) // taken by move, so move out
    }
}

impl<'a> From<&'a BidegreeElement> for (Bidegree, &'a FpVector) {
    fn from(elt: &'a BidegreeElement) -> Self {
        (elt.degree, elt.vec()) // use method .vec() to avoid moving
    }
}
