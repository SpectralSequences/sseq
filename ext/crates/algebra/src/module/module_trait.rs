use std::sync::Arc;

use auto_impl::auto_impl;
use fp::{
    prime::ValidPrime,
    vector::{Slice, SliceMut},
};
use itertools::Itertools;

use crate::algebra::Algebra;

/// A bounded below module over an algebra. To accommodate for infinite modules (e.g. modules in a
/// free resolution), every module is potentially only define up to a degree. The extent to which
/// the module is defined is kept track by two functions:
///
///  - [`Module::max_computed_degree`] gives the maximum degree for which the module is fully
///    defined. It is guaranteed that the module will never change up to this degree in the future.
///
///  - [`Module::compute_basis`] extends the internal data to support querying data up to (and
///    including) a given degree. In general, we can run this beyond the max computed degree.
///
/// A useful example to keep in mind is a [`FreeModule`](crate::module::FreeModule), where we have
/// specified the generators up to some degree `t`. Then `t` is the max computed degree, while
/// `compute_basis` computes data such as the offset of existing generators in potentially higher
/// degrees.
#[auto_impl(Box)]
pub trait Module: std::fmt::Display + std::any::Any + Send + Sync {
    type Algebra: Algebra;

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
    fn compute_basis(&self, degree: i32) {}

    /// The maximum `t` for which the module is fully defined at `t`. See [`Module`] documentation
    /// for more details.
    fn max_computed_degree(&self) -> i32;

    /// The dimension of a module at the given degree
    fn dimension(&self, degree: i32) -> usize;
    fn act_on_basis(
        &self,
        result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    );

    /// The name of a basis element. This is useful for debugging and printing results.
    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String;

    /// Whether this is the unit module.
    fn is_unit(&self) -> bool {
        self.min_degree() == 0 && self.max_degree() == Some(0) && self.dimension(0) == 1
    }

    /// The prime the module is over, which should be equal to the prime of the algebra.
    fn prime(&self) -> ValidPrime {
        self.algebra().prime()
    }

    /// `max_degree` is the a degree such that if t > `max_degree`, then `self.dimension(t) = 0`.
    fn max_degree(&self) -> Option<i32> {
        None
    }

    /// Maximum degree of a generator under the Steenrod action. Every element in higher degree
    /// must be obtainable from applying a Steenrod action to a lower degree element.
    fn max_generator_degree(&self) -> Option<i32> {
        self.max_degree()
    }

    fn total_dimension(&self) -> usize {
        let max_degree = self
            .max_degree()
            .expect("total_dimension requires module to be bounded");

        (self.min_degree()..=max_degree)
            .map(|i| self.dimension(i))
            .sum()
    }

    /// The length of `input` need not be equal to the dimension of the module in said degree.
    /// Missing entries are interpreted to be 0, while extra entries must be zero.
    ///
    /// This flexibility is useful when resolving to a stem. The point is that we have elements in
    /// degree `t` that are guaranteed to not contain generators of degree `t`, and we don't know
    /// what generators will be added in degree `t` yet.
    fn act(
        &self,
        mut result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        input_degree: i32,
        input: Slice,
    ) {
        assert!(input.len() <= self.dimension(input_degree));
        let p = self.prime();
        for (i, v) in input.iter_nonzero() {
            self.act_on_basis(
                result.copy(),
                (coeff * v) % p,
                op_degree,
                op_index,
                input_degree,
                i,
            );
        }
    }

    fn act_by_element(
        &self,
        mut result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op: Slice,
        input_degree: i32,
        input: Slice,
    ) {
        assert_eq!(input.len(), self.dimension(input_degree));
        assert_eq!(op.len(), self.algebra().dimension(op_degree));
        let p = self.prime();
        for (i, v) in op.iter_nonzero() {
            self.act(
                result.copy(),
                (coeff * v) % p,
                op_degree,
                i,
                input_degree,
                input,
            );
        }
    }

    fn act_by_element_on_basis(
        &self,
        mut result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op: Slice,
        input_degree: i32,
        input_index: usize,
    ) {
        assert_eq!(op.len(), self.algebra().dimension(op_degree));
        let p = self.prime();
        for (i, v) in op.iter_nonzero() {
            self.act_on_basis(
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
    /// [`Module::basis_element_to_string`] in the obvious way.
    fn element_to_string(&self, degree: i32, element: Slice) -> String {
        let result = element
            .iter_nonzero()
            .map(|(idx, value)| {
                let coeff = if value == 1 {
                    "".to_string()
                } else {
                    format!("{value} ")
                };
                let basis_elt = self.basis_element_to_string(degree, idx);
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
            &self.relation, &self.value
        )
    }
}

impl std::error::Error for ModuleFailedRelationError {}
