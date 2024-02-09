//! Finite fields over a prime.

use fp::{
    prime::ValidPrime,
    vector::{Slice, SliceMut},
};

use crate::algebra::{Algebra, Bialgebra};

/// $\mathbb{F}_p$, viewed as an [`Algebra`] over itself.
///
/// As an [`Algebra`], a field is one-dimensional, with basis element `1`.
/// It is also trivially a coalgebra via the trivial diagonal comultiplication,
/// and thus a [`Bialgebra`].
pub struct Field {
    prime: ValidPrime,
}

impl Field {
    /// Returns a new `Field` over the given prime `p`.
    pub fn new(p: ValidPrime) -> Self {
        Self { prime: p }
    }
}

impl std::fmt::Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "F_{}", self.prime)
    }
}

impl Algebra for Field {
    fn prime(&self) -> ValidPrime {
        self.prime
    }

    fn compute_basis(&self, _degree: i32) {}

    fn dimension(&self, degree: i32) -> usize {
        usize::from(degree == 0)
    }

    fn multiply_basis_elements(
        &self,
        mut result: SliceMut,
        coeff: u32,
        _r_degree: i32,
        _r_idx: usize,
        _s_degree: i32,
        _s_idx: usize,
    ) {
        result.add_basis_element(0, coeff)
    }

    fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
        vec![]
    }

    fn basis_element_to_string(&self, degree: i32, _idx: usize) -> String {
        assert!(degree == 0);
        "1".to_string()
    }

    fn element_to_string(&self, degree: i32, element: Slice) -> String {
        assert!(degree == 0);
        format!("{}", element.entry(0))
    }

    fn basis_element_from_string(&self, _elt: &str) -> Option<(i32, usize)> {
        Some((0, 0))
    }
}

impl Bialgebra for Field {
    fn coproduct(&self, _op_deg: i32, _op_idx: usize) -> Vec<(i32, usize, i32, usize)> {
        vec![(1, 0, 1, 0)]
    }

    fn decompose(&self, _op_deg: i32, _op_idx: usize) -> Vec<(i32, usize)> {
        vec![(1, 0)]
    }
}
