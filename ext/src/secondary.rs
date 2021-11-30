use crate::chain_complex::{
    AugmentedChainComplex, BoundedChainComplex, ChainComplex, FreeChainComplex,
};
use crate::resolution_homomorphism::ResolutionHomomorphism;
use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::{BoundedModule, FreeModule, Module};
use algebra::pair_algebra::PairAlgebra;
use bivec::BiVec;
use fp::vector::{FpVector, Slice, SliceMut};
use once::OnceBiVec;
use std::sync::Arc;

use crate::resolution::Resolution as Resolution_;
use crate::CCC;
type Resolution = Resolution_<CCC>;

/// A homotopy of a map A -> M of pair modules. We assume this map does not hit generators.
pub struct SingleSecondaryHomotopy<A: PairAlgebra> {
    target: Arc<FreeModule<A>>,
    degree: i32,
    /// The component of the map on the R_B portion
    composite: BiVec<Vec<A::Element>>,
    /// The component of the map on the A portion
    pub homotopy: FpVector,
}

impl<A: PairAlgebra> SingleSecondaryHomotopy<A> {
    pub fn algebra(&self) -> Arc<A> {
        self.target.algebra()
    }

    pub fn new(target: Arc<FreeModule<A>>, degree: i32) -> Self {
        let algebra = target.algebra();
        let min_degree = target.min_degree();

        let mut composite = BiVec::with_capacity(min_degree, degree);

        for t_ in min_degree..degree {
            let num_gens = target.number_of_gens_in_degree(t_);
            let mut c = Vec::with_capacity(num_gens);
            c.resize_with(num_gens, || algebra.new_pair_element(degree - t_));
            composite.push(c);
        }

        let homotopy = FpVector::new(target.prime(), target.dimension(degree - 1));

        Self {
            target,
            degree,
            composite,
            homotopy,
        }
    }

    pub fn add_composite(
        &mut self,
        coeff: u32,
        gen_degree: i32,
        gen_idx: usize,
        d1: &FreeModuleHomomorphism<FreeModule<A>>,
        d0: &FreeModuleHomomorphism<FreeModule<A>>,
    ) {
        assert!(Arc::ptr_eq(&d1.target(), &d0.source()));
        assert!(Arc::ptr_eq(&d0.target(), &self.target));

        let middle = d1.target();
        let dx = d1.output(gen_degree, gen_idx);
        let algebra = self.algebra();

        for (gen_deg1, gen_idx1, op_deg1, slice1) in
            middle.iter_slices(gen_degree - d1.degree_shift(), dx.as_slice())
        {
            if slice1.is_zero() {
                continue;
            }
            if gen_deg1 < d0.degree_shift() {
                continue;
            }
            let dy = d0.output(gen_deg1, gen_idx1);

            for (gen_deg2, gen_idx2, op_deg2, slice2) in self
                .target
                .iter_slices(gen_deg1 - d0.degree_shift(), dy.as_slice())
            {
                if slice2.is_zero() {
                    continue;
                }
                algebra.sigma_multiply(
                    &mut self.composite[gen_deg2][gen_idx2],
                    coeff,
                    op_deg1,
                    slice1,
                    op_deg2,
                    slice2,
                )
            }
        }
    }

    pub fn act(&self, mut result: SliceMut, coeff: u32, op_degree: i32, op: Slice) {
        let algebra = self.algebra();

        if self.degree > self.target.min_degree() {
            self.target.act_by_element(
                result.copy(),
                coeff,
                op_degree,
                op,
                self.degree - 1,
                self.homotopy.as_slice(),
            );
        }
        for (gen_deg, row) in self.composite.iter_enum() {
            let module_op_deg = self.degree - gen_deg;
            for (gen_idx, c) in row.iter().enumerate() {
                let offset =
                    self.target
                        .generator_offset(self.degree + op_degree - 1, gen_deg, gen_idx);
                let len = algebra.dimension(module_op_deg + op_degree - 1, 0);

                algebra.a_multiply(
                    result.slice_mut(offset, offset + len),
                    coeff,
                    op_degree,
                    op,
                    module_op_deg,
                    c,
                );
            }
        }
    }
}

