use crate::module::homomorphism::ModuleHomomorphism;
use crate::module::TruncatedModule;
use fp::matrix::{QuasiInverse, Subspace};
use fp::vector::SliceMut;
use std::sync::Arc;

pub struct TruncatedHomomorphism<F: ModuleHomomorphism> {
    f: Arc<F>,
    s: Arc<TruncatedModule<F::Source>>,
    t: Arc<TruncatedModule<F::Target>>,
}

pub struct TruncatedHomomorphismSource<F: ModuleHomomorphism> {
    f: Arc<F>,
    s: Arc<TruncatedModule<F::Source>>,
    t: Arc<F::Target>,
}

impl<F: ModuleHomomorphism> TruncatedHomomorphism<F> {
    pub fn new(
        f: Arc<F>,
        s: Arc<TruncatedModule<F::Source>>,
        t: Arc<TruncatedModule<F::Target>>,
    ) -> Self {
        TruncatedHomomorphism { f, s, t }
    }

    fn truncated_degree(&self) -> i32 {
        std::cmp::min(self.s.truncation - self.f.degree_shift(), self.t.truncation)
    }
}

impl<F: ModuleHomomorphism> ModuleHomomorphism for TruncatedHomomorphism<F> {
    type Source = TruncatedModule<F::Source>;
    type Target = TruncatedModule<F::Target>;

    fn source(&self) -> Arc<Self::Source> {
        Arc::clone(&self.s)
    }
    fn target(&self) -> Arc<Self::Target> {
        Arc::clone(&self.t)
    }
    fn degree_shift(&self) -> i32 {
        self.f.degree_shift()
    }

    fn apply_to_basis_element(
        &self,
        result: SliceMut,
        coeff: u32,
        input_degree: i32,
        input_idx: usize,
    ) {
        if input_degree - self.degree_shift() <= self.truncated_degree() {
            self.f
                .apply_to_basis_element(result, coeff, input_degree, input_idx);
        }
    }

    fn image(&self, degree: i32) -> Option<&Subspace> {
        if degree > self.truncated_degree() {
            None
        } else {
            self.f.image(degree)
        }
    }

    fn kernel(&self, degree: i32) -> Option<&Subspace> {
        if degree > self.truncated_degree() {
            None
        } else {
            self.f.kernel(degree)
        }
    }

    fn quasi_inverse(&self, degree: i32) -> Option<&QuasiInverse> {
        if degree > self.truncated_degree() {
            None
        } else {
            self.f.quasi_inverse(degree)
        }
    }

    fn compute_auxiliary_data_through_degree(&self, degree: i32) {
        self.f.compute_auxiliary_data_through_degree(degree);
    }
}

impl<F: ModuleHomomorphism> TruncatedHomomorphismSource<F> {
    pub fn new(f: Arc<F>, s: Arc<TruncatedModule<F::Source>>, t: Arc<F::Target>) -> Self {
        TruncatedHomomorphismSource { f, s, t }
    }

    fn truncated_degree(&self) -> i32 {
        self.s.truncation - self.f.degree_shift()
    }
}

impl<F: ModuleHomomorphism> ModuleHomomorphism for TruncatedHomomorphismSource<F> {
    type Source = TruncatedModule<F::Source>;
    type Target = F::Target;

    fn source(&self) -> Arc<Self::Source> {
        Arc::clone(&self.s)
    }
    fn target(&self) -> Arc<Self::Target> {
        Arc::clone(&self.t)
    }
    fn degree_shift(&self) -> i32 {
        self.f.degree_shift()
    }

    fn apply_to_basis_element(
        &self,
        result: SliceMut,
        coeff: u32,
        input_degree: i32,
        input_idx: usize,
    ) {
        if input_degree <= self.s.truncation {
            self.f
                .apply_to_basis_element(result, coeff, input_degree, input_idx);
        }
    }

    fn image(&self, degree: i32) -> Option<&Subspace> {
        if degree > self.truncated_degree() {
            None
        } else {
            self.f.image(degree)
        }
    }

    fn kernel(&self, degree: i32) -> Option<&Subspace> {
        if degree > self.truncated_degree() {
            None
        } else {
            self.f.kernel(degree)
        }
    }

    fn quasi_inverse(&self, degree: i32) -> Option<&QuasiInverse> {
        if degree > self.truncated_degree() {
            None
        } else {
            self.f.quasi_inverse(degree)
        }
    }

    fn compute_auxiliary_data_through_degree(&self, degree: i32) {
        self.f.compute_auxiliary_data_through_degree(degree);
    }
}
