use crate::module::{Module, ZeroModule};
use bivec::BiVec;
use fp::matrix::Subspace;
use fp::vector::{prelude::*, FpVector, Slice, SliceMut};
use std::sync::Arc;

/// A quotient of a module truncated below a fix degree.
pub struct QuotientModule<M: Module> {
    /// The underlying module
    pub module: Arc<M>,
    /// The subspaces that we quotient out by
    pub subspaces: BiVec<Subspace>,
    /// For each degree `d`, `basis_list[d]` is a list of basis elements of `self.module` that
    /// generates the quotient.
    pub basis_list: BiVec<Vec<usize>>,
    /// Everything above this degree is quotiented out.
    pub truncation: i32,
}

impl<M: Module> std::fmt::Display for QuotientModule<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Quotient of {}", self.module)
    }
}

impl<M: Module> QuotientModule<M> {
    pub fn new(module: Arc<M>, truncation: i32) -> Self {
        module.compute_basis(truncation);

        let p = module.prime();
        let min_degree = module.min_degree();

        let mut subspaces = BiVec::with_capacity(min_degree, truncation + 1);
        let mut basis_list = BiVec::with_capacity(min_degree, truncation + 1);

        for t in min_degree..=truncation {
            let dim = module.dimension(t);
            subspaces.push(Subspace::new(p, dim + 1, dim));
            basis_list.push((0..dim).collect());
        }
        QuotientModule {
            module,
            subspaces,
            basis_list,
            truncation,
        }
    }

    pub fn quotient(&mut self, degree: i32, element: Slice) {
        if degree <= self.truncation {
            self.subspaces[degree].add_vector(element);
            self.flush(degree);
        }
    }

    pub fn quotient_basis_elements(
        &mut self,
        degree: i32,
        elements: impl std::iter::Iterator<Item = usize>,
    ) {
        self.subspaces[degree].add_basis_elements(elements);
        self.flush(degree);
    }

    /// # Arguments
    ///  - `degree`: The degree to quotient in
    ///  - `vecs`: See [`Subspace::add_vectors`]
    pub fn quotient_vectors(
        &mut self,
        degree: i32,
        vecs: impl for<'a> FnMut(SliceMut<'a>) -> Option<()>,
    ) {
        if degree > self.truncation {
            return;
        }
        self.subspaces[degree].add_vectors(vecs);
        self.flush(degree);
    }

    fn flush(&mut self, degree: i32) {
        self.basis_list[degree].clear();
        self.basis_list[degree].extend(
            self.subspaces[degree]
                .pivots()
                .iter()
                .enumerate()
                .filter_map(|(idx, &row)| if row < 0 { Some(idx) } else { None }),
        );
    }

    pub fn quotient_all(&mut self, degree: i32) {
        self.subspaces[degree].set_to_entire();
        self.basis_list[degree] = Vec::new();
    }

    pub fn act_on_original_basis(
        &self,
        mut result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) {
        if op_degree + mod_degree > self.truncation {
            return;
        }
        self.module.act_on_basis(
            result.copy(),
            coeff,
            op_degree,
            op_index,
            mod_degree,
            mod_index,
        );
        self.reduce(op_degree + mod_degree, result)
    }

    pub fn reduce(&self, degree: i32, mut vec: SliceMut) {
        if degree > self.truncation {
            vec.set_to_zero()
        } else {
            self.subspaces[degree].reduce(vec);
        }
    }

    pub fn old_basis_to_new(&self, degree: i32, mut new: SliceMut, old: Slice) {
        for (i, idx) in self.basis_list[degree].iter().enumerate() {
            new.add_basis_element(i, old.entry(*idx));
        }
    }
}

impl<M: Module> Module for QuotientModule<M> {
    type Algebra = M::Algebra;
    fn algebra(&self) -> Arc<Self::Algebra> {
        self.module.algebra()
    }

    fn min_degree(&self) -> i32 {
        self.module.min_degree()
    }

    fn max_computed_degree(&self) -> i32 {
        self.module.max_computed_degree()
    }

    fn dimension(&self, degree: i32) -> usize {
        if degree > self.truncation {
            0
        } else {
            self.module.dimension(degree) - self.subspaces[degree].dimension()
        }
    }

    fn act_on_basis(
        &self,
        result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) {
        let target_deg = op_degree + mod_degree;
        if target_deg > self.truncation {
            return;
        }

        let mut result_ = FpVector::new(self.prime(), self.module.dimension(target_deg));
        self.act_on_original_basis(
            result_.as_slice_mut(),
            coeff,
            op_degree,
            op_index,
            mod_degree,
            self.basis_list[mod_degree][mod_index],
        );
        self.reduce(target_deg, result_.as_slice_mut());
        self.old_basis_to_new(target_deg, result, result_.as_slice());
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        self.module
            .basis_element_to_string(degree, self.basis_list[degree][idx])
    }

    fn max_degree(&self) -> Option<i32> {
        Some(match self.module.max_degree() {
            Some(max_degree) => std::cmp::min(max_degree, self.truncation),
            None => self.truncation,
        })
    }
}

impl<M: ZeroModule> ZeroModule for QuotientModule<M> {
    fn zero_module(algebra: Arc<M::Algebra>, min_degree: i32) -> Self {
        Self::new(Arc::new(M::zero_module(algebra, min_degree)), min_degree)
    }
}
