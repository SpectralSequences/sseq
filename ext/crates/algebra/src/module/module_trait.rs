use std::sync::Arc;

use auto_impl::auto_impl;
use fp::{
    prime::ValidPrime,
    vector::{FpSlice, FpSliceMut},
};
use itertools::Itertools;
use sseq::coordinates::MultiDegree;

use crate::algebra::Algebra;

/// A bounded below module over an algebra.
///
/// To accommodate for infinite modules (e.g. modules in a free resolution), every module is
/// potentially only define up to a degree. The extent to which the module is defined is kept track
/// by two functions:
///
///  - [`Module::max_computed_degree`] gives the maximum degree for which the module is fully
///    defined. It is guaranteed that the module will never change up to this degree in the future.
///
///  - [`Module::compute_basis_multi`] extends the internal data to support querying data up to (and
///    including) a given degree. In general, we can run this beyond the max computed degree.
///
/// A useful example to keep in mind is a [`FreeModule`](crate::module::FreeModule), where we have
/// specified the generators up to some degree `t`. Then `t` is the max computed degree, while
/// `compute_basis` computes data such as the offset of existing generators in potentially higher
/// degrees.
#[auto_impl(Arc, Box)]
pub trait Module<const N: usize = 1>: std::fmt::Display + std::any::Any + Send + Sync {
    type Algebra: Algebra<N>;

    /// The algebra the module is over.
    fn algebra(&self) -> Arc<Self::Algebra>;

    /// The minimum degree of the module, which is required to be bounded below
    fn min_degree(&self) -> i32;

    /// Compute internal data of the module so that we can query information up to degree `degree`.
    /// This should be run by the user whenever they want to query such information.
    ///
    /// This function must be idempotent, and defaults to a no-op.
    ///
    /// See [`Module`] documentation for more details.
    #[allow(unused_variables)]
    fn compute_basis_multi(&self, degree: MultiDegree<N>) {}

    /// The maximum `t` for which the module is fully defined at `t`. See [`Module`] documentation
    /// for more details.
    fn max_computed_degree(&self) -> i32;

    /// The dimension of a module at the given degree
    fn dimension_multi(&self, degree: MultiDegree<N>) -> usize;
    fn act_on_basis_multi(
        &self,
        result: FpSliceMut,
        coeff: u32,
        op_degree: MultiDegree<N>,
        op_index: usize,
        mod_degree: MultiDegree<N>,
        mod_index: usize,
    );

    /// Non-panicking variant of [`Module::act_on_basis_multi`]. Validates the operation and
    /// module degrees/indices and returns an [`ActError`] describing the problem instead of
    /// panicking. On success it delegates to [`Module::act_on_basis_multi`] and returns `Ok(())`.
    fn try_act_on_basis_multi(
        &self,
        result: FpSliceMut,
        coeff: u32,
        op_degree: MultiDegree<N>,
        op_index: usize,
        mod_degree: MultiDegree<N>,
        mod_index: usize,
    ) -> Result<(), ActError> {
        if op_degree.t() < 0 {
            return Err(ActError::IndexOutOfRange(format!(
                "op_degree {op_degree} is negative"
            )));
        }
        self.algebra().compute_basis(op_degree);
        let op_dim = self.algebra().dimension(op_degree);
        if op_index >= op_dim {
            return Err(ActError::IndexOutOfRange(format!(
                "op_index {op_index} out of range for algebra dimension {op_dim} in degree \
                 {op_degree}"
            )));
        }
        let min_degree = self.min_degree();
        if mod_degree.t() < min_degree {
            return Err(ActError::IndexOutOfRange(format!(
                "mod_degree {mod_degree} is below the module's min degree {min_degree}"
            )));
        }
        self.compute_basis_multi(mod_degree);
        let mod_dim = self.dimension_multi(mod_degree);
        if mod_index >= mod_dim {
            return Err(ActError::IndexOutOfRange(format!(
                "mod_index {mod_index} out of range for module dimension {mod_dim} in degree \
                 {mod_degree}"
            )));
        }
        self.act_on_basis_multi(result, coeff, op_degree, op_index, mod_degree, mod_index);
        Ok(())
    }

