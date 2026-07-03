//! $\mathbb{F}_2[\tau]$, the C-motivic coefficient ring, as a graded algebra.

use fp::{
    prime::{TWO, ValidPrime},
    vector::{FpSlice, FpSliceMut},
};

use crate::algebra::Algebra;

/// $\mathbb{F}_2[\tau]$ — the C-motivic (prime 2) coefficient ring, presented as a connected
/// graded $\mathbb{F}_2$-algebra on the single generator $\tau$.
///
/// It is one-dimensional in every non-negative degree, with basis element $\tau^d$ in degree
/// `d`, and $\tau^a \cdot \tau^b = \tau^{a+b}$. The grading is the $\tau$-power (so
/// $|\tau| = 1$), which is the *negative* of the motivic weight (motivically $|\tau| = (0,-1)$).
///
/// This is the base ring over which the C-motivic Steenrod algebra is a free module; it is
/// intended to become the `BaseRing` of that algebra once the coefficient-ring (`GradedDvr`)
/// scaffolding lands.
pub struct FpTau;

impl std::fmt::Display for FpTau {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "F_2[τ]")
    }
}

impl Algebra for FpTau {
    fn prime(&self) -> ValidPrime {
        TWO
    }

    fn compute_basis(&self, _degree: i32) {}

    fn dimension(&self, degree: i32) -> usize {
        usize::from(degree >= 0)
    }

    fn multiply_basis_elements(
        &self,
        mut result: FpSliceMut,
        coeff: u32,
        r_degree: i32,
        _r_idx: usize,
        s_degree: i32,
        _s_idx: usize,
    ) {
        debug_assert!(r_degree >= 0 && s_degree >= 0);
        // τ^{r_degree} · τ^{s_degree} = τ^{r_degree + s_degree}, the unique basis element of the
        // target degree.
        result.add_basis_element(0, coeff);
    }

    fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
        vec![]
    }

    fn basis_element_to_string(&self, degree: i32, _idx: usize) -> String {
        match degree {
            0 => "1".to_string(),
            1 => "τ".to_string(),
            d => format!("τ^{d}"),
        }
    }

    fn element_to_string(&self, degree: i32, element: FpSlice) -> String {
        format!(
            "{} {}",
            element.entry(0),
            self.basis_element_to_string(degree, 0)
        )
    }

    fn basis_element_from_string(&self, elt: &str) -> Option<(i32, usize)> {
        match elt {
            "1" => Some((0, 0)),
            "τ" | "tau" => Some((1, 0)),
            s => s
                .strip_prefix("τ^")
                .or_else(|| s.strip_prefix("tau^"))
                .and_then(|k| k.parse().ok())
                .map(|k| (k, 0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use fp::vector::FpVector;

    use super::*;

    #[test]
    fn test_fp_tau() {
        let a = FpTau;
        assert_eq!(a.prime(), TWO);
        assert_eq!(a.dimension(0), 1);
        assert_eq!(a.dimension(7), 1);
        assert_eq!(a.dimension(-1), 0);

        // τ^2 · τ^3 = τ^5.
        let mut res = FpVector::new(TWO, 1);
        a.multiply_basis_elements(res.as_slice_mut(), 1, 2, 0, 3, 0);
        assert_eq!(res.entry(0), 1);

        assert_eq!(a.basis_element_to_string(0, 0), "1");
        assert_eq!(a.basis_element_to_string(1, 0), "τ");
        assert_eq!(a.basis_element_to_string(5, 0), "τ^5");
        assert_eq!(a.basis_element_from_string("1"), Some((0, 0)));
        assert_eq!(a.basis_element_from_string("τ"), Some((1, 0)));
        assert_eq!(a.basis_element_from_string("τ^5"), Some((5, 0)));
    }
}
