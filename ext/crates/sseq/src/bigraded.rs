use std::cmp::Ordering::*;

use once::OnceBiVec;

use crate::coordinates::Bidegree;

pub struct DenseBigradedModule {
    dimensions: OnceBiVec<OnceBiVec<usize>>,
    min_y: i32,
}

impl DenseBigradedModule {
    pub fn new(min: Bidegree) -> Self {
        let dimensions = OnceBiVec::new(min.x());
        dimensions.push(OnceBiVec::new(min.y()));
        Self {
            dimensions,
            min_y: min.y(),
        }
    }

    pub const fn min(&self) -> Bidegree {
        Bidegree::x_y(self.dimensions.min_degree(), self.min_y)
    }

    pub fn max(&self) -> Bidegree {
        Bidegree::x_y(
            self.dimensions.max_degree(),
            self.dimensions
                .iter()
                .map(OnceBiVec::max_degree)
                .max()
                .unwrap_or(self.min_y),
        )
    }

    pub fn range(&self, x: i32) -> std::ops::Range<i32> {
        self.dimensions[x].range()
    }

    pub fn defined(&self, b: Bidegree) -> bool {
        self.dimensions.get(b.x()).is_some() && self.dimensions[b.x()].get(b.y()).is_some()
    }

    /// This can only be set when bidegrees to the left and bottom of `b` have been set.
    pub fn set_dimension(&self, b: Bidegree, dim: usize) {
        assert!(
            b.x() <= self.dimensions.len(),
            "Cannot set dimension at {b} before {b_minus_1x}.",
            b_minus_1x = b - Bidegree::x_y(1, 0)
        );
        if b.x() == self.dimensions.len() {
            self.dimensions
                .push_checked(OnceBiVec::new(self.min().y()), b.x());
        }
        match b.y().cmp(&self.dimensions[b.x()].len()) {
            Less => panic!("Already set dimension at {b}"),
            Equal => self.dimensions[b.x()].push_checked(dim, b.y()),
            Greater => panic!(
                "Cannot set dimension at {b} before {b_minus_1y}",
                b_minus_1y = b - Bidegree::x_y(0, 1)
            ),
        }
    }

    /// The dimension in a bidegree, None if not yet defined
    pub fn get_dimension(&self, b: Bidegree) -> Option<usize> {
        Some(*self.dimensions.get(b.x())?.get(b.y())?)
    }

    pub fn dimension(&self, b: Bidegree) -> usize {
        self.get_dimension(b).unwrap()
    }
}