    /// Non-panicking variant of [`Module::act_multi`]. Validates the operation degree/index and
    /// the input degree/length and returns an [`ActError`] describing the problem instead of
    /// panicking. On success it delegates to [`Module::act_multi`] and returns `Ok(())`.
    fn try_act_multi(
        &self,
        result: FpSliceMut,
        coeff: u32,
        op_degree: MultiDegree<N>,
        op_index: usize,
        input_degree: MultiDegree<N>,
        input: FpSlice,
    ) -> Result<(), ActError> {
        if op_degree.t() < 0 {
            return Err(ActError::IndexOutOfRange(format!(
                "op_degree {op_degree} is negative"
            )));
        }
        self.algebra().compute_basis(op_degree);
        let op_dim = self.algebra().dimension(op_degree);
        if op_index >= op_dim {
            return Err(ActError::IndexOutOfRange(format!(
                "op_index {op_index} out of range for algebra dimension {op_dim} in degree \
                 {op_degree}"
            )));
        }
        let min_degree = self.min_degree();
        if input_degree.t() < min_degree {
            return Err(ActError::IndexOutOfRange(format!(
                "input_degree {input_degree} is below the module's min degree {min_degree}"
            )));
        }
        self.compute_basis_multi(input_degree);
        let input_dim = self.dimension_multi(input_degree);
        if input.len() > input_dim {
            return Err(ActError::InvalidInput(format!(
                "input length {} exceeds module dimension {input_dim} in degree {input_degree}",
                input.len()
            )));
        }
        self.act_multi(result, coeff, op_degree, op_index, input_degree, input);
        Ok(())
    }

    /// The name of a basis element. This is useful for debugging and printing results.
    fn basis_element_to_string_multi(&self, degree: MultiDegree<N>, idx: usize) -> String;

    /// Whether this is the unit module.
    ///
    /// The default answers this from `min_degree`/`max_degree`, which are the (single) `i32`
    /// filtration direction. That cannot characterize the unit when `N > 1` (distinct multidegrees
    /// can share a `t`, so an extra component is invisible here), so the default conservatively
    /// returns `false` for multigraded modules; such modules should override this if they need it.
    fn is_unit(&self) -> bool {
        if N > 1 {
            return false;
        }
        self.min_degree() == 0
            && self.max_degree() == Some(0)
            && self.dimension_multi(MultiDegree::zero()) == 1
    }

    /// The prime the module is over, which should be equal to the prime of the algebra.
    fn prime(&self) -> ValidPrime {
        self.algebra().prime()
    }

    /// `max_degree` is the a degree such that if t > `max_degree`, then `self.dimension_multi(t) = 0`.
    fn max_degree(&self) -> Option<i32> {
        None
    }

    /// Maximum degree of a generator under the Steenrod action. Every element in higher degree
    /// must be obtainable from applying a Steenrod action to a lower degree element.
    fn max_generator_degree(&self) -> Option<i32> {
        self.max_degree()
    }

    fn total_dimension(&self) -> usize
    where
        MultiDegree<N>: From<i32>,
    {
        let max_degree = self
            .max_degree()
            .expect("total_dimension requires module to be bounded");

        (self.min_degree()..=max_degree)
            .map(|i| self.dimension_multi(MultiDegree::from(i)))
            .sum()
    }

