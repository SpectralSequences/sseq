use std::{
    fmt::{self, Display, Formatter},
    ops::{Add, Sub},
};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

/// A multi-degree used to index multiply-graded objects.
///
/// In particular,
/// - `MultiDegree<0>` is a ZST, used to model ungraded objects.
/// - `MultiDegree<1>` is a single integer, used to model graded objects (e.g. homotopy groups).
/// - `MultiDegree<2>` is a regular old bidegree, used for most spectral sequences.
/// - `MultiDegree<N>` for `N > 2` is useful for some more structured spectral sequences, like the
///   ones that arise in motivic, synthetic, or equivariant homotopy theory.
///
/// For `N > 1`, we use the convention that the first coordinate is `n` and the second is `s`. This
/// makes multi-degrees easier to work with graphically, and to display as a string. For smaller
/// values of `N`, the distinction is irrelevant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MultiDegree<const N: usize> {
    // Serde doesn't support const-generic arrays due to backwards compatibility issues, so we have
    // to use a dedicated external crate.
    #[serde(with = "serde_arrays")]
    coords: [i32; N],
}

impl<const N: usize> MultiDegree<N> {
    pub const fn new(coords: [i32; N]) -> Self {
        Self { coords }
    }

    pub const fn zero() -> Self {
        Self { coords: [0; N] }
    }

    pub fn n(&self) -> i32 {
        self.coords.first().copied().unwrap_or(0)
    }

    pub fn s(&self) -> i32 {
        self.coords.get(1).copied().unwrap_or(0)
    }

    pub fn t(&self) -> i32 {
        self.n() + self.s()
    }

    pub fn x(&self) -> i32 {
        self.n()
    }

    pub fn y(&self) -> i32 {
        self.s()
    }

    pub fn coords(&self) -> [i32; N] {
        self.coords
    }
}

impl<const N: usize> Default for MultiDegree<N> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<const N: usize> Display for MultiDegree<N> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let inner = self.coords.iter().map(|i| i.to_string()).join(", ");
        write!(f, "({inner})")
    }
}

impl<const N: usize> Add for MultiDegree<N> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let coords = std::array::from_fn(|i| self.coords[i] + other.coords[i]);
        Self { coords }
    }
}

impl<const N: usize> Sub for MultiDegree<N> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let coords = std::array::from_fn(|i| self.coords[i] - other.coords[i]);
        Self { coords }
    }
}
