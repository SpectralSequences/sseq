use std::fmt::{self, Display, Formatter};

use fp::vector::{FpSlice, FpVector};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use super::{MultiDegree, MultiDegreeGenerator};

/// An element of a graded vector space. Most commonly used to index elements of spectral
/// sequences.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MultiDegreeElement<const N: usize> {
    /// Degree of the element
    degree: MultiDegree<N>,
    /// Representing vector
    vec: FpVector,
}

impl<const N: usize> MultiDegreeElement<N> {
    pub fn new(degree: MultiDegree<N>, vec: FpVector) -> Self {
        Self { degree, vec }
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

    pub fn vec(&self) -> FpSlice<'_> {
        self.vec.as_slice()
    }

    pub fn into_vec(self) -> FpVector {
        self.vec
    }

    /// Get the string representation of the element as a linear combination of generators. For
    /// example, an element in bidegree `(n,s)` with vector `[0,2,1]` will be printed as `2 x_(n, s,
    /// 1) + x_(n, s, 2)`.
    pub fn to_basis_string(&self) -> String {
        self.vec
            .iter_nonzero()
            .map(|(i, v)| {
                let g = MultiDegreeGenerator::new(self.degree(), i);
                let coeff_str = if v != 1 {
                    format!("{v} ")
                } else {
                    String::new()
                };
                format!("{coeff_str}x_{g}")
            })
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

impl<const N: usize> Display for MultiDegreeElement<N> {
    /// ```
    /// # use sseq::coordinates::{MultiDegreeElement, MultiDegree};
    /// # use fp::{prime::P2, vector::FpVector};
    ///
    /// let vec = FpVector::from_slice(P2, &[1, 0, 1]);
    ///
    /// let el3 = MultiDegreeElement::new(MultiDegree::new([1, 2, 3]), vec.clone());
    ///
    /// assert_eq!(format!("{el3}"), String::from("(1, 2, 3, [1, 0, 1])"));
    /// assert_eq!(format!("{el3:#}"), String::from("(1,2,3)[101]"));
    ///
    /// let el0 = MultiDegreeElement::new(MultiDegree::new([]), vec);
    ///
    /// assert_eq!(format!("{el0}"), String::from("([1, 0, 1])"));
    /// assert_eq!(format!("{el0:#}"), String::from("()[101]"));
    /// ```
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let sep = if f.alternate() {
            // Compact representation
            ","
        } else {
            ", "
        };
        let coord_str = self.degree.coords().iter().map(i32::to_string).join(sep);
        if f.alternate() {
            write!(f, "({coord_str})[{:#}]", self.vec())
        } else if N == 0 {
            write!(f, "({})", self.vec())
        } else {
            write!(f, "({coord_str}, {})", self.vec())
        }
    }
}