pub struct SecondaryHomotopy<A: PairAlgebra> {
    pub source: Arc<FreeModule<A>>,
    pub target: Arc<FreeModule<A>>,
    /// output_t = input_t - shift_t
    pub shift_t: i32,

    /// gen_deg -> gen_idx -> homotopy
    pub(crate) homotopies: OnceBiVec<Vec<SingleSecondaryHomotopy<A>>>,
}

impl<A: PairAlgebra> SecondaryHomotopy<A> {
    pub fn new(source: Arc<FreeModule<A>>, target: Arc<FreeModule<A>>, shift_t: i32) -> Self {
        Self {
            homotopies: OnceBiVec::new(std::cmp::max(
                source.min_degree(),
                target.min_degree() + shift_t,
            )),
            source,
            target,
            shift_t,
        }
    }

    pub fn min_degree(&self) -> i32 {
        self.homotopies.min_degree()
    }

    pub fn max_degree(&self) -> i32 {
        self.homotopies.max_degree()
    }

    pub fn initialize(&self, degree: i32) {
        self.homotopies.extend(degree, |t| {
            let num_gens = self.source.number_of_gens_in_degree(t);
            let mut v = Vec::with_capacity(num_gens);
            v.resize_with(num_gens, || {
                SingleSecondaryHomotopy::new(Arc::clone(&self.target), t - self.shift_t)
            });
            v
        })
    }

    pub fn add_composite(
        &mut self,
        coeff: u32,
        gen_degree: i32,
        d1: &FreeModuleHomomorphism<FreeModule<A>>,
        d0: &FreeModuleHomomorphism<FreeModule<A>>,
    ) {
        assert!(Arc::ptr_eq(&d1.target(), &d0.source()));
        assert!(Arc::ptr_eq(&d0.target(), &self.target));

        for gen_idx in 0..self.source.number_of_gens_in_degree(gen_degree) {
            self.homotopies[gen_degree][gen_idx].add_composite(coeff, gen_degree, gen_idx, d1, d0);
        }
    }

    /// Compute the image of an element in the source under the homotopy, writing the result in
    /// `result`. It is assumed that the coefficients of generators are zero in `op`
    pub fn act(&self, mut result: SliceMut, coeff: u32, elt_degree: i32, elt: Slice) {
        for (gen_deg, gen_idx, op_deg, slice) in self.source.iter_slices(elt_degree, elt) {
            if gen_deg < self.homotopies.min_degree() {
                continue;
            }
            // This is actually necessary. We don't have the homotopies on the
            // generators at the edge of the resolution, but we don't need them since they never
            // get hit.
            if slice.is_zero() {
                continue;
            }
            self.homotopies[gen_deg][gen_idx].act(result.copy(), coeff, op_deg, slice);
        }
    }

    pub fn output(&self, gen_deg: i32, gen_idx: usize) -> &SingleSecondaryHomotopy<A> {
        &self.homotopies[gen_deg][gen_idx]
    }

    pub fn output_mut(&mut self, gen_deg: i32, gen_idx: usize) -> &mut SingleSecondaryHomotopy<A> {
        &mut self.homotopies[gen_deg][gen_idx]
    }

    /// Apply Hom(-, k) to the A part of the homotopy. Degree is the degree of the source after
    /// dualizing (i.e. if the original map is M -> N, then this is the degree in N).
    pub fn hom_k(&self, t: i32) -> Vec<Vec<u32>> {
        let source_dim = self.source.number_of_gens_in_degree(t + self.shift_t + 1);
        let target_dim = self.target.number_of_gens_in_degree(t);
        if target_dim == 0 {
            return vec![];
        }
        let mut result = vec![vec![0; source_dim]; target_dim];

        let offset = self.target.generator_offset(t, t, 0);
        for i in 0..source_dim {
            let output = self.output(t + self.shift_t + 1, i);
            #[allow(clippy::needless_range_loop)]
            for j in 0..target_dim {
                result[j][i] = output.homotopy.entry(offset + j);
            }
        }
        result
    }
}