    /// The length of `input` need not be equal to the dimension of the module in said degree.
    /// Missing entries are interpreted to be 0, while extra entries must be zero.
    ///
    /// This flexibility is useful when resolving to a stem. The point is that we have elements in
    /// degree `t` that are guaranteed to not contain generators of degree `t`, and we don't know
    /// what generators will be added in degree `t` yet.
    fn act_multi(
        &self,
        mut result: FpSliceMut,
        coeff: u32,
        op_degree: MultiDegree<N>,
        op_index: usize,
        input_degree: MultiDegree<N>,
        input: FpSlice,
    ) {
        assert!(input.len() <= self.dimension_multi(input_degree));
        let p = self.prime();
        for (i, v) in input.iter_nonzero() {
            self.act_on_basis_multi(
                result.copy(),
                (coeff * v) % p,
                op_degree,
                op_index,
                input_degree,
                i,
            );
        }
    }

    fn act_by_element_multi(
        &self,
        mut result: FpSliceMut,
        coeff: u32,
        op_degree: MultiDegree<N>,
        op: FpSlice,
        input_degree: MultiDegree<N>,
        input: FpSlice,
    ) {
        assert_eq!(input.len(), self.dimension_multi(input_degree));
        assert_eq!(op.len(), self.algebra().dimension(op_degree));
        let p = self.prime();
        for (i, v) in op.iter_nonzero() {
            self.act_multi(
                result.copy(),
                (coeff * v) % p,
                op_degree,
                i,
                input_degree,
                input,
            );
        }
    }

    fn act_by_element_on_basis_multi(
        &self,
        mut result: FpSliceMut,
        coeff: u32,
        op_degree: MultiDegree<N>,
        op: FpSlice,
        input_degree: MultiDegree<N>,
        input_index: usize,
    ) {
        assert_eq!(op.len(), self.algebra().dimension(op_degree));
        let p = self.prime();
        for (i, v) in op.iter_nonzero() {
            self.act_on_basis_multi(
                result.copy(),
                (coeff * v) % p,
                op_degree,
                i,
                input_degree,
                input_index,
            );
        }
    }

    /// Gives the name of an element. The default implementation is derived from
    /// [`Module::basis_element_to_string_multi`] in the obvious way.
    fn element_to_string_multi(&self, degree: MultiDegree<N>, element: FpSlice) -> String {
        let result = element
            .iter_nonzero()
            .map(|(idx, value)| {
                let coeff = if value == 1 {
                    "".to_string()
                } else {
                    format!("{value} ")
                };
                let basis_elt = self.basis_element_to_string_multi(degree, idx);
                format!("{coeff}{basis_elt}")
            })
            .join(" + ");
        if result.is_empty() {
            "0".to_string()
        } else {
            result
        }
    }
}

/// Ergonomic, singly-graded-friendly wrappers over [`Module`].
///
/// The object-safe [`Module`] trait takes concrete [`MultiDegree<N>`] degrees, so it stays usable
/// as `dyn Module`. This blanket extension exposes the same operations under their canonical names
/// (`dimension`, `act_on_basis`, …) taking `impl Into<MultiDegree<N>>`, so callers keep passing
/// bare `i32`s in the singly-graded (`N = 1`) world. It is implemented for every module, including
/// `dyn Module` (via `?Sized`), so the ergonomic names are always available.
pub trait ModuleExt<const N: usize>: Module<N> {
    /// See [`Module::compute_basis_multi`].
    fn compute_basis(&self, degree: impl Into<MultiDegree<N>>) {
        self.compute_basis_multi(degree.into())
    }

    /// See [`Module::dimension_multi`].
    fn dimension(&self, degree: impl Into<MultiDegree<N>>) -> usize {
        self.dimension_multi(degree.into())
    }

    /// See [`Module::act_on_basis_multi`].
    #[allow(clippy::too_many_arguments)]
    fn act_on_basis(
        &self,
        result: FpSliceMut,
        coeff: u32,
        op_degree: impl Into<MultiDegree<N>>,
        op_index: usize,
        mod_degree: impl Into<MultiDegree<N>>,
        mod_index: usize,
    ) {
        self.act_on_basis_multi(
            result,
            coeff,
            op_degree.into(),
            op_index,
            mod_degree.into(),
            mod_index,
        )
    }

