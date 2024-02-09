use std::cmp::Ordering::*;

use once::OnceBiVec;

pub struct DenseBigradedModule {
    dimensions: OnceBiVec<OnceBiVec<usize>>,
    min_y: i32,
}

impl DenseBigradedModule {
    pub fn new(min_x: i32, min_y: i32) -> Self {
        let dimensions = OnceBiVec::new(min_x);
        dimensions.push(OnceBiVec::new(min_y));
        Self { dimensions, min_y }
    }

    pub const fn min_x(&self) -> i32 {
        self.dimensions.min_degree()
    }

    pub const fn min_y(&self) -> i32 {
        self.min_y
    }

    pub fn max_x(&self) -> i32 {
        self.dimensions.max_degree()
    }

    pub fn max_y(&self) -> i32 {
        self.dimensions
            .iter()
            .map(OnceBiVec::max_degree)
            .max()
            .unwrap_or_else(|| self.min_y())
    }

    pub fn range(&self, x: i32) -> std::ops::Range<i32> {
        self.dimensions[x].range()
    }

    pub fn defined(&self, x: i32, y: i32) -> bool {
        self.dimensions.get(x).is_some() && self.dimensions[x].get(y).is_some()
    }

    /// This can only be set when bidegrees to the left and bottom of (x, y) have been set.
    pub fn set_dimension(&self, x: i32, y: i32, dim: usize) {
        assert!(
            x <= self.dimensions.len(),
            "Cannot set dimension at ({}, {}) before ({}, {}).",
            x,
            y,
            x - 1,
            y
        );
        if x == self.dimensions.len() {
            self.dimensions
                .push_checked(OnceBiVec::new(self.min_y()), x);
        }
        match y.cmp(&self.dimensions[x].len()) {
            Less => panic!("Already set dimension at ({x}, {y})"),
            Equal => self.dimensions[x].push_checked(dim, y),
            Greater => panic!(
                "Cannot set dimension at ({}, {}) before ({}, {})",
                x,
                y,
                x,
                y - 1
            ),
        }
    }

    /// The dimension in a bidegree, None if not yet defined
    pub fn get_dimension(&self, x: i32, y: i32) -> Option<usize> {
        Some(*self.dimensions.get(x)?.get(y)?)
    }

    pub fn dimension(&self, x: i32, y: i32) -> usize {
        self.get_dimension(x, y).unwrap()
    }
}