pub struct SecondaryLift<A: PairAlgebra, CC: FreeChainComplex<Algebra = A>> {
    pub chain_complex: Arc<CC>,
    /// s -> t -> idx -> homotopy
    pub(crate) homotopies: OnceBiVec<SecondaryHomotopy<A>>,
}

impl<A: PairAlgebra, CC: FreeChainComplex<Algebra = A>> SecondaryLift<A, CC> {
    pub fn new(cc: Arc<CC>) -> Self {
        Self {
            chain_complex: cc,
            homotopies: OnceBiVec::new(2),
        }
    }
    pub fn algebra(&self) -> Arc<A> {
        self.chain_complex.algebra()
    }

    pub fn initialize_homotopies(&self) {
        let max_s = self.chain_complex.next_homological_degree();

        if max_s < 3 {
            return;
        }
        let max_t = |s| {
            std::cmp::min(
                self.chain_complex.module(s).max_computed_degree(),
                self.chain_complex.module(s - 2).max_computed_degree() + 1,
            )
        };

        self.homotopies.extend(max_s as i32 - 1, |s| {
            let s = s as u32;
            let h = SecondaryHomotopy::new(
                self.chain_complex.module(s),
                self.chain_complex.module(s - 2),
                0,
            );
            h.initialize(max_t(s));
            h
        });
    }

    pub fn compute_composites(&mut self) {
        for s in self.homotopies.range() {
            let d1 = &*self.chain_complex.differential(s as u32);
            let d0 = &*self.chain_complex.differential(s as u32 - 1);
            for t in self.homotopies[s].min_degree()..=self.homotopies[s].max_degree() {
                self.homotopies[s].add_composite(1, t, d1, d0);
            }
        }
    }

    pub fn compute_homotopies(&mut self) {
        let mut scratch = FpVector::new(self.chain_complex.prime(), 0);
        let min_degree = self.chain_complex.min_degree();

        for s in 3..self.homotopies.len() as i32 {
            let source = self.chain_complex.module(s as u32);
            let target = self.chain_complex.module(s as u32 - 3);

            for t in min_degree..=self.homotopies[s].max_degree() {
                let num_gens = source.number_of_gens_in_degree(t);
                for idx in 0..num_gens {
                    scratch.set_scratch_vector_size(target.dimension(t - 1));
                    let d = self.chain_complex.differential(s as u32);
                    self.homotopies[s - 1].act(
                        scratch.as_slice_mut(),
                        1,
                        t,
                        d.output(t, idx).as_slice(),
                    );

                    self.chain_complex
                        .differential(s as u32 - 2)
                        .apply_quasi_inverse(
                            self.homotopies[s]
                                .output_mut(t, idx)
                                .homotopy
                                .as_slice_mut(),
                            t - 1,
                            scratch.as_slice(),
                        );
                }
            }
        }
    }

    pub fn homotopy(&self, s: u32) -> &SecondaryHomotopy<A> {
        &self.homotopies[s as i32]
    }
}

#[doc(hidden)]
pub struct SecondaryResolutionHomomorphism<
    A: PairAlgebra,
    CC1: FreeChainComplex<Algebra = A>,
    CC2: FreeChainComplex<Algebra = A> + AugmentedChainComplex,
> {
    source: Arc<SecondaryLift<A, CC1>>,
    target: Arc<SecondaryLift<A, CC2>>,
    underlying: Arc<ResolutionHomomorphism<CC1, CC2>>,
    /// input s -> homotopy
    homotopies: OnceBiVec<SecondaryHomotopy<A>>,
}

