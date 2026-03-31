use std::fmt::{self, Display, Formatter};

use fp::{prime::ValidPrime, vector::FpVector};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use super::{degree::MultiDegree, element::MultiDegreeElement};

/// A *basis* element of a multigraded vector space. Most commonly used to index elements of
/// spectral sequences.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MultiDegreeGenerator<const N: usize> {
    /// Degree of the element.
    degree: MultiDegree<N>,
    /// Position in the canonical basis for this multigraded vector space.
    idx: usize,
}

pub type BidegreeGenerator = MultiDegreeGenerator<2>;

impl<const N: usize> MultiDegreeGenerator<N> {
    pub fn new<T: Into<MultiDegree<N>>>(degree: T, idx: usize) -> Self {
        Self {
            degree: degree.into(),
            idx,
        }
    }

    pub fn s(&self) -> i32 {
        self.degree.s()
    }

    pub fn t(&self) -> i32 {
        self.degree.t()
    }

    pub fn degree(&self) -> MultiDegree<N> {
        self.degree
    }

    pub fn n(&self) -> i32 {
        self.degree.n()
    }

    pub fn x(&self) -> i32 {
        self.degree.x()
    }

    pub fn y(&self) -> i32 {
        self.degree.y()
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn into_element(self, p: ValidPrime, ambient: usize) -> MultiDegreeElement<N> {
        let mut vec = FpVector::new(p, ambient);
        vec.set_entry(self.idx, 1);
        MultiDegreeElement::new(self.degree, vec)
    }
}

impl<const N: usize> Display for MultiDegreeGenerator<N> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let sep = if f.alternate() {
            // Compact representation
            ","
        } else {
            ", "
        };
        let inner = self
            .degree
            .coords()
            .iter()
            .map(i32::to_string)
            .chain(std::iter::once(self.idx.to_string()))
            .join(sep);
        write!(f, "({inner})")
    }
}

impl<const N: usize> From<(MultiDegree<N>, usize)> for MultiDegreeGenerator<N> {
    fn from(tuple: (MultiDegree<N>, usize)) -> Self {
        Self::new(tuple.0, tuple.1)
    }
}

impl<const N: usize> TryFrom<MultiDegreeElement<N>> for MultiDegreeGenerator<N> {
    type Error = ();

    fn try_from(value: MultiDegreeElement<N>) -> Result<Self, Self::Error> {
        if value.vec().iter().sum::<u32>() == 1 {
            let (idx, _) = value.vec().iter_nonzero().next().unwrap();
            Ok((value.degree(), idx).into())
        } else {
            Err(())
        }
    }
}
