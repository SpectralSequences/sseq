use itertools::Itertools;
use std::sync::Arc;

use fp::prime::ValidPrime;
use fp::vector::{Slice, SliceMut};

use crate::algebra::Algebra;

pub trait Module: std::fmt::Display + std::any::Any + Send + Sync {
    type Algebra: Algebra;

    fn algebra(&self) -> Arc<Self::Algebra>;
    fn min_degree(&self) -> i32;
    fn compute_basis(&self, _degree: i32) {}
    /// The maximum `t` for which the module is defined at `t`.
    fn max_computed_degree(&self) -> i32;
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

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String;

    /// Whether this is the unit module.
    fn is_unit(&self) -> bool {
        self.min_degree() == 0 && self.max_degree() == Some(0) && self.dimension(0) == 1
    }

    fn prime(&self) -> ValidPrime {
        self.algebra().prime()
    }

    /// `max_degree` is the a degree such that if t > `max_degree`, then `self.dimension(t) = 0`.
    fn max_degree(&self) -> Option<i32> {
        None
    }

    /// Maximum degree of a generator.
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
                (coeff * v) % *p,
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
                (coeff * v) % *p,
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
                (coeff * v) % *p,
                op_degree,
                i,
                input_degree,
                input_index,
            );
        }
    }

    fn element_to_string(&self, degree: i32, element: Slice) -> String {
        let result = element
            .iter_nonzero()
            .map(|(idx, value)| {
                let coeff = if value == 1 {
                    "".to_string()
                } else {
                    format!("{} ", value)
                };
                let basis_elt = self.basis_element_to_string(degree, idx);
                format!("{}{}", coeff, basis_elt)
            })
            .join(" + ");
        if result.is_empty() {
            "0".to_string()
        } else {
            result
        }
    }
}

macro_rules! dispatch {
    () => {};
    ($vis:vis fn $method:ident(&self$(, $arg:ident: $ty:ty )*$(,)?) $(-> $ret:ty)?; $($tail:tt)*) => {
        $vis fn $method(&self, $($arg: $ty),* ) $(-> $ret)* {
            (**self).$method($($arg),*)
        }
        dispatch!{$($tail)*}
    };
}

impl<A: Algebra> Module for Box<dyn Module<Algebra = A>> {
    type Algebra = A;

    dispatch! {
        fn algebra(&self) -> Arc<Self::Algebra>;
        fn min_degree(&self) -> i32;
        fn max_computed_degree(&self) -> i32;
        fn compute_basis(&self, degree: i32);
        fn dimension(&self, degree: i32) -> usize;
        fn basis_element_to_string(&self, degree: i32, idx: usize) -> String;
        fn is_unit(&self) -> bool;
        fn prime(&self) -> ValidPrime;
        fn max_degree(&self) -> Option<i32>;
        fn max_generator_degree(&self) -> Option<i32>;

        fn act_on_basis(
            &self,
            result: SliceMut,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            mod_degree: i32,
            mod_index: usize,
        );
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