impl<
        A: PairAlgebra,
        CC1: FreeChainComplex<Algebra = A>,
        CC2: FreeChainComplex<Algebra = A> + AugmentedChainComplex,
    > SecondaryResolutionHomomorphism<A, CC1, CC2>
{
    pub fn new(
        source: Arc<SecondaryLift<A, CC1>>,
        target: Arc<SecondaryLift<A, CC2>>,
        underlying: Arc<ResolutionHomomorphism<CC1, CC2>>,
    ) -> Self {
        Self {
            source,
            target,
            homotopies: OnceBiVec::new(underlying.shift_s as i32 + 1),
            underlying,
        }
    }

    pub fn shift_s(&self) -> u32 {
        self.underlying.shift_s
    }

    pub fn shift_t(&self) -> i32 {
        self.underlying.shift_t
    }

    pub fn initialize_homotopies(&self) {
        let shift_s = self.shift_s();
        let shift_t = self.shift_t();

        let max_s = self.underlying.next_homological_degree();

        let max_t = |s| {
            std::cmp::min(
                self.underlying.get_map(s).next_degree() - 1,
                std::cmp::min(
                    self.source.homotopies[s as i32].homotopies.max_degree(),
                    if s == shift_s + 1 {
                        i32::MAX
                    } else {
                        self.target.homotopies[(s - shift_s) as i32]
                            .homotopies
                            .max_degree()
                            + shift_t
                    },
                ),
            )
        };

        self.homotopies.extend(max_s as i32 - 1, |s| {
            let s = s as u32;
            let h = SecondaryHomotopy::new(
                self.source.chain_complex.module(s),
                self.target.chain_complex.module(s - shift_s - 1),
                self.shift_t(),
            );
            h.initialize(max_t(s));
            h
        });
    }

    pub fn compute_composites(&mut self) {
        let range = self.homotopies.range();
        let shift_s = self.shift_s();
        for s in range.start..range.end - 1 {
            let d_source = &*self.source.chain_complex.differential(s as u32);
            let d_target = &*self.target.chain_complex.differential(s as u32 - shift_s);

            let c1 = &*self.underlying.get_map(s as u32);
            let c0 = &*self.underlying.get_map(s as u32 - 1);

            for t in self.homotopies[s].min_degree()..=self.homotopies[s].max_degree() {
                self.homotopies[s].add_composite(1, t, d_source, c0);
                self.homotopies[s].add_composite(3, t, c1, d_target);
            }
        }
    }

    pub fn compute_homotopies(&mut self) {
        let range = self.homotopies.range();
        let mut scratch = FpVector::new(self.source.chain_complex.prime(), 0);

        let shift_t = self.shift_t();
        let shift_s = self.shift_s();

        for s in range.start as u32 + 1..range.end as u32 {
            let source = self.source.chain_complex.module(s);
            let target = self.target.chain_complex.module(s - shift_s - 2);

            for t in
                self.homotopies[s as i32].min_degree() + 1..=self.homotopies[s as i32].max_degree()
            {
                let num_gens = source.number_of_gens_in_degree(t);
                for idx in 0..num_gens {
                    scratch.set_scratch_vector_size(target.dimension(t - 1 - shift_t));
                    let d = self.source.chain_complex.differential(s);
                    self.homotopies[s as i32 - 1].act(
                        scratch.as_slice_mut(),
                        1,
                        t,
                        d.output(t, idx).as_slice(),
                    );
                    self.target.homotopy(s - shift_s).act(
                        scratch.as_slice_mut(),
                        1,
                        t - shift_t,
                        self.underlying.get_map(s).output(t, idx).as_slice(),
                    );

                    self.underlying.get_map(s - 2).apply(
                        scratch.as_slice_mut(),
                        1,
                        t - 1,
                        self.source.homotopy(s).output(t, idx).homotopy.as_slice(),
                    );

                    self.target
                        .chain_complex
                        .differential(s - shift_s - 1)
                        .apply_quasi_inverse(
                            self.homotopies[s as i32]
                                .output_mut(t, idx)
                                .homotopy
                                .as_slice_mut(),
                            t - shift_t - 1,
                            scratch.as_slice(),
                        );
                }
            }
        }
    }

    pub fn homotopy(&self, s: u32) -> &SecondaryHomotopy<A> {
        &self.homotopies[s as i32]
    }
}

