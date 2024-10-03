use std::sync::Arc;

use algebra::module::Module;
use double::DoubleChainComplex;
use ext::{
    chain_complex::{AugmentedChainComplex, BoundedChainComplex, ChainComplex, FreeChainComplex},
    resolution_homomorphism::ResolutionHomomorphism,
    utils,
};
use fp::vector::FpVector;
use itertools::Itertools;
use sseq::coordinates::{Bidegree, BidegreeGenerator};

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging();

    let res = utils::query_module(None, true)?;
    assert!(
        res.prime() == 2 && res.target().max_s() == 1 && res.target().module(0).is_unit(),
        "Sq^0 can only be computed for the sphere at the prime 2"
    );

    let res = Arc::new(res);
    let doubled = Arc::new(DoubleChainComplex::new(Arc::clone(&res)));
    doubled.compute_through_bidegree(Bidegree::s_t(res.next_homological_degree() - 1, 0));

    let hom = ResolutionHomomorphism::new(
        String::from("Sq^0"),
        Arc::clone(&res),
        doubled,
        Bidegree::zero(),
    );
    hom.extend_step_raw(
        Bidegree::zero(),
        Some(vec![FpVector::from_slice(res.prime(), &[1])]),
    );
    hom.extend_all();

    for b in res.iter_nonzero_stem() {
        let doubled_b = Bidegree::s_t(b.s(), 2 * b.t());
        if !res.has_computed_bidegree(doubled_b) {
            continue;
        }

        let source_num_gens = res.number_of_gens_in_bidegree(doubled_b);
        let module = res.module(b.s());
        let offset = module.generator_offset(b.t(), b.t(), 0);
        let map = hom.get_map(b.s());

        for i in 0..res.number_of_gens_in_bidegree(b) {
            let gen = BidegreeGenerator::new(b, i);
            println!(
                "Sq^0 x_{gen} = [{}]",
                (0..source_num_gens)
                    .map(|j| map.output(doubled_b.t(), j).entry(offset + i))
                    .format(", ")
            )
        }
    }
    Ok(())
}

mod double {
    use double_algebra::DoubleAlgebra;
    pub use double_chain_complex::DoubleChainComplex;
    use double_module::{DoubleModule, DoubleModuleHomomorphism};
    use sseq::coordinates::Bidegree;

    /// Divide by 2 and round towards -infty
    fn div_floor(x: i32) -> i32 {
        ((x as u32) / 2) as i32
    }

    fn div_bidegree(b: Bidegree) -> Bidegree {
        Bidegree::s_t(b.s(), div_floor(b.t()))
    }

    mod double_algebra {
        use algebra::{
            adem_algebra::AdemBasisElement, milnor_algebra::MilnorBasisElement, AdemAlgebra,
            Algebra, MilnorAlgebra, SteenrodAlgebra,
        };

        pub trait DoubleAlgebra: Algebra {
            /// `degree` is guaranteed to be even
            fn halve(&self, degree: i32, idx: usize) -> Option<usize>;
        }

        impl DoubleAlgebra for MilnorAlgebra {
            fn halve(&self, degree: i32, idx: usize) -> Option<usize> {
                let elt = self.basis_element_from_index(degree, idx);
                let p_part = elt
                    .p_part
                    .iter()
                    .map(|&x| if x % 2 == 0 { Some(x / 2) } else { None })
                    .collect::<Option<Vec<_>>>()?;
                Some(self.basis_element_to_index(&MilnorBasisElement {
                    degree: degree / 2,
                    p_part,
                    q_part: 0,
                }))
            }
        }

        impl DoubleAlgebra for AdemAlgebra {
            fn halve(&self, degree: i32, idx: usize) -> Option<usize> {
                let elt = self.basis_element_from_index(degree, idx);
                let ps = elt
                    .ps
                    .iter()
                    .map(|&x| if x % 2 == 0 { Some(x / 2) } else { None })
                    .collect::<Option<Vec<_>>>()?;
                Some(self.basis_element_to_index(&AdemBasisElement {
                    degree: degree / 2,
                    ps,
                    bocksteins: 0,
                    p_or_sq: false,
                }))
            }
        }

