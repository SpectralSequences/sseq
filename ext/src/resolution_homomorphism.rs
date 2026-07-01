//! This module defines [`MuResolutionHomomorphism`], which is a chain map from a
//! [`FreeChainComplex`].
use std::{ops::Range, sync::Arc};

use algebra::{
    MuAlgebra,
    module::{
        Module,
        homomorphism::{ModuleHomomorphism, MuFreeModuleHomomorphism},
    },
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use fp::{
    matrix::Matrix,
    vector::{FpSliceMut, FpVector},
};
use maybe_rayon::prelude::*;
use once::OnceBiVec;
use sseq::coordinates::{Bidegree, BidegreeGenerator, BidegreeRange};

use crate::{
    chain_complex::{AugmentedChainComplex, BoundedChainComplex, ChainComplex, FreeChainComplex},
    save::{SaveDirectory, SaveKind},
};

pub type ResolutionHomomorphism<CC1, CC2> = MuResolutionHomomorphism<false, CC1, CC2>;
pub type UnstableResolutionHomomorphism<CC1, CC2> = MuResolutionHomomorphism<true, CC1, CC2>;

/// A chain complex homomorphims from a [`FreeChainComplex`]. This contains logic to lift chain
/// maps using the freeness.
pub struct MuResolutionHomomorphism<const U: bool, CC1, CC2>
where
    CC1: FreeChainComplex<U>,
    CC1::Algebra: MuAlgebra<U>,
    CC2: ChainComplex<Algebra = CC1::Algebra>,
{
    name: String,
    pub source: Arc<CC1>,
    pub target: Arc<CC2>,
    maps: OnceBiVec<Arc<MuFreeModuleHomomorphism<U, CC2::Module>>>,
    pub shift: Bidegree,
    save_dir: SaveDirectory,
}

impl<const U: bool, CC1, CC2> MuResolutionHomomorphism<U, CC1, CC2>
where
    CC1: FreeChainComplex<U>,
    CC1::Algebra: MuAlgebra<U>,
    CC2: ChainComplex<Algebra = CC1::Algebra>,
{
    pub fn new(name: String, source: Arc<CC1>, target: Arc<CC2>, shift: Bidegree) -> Self {
        let save_dir = if source.save_dir().is_some() && !name.is_empty() {
            let mut save_dir = source.save_dir().clone();
            save_dir.push(format!("products/{name}"));
            SaveKind::ChainMap
                .create_dir(save_dir.write().unwrap())
                .unwrap();
            save_dir
        } else {
            SaveDirectory::None
        };

        Self {
            name,
            source,
            target,
            maps: OnceBiVec::new(shift.s()),
            shift,
            save_dir,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn algebra(&self) -> Arc<CC1::Algebra> {
        self.source.algebra()
    }

    pub fn next_homological_degree(&self) -> i32 {
        self.maps.len()
    }

    fn get_map_ensure_length(&self, input_s: i32) -> &MuFreeModuleHomomorphism<U, CC2::Module> {
        self.maps.extend(input_s, |input_s| {
            let output_s = input_s - self.shift.s();
            Arc::new(MuFreeModuleHomomorphism::new(
                self.source.module(input_s),
                self.target.module(output_s),
                self.shift.t(),
            ))
        });
        &self.maps[input_s]
    }

    /// Returns the chain map on the `s`th source module.
    pub fn get_map(&self, input_s: i32) -> Arc<MuFreeModuleHomomorphism<U, CC2::Module>> {
        Arc::clone(&self.maps[input_s])
    }

    pub fn save_dir(&self) -> &SaveDirectory {
        &self.save_dir
    }
}

impl<const U: bool, CC1, CC2> MuResolutionHomomorphism<U, CC1, CC2>
where
    CC1: FreeChainComplex<U>,
    CC1::Algebra: MuAlgebra<U>,
    CC2: ChainComplex<Algebra = CC1::Algebra>,
{
    /// Extend the resolution homomorphism such that it is defined on degrees
    /// (`max_s`, `max_t`).
    ///
    /// This assumes in yet-uncomputed bidegrees, the homology of the source consists only of
    /// decomposables (e.g. it is trivial). More precisely, we assume
    /// [`MuResolutionHomomorphism::extend_step_raw`] can be called with `extra_images = None`.
    #[tracing::instrument(fields(self = self.name, %max))]
    pub fn extend(&self, max: Bidegree) {
        self.extend_profile(BidegreeRange::new(&(), max.s() + 1, &|_, _| max.t() + 1))
    }

    /// Extend the resolution homomorphism such that it is defined on degrees
    /// (`max_n`, `max_s`).
    ///
    /// This assumes in yet-uncomputed bidegrees, the homology of the source consists only of
    /// decomposables (e.g. it is trivial). More precisely, we assume
    /// [`MuResolutionHomomorphism::extend_step_raw`] can be called with `extra_images = None`.
    #[tracing::instrument(fields(self = self.name, %max))]
    pub fn extend_through_stem(&self, max: Bidegree) {
        self.extend_profile(BidegreeRange::new(&(), max.s() + 1, &|_, s| {
            max.n() + s + 1
        }))
    }

    /// Extend the resolution homomorphism as far as possible, as constrained by how much the
    /// source and target have been resolved.
    ///
    /// This assumes in yet-uncomputed bidegrees, the homology of the source consists only of
    /// decomposables (e.g. it is trivial). More precisely, we assume
    /// [`MuResolutionHomomorphism::extend_step_raw`] can be called with `extra_images = None`.
    #[tracing::instrument(fields(self = self.name))]
    pub fn extend_all(&self) {
        self.extend_profile(BidegreeRange::new(
            self,
            std::cmp::min(
                self.target.next_homological_degree() + self.shift.s(),
                self.source.next_homological_degree(),
            ),
            &|selff, s| {
                std::cmp::min(
                    selff
                        .target
                        .module(s - selff.shift.s())
                        .max_computed_degree()
                        + selff.shift.t(),
                    selff.source.module(s).max_computed_degree(),
                ) + 1
            },
        ));
    }

    /// Extends the resolution homomorphism up to a given range. This range is first specified by
    /// the maximum `s`, then the maximum `t` for each `s`. This should rarely be used directly;
    /// instead one should use [`MuResolutionHomomorphism::extend`],
    /// [`MuResolutionHomomorphism::extend_through_stem`] and [`ResolutionHomomorphism::extend_all`]
    /// as appropriate.
    ///
    /// Note that unlike the more specific versions of this function, the bounds `max_s` and
    /// `max_t` are exclusive.
    ///
    /// This assumes in yet-uncomputed bidegrees, the homology of the source consists only of
    /// decomposables (e.g. it is trivial). More precisely, we assume
    /// [`MuResolutionHomomorphism::extend_step_raw`] can be called with `extra_images = None`.
    pub fn extend_profile<AUX: Sync>(&self, max: BidegreeRange<AUX>) {
        self.get_map_ensure_length(max.s() - 1);

        sseq::coordinates::iter_s_t(
            &|b| self.extend_step_raw(b, None),
            Bidegree::s_t(
                self.shift.s(),
                self.get_map_ensure_length(self.shift.s()).min_degree(),
            ),
            max,
        );

        for s in self.shift.s()..max.s() {
            assert_eq!(
                Vec::<i32>::new(),
                self.maps[s].ooo_outputs(),
                "Map {s} has out of order elements"
            );
        }
    }

    /// Extend the [`MuResolutionHomomorphism`] to be defined on `(input_s, input_t)`. The resulting
    /// homomorphism `f` is a chain map such that if `g` is the `k`th generator in the source such
    /// that `d(g) = 0`, then `f(g)` is the `k`th row of `extra_images`.
    ///
    /// The user should call this function explicitly to manually define the chain map where the
    /// chain complex is not exact, and then call [`MuResolutionHomomorphism::extend_all`] to extend
    /// the rest by exactness.
    #[tracing::instrument(skip(self, extra_images), fields(self = self.name, %input))]
    pub fn extend_step_raw(
        &self,
        input: Bidegree,
        extra_images: Option<Vec<FpVector>>,
    ) -> Range<i32> {
        let output = input - self.shift;
        assert!(self.target.has_computed_bidegree(output));
        assert!(self.source.has_computed_bidegree(input));
        assert!(input.s() >= self.shift.s());

        let f_cur = self.get_map_ensure_length(input.s());
        if input.t() < f_cur.next_degree() {
            assert!(extra_images.is_none());
            // We need to signal to compute the dependents of this
            return input.t()..input.t() + 1;
        }

        let p = self.source.prime();

        let num_gens = f_cur.source().number_of_gens_in_degree(input.t());
        let fx_dimension = f_cur.target().dimension(output.t());

        if num_gens == 0 || fx_dimension == 0 {
            return f_cur.add_generators_from_rows_ooo(
                input.t(),
                vec![FpVector::new(p, fx_dimension); num_gens],
            );
        }

        if let Some(dir) = self.save_dir.read() {
            let mut outputs = Vec::with_capacity(num_gens);

            if let Some(mut f) = self
                .source
                .save_file(SaveKind::ChainMap, input)
                .open_file(dir.to_owned())
            {
                let fx_dimension = f.read_u64::<LittleEndian>().unwrap() as usize;
                for _ in 0..num_gens {
                    outputs.push(FpVector::from_bytes(p, fx_dimension, &mut f).unwrap());
                }
                return f_cur.add_generators_from_rows_ooo(input.t(), outputs);
            }
        }

        if output.s() == 0 {
            let outputs =
                extra_images.unwrap_or_else(|| vec![FpVector::new(p, fx_dimension); num_gens]);

            if let Some(dir) = self.save_dir.write() {
                let mut f = self
                    .source
                    .save_file(SaveKind::ChainMap, input)
                    .create_file(dir.clone(), false);
                f.write_u64::<LittleEndian>(fx_dimension as u64).unwrap();
                for row in &outputs {
                    row.to_bytes(&mut f).unwrap();
                }
            }

            return f_cur.add_generators_from_rows_ooo(input.t(), outputs);
        }
        let mut outputs = vec![FpVector::new(p, fx_dimension); num_gens];
        let d_source = self.source.differential(input.s());
        let d_target = self.target.differential(output.s());
        let f_prev = self.get_map(input.s() - 1);
        assert!(Arc::ptr_eq(&d_source.source(), &f_cur.source()));
        assert!(Arc::ptr_eq(&d_source.target(), &f_prev.source()));
        assert!(Arc::ptr_eq(&d_target.source(), &f_cur.target()));
        assert!(Arc::ptr_eq(&d_target.target(), &f_prev.target()));
        let fdx_dimension = f_prev.target().dimension(output.t());

        // First take care of generators that hit the target chain complex.
        let mut extra_image_row = 0;
        for (k, output_row) in outputs.iter_mut().enumerate() {
            if d_source.output(input.t(), k).is_zero() {
                let extra_image_matrix = extra_images.as_ref().expect("Missing extra image rows");
                output_row.assign(&extra_image_matrix[extra_image_row]);
                extra_image_row += 1;
            }
        }

        // Now do the rest
        d_target.compute_auxiliary_data_through_degree(output.t());

        let compute_fdx_vector = |k| {
            let dx_vector = d_source.output(input.t(), k);
            if dx_vector.is_zero() {
                None
            } else {
                let mut fdx_vector = FpVector::new(p, fdx_dimension);
                f_prev.apply(
                    fdx_vector.as_slice_mut(),
                    1,
                    input.t(),
                    dx_vector.as_slice(),
                );
                Some(fdx_vector)
            }
        };

        let fdx_vectors: Vec<FpVector> = (0..num_gens)
            .into_maybe_par_iter()
            .filter_map(compute_fdx_vector)
            .collect();

        let mut qi_outputs: Vec<_> = outputs
            .iter_mut()
            .enumerate()
            .filter_map(|(k, v)| {
                if d_source.output(input.t(), k).is_zero() {
                    None
                } else {
                    Some(v.as_slice_mut())
                }
            })
            .collect();

        if !fdx_vectors.is_empty() {
            assert!(
                self.target
                    .apply_quasi_inverse(&mut qi_outputs, output, &fdx_vectors)
            );
        }

        if let Some(dir) = self.save_dir.write() {
            let mut f = self
                .source
                .save_file(SaveKind::ChainMap, input)
                .create_file(dir.clone(), false);
            f.write_u64::<LittleEndian>(fx_dimension as u64).unwrap();
            for row in &outputs {
                row.to_bytes(&mut f).unwrap();
            }
        }
        f_cur.add_generators_from_rows_ooo(input.t(), outputs)
    }
}

impl<const U: bool, CC1, CC2> MuResolutionHomomorphism<U, CC1, CC2>
where
    CC1: FreeChainComplex<U>,
    CC1::Algebra: MuAlgebra<U>,
    CC2: AugmentedChainComplex<Algebra = CC1::Algebra>,
{
    pub fn from_class(
        name: String,
        source: Arc<CC1>,
        target: Arc<CC2>,
        shift: Bidegree,
        class: &[u32],
    ) -> Self {
        let result = Self::new(name, source, target, shift);

        let num_gens = result
            .source
            .module(shift.s())
            .number_of_gens_in_degree(shift.t());
        assert_eq!(num_gens, class.len());

        let mut matrix = Matrix::new(result.source.prime(), num_gens, 1);
        for (k, &v) in class.iter().enumerate() {
            matrix.row_mut(k).set_entry(0, v);
        }

        result.extend_step(shift, Some(&matrix));
        result
    }

    /// Extend the [`MuResolutionHomomorphism`] to be defined on `(input_s, input_t)`. The resulting
    /// homomorphism `f` is a chain map such that if `g` is the `k`th generator in the source such
    /// that `d(g) = 0`, then the image of `f(g)` in the augmentation of the target is the `k`th
    /// row of `extra_images`.
    ///
    /// The user should call this function explicitly to manually define the chain map where the
    /// chain complex is not exact, and then call [`MuResolutionHomomorphism::extend_all`] to extend
    /// the rest by exactness.
    pub fn extend_step(&self, input: Bidegree, extra_images: Option<&Matrix>) -> Range<i32> {
        self.extend_step_raw(
            input,
            extra_images.map(|m| {
                let p = self.target.prime();
                let output = input - self.shift;

                let mut outputs =
                    vec![
                        FpVector::new(p, self.target.module(output.s()).dimension(output.t()));
                        m.rows()
                    ];
                let chain_map = self.target.chain_map(output.s());
                chain_map.compute_auxiliary_data_through_degree(output.t());
                for (output_vec, input) in std::iter::zip(&mut outputs, m.iter()) {
                    assert!(chain_map.apply_quasi_inverse(
                        output_vec.as_slice_mut(),
                        output.t(),
                        input,
                    ));
                }
                outputs
            }),
        )
    }
}

impl<const U: bool, CC1, CC2> MuResolutionHomomorphism<U, CC1, CC2>
where
    CC1: AugmentedChainComplex + FreeChainComplex<U>,
    CC1::Algebra: MuAlgebra<U>,
    CC2: AugmentedChainComplex<Algebra = CC1::Algebra>,
    CC1::TargetComplex: BoundedChainComplex,
    CC2::TargetComplex: BoundedChainComplex,
{
    /// Construct a chain map that lifts a given module homomorphism.
    pub fn from_module_homomorphism(
        name: String,
        source: Arc<CC1>,
        target: Arc<CC2>,
        f: &impl ModuleHomomorphism<
            Source = <<CC1 as AugmentedChainComplex>::TargetComplex as ChainComplex>::Module,
            Target = <<CC2 as AugmentedChainComplex>::TargetComplex as ChainComplex>::Module,
        >,
    ) -> Self {
        assert_eq!(source.target().max_s(), 1);
        assert_eq!(target.target().max_s(), 1);

        let source_module = source.target().module(0);
        let target_module = target.target().module(0);
        assert!(Arc::ptr_eq(&source_module, &f.source()));
        assert!(Arc::ptr_eq(&target_module, &f.target()));

        let p = source.prime();
        let shift = Bidegree::s_t(0, f.degree_shift());

        let max_degree = source_module.max_generator_degree().expect(
            "MuResolutionHomomorphism::from_module_homomorphism requires finite \
             max_generator_degree",
        );

        let hom = Self::new(name, source, target, shift);

        source_module.compute_basis(max_degree);
        target_module.compute_basis(shift.t() + max_degree);

        let max = Bidegree::s_t(0, max_degree);
        hom.source.compute_through_bidegree(max);
        hom.target.compute_through_bidegree(max + shift);

        for t in source_module.min_degree()..=max_degree {
            let mut m = Matrix::new(
                p,
                source_module.dimension(t),
                target_module.dimension(t + shift.t()),
            );

            f.get_matrix(m.as_slice_mut(), t);
            hom.extend_step(Bidegree::s_t(0, t), Some(&m));
        }
        hom
    }
}

impl<const U: bool, CC1, CC2> MuResolutionHomomorphism<U, CC1, CC2>
where
    CC1: FreeChainComplex<U>,
    CC1::Algebra: MuAlgebra<U>,
    CC2: FreeChainComplex<U, Algebra = CC1::Algebra>,
{
    /// Given a chain map $f: C \to C'$ between free chain complexes, apply
    /// $$ \Hom(f, k): \Hom(C', k) \to \Hom(C, k) $$
    /// to the specified generator of $\Hom(C', k)$.
    pub fn act(&self, mut result: FpSliceMut, coef: u32, g: BidegreeGenerator) {
        let source = g.degree() + self.shift;

        assert_eq!(
            result.as_slice().len(),
            self.source
                .module(source.s())
                .number_of_gens_in_degree(source.t())
        );

        let target_module = self.target.module(g.s());

        let map = self.get_map(source.s());
        let j = target_module.operation_generator_to_index(0, 0, g.t(), g.idx());
        for i in 0..result.as_slice().len() {
            result.add_basis_element(i, coef * map.output(source.t(), i).entry(j));
        }
    }
}

// The secondary lift of a `ResolutionHomomorphism` lives here, beside the primary object it lifts,
// rather than in the monolithic `secondary` module. This keeps `secondary.rs` to the shared lift
// machinery and pairs each variant with its primary for locality. The module is `pub(crate)`;
// `SecondaryResolutionHomomorphism` is re-exported from `crate::secondary` so the public API path is
// unchanged.
pub(crate) mod secondary {
    use std::sync::Arc;

    use algebra::{
        module::{Module, homomorphism::ModuleHomomorphism},
        pair_algebra::PairAlgebra,
    };
    use dashmap::DashMap;
    use fp::{
        matrix::Matrix,
        vector::{FpSlice, FpSliceMut, FpVector},
    };
    use itertools::Itertools;
    use once::OnceBiVec;
    use sseq::coordinates::{Bidegree, BidegreeGenerator, BidegreeRange};

    use super::ResolutionHomomorphism;
    use crate::{
        chain_complex::FreeChainComplex,
        save::{SaveDirectory, SaveKind},
        secondary::{
            CompositeData, LAMBDA_BIDEGREE, SecondaryHomotopy, SecondaryLift, SecondaryResolution,
        },
    };

    // Rustdoc ICE's when trying to document this struct. See
    // https://github.com/rust-lang/rust/issues/91380
    #[doc(hidden)]
    pub struct SecondaryResolutionHomomorphism<
        CC1: FreeChainComplex,
        CC2: FreeChainComplex<Algebra = CC1::Algebra>,
    >
    where
        CC1::Algebra: PairAlgebra,
    {
        source: Arc<SecondaryResolution<CC1>>,
        target: Arc<SecondaryResolution<CC2>>,
        underlying: Arc<ResolutionHomomorphism<CC1, CC2>>,
        /// input s -> homotopy
        homotopies: OnceBiVec<SecondaryHomotopy<CC1::Algebra>>,
        intermediates: DashMap<BidegreeGenerator, FpVector>,
    }

    impl<CC1: FreeChainComplex, CC2: FreeChainComplex<Algebra = CC1::Algebra>> SecondaryLift
        for SecondaryResolutionHomomorphism<CC1, CC2>
    where
        CC1::Algebra: PairAlgebra,
    {
        type Algebra = CC1::Algebra;
        type Source = CC1;
        type Target = CC2;
        type Underlying = ResolutionHomomorphism<CC1, CC2>;

        fn underlying(&self) -> Arc<Self::Underlying> {
            Arc::clone(&self.underlying)
        }

        fn algebra(&self) -> Arc<Self::Algebra> {
            self.source.algebra()
        }

        fn source(&self) -> Arc<Self::Source> {
            Arc::clone(&self.source.underlying())
        }

        fn target(&self) -> Arc<Self::Target> {
            Arc::clone(&self.target.underlying())
        }

        fn shift(&self) -> Bidegree {
            self.underlying.shift + Bidegree::s_t(1, 0)
        }

        fn max(&self) -> BidegreeRange<'_, Self> {
            BidegreeRange::new(
                self,
                self.underlying.next_homological_degree(),
                &|selff, s| {
                    std::cmp::min(
                        selff.underlying.get_map(s).next_degree(),
                        std::cmp::min(
                            selff.source.homotopies[s].homotopies.next_degree(),
                            if s == selff.shift().s() {
                                i32::MAX
                            } else {
                                selff.target.homotopies[s + 1 - selff.shift().s()]
                                    .composites
                                    .max_degree()
                                    + selff.shift().t()
                                    + 1
                            },
                        ),
                    )
                },
            )
        }

        fn homotopies(&self) -> &OnceBiVec<SecondaryHomotopy<Self::Algebra>> {
            &self.homotopies
        }

        fn intermediates(&self) -> &DashMap<BidegreeGenerator, FpVector> {
            &self.intermediates
        }

        fn save_dir(&self) -> &SaveDirectory {
            self.underlying.save_dir()
        }

        fn composite(&self, s: i32) -> CompositeData<Self::Algebra> {
            let p = self.prime();
            // This is -1 mod p^2
            let neg_1 = p * p - 1;

            let d_source = self.source.underlying().differential(s);
            let d_target = self
                .target
                .underlying()
                .differential(s + 1 - self.shift().s());

            let c1 = self.underlying.get_map(s);
            let c0 = self.underlying.get_map(s - 1);

            vec![(neg_1, d_source, c0), (1, c1, d_target)]
        }

        fn compute_intermediate(&self, g: BidegreeGenerator) -> FpVector {
            let p = self.prime();
            let neg_1 = p - 1;
            let shifted_b = g.degree() - self.shift();
            let target = self.target().module(shifted_b.s() - 1);

            let mut result = FpVector::new(p, target.dimension(shifted_b.t() - 1));
            let d = self.source().differential(g.s());

            self.homotopies[g.s() - 1].act(
                result.as_slice_mut(),
                neg_1,
                g.t(),
                d.output(g.t(), g.idx()).as_slice(),
                false,
            );
            self.target.homotopy(shifted_b.s() + 1).act(
                result.as_slice_mut(),
                neg_1,
                shifted_b.t(),
                self.underlying
                    .get_map(g.s())
                    .output(g.t(), g.idx())
                    .as_slice(),
                true,
            );
            self.underlying.get_map(g.s() - 2).apply(
                result.as_slice_mut(),
                1,
                g.t() - 1,
                self.source
                    .homotopy(g.s())
                    .homotopies
                    .output(g.t(), g.idx())
                    .as_slice(),
            );

            result
        }
    }

    impl<CC1: FreeChainComplex, CC2: FreeChainComplex<Algebra = CC1::Algebra>>
        SecondaryResolutionHomomorphism<CC1, CC2>
    where
        CC1::Algebra: PairAlgebra,
    {
        pub fn new(
            source: Arc<SecondaryResolution<CC1>>,
            target: Arc<SecondaryResolution<CC2>>,
            underlying: Arc<ResolutionHomomorphism<CC1, CC2>>,
        ) -> Self {
            assert!(Arc::ptr_eq(&underlying.source, &source.underlying()));
            assert!(Arc::ptr_eq(&underlying.target, &target.underlying()));

            if let Some(p) = underlying.save_dir().write() {
                for subdir in SaveKind::secondary_data() {
                    subdir.create_dir(p).unwrap();
                }
            }

            Self {
                source,
                target,
                homotopies: OnceBiVec::new(underlying.shift.s() + 1),
                underlying,
                intermediates: DashMap::new(),
            }
        }

        pub fn name(&self) -> String {
            let name = self.underlying.name();
            if name.starts_with('[') || name.starts_with('λ') {
                name.to_owned()
            } else {
                format!("[{name}]")
            }
        }

        pub fn homotopy(&self, s: i32) -> &SecondaryHomotopy<CC1::Algebra> {
            &self.homotopies[s]
        }

        pub fn secondary_source(&self) -> Arc<SecondaryResolution<CC1>> {
            Arc::clone(&self.source)
        }

        pub fn secondary_target(&self) -> Arc<SecondaryResolution<CC2>> {
            Arc::clone(&self.target)
        }

        /// A version of [`hom_k`] but with a non-trivial λ part.
        pub fn hom_k_with<'a>(
            &self,
            lambda_part: Option<&ResolutionHomomorphism<CC1, CC2>>,
            sseq: Option<&sseq::Sseq<2, sseq::Adams>>,
            b: Bidegree,
            inputs: impl Iterator<Item = FpSlice<'a>>,
            outputs: impl Iterator<Item = FpSliceMut<'a>>,
        ) {
            let source = b + self.shift() - Bidegree::s_t(1, 0);
            let lambda_source = source + LAMBDA_BIDEGREE;

            let p = self.prime();
            let h_0 = self.algebra().p_tilde();

            let source_num_gens = self.source().number_of_gens_in_bidegree(source);
            let lambda_num_gens = self.source().number_of_gens_in_bidegree(lambda_source);

            let m0 = self.underlying.get_map(source.s()).hom_k(b.t());
            let mut m1 =
                Matrix::from_vec(p, &self.homotopy(lambda_source.s()).homotopies.hom_k(b.t()));
            if let Some(lambda_part) = lambda_part {
                m1 += &Matrix::from_vec(p, &lambda_part.get_map(lambda_source.s()).hom_k(b.t()));
            }

            // The multiplication by p map
            let mp = Matrix::from_vec(
                p,
                &self
                    .source()
                    .filtration_one_product(1, h_0, source)
                    .unwrap(),
            );

            let sign = if (self.underlying.shift.s() * b.t()) % 2 == 1 {
                p * p - 1
            } else {
                1
            };
            let filtration_one_sign = if (b.t() % 2) == 1 { p - 1 } else { 1 };

            let page_data = sseq.map(|sseq| {
                let d = sseq.page_data(lambda_source);
                &d[std::cmp::min(3, d.len() - 1)]
            });

            let mut scratch0: Vec<u32> = Vec::new();
            for (input, mut out) in inputs.zip_eq(outputs) {
                scratch0.clear();
                scratch0.resize(source_num_gens, 0);
                for (i, v) in input.iter_nonzero() {
                    scratch0
                        .iter_mut()
                        .zip_eq(&m0[i])
                        .for_each(|(a, b)| *a += v * b * sign);
                    out.slice_mut(source_num_gens, source_num_gens + lambda_num_gens)
                        .add(m1.row(i), (v * sign) % p);
                }
                for (i, v) in scratch0.iter().enumerate() {
                    out.add_basis_element(i, *v % p);

                    let extra = *v / p;
                    out.slice_mut(source_num_gens, source_num_gens + lambda_num_gens)
                        .add(mp.row(i), (extra * filtration_one_sign) % p);
                }
                if let Some(page_data) = page_data {
                    page_data.reduce_by_quotient(
                        out.slice_mut(source_num_gens, source_num_gens + lambda_num_gens),
                    );
                }
            }
        }

        /// Compute the induced map on Mod_{C\lambda^2} homotopy groups. This only computes it on
        /// standard lifts on elements in Ext. `outputs` is an iterator of `FpSliceMut`s whose lengths
        /// are equal to the total dimension of `(s + shift_s, t + shift_t)` and `(s + shift_s + 1, t +
        /// shift_t + 1)`. The first chunk records the Ext part of the result, and the second chunk
        /// records the λ part of the result.
        ///
        /// This reduces the λ part of the result by the image of d₂.
        ///
        /// # Arguments
        /// - `sseq`: A sseq object that records the $d_2$ differentials. If present, reduce the value
        ///   of the map by the image of $d_2$.
        pub fn hom_k<'a>(
            &self,
            sseq: Option<&sseq::Sseq<2, sseq::Adams>>,
            b: Bidegree,
            inputs: impl Iterator<Item = FpSlice<'a>>,
            outputs: impl Iterator<Item = FpSliceMut<'a>>,
        ) {
            self.hom_k_with(None, sseq, b, inputs, outputs);
        }

        /// Given an element b whose product with this is null, find the element whose $d_2$ hits the
        /// λ part of the composition.
        ///
        /// # Arguments:
        /// - `sseq`: spectral sequence object of the source
        pub fn product_nullhomotopy(
            &self,
            lambda_part: Option<&ResolutionHomomorphism<CC1, CC2>>,
            sseq: &sseq::Sseq<2, sseq::Adams>,
            b: Bidegree,
            class: FpSlice,
        ) -> FpVector {
            let p = self.prime();
            let shift = self.underlying.shift;

            let result_num_gens = self
                .source()
                .number_of_gens_in_bidegree(shift + b - Bidegree::s_t(1, 0));

            let lambda_num_gens = self
                .source()
                .number_of_gens_in_bidegree(b + shift + LAMBDA_BIDEGREE);

            let lower_num_gens = self.source().number_of_gens_in_bidegree(b + shift);

            let target_num_gens = self.target().number_of_gens_in_bidegree(b);
            let target_lambda_num_gens = self
                .target()
                .number_of_gens_in_bidegree(b + LAMBDA_BIDEGREE);

            let mut output_class = FpVector::new(p, result_num_gens);
            if result_num_gens == 0 || lambda_num_gens == 0 {
                return output_class;
            }

            let mut prod_value = FpVector::new(p, lower_num_gens + lambda_num_gens);
            self.hom_k_with(
                lambda_part,
                None,
                b,
                [class.restrict(0, target_num_gens)].into_iter(),
                [prod_value.as_slice_mut()].into_iter(),
            );
            assert!(prod_value.slice(0, lower_num_gens).is_zero());

            let matrix = Matrix::from_vec(
                p,
                &self
                    .underlying
                    .get_map((b + shift + LAMBDA_BIDEGREE).s())
                    .hom_k((b + LAMBDA_BIDEGREE).t()),
            );
            matrix.apply(
                prod_value.slice_mut(lower_num_gens, lower_num_gens + lambda_num_gens),
                1,
                class.restrict(target_num_gens, target_num_gens + target_lambda_num_gens),
            );

            let diff_source = b + shift - Bidegree::n_s(-1, 1);
            sseq.differentials(diff_source)[2].quasi_inverse(
                output_class.as_slice_mut(),
                prod_value.slice(lower_num_gens, lower_num_gens + lambda_num_gens),
            );

            output_class
        }
    }
}
