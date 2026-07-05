//! $\mathbb{F}_2[\tau]$, the coefficient ring of the C-motivic Steenrod algebra.
//!
//! The C-motivic (over $\mathbb{C}$, $p = 2$) Steenrod algebra is linear over the graded polynomial
//! ring $\mathbb{F}_2[\tau]$ rather than the field $\mathbb{F}_2$. [`FpTau`] is that ring, and it
//! plays two roles:
//!
//! * **As an [`Algebra`]** it is $\mathbb{F}_2[\tau]$ itself — a connected graded
//!   $\mathbb{F}_2$-algebra on the single generator $\tau$ (`BaseRing = Field`). It is one-dimensional
//!   in every non-negative degree, with basis element $\tau^d$ in degree `d` and
//!   $\tau^a \cdot \tau^b = \tau^{a+b}$; the grading is the $\tau$-power (so $|\tau| = 1$, the
//!   negative of the motivic weight $|\tau| = (0, -1)$). This is what makes a `Module<Algebra =
//!   FpTau>` a genuine graded $\mathbb{F}_2[\tau]$-module — a graded $\mathbb{F}_2$-vector space with
//!   a degree-raising $\tau$-action — rather than collapsing to $\mathbb{F}_2$.
//!
//! * **As a [`Ring`]** it provides the *coefficient* capacity used when it is the
//!   [`BaseRing`](Algebra::BaseRing) of another algebra (the C-motivic Steenrod algebra). Everything
//!   the engine handles is homogeneous, and $\mathbb{F}_2[\tau]$ is generated in a single degree, so
//!   mod 2 a homogeneous element is either $0$ or exactly $\tau^k$. A scalar is therefore *which power
//!   of $\tau$* — [`TauPower`], an `Option<usize>` where `None` is $0$ and `Some(k)` is $\tau^k$.
//!   Multiplication adds exponents; addition of two same-degree nonzero scalars — necessarily equal —
//!   is $2\tau^k = 0$. The [`Ring::Vector`] representation of a homogeneous element of a free
//!   $\mathbb{F}_2[\tau]$-module stores one [`TauPower`] per coordinate.
//!
//! The graded-DVR *solving* linear algebra (kernels, quasi-inverses) is the
//! [`Solvable`](crate::linear_algebra::Solvable) layer, implemented elsewhere.

use std::sync::Arc;

use fp::{
    prime::{TWO, ValidPrime},
    vector::{FpSlice, FpSliceMut},
};

use crate::{
    algebra::{Algebra, Field, Ring},
    linear_algebra::{BaseSlice, BaseSliceMut},
    module::{FreeModule, Module},
};

/// A homogeneous scalar of $\mathbb{F}_2[\tau]$: `None` is $0$, `Some(k)` is $\tau^k$.
pub type TauPower = Option<usize>;

/// $\mathbb{F}_2[\tau]$, the coefficient ring of the C-motivic Steenrod algebra.
///
/// As an [`Algebra`] it is $\mathbb{F}_2[\tau]$ over $\mathbb{F}_2$ (`BaseRing = Field`), graded by
/// the $\tau$-power; as a [`Ring`] it supplies $\tau$-power scalars ([`TauPower`]) for the algebras
/// built over it. See the [module documentation](self).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FpTau;

impl FpTau {
    /// $\mathbb{F}_2[\tau]$.
    pub fn new() -> Self {
        Self
    }
}

impl Default for FpTau {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for FpTau {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "F_2[τ]")
    }
}

impl Algebra for FpTau {
    type BaseRing = Field;
    type GradedPiece = FreeModule<Field>;

    fn base_ring(&self) -> Field {
        Field::new(TWO)
    }

    fn module_at(&self, t: i32) -> FreeModule<Field> {
        let piece = FreeModule::new(Arc::new(self.base_ring()), String::new(), 0);
        piece.add_generators(0, self.dimension(t), None);
        piece.compute_basis(0);
        piece
    }

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

impl Ring for FpTau {
    type Element = TauPower;
    type Slice<'a> = FpTauSlice<'a>;
    type SliceMut<'a> = FpTauSliceMut<'a>;
    type Vector = FpTauVector;

    fn zero(self) -> TauPower {
        None
    }

    fn one(self) -> TauPower {
        Some(0)
    }

    fn add(self, a: TauPower, b: TauPower) -> TauPower {
        match (a, b) {
            (x, None) => x,
            (None, y) => y,
            // Two nonzero homogeneous elements of the same degree are equal (both $\tau^k$), so their
            // sum is $2\tau^k = 0$. Distinct powers would be inhomogeneous and cannot occur — this is
            // a hard invariant of the (fully homogeneous) engine, so assert it unconditionally.
            (Some(i), Some(j)) => {
                assert_eq!(
                    i, j,
                    "adding inhomogeneous F_2[tau] scalars tau^{i} + tau^{j}"
                );
                None
            }
        }
    }

    fn mul(self, a: TauPower, b: TauPower) -> TauPower {
        match (a, b) {
            (Some(i), Some(j)) => Some(i + j),
            _ => None,
        }
    }

    fn is_zero(self, a: TauPower) -> bool {
        a.is_none()
    }

    fn embed_field(self, c: u32) -> TauPower {
        if c.is_multiple_of(2) { None } else { Some(0) }
    }

    fn new_vector(self, len: usize) -> FpTauVector {
        FpTauVector {
            entries: vec![None; len],
        }
    }

    fn vector_len(self, vector: &FpTauVector) -> usize {
        vector.entries.len()
    }