        impl DoubleAlgebra for SteenrodAlgebra {
            fn halve(&self, degree: i32, idx: usize) -> Option<usize> {
                match self {
                    SteenrodAlgebra::AdemAlgebra(a) => a.halve(degree, idx),
                    SteenrodAlgebra::MilnorAlgebra(a) => a.halve(degree, idx),
                }
            }
        }
    }

    pub mod double_module {
        use std::sync::Arc;

        use algebra::module::{homomorphism::ModuleHomomorphism, Module};
        use fp::{
            matrix::{Matrix, MatrixSliceMut, QuasiInverse, Subspace},
            vector::{FpSlice, FpSliceMut},
        };

        use super::DoubleAlgebra;

        pub struct DoubleModule<M: Module> {
            inner: Arc<M>,
        }

        impl<M: Module> std::fmt::Display for DoubleModule<M> {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "Double({}", &*self.inner)
            }
        }

        impl<M: Module> DoubleModule<M>
        where
            M::Algebra: DoubleAlgebra,
        {
            pub fn new(inner: Arc<M>) -> Self {
                Self { inner }
            }
        }

        impl<M: Module> Module for DoubleModule<M>
        where
            M::Algebra: DoubleAlgebra,
        {
            type Algebra = M::Algebra;

            fn algebra(&self) -> Arc<Self::Algebra> {
                self.inner.algebra()
            }

            fn min_degree(&self) -> i32 {
                self.inner.min_degree() * 2
            }

            fn max_computed_degree(&self) -> i32 {
                self.inner.max_computed_degree() * 2
            }

            fn dimension(&self, degree: i32) -> usize {
                if degree % 2 == 0 {
                    self.inner.dimension(degree / 2)
                } else {
                    0
                }
            }

            fn act_on_basis(
                &self,
                result: fp::vector::FpSliceMut,
                coeff: u32,
                op_degree: i32,
                op_index: usize,
                mod_degree: i32,
                mod_index: usize,
            ) {
                if op_degree % 2 == 1 {
                    return;
                }
                if let Some(op_index) = self.algebra().halve(op_degree, op_index) {
                    self.inner.act_on_basis(
                        result,
                        coeff,
                        op_degree / 2,
                        op_index,
                        mod_degree / 2,
                        mod_index,
                    );
                }
            }

            fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
                self.inner.basis_element_to_string(degree / 2, idx)
            }

            /// Whether this is the unit module.
            fn is_unit(&self) -> bool {
                self.inner.is_unit()
            }

            /// `max_degree` is the a degree such that if t > `max_degree`, then `self.dimension(t) = 0`.
            fn max_degree(&self) -> Option<i32> {
                self.inner.max_degree().map(|x| x * 2)
            }

            /// Maximum degree of a generator under the Steenrod action. Every element in higher degree
            /// must be obtainable from applying a Steenrod action to a lower degree element.
            fn max_generator_degree(&self) -> Option<i32> {
                self.inner.max_generator_degree().map(|x| x * 2)
            }

            fn total_dimension(&self) -> usize {
                self.inner.total_dimension()
            }

            /// The length of `input` need not be equal to the dimension of the module in said degree.
            /// Missing entries are interpreted to be 0, while extra entries must be zero.
            ///
            /// This flexibility is useful when resolving to a stem. The point is that we have elements in
            /// degree `t` that are guaranteed to not contain generators of degree `t`, and we don't know
            /// what generators will be added in degree `t` yet.
            fn act(
                &self,
                result: FpSliceMut,
                coeff: u32,
                op_degree: i32,
                op_index: usize,
                input_degree: i32,
                input: FpSlice,
            ) {
                if op_degree % 2 == 1 {
                    return;
                }
                if let Some(op_index) = self.algebra().halve(op_degree, op_index) {
                    self.inner.act(
                        result,
                        coeff,
                        op_degree / 2,
                        op_index,
                        input_degree / 2,
                        input,
                    );
                }
            }

            /// Gives the name of an element. The default implementation is derived from
            /// [`Module::basis_element_to_string`] in the obvious way.
            fn element_to_string(&self, degree: i32, element: FpSlice) -> String {
                self.inner.element_to_string(degree, element)
            }
        }

        pub struct DoubleModuleHomomorphism<F: ModuleHomomorphism> {
            source: Arc<DoubleModule<F::Source>>,
            target: Arc<DoubleModule<F::Target>>,
            inner: Arc<F>,
            trivial_subspace: Subspace,
            trivial_qi: QuasiInverse,
        }

        impl<F: ModuleHomomorphism> DoubleModuleHomomorphism<F>
        where
            <F::Source as Module>::Algebra: DoubleAlgebra,
        {
            pub fn new(
                source: Arc<DoubleModule<F::Source>>,
                target: Arc<DoubleModule<F::Target>>,
                inner: Arc<F>,
            ) -> Self {
                Self {
                    trivial_subspace: Subspace::new(source.prime(), 0),
                    trivial_qi: QuasiInverse::new(None, Matrix::new(source.prime(), 0, 0)),
                    source,
                    target,
                    inner,
                }
            }
        }

        impl<F: ModuleHomomorphism> ModuleHomomorphism for DoubleModuleHomomorphism<F>
        where
            <F::Source as Module>::Algebra: DoubleAlgebra,
        {
            type Source = DoubleModule<F::Source>;
            type Target = DoubleModule<F::Target>;

            fn source(&self) -> Arc<Self::Source> {
                Arc::clone(&self.source)
            }

            fn target(&self) -> Arc<Self::Target> {
                Arc::clone(&self.target)
            }

            fn degree_shift(&self) -> i32 {
                self.inner.degree_shift() * 2
            }

            fn apply_to_basis_element(
                &self,
                result: FpSliceMut,
                coeff: u32,
                input_degree: i32,
                input_idx: usize,
            ) {
                self.inner
                    .apply_to_basis_element(result, coeff, input_degree / 2, input_idx)
            }

            fn apply(&self, result: FpSliceMut, coeff: u32, input_degree: i32, input: FpSlice) {
                if input_degree % 2 == 0 {
                    self.inner.apply(result, coeff, input_degree / 2, input)
                }
            }

            fn kernel(&self, degree: i32) -> Option<&Subspace> {
                if degree % 2 == 0 {
                    self.inner.kernel(degree / 2)
                } else {
                    Some(&self.trivial_subspace)
                }
            }

            fn quasi_inverse(&self, degree: i32) -> Option<&QuasiInverse> {
                if degree % 2 == 0 {
                    self.inner.quasi_inverse(degree / 2)
                } else {
                    Some(&self.trivial_qi)
                }
            }

            fn image(&self, degree: i32) -> Option<&Subspace> {
                if degree % 2 == 0 {
                    self.inner.image(degree / 2)
                } else {
                    Some(&self.trivial_subspace)
                }
            }

            fn compute_auxiliary_data_through_degree(&self, degree: i32) {
                // trick to round towards -infty
                self.inner
                    .compute_auxiliary_data_through_degree(super::div_floor(degree))
            }

            fn get_matrix(&self, matrix: MatrixSliceMut, degree: i32) {
                if degree % 2 == 0 {
                    self.inner.get_matrix(matrix, degree / 2)
                }
            }

            /// Get the values of the homomorphism on the specified inputs to `matrix`.
            fn get_partial_matrix(&self, degree: i32, inputs: &[usize]) -> Matrix {
                if degree % 2 == 0 {
                    self.inner.get_partial_matrix(degree / 2, inputs)
                } else {
                    Matrix::new(self.prime(), 0, 0)
                }
            }

            /// Attempt to apply quasi inverse to the input. Returns whether the operation was
            /// successful. This is required to either always succeed or always fail for each degree.
            #[must_use]
            fn apply_quasi_inverse(&self, result: FpSliceMut, degree: i32, input: FpSlice) -> bool {
                if degree % 2 == 0 {
                    self.inner.apply_quasi_inverse(result, degree / 2, input)
                } else {
                    true
                }
            }
        }
    }

    mod double_chain_complex {
        use std::sync::Arc;

        use ext::chain_complex::ChainComplex;
        use once::OnceVec;
        use sseq::coordinates::Bidegree;

        use super::{DoubleAlgebra, DoubleModule, DoubleModuleHomomorphism};

        pub struct DoubleChainComplex<CC: ChainComplex> {
            inner: Arc<CC>,
            zero_module: Arc<DoubleModule<CC::Module>>,
            modules: OnceVec<Arc<DoubleModule<CC::Module>>>,
            differentials: OnceVec<Arc<DoubleModuleHomomorphism<CC::Homomorphism>>>,
        }

        impl<CC: ChainComplex> DoubleChainComplex<CC>
        where
            CC::Algebra: DoubleAlgebra,
        {
            pub fn new(inner: Arc<CC>) -> Self {
                Self {
                    zero_module: Arc::new(DoubleModule::new(inner.zero_module())),
                    inner,
                    modules: OnceVec::new(),
                    differentials: OnceVec::new(),
                }
            }
        }

        impl<CC: ChainComplex> ChainComplex for DoubleChainComplex<CC>
        where
            CC::Algebra: DoubleAlgebra,
        {
            type Algebra = CC::Algebra;
            type Homomorphism = DoubleModuleHomomorphism<CC::Homomorphism>;
            type Module = DoubleModule<CC::Module>;

            fn algebra(&self) -> Arc<Self::Algebra> {
                self.inner.algebra()
            }

            fn min_degree(&self) -> i32 {
                self.inner.min_degree() * 2
            }

            fn zero_module(&self) -> Arc<Self::Module> {
                Arc::clone(&self.zero_module)
            }

            fn module(&self, s: u32) -> Arc<Self::Module> {
                Arc::clone(&self.modules[s])
            }

            fn differential(&self, s: u32) -> Arc<Self::Homomorphism> {
                Arc::clone(&self.differentials[s])
            }

            fn has_computed_bidegree(&self, b: Bidegree) -> bool {
                self.inner.has_computed_bidegree(super::div_bidegree(b))
            }

            fn compute_through_bidegree(&self, b: Bidegree) {
                self.inner.compute_through_bidegree(super::div_bidegree(b));
                self.modules.extend(b.s() as usize, |s| {
                    Arc::new(DoubleModule::new(self.inner.module(s as u32)))
                });
                self.differentials.extend(b.s() as usize, |s| {
                    let s = s as u32;
                    Arc::new(DoubleModuleHomomorphism::new(
                        self.module(s),
                        if s == 0 {
                            self.zero_module()
                        } else {
                            self.module(s - 1)
                        },
                        self.inner.differential(s),
                    ))
                });
            }

            fn apply_quasi_inverse<T, S>(
                &self,
                results: &mut [T],
                b: Bidegree,
                inputs: &[S],
            ) -> bool
            where
                for<'a> &'a mut T: Into<fp::vector::FpSliceMut<'a>>,
                for<'a> &'a S: Into<fp::vector::FpSlice<'a>>,
            {
                if b.t() % 2 == 0 {
                    let halved_b = Bidegree::s_t(b.s(), b.t() / 2);
                    self.inner.apply_quasi_inverse(results, halved_b, inputs)
                } else {
                    true
                }
            }

            fn next_homological_degree(&self) -> u32 {
                self.inner.next_homological_degree()
            }
        }
    }
}
