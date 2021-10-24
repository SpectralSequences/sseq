use crate::algebra::{Algebra, Bialgebra, BasisElem, Elem};
use fp::prime::ValidPrime;
use fp::vector::{Slice, SliceMut};

pub struct Field {
    prime: ValidPrime,
}

impl Field {
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
    /// Returns the prime the algebra is over.
    fn prime(&self) -> ValidPrime {
        self.prime
    }

    fn compute_basis(&self, _degree: i32) {}

    /// Gets the dimension of the algebra in degree `degree`.
    fn dimension(&self, degree: i32, _excess: i32) -> usize {
        if degree == 0 {
            1
        } else {
            0
        }
    }

    fn multiply_basis_elements(
        &self,
        mut result: SliceMut,
        coeff: u32,
        _r: BasisElem<Self>,
        _s: BasisElem<Self>,
    ) {
        result.add_basis_element(0, coeff)
    }

    fn default_filtration_one_products(&self) -> Vec<(String, BasisElem<Self>)> {
        vec![]
    }

    /// Converts a basis element into a string for display.
    fn basis_element_to_string(&self, b: BasisElem<Self>) -> String {
        assert!(b.degree() == 0);
        "1".to_string()
    }

    fn element_to_string(&self, x: impl Into<Elem<Self, Slice>>) -> String {
        let x = x.into()
        assert!(x.degree() == 0);
        format!("{}", x.coeffs().entry(0))
    }
}

impl Bialgebra for Field {
    fn coproduct(&self, _x: BasisElem<Self>) -> Vec<(BasisElem<Self>, BasisElem<Self>)> {
        vec![(BasisElem::new(1, 0), BasisElem::new(1, 0))]
    }
    fn decompose(&self, _x: BasisElem<Self>) -> Vec<BasisElem<Self>> {
        vec![(BasisElem::new(1, 0))]
    }
}