    fn as_slice(self, vector: &FpTauVector) -> FpTauSlice<'_> {
        FpTauSlice {
            entries: &vector.entries,
        }
    }

    fn as_slice_mut(self, vector: &mut FpTauVector) -> FpTauSliceMut<'_> {
        FpTauSliceMut {
            entries: &mut vector.entries,
        }
    }
}

/// An owned homogeneous vector over $\mathbb{F}_2[\tau]$: one [`TauPower`] per coordinate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpTauVector {
    entries: Vec<TauPower>,
}

impl FpTauVector {
    /// A zero vector of the given length.
    pub fn new(len: usize) -> Self {
        Self {
            entries: vec![None; len],
        }
    }

    /// The coordinatewise $\tau$-powers.
    pub fn entries(&self) -> &[TauPower] {
        &self.entries
    }
}

/// An immutable slice of an [`FpTauVector`].
#[derive(Clone, Copy)]
pub struct FpTauSlice<'a> {
    entries: &'a [TauPower],
}

/// A mutable slice of an [`FpTauVector`].
pub struct FpTauSliceMut<'a> {
    entries: &'a mut [TauPower],
}

impl<'a> BaseSlice<'a, FpTau> for FpTauSlice<'a> {
    fn len(self) -> usize {
        self.entries.len()
    }

    fn entry(self, index: usize) -> TauPower {
        self.entries[index]
    }

    fn is_zero(self) -> bool {
        self.entries.iter().all(Option::is_none)
    }

    fn restrict(self, start: usize, end: usize) -> Self {
        Self {
            entries: &self.entries[start..end],
        }
    }

    fn iter_nonzero(self) -> impl Iterator<Item = (usize, TauPower)> + 'a {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(i, &e)| e.map(|k| (i, Some(k))))
    }
}

impl<'a> BaseSliceMut<'a, FpTau> for FpTauSliceMut<'a> {
    fn reborrow(&mut self) -> FpTauSliceMut<'_> {
        FpTauSliceMut {
            entries: self.entries,
        }
    }

    fn as_slice(&self) -> FpTauSlice<'_> {
        FpTauSlice {
            entries: self.entries,
        }
    }

    fn add_basis_element(&mut self, index: usize, coeff: TauPower) {
        self.entries[index] = FpTau.add(self.entries[index], coeff);
    }

    fn add(&mut self, other: FpTauSlice<'_>, coeff: TauPower) {
        for (i, c) in other.iter_nonzero() {
            self.add_basis_element(i, FpTau.mul(coeff, c));
        }
    }

    fn set_entry(&mut self, index: usize, value: TauPower) {
        self.entries[index] = value;
    }

    fn set_to_zero(&mut self) {
        for e in self.entries.iter_mut() {
            *e = None;
        }
    }

    fn slice_mut(&mut self, start: usize, end: usize) -> FpTauSliceMut<'_> {
        FpTauSliceMut {
            entries: &mut self.entries[start..end],
        }
    }
}

#[cfg(test)]
mod tests {
    use fp::vector::FpVector;

    use super::*;

    #[test]
    fn is_an_f2_algebra() {
        // As an F_2-algebra, F_2[τ] is one-dimensional in each non-negative degree (basis τ^d) with
        // F_2 coefficients.
        let r = FpTau::new();
        assert_eq!(Algebra::dimension(&r, 0), 1);
        assert_eq!(Algebra::dimension(&r, 7), 1);
        assert_eq!(Algebra::dimension(&r, -1), 0);
        // τ^2 · τ^3 = τ^5.
        let mut v = FpVector::new(TWO, 1);
        r.multiply_basis_elements(v.as_slice_mut(), 1, 2, 0, 3, 0);
        assert_eq!(v.entry(0), 1);

        assert_eq!(r.basis_element_to_string(0, 0), "1");
        assert_eq!(r.basis_element_to_string(1, 0), "τ");
        assert_eq!(r.basis_element_to_string(5, 0), "τ^5");
        assert_eq!(r.basis_element_from_string("τ^5"), Some((5, 0)));
    }

    #[test]
    fn ring_scalar_arithmetic() {
        // As a Ring, F_2[τ]'s scalars are τ-powers.
        let r = FpTau::new();
        assert_eq!(r.zero(), None);
        assert_eq!(r.one(), Some(0));
        assert_eq!(r.mul(Some(2), Some(3)), Some(5)); // τ^2 · τ^3 = τ^5
        assert_eq!(r.add(Some(4), Some(4)), None); // τ^k + τ^k = 0
        assert_eq!(r.add(None, Some(4)), Some(4));
        assert!(r.is_zero(None));
        assert_eq!(r.embed_field(0), None);
        assert_eq!(r.embed_field(1), Some(0));
        assert_eq!(r.embed_field(3), Some(0));
    }

    #[test]
    fn ring_vector_axpy_cancels_mod_two() {
        let r = FpTau::new();
        let mut v = r.new_vector(3);
        {
            let mut s = r.as_slice_mut(&mut v);
            s.add_basis_element(0, Some(1)); // τ
            s.add_basis_element(2, Some(0)); // 1
        }
        assert_eq!(r.as_slice(&v).entry(0), Some(1));
        assert_eq!(r.as_slice(&v).entry(2), Some(0));
        // Adding v to itself cancels every coordinate mod 2.
        let snapshot = v.clone();
        {
            let src = r.as_slice(&snapshot);
            r.as_slice_mut(&mut v).add(src, Some(0));
        }
        assert!(r.as_slice(&v).is_zero());
    }
}
