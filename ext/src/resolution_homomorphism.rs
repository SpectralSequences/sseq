//! This module defines [`ResolutionHomomorphism`], which is a chain map from a
//! [`FreeChainComplex`].
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

use crate::chain_complex::{
    AugmentedChainComplex, BoundedChainComplex, ChainComplex, FreeChainComplex,
};
use crate::save::SaveKind;
use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::Module;
use fp::matrix::Matrix;
use fp::vector::{FpVector, SliceMut};
use once::OnceBiVec;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

#[cfg(feature = "concurrent")]
use rayon::prelude::*;

/// A chain complex homomorphims from a [`FreeChainComplex`]. This contains logic to lift chain
/// maps using the freeness.
pub struct ResolutionHomomorphism<CC1, CC2>
where
    CC1: FreeChainComplex,
    CC2: ChainComplex<Algebra = CC1::Algebra>,
{
    name: String,
    pub source: Arc<CC1>,
    pub target: Arc<CC2>,
    maps: OnceBiVec<Arc<FreeModuleHomomorphism<CC2::Module>>>,
    pub shift_s: u32,
    pub shift_t: i32,
    save_dir: Option<PathBuf>,
}

impl<CC1, CC2> ResolutionHomomorphism<CC1, CC2>
where
    CC1: FreeChainComplex,
    CC2: ChainComplex<Algebra = CC1::Algebra>,
{
    pub fn new(
        name: String,
        source: Arc<CC1>,
        target: Arc<CC2>,
        shift_s: u32,
        shift_t: i32,
    ) -> Self {
        let save_dir = if source.save_dir().is_some() && !name.is_empty() {
            let mut path = source.save_dir().unwrap().to_owned();
            path.push(format!("products/{name}"));
            SaveKind::ChainMap.create_dir(&path).unwrap();
            Some(path)
        } else {
            None
        };

        Self {
            name,
            source,
            target,
            maps: OnceBiVec::new(shift_s as i32),
            shift_s,
            shift_t,
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

    fn get_map_ensure_length(&self, input_s: u32) -> &FreeModuleHomomorphism<CC2::Module> {
        self.maps.extend(input_s as i32, |input_s| {
            let output_s = input_s as u32 - self.shift_s;
            Arc::new(FreeModuleHomomorphism::new(
                self.source.module(input_s as u32),
                self.target.module(output_s),
                self.shift_t,
            ))
        });
        &self.maps[input_s as i32]
    }

    /// Returns the chain map on the `s`th source module.
    pub fn get_map(&self, input_s: u32) -> Arc<FreeModuleHomomorphism<CC2::Module>> {
        Arc::clone(&self.maps[input_s as i32])
    }

    pub fn save_dir(&self) -> Option<&std::path::Path> {
        self.save_dir.as_deref()
    }
}

impl<CC1, CC2> ResolutionHomomorphism<CC1, CC2>
where
    CC1: FreeChainComplex,
    CC2: AugmentedChainComplex<Algebra = CC1::Algebra>,
{
    pub fn from_class(
        name: String,
        source: Arc<CC1>,
        target: Arc<CC2>,
        shift_s: u32,
        shift_t: i32,
        class: &[u32],
    ) -> Self {
        let result = Self::new(name, source, target, shift_s, shift_t);

        let num_gens = result
            .source
            .module(shift_s)
            .number_of_gens_in_degree(shift_t);
        assert_eq!(num_gens, class.len());

        let mut matrix = Matrix::new(result.source.prime(), num_gens, 1);
        for (k, &v) in class.iter().enumerate() {
            matrix[k].set_entry(0, v);
        }

        result.extend_step(shift_s, shift_t, Some(&matrix));
        result
    }

    /// Extend the resolution homomorphism such that it is defined on degrees
    /// (`max_s`, `max_t`).
    ///
    /// This assumes in yet-uncomputed bidegrees, the homology of the source consists only of
    /// decomposables (e.g. it is trivial). More precisely, we assume
    /// [`ResolutionHomomorphism::extend_step`] can be called with `extra_images = None`.
    pub fn extend(&self, max_s: u32, max_t: i32) {
        self.extend_profile(max_s + 1, |_s| max_t + 1)
    }

    /// Extend the resolution homomorphism such that it is defined on degrees
    /// (`max_n`, `max_s`).
    ///
    /// This assumes in yet-uncomputed bidegrees, the homology of the source consists only of
    /// decomposables (e.g. it is trivial). More precisely, we assume
    /// [`ResolutionHomomorphism::extend_step`] can be called with `extra_images = None`.
    pub fn extend_through_stem(&self, max_s: u32, max_n: i32) {
        self.extend_profile(max_s + 1, |s| max_n + s as i32 + 1)
    }

    /// Extend the resolution homomorphism as far as possible, as constrained by how much the
    /// source and target have been resolved.
    ///
    /// This assumes in yet-uncomputed bidegrees, the homology of the source consists only of
    /// decomposables (e.g. it is trivial). More precisely, we assume
    /// [`ResolutionHomomorphism::extend_step`] can be called with `extra_images = None`.
    pub fn extend_all(&self) {
        self.extend_profile(
            std::cmp::min(
                self.target.next_homological_degree() + self.shift_s,
                self.source.next_homological_degree(),
            ),
            |s| {
                std::cmp::min(
                    self.target.module(s - self.shift_s).max_computed_degree() + self.shift_t,
                    self.source.module(s).max_computed_degree(),
                ) + 1
            },
        );
    }

    // See the concurrent version for documentation
    #[cfg(not(feature = "concurrent"))]
    pub fn extend_profile(&self, max_s: u32, max_t: impl Fn(u32) -> i32 + Sync) {
        self.get_map_ensure_length(max_s - 1);
        for s in self.shift_s..max_s {
            let f_cur = self.get_map_ensure_length(s);
            for t in f_cur.next_degree()..max_t(s) {
                self.extend_step(s, t, None);
            }
        }
    }

    /// Extends the resolution homomorphism up to a given range. This range is first specified by
    /// the maximum `s`, then the maximum `t` for each `s`. This should rarely be used directly;
    /// instead one should use [`ResolutionHomomorphism::extend`],
    /// [`ResolutionHomomorphism::extend_through_stem`] and [`ResolutionHomomorphism::extend_all`]
    /// as appropriate.
    ///
    /// Note that unlike the more specific versions of this function, the bounds `max_s` and
    /// `max_t` are exclusive.
    ///
    /// This assumes in yet-uncomputed bidegrees, the homology of the source consists only of
    /// decomposables (e.g. it is trivial). More precisely, we assume
    /// [`ResolutionHomomorphism::extend_step`] can be called with `extra_images = None`.
    #[cfg(feature = "concurrent")]
    pub fn extend_profile(&self, max_s: u32, max_t: impl Fn(u32) -> i32 + Sync) {
        self.get_map_ensure_length(max_s - 1);

        crate::utils::iter_s_t(
            &|s, t| self.extend_step(s, t, None),
            self.shift_s,
            self.get_map_ensure_length(self.shift_s).min_degree(),
            max_s,
            &max_t,
        );

        for s in self.shift_s..max_s {
            assert_eq!(
                Vec::<i32>::new(),
                self.maps[s as i32].ooo_outputs(),
                "Map {s} has out of order elements"
            );
        }
    }

    /// Extend the [`ResolutionHomomorphism`] to be defined on `(input_s, input_t)`. The resulting
    /// homomorphism `f` is a chain map such that if `g` is the `k`th generator in the source such
    /// that `d(g) = 0`, then the image of `f(g)` in the augmentation of the target is the `k`th
    /// row of `extra_images`.
    ///
    /// The user should call this function explicitly to manually define the chain map where the
    /// chain complex is not exact, and then call [`ResolutionHomomorphism::extend_all`] to extend
    /// the rest by exactness.
    pub fn extend_step(
        &self,
        input_s: u32,
        input_t: i32,
        extra_images: Option<&Matrix>,
    ) -> Range<i32> {
        let output_s = input_s - self.shift_s;
        let output_t = input_t - self.shift_t;
        assert!(self.target.has_computed_bidegree(output_s, output_t));
        assert!(self.source.has_computed_bidegree(input_s, input_t));
        assert!(input_s >= self.shift_s);

        let f_cur = self.get_map_ensure_length(input_s);
        if input_t < f_cur.next_degree() {
            assert!(extra_images.is_none());
            // We need to signal to compute the dependents of this
            return input_t..input_t + 1;
        }

        let p = self.source.prime();

        let num_gens = f_cur.source().number_of_gens_in_degree(input_t);
        let fx_dimension = f_cur.target().dimension(output_t);

        if num_gens == 0 || fx_dimension == 0 {
            return f_cur.add_generators_from_rows_ooo(
                input_t,
                vec![FpVector::new(p, fx_dimension); num_gens],
            );
        }

        if let Some(dir) = self.save_dir.as_ref() {
            let mut outputs = Vec::with_capacity(num_gens);

            if let Some(mut f) = self
                .source
                .save_file(SaveKind::ChainMap, input_s, input_t)
                .open_file(dir.to_owned())
            {
                let fx_dimension = f.read_u64::<LittleEndian>().unwrap() as usize;
                for _ in 0..num_gens {
                    outputs.push(FpVector::from_bytes(p, fx_dimension, &mut f).unwrap());
                }
                return f_cur.add_generators_from_rows_ooo(input_t, outputs);
            }
        }

        let mut outputs = vec![FpVector::new(p, fx_dimension); num_gens];
        if output_s == 0 {
            if let Some(extra_images_matrix) = extra_images {
                let target_chain_map = self.target.chain_map(output_s);
                let target_cc_dimension = target_chain_map.target().dimension(output_t);
                assert!(target_cc_dimension == extra_images_matrix.columns());

                target_chain_map.compute_auxiliary_data_through_degree(output_t);
                assert!(
                    num_gens == extra_images_matrix.rows(),
                    "num_gens : {} greater than rows : {} hom_deg : {}, int_deg : {}",
                    num_gens,
                    extra_images_matrix.rows(),
                    input_s,
                    input_t
                );
                for k in 0..num_gens {
                    assert!(target_chain_map.apply_quasi_inverse(
                        outputs[k].as_slice_mut(),
                        output_t,
                        extra_images_matrix[k].as_slice(),
                    ));
                }
            }

            if let Some(dir) = self.save_dir.as_ref() {
                let mut f = self
                    .source
                    .save_file(SaveKind::ChainMap, input_s, input_t)
                    .create_file(dir.to_owned());
                f.write_u64::<LittleEndian>(fx_dimension as u64).unwrap();
                for row in &outputs {
                    row.to_bytes(&mut f).unwrap();
                }
            }

            return f_cur.add_generators_from_rows_ooo(input_t, outputs);
        }
        let d_source = self.source.differential(input_s);
        let d_target = self.target.differential(output_s);
        let f_prev = self.get_map(input_s - 1);
        assert!(Arc::ptr_eq(&d_source.source(), &f_cur.source()));
        assert!(Arc::ptr_eq(&d_source.target(), &f_prev.source()));
        assert!(Arc::ptr_eq(&d_target.source(), &f_cur.target()));
        assert!(Arc::ptr_eq(&d_target.target(), &f_prev.target()));
        let fdx_dimension = f_prev.target().dimension(output_t);

        // First take care of generators that hit the target chain complex.
        let mut extra_image_row = 0;
        for (k, output_row) in outputs.iter_mut().enumerate() {
            if d_source.output(input_t, k).is_zero() {
                let target_chain_map = self.target.chain_map(output_s);
                let target_cc_dimension = target_chain_map.target().dimension(output_t);

                let extra_image_matrix = extra_images.as_ref().expect("Missing extra image rows");
                assert!(target_cc_dimension == extra_image_matrix.columns());

                target_chain_map.compute_auxiliary_data_through_degree(output_t);
                assert!(target_chain_map.apply_quasi_inverse(
                    output_row.as_slice_mut(),
                    output_t,
                    extra_image_matrix[extra_image_row].as_slice(),
                ));
                extra_image_row += 1;
            }
        }

        // Now do the rest
        d_target.compute_auxiliary_data_through_degree(output_t);

        let compute_fdx_vector = |k| {
            let dx_vector = d_source.output(input_t, k);
            if dx_vector.is_zero() {
                None
            } else {
                let mut fdx_vector = FpVector::new(p, fdx_dimension);
                f_prev.apply(fdx_vector.as_slice_mut(), 1, input_t, dx_vector.as_slice());
                Some(fdx_vector)
            }
        };

        #[cfg(not(feature = "concurrent"))]
        let fdx_vectors: Vec<FpVector> = (0..num_gens)
            .into_iter()
            .filter_map(compute_fdx_vector)
            .collect();

        #[cfg(feature = "concurrent")]
        let fdx_vectors: Vec<FpVector> = (0..num_gens)
            .into_par_iter()
            .filter_map(compute_fdx_vector)
            .collect();

        let mut qi_outputs: Vec<_> = outputs
            .iter_mut()
            .enumerate()
            .filter_map(|(k, v)| {
                if d_source.output(input_t, k).is_zero() {
                    None
                } else {
                    Some(v.as_slice_mut())
                }
            })
            .collect();

        if !fdx_vectors.is_empty() {
            assert!(self.target.apply_quasi_inverse(
                &mut qi_outputs,
                output_s,
                output_t,
                &fdx_vectors
            ));
        }

        if let Some(dir) = self.save_dir.as_ref() {
            let mut f = self
                .source
                .save_file(SaveKind::ChainMap, input_s, input_t)
                .create_file(dir.to_owned());
            f.write_u64::<LittleEndian>(fx_dimension as u64).unwrap();
            for row in &outputs {
                row.to_bytes(&mut f).unwrap();
            }
        }
        f_cur.add_generators_from_rows_ooo(input_t, outputs)
    }
}

impl<CC1, CC2> ResolutionHomomorphism<CC1, CC2>
where
    CC1: AugmentedChainComplex + FreeChainComplex,
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
        let degree_shift = f.degree_shift();

        let max_degree = source_module.max_generator_degree().expect(
            "ResolutionHomomorphism::from_module_homomorphism requires finite max_generator_degree",
        );

        let hom = Self::new(name, source, target, 0, degree_shift);

        source_module.compute_basis(max_degree);
        target_module.compute_basis(degree_shift + max_degree);

        hom.source.compute_through_bidegree(0, max_degree);
        hom.target
            .compute_through_bidegree(0, degree_shift + max_degree);

        let source_chain_map = hom.source.chain_map(0);
        let target_chain_map = hom.target.chain_map(0);
        target_chain_map.compute_auxiliary_data_through_degree(degree_shift + max_degree);

        let g = hom.get_map_ensure_length(0);
        let mut scratch = FpVector::new(hom.source.prime(), 0);

        for t in source_module.min_degree()..=max_degree {
            let num_gens = hom.source.module(0).number_of_gens_in_degree(t);

            let mut fx = FpVector::new(p, target_module.dimension(t + degree_shift));

            let mut outputs_matrix = Matrix::new(
                p,
                num_gens,
                hom.target.module(0).dimension(t + degree_shift),
            );
            if num_gens == 0 || fx.is_empty() {
                g.add_generators_from_matrix_rows(t, outputs_matrix.as_slice_mut());
                continue;
            }
            for j in 0..num_gens {
                scratch.set_scratch_vector_size(target_module.dimension(t + degree_shift));
                source_chain_map.apply_to_basis_element(
                    scratch.as_slice_mut(),
                    1,
                    t,
                    hom.source.module(0).generator_offset(t, t, j),
                );
                f.apply(fx.as_slice_mut(), 1, t, scratch.as_slice());
                assert!(target_chain_map.apply_quasi_inverse(
                    outputs_matrix[j].as_slice_mut(),
                    t + degree_shift,
                    fx.as_slice(),
                ));
                fx.set_to_zero();
            }
            g.add_generators_from_matrix_rows(t, outputs_matrix.as_slice_mut());
        }
        hom
    }
}

impl<CC1, CC2> ResolutionHomomorphism<CC1, CC2>
where
    CC1: FreeChainComplex,
    CC2: FreeChainComplex<Algebra = CC1::Algebra>,
{
    /// Given a chain map $f: C \to C'$ between free chain complexes, apply
    /// $$ \Hom(f, k): \Hom(C', k) \to \Hom(C, k) $$
    /// to the specified generator of $\Hom(C', k)$.
    pub fn act(&self, mut result: SliceMut, coef: u32, s: u32, t: i32, idx: usize) {
        let source_s = s + self.shift_s;
        let source_t = t + self.shift_t;

        assert_eq!(
            result.as_slice().len(),
            self.source
                .module(source_s)
                .number_of_gens_in_degree(source_t)
        );

        let target_module = self.target.module(s);

        let map = self.get_map(source_s);
        let j = target_module.operation_generator_to_index(0, 0, t, idx);
        for i in 0..result.as_slice().len() {
            result.add_basis_element(i, coef * map.output(source_t, i).entry(j));
        }
    }
}