/// Whether picking δ₂ = 0 gives a valid secondary refinement. This requires
///  1. The chain complex is concentrated in degree zero;
///  2. The module is finite dimensional; and
///  3. $\mathrm{Hom}(\mathrm{Ext}^{2, t}_A(H^*X, k), H^{t - 1} X) = 0$ for all $t$ or $\mathrm{Hom}(\mathrm{Ext}^{3, t}_A(H^*X, k), H^{t - 1} X) = 0$ for all $t$.
pub fn can_compute(res: &Resolution) -> bool {
    let complex = res.complex();
    if *complex.prime() != 2 {
        eprintln!("Prime is not 2");
        return false;
    }
    if complex.max_s() != 1 {
        eprintln!("Complex is not concentrated in degree 0.");
        return false;
    }
    let module = complex.module(0);
    let module = module.as_fd_module();
    if module.is_none() {
        eprintln!("Module is not finite dimensional");
        return false;
    }
    let module = module.unwrap();
    let max_degree = module.max_degree();

    (0..max_degree)
        .all(|t| module.dimension(t) == 0 || res.number_of_gens_in_bidegree(2, t + 1) == 0)
        || (0..max_degree)
            .all(|t| module.dimension(t) == 0 || res.number_of_gens_in_bidegree(3, t + 1) == 0)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::construct;
    use expect_test::expect;
    use itertools::Itertools;
    use std::fmt::Write;

    #[test]
    fn test_compute_differentials() {
        let mut result = String::new();
        let resolution = construct("S_2@milnor", None).unwrap();

        let max_s = 7;
        let max_t = 30;

        resolution.compute_through_bidegree(max_s, max_t);

        let mut lift = SecondaryLift::new(Arc::new(resolution));
        lift.initialize_homotopies();
        lift.compute_composites();
        lift.compute_homotopies();

        // Iterate through the bidegree of the source of the differential.
        for s in 0..(max_s - 1) {
            let homotopy = lift.homotopy(s + 2);

            for t in s as i32..max_t {
                let source_num_gens = homotopy.source.number_of_gens_in_degree(t + 1);
                let target_num_gens = homotopy.target.number_of_gens_in_degree(t);
                if source_num_gens == 0 || target_num_gens == 0 {
                    continue;
                }
                let mut entries = vec![vec![0; target_num_gens]; source_num_gens];

                let offset = homotopy.target.generator_offset(t, t, 0);

                for (n, row) in entries.iter_mut().enumerate() {
                    let dx = &homotopy.output(t + 1, n).homotopy;

                    for (k, entry) in row.iter_mut().enumerate() {
                        *entry = dx.entry(offset + k);
                    }
                }

                let x = t - s as i32;
                for k in 0..target_num_gens {
                    writeln!(
                        &mut result,
                        "d_2 x_({x}, {s}, {k}) = [{}]",
                        (0..source_num_gens).map(|n| entries[n][k]).format(", ")
                    )
                    .unwrap();
                }
            }
        }

        expect![[r#"
            d_2 x_(1, 1, 0) = [0]
            d_2 x_(15, 1, 0) = [1]
            d_2 x_(8, 2, 0) = [0]
            d_2 x_(15, 2, 0) = [0]
            d_2 x_(16, 2, 0) = [0]
            d_2 x_(18, 2, 0) = [0]
            d_2 x_(15, 3, 0) = [0]
            d_2 x_(18, 3, 0) = [0]
            d_2 x_(19, 3, 0) = [0]
            d_2 x_(21, 3, 0) = [0]
            d_2 x_(15, 4, 0) = [0]
            d_2 x_(17, 4, 0) = [1]
            d_2 x_(18, 4, 0) = [0]
            d_2 x_(18, 4, 1) = [1]
            d_2 x_(17, 5, 0) = [0]
            d_2 x_(18, 5, 0) = [1]
            d_2 x_(24, 5, 0) = [0]
        "#]]
        .assert_eq(&result);
    }
}
