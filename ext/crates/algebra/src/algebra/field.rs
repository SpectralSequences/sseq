//! Finite fields over a prime.

use std::sync::Arc;

use fp::{
    prime::ValidPrime,
    vector::{FpSlice, FpSliceMut},
};

use crate::{
    algebra::{Algebra, Bialgebra},
    module::{FreeModule, Module},
};

/// $\mathbb{F}_p$, viewed as an [`Algebra`] over itself.
///
/// As an [`Algebra`], a field is one-dimensional, with basis element `1`.
/// It is also trivially a coalgebra via the trivial diagonal comultiplication,
/// and thus a [`Bialgebra`]. It is also the base ring (a [`Ring`](crate::algebra::Ring))
/// of every classical $\mathbb{F}_p$ algebra.
#[derive(Clone, Copy)]
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

/// Build the graded piece of a classical (`BaseRing = Field`) algebra: a free `Field`-module with
/// `num_gens` generators concentrated in weight 0. This is the shared body of the
/// [`module_at`](Algebra::module_at) accessor for every classical algebra ([`Field`],
/// [`AdemAlgebra`](crate::algebra::AdemAlgebra), [`MilnorAlgebra`](crate::algebra::MilnorAlgebra)).
pub(crate) fn classical_graded_piece(base_ring: Field, num_gens: usize) -> FreeModule<Field> {
    let piece = FreeModule::new(Arc::new(base_ring), String::new(), 0);
    piece.add_generators(0, num_gens, None);
    piece.compute_basis(0);
    piece
}

impl Algebra for Field {
    type BaseRing = Self;
    type GradedPiece = FreeModule<Self>;

    fn base_ring(&self) -> Self {
        *self
    }

    fn module_at(&self, t: i32) -> FreeModule<Self> {
        classical_graded_piece(self.base_ring(), self.dimension(t))
    }

    fn prime(&self) -> ValidPrime {
        self.prime
    }

    fn compute_basis(&self, _degree: i32) {}

    fn dimension(&self, degree: i32) -> usize {
        usize::from(degree == 0)
    }

    fn multiply_basis_elements(
        &self,
        mut result: FpSliceMut,
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

    fn element_to_string(&self, degree: i32, element: FpSlice) -> String {
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
