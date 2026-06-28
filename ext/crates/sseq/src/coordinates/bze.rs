use std::fmt::{self, Display, Formatter};

use fp::matrix::Subquotient;

/// Classification of a spectral sequence generator by its role relative to a differential on a
/// given page. At page $r$, each generator in bidegree $b$ is exactly one of:
///
/// - **B** (boundary): in the image of $d_{r-1}$ from another bidegree.
/// - **Z** (cycle): in the kernel of $d_r$, and not a boundary.
/// - **E** (supports $d_r$): $d_r(x) \neq 0$.
///
/// Every spectral sequence admits this decomposition; the Adams spectral sequence for $S$ at $E_2$
/// has everything as **Z** (degenerate), while $S/\lambda^2$ at $E_3$ has a nontrivial splitting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BZE {
    /// Boundary: in the image of an incoming differential.
    B,
    /// Cycle mod boundary: survives to the next page.
    Z,
    /// Supports a differential: $d_r(x) \neq 0$.
    E,
}

impl BZE {
    /// Classify generator `idx` from a page's [`Subquotient`].
    ///
    /// The subquotient encodes $E_r = Z_r / B_r$ at a given bidegree:
    /// - `page.zeros().pivots()[idx] >= 0` means `idx` is a boundary pivot (**B**).
    /// - `page.complement_pivots()` yields generators not in the cycle subspace (**E**).
    /// - Everything else is a cycle that is not a boundary (**Z**).
    pub fn from_page_data(page: &Subquotient, idx: usize) -> Self {
        if page.zeros().pivots()[idx] >= 0 {
            return Self::B;
        }
        if page.complement_pivots().any(|p| p == idx) {
            return Self::E;
        }
        Self::Z
    }
}

impl Display for BZE {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::B => write!(f, "B"),
            Self::Z => write!(f, "Z"),
            Self::E => write!(f, "E"),
        }
    }
}