    /// See [`Module::try_act_on_basis_multi`].
    #[allow(clippy::too_many_arguments)]
    fn try_act_on_basis(
        &self,
        result: FpSliceMut,
        coeff: u32,
        op_degree: impl Into<MultiDegree<N>>,
        op_index: usize,
        mod_degree: impl Into<MultiDegree<N>>,
        mod_index: usize,
    ) -> Result<(), ActError> {
        self.try_act_on_basis_multi(
            result,
            coeff,
            op_degree.into(),
            op_index,
            mod_degree.into(),
            mod_index,
        )
    }

    /// See [`Module::try_act_multi`].
    fn try_act(
        &self,
        result: FpSliceMut,
        coeff: u32,
        op_degree: impl Into<MultiDegree<N>>,
        op_index: usize,
        input_degree: impl Into<MultiDegree<N>>,
        input: FpSlice,
    ) -> Result<(), ActError> {
        self.try_act_multi(
            result,
            coeff,
            op_degree.into(),
            op_index,
            input_degree.into(),
            input,
        )
    }

    /// See [`Module::basis_element_to_string_multi`].
    fn basis_element_to_string(&self, degree: impl Into<MultiDegree<N>>, idx: usize) -> String {
        self.basis_element_to_string_multi(degree.into(), idx)
    }

    /// See [`Module::act_multi`].
    fn act(
        &self,
        result: FpSliceMut,
        coeff: u32,
        op_degree: impl Into<MultiDegree<N>>,
        op_index: usize,
        input_degree: impl Into<MultiDegree<N>>,
        input: FpSlice,
    ) {
        self.act_multi(
            result,
            coeff,
            op_degree.into(),
            op_index,
            input_degree.into(),
            input,
        )
    }

    /// See [`Module::act_by_element_multi`].
    fn act_by_element(
        &self,
        result: FpSliceMut,
        coeff: u32,
        op_degree: impl Into<MultiDegree<N>>,
        op: FpSlice,
        input_degree: impl Into<MultiDegree<N>>,
        input: FpSlice,
    ) {
        self.act_by_element_multi(
            result,
            coeff,
            op_degree.into(),
            op,
            input_degree.into(),
            input,
        )
    }

    /// See [`Module::act_by_element_on_basis_multi`].
    fn act_by_element_on_basis(
        &self,
        result: FpSliceMut,
        coeff: u32,
        op_degree: impl Into<MultiDegree<N>>,
        op: FpSlice,
        input_degree: impl Into<MultiDegree<N>>,
        input_index: usize,
    ) {
        self.act_by_element_on_basis_multi(
            result,
            coeff,
            op_degree.into(),
            op,
            input_degree.into(),
            input_index,
        )
    }

    /// See [`Module::element_to_string_multi`].
    fn element_to_string(&self, degree: impl Into<MultiDegree<N>>, element: FpSlice) -> String {
        self.element_to_string_multi(degree.into(), element)
    }
}

impl<const N: usize, M: Module<N> + ?Sized> ModuleExt<N> for M {}

#[derive(Debug)]
pub struct ModuleFailedRelationError {
    pub relation: String,
    pub value: String,
}

impl std::fmt::Display for ModuleFailedRelationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Relation failed:\n    {}  !=  0\nInstead it is equal to {}\n",
            self.relation, self.value
        )
    }
}

impl std::error::Error for ModuleFailedRelationError {}

/// Error returned by [`Module::try_act_multi`] and [`Module::try_act_on_basis_multi`].
///
/// The variants separate the distinct failure categories so callers (e.g. the
/// Python bindings) can map them to different error types.
#[derive(Debug)]
pub enum ActError {
    /// A degree is negative, or an operation/module index is beyond the dimension in its degree.
    IndexOutOfRange(String),
    /// The input vector is longer than the module dimension in its degree.
    InvalidInput(String),
}

impl std::fmt::Display for ActError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IndexOutOfRange(m) | Self::InvalidInput(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for ActError {}
