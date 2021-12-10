use crate::chain_complex::{
    AugmentedChainComplex, BoundedChainComplex, ChainComplex, FreeChainComplex,
};
use crate::resolution::Resolution;
use crate::resolution_homomorphism::ResolutionHomomorphism;
use crate::save::{SaveFile, SaveKind};

use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::{BoundedModule, FreeModule, Module};
use algebra::pair_algebra::PairAlgebra;
use algebra::Algebra;
use anyhow::Context;
use bivec::BiVec;
use fp::vector::{FpVector, Slice, SliceMut};
use once::OnceBiVec;

use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

use crate::CCC;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use dashmap::DashMap;
#[cfg(feature = "concurrent")]
use {
    rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator},
    thread_token::TokenBucket,
};

/// A homotopy of a map A -> M of pair modules. We assume this map does not hit generators.
pub struct SecondaryComposite<A: PairAlgebra> {
    target: Arc<FreeModule<A>>,
    degree: i32,
    /// The component of the map on the R_B portion.
    /// gen_deg -> gen_idx -> coefficient
    composite: BiVec<Vec<A::Element>>,
}

impl<A: PairAlgebra> SecondaryComposite<A> {
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

        Self {
            target,
            degree,
            composite,
        }
    }

    pub fn to_bytes(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        let algebra = self.target.algebra();
        for composites in self.composite.iter() {
            buffer.write_u64::<LittleEndian>(composites.len() as u64)?;
            for composite in composites {
                algebra.element_to_bytes(composite, buffer)?;
            }
        }
        Ok(())
    }

    pub fn from_bytes(
        target: Arc<FreeModule<A>>,
        degree: i32,
        buffer: &mut impl Read,
    ) -> std::io::Result<Self> {
        let min_degree = target.min_degree();
        let algebra = target.algebra();
        let mut composite = BiVec::with_capacity(min_degree, degree);

        for t in min_degree..degree {
            let num_gens = buffer.read_u64::<LittleEndian>()? as usize;
            let mut c = Vec::with_capacity(num_gens);
            for _ in 0..num_gens {
                c.push(algebra.element_from_bytes(degree - t, buffer)?);
            }
            composite.push(c);
        }

        Ok(Self {
            target,
            degree,
            composite,
        })
    }

    pub fn finalize(&mut self) {
        for r in self.composite.iter_mut() {
            for r in r.iter_mut() {
                A::finalize_element(r);
            }
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
        for (gen_deg, row) in self.composite.iter_enum() {
            let module_op_deg = self.degree - gen_deg;
            for (gen_idx, c) in row.iter().enumerate() {
                if gen_deg > self.target.max_computed_degree() {
                    // If we are resolving up to a stem then the target might be missing some
                    // degrees. This is fine but we want to assert that c is zero.
                    assert!(A::element_is_zero(c));
                    continue;
                }

                let offset =
                    self.target
                        .generator_offset(self.degree + op_degree - 1, gen_deg, gen_idx);
                let len = Algebra::dimension(&*algebra, module_op_deg + op_degree - 1, 0);

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

    /// gen_deg -> gen_idx -> composite
    pub(crate) composites: OnceBiVec<Vec<SecondaryComposite<A>>>,

    /// gen_deg -> gen_idx -> homotopy
    pub homotopies: FreeModuleHomomorphism<FreeModule<A>>,
}

impl<A: PairAlgebra + Send + Sync> SecondaryHomotopy<A> {
    pub fn new(source: Arc<FreeModule<A>>, target: Arc<FreeModule<A>>, shift_t: i32) -> Self {
        Self {
            composites: OnceBiVec::new(std::cmp::max(
                source.min_degree(),
                target.min_degree() + shift_t,
            )),
            homotopies: FreeModuleHomomorphism::new(
                Arc::clone(&source),
                Arc::clone(&target),
                shift_t + 1,
            ),
            source,
            target,
            shift_t,
        }
    }

    /// Add composites up to and including the specified degree
    pub fn add_composite(
        &self,
        s: u32,
        degree: i32,
        maps: &[(
            u32,
            &FreeModuleHomomorphism<FreeModule<A>>,
            &FreeModuleHomomorphism<FreeModule<A>>,
        )],
        dir: Option<&Path>,
    ) {
        for (_, d1, d0) in maps {
            assert!(Arc::ptr_eq(&d1.target(), &d0.source()));
            assert!(Arc::ptr_eq(&d0.target(), &self.target));
            assert_eq!(d1.degree_shift() + d0.degree_shift(), self.shift_t);
        }

        let f = |t, idx| {
            let save_file = SaveFile {
                algebra: self.target.algebra(),
                kind: SaveKind::SecondaryComposite,
                s,
                t,
                idx: Some(idx),
            };
            if let Some(dir) = dir {
                if let Some(mut f) = save_file.open_file(dir.to_owned()) {
                    return SecondaryComposite::from_bytes(
                        Arc::clone(&self.target),
                        t - self.shift_t,
                        &mut f,
                    )
                    .unwrap();
                }
            }

            let mut composite = SecondaryComposite::new(Arc::clone(&self.target), t - self.shift_t);
            for (coef, d1, d0) in maps {
                composite.add_composite(*coef, t, idx, d1, d0);
            }
            composite.finalize();

            if let Some(dir) = dir {
                let mut f = save_file.create_file(dir.to_owned());
                composite.to_bytes(&mut f).unwrap();
            }

            composite
        };

        #[cfg(not(feature = "concurrent"))]
        self.composites.extend(degree, |t| {
            (0..self.source.number_of_gens_in_degree(t))
                .map(|i| f(t, i))
                .collect()
        });

        #[cfg(feature = "concurrent")]
        self.composites.par_extend(degree, |t| {
            (0..self.source.number_of_gens_in_degree(t))
                .into_par_iter()
                .map(|i| f(t, i))
                .collect()
        });
    }

    /// Compute the image of an element in the source under the homotopy, writing the result in
    /// `result`. It is assumed that the coefficients of generators are zero in `op`.
    ///
    /// # Arguments
    ///  - full: Whether to include the action of the homotopy part as well
    pub fn act(&self, mut result: SliceMut, coeff: u32, elt_degree: i32, elt: Slice, full: bool) {
        for (gen_deg, gen_idx, op_deg, slice) in self.source.iter_slices(elt_degree, elt) {
            if gen_deg < self.composites.min_degree() {
                continue;
            }
            // This is actually necessary. We don't have the homotopies on the
            // generators at the edge of the resolution, but we don't need them since they never
            // get hit.
            if slice.is_zero() {
                continue;
            }
            self.composites[gen_deg][gen_idx].act(result.copy(), coeff, op_deg, slice);
        }

        if full {
            self.homotopies.apply(result, coeff, elt_degree, elt);
        }
    }

    pub fn composite(&self, gen_deg: i32, gen_idx: usize) -> &SecondaryComposite<A> {
        &self.composites[gen_deg][gen_idx]
    }
}

pub struct SecondaryLift<A: PairAlgebra, CC: FreeChainComplex<Algebra = A>> {
    pub chain_complex: Arc<CC>,
    /// s -> t -> idx -> homotopy
    pub(crate) homotopies: OnceBiVec<SecondaryHomotopy<A>>,
    intermediates: DashMap<(u32, i32, usize), FpVector>,
}

impl<A: PairAlgebra + Send + Sync, CC: FreeChainComplex<Algebra = A>> SecondaryLift<A, CC> {
    pub fn new(cc: Arc<CC>) -> Self {
        if let Some(p) = cc.save_dir() {
            let mut p = p.to_owned();

            for subdir in SaveKind::secondary_data() {
                p.push(format!("{}s", subdir.name()));
                if !p.exists() {
                    std::fs::create_dir_all(&p)
                        .with_context(|| format!("Failed to create directory {p:?}"))
                        .unwrap();
                } else if !p.is_dir() {
                    panic!("{p:?} is not a directory");
                }
                p.pop();
            }
        }

        Self {
            chain_complex: cc,
            homotopies: OnceBiVec::new(2),
            intermediates: DashMap::new(),
        }
    }

    pub fn algebra(&self) -> Arc<A> {
        self.chain_complex.algebra()
    }

    pub fn initialize_homotopies(&self) {
        self.homotopies.extend(
            self.chain_complex.next_homological_degree() as i32 - 1,
            |s| {
                SecondaryHomotopy::new(
                    self.chain_complex.module(s as u32),
                    self.chain_complex.module(s as u32 - 2),
                    0,
                )
            },
        );
    }

    pub fn compute_composites(&self) {
        let max_t = |s| {
            std::cmp::min(
                self.chain_complex.module(s).max_computed_degree(),
                self.chain_complex.module(s - 2).max_computed_degree() + 1,
            )
        };

        let f = |s| {
            let s = s as u32;
            let d1 = &*self.chain_complex.differential(s);
            let d0 = &*self.chain_complex.differential(s - 1);
            self.homotopies[s as i32].add_composite(
                s,
                max_t(s),
                &[(1, d1, d0)],
                self.chain_complex.save_dir(),
            );
        };

        #[cfg(not(feature = "concurrent"))]
        self.homotopies.range().for_each(f);

        #[cfg(feature = "concurrent")]
        self.homotopies.range().into_par_iter().for_each(f);
    }

    pub fn get_intermediate(&self, s: u32, t: i32, idx: usize) -> FpVector {
        if let Some((_, v)) = self.intermediates.remove(&(s, t, idx)) {
            return v;
        }
        let p = self.chain_complex.prime();
        let target = self.chain_complex.module(s as u32 - 3);

        let save_file = SaveFile {
            algebra: self.chain_complex.algebra(),
            kind: SaveKind::SecondaryIntermediate,
            s,
            t,
            idx: Some(idx),
        };

        if let Some(dir) = self.chain_complex.save_dir() {
            if let Some(mut f) = save_file.open_file(dir.to_owned()) {
                // The target dimension can depend on whether we resolved to stem
                let dim = f.read_u64::<LittleEndian>().unwrap() as usize;
                return FpVector::from_bytes(p, dim, &mut f).unwrap();
            }
        }

        let mut result = FpVector::new(p, target.dimension(t - 1));
        let d = self.chain_complex.differential(s as u32);
        self.homotopies[s as i32 - 1].act(
            result.as_slice_mut(),
            1,
            t,
            d.output(t, idx).as_slice(),
            false,
        );

        if let Some(dir) = self.chain_complex.save_dir() {
            let mut f = save_file.create_file(dir.to_owned());
            f.write_u64::<LittleEndian>(result.len() as u64).unwrap();
            result.to_bytes(&mut f).unwrap();
        }

        result
    }

    pub fn compute_intermediates(&self) {
        let f = |s, t, i| {
            // If we already have homotopies, we don't need to compute intermediate
            if self.homotopies[s as i32].homotopies.next_degree() >= t {
                return;
            }
            // Check if we have a saved homotopy
            if let Some(dir) = self.chain_complex.save_dir() {
                let save_file = self
                    .chain_complex
                    .save_file(SaveKind::SecondaryHomotopy, s, t);
                if save_file.exists(dir.to_owned()) {
                    return;
                }
            }
            self.intermediates
                .insert((s, t, i), self.get_intermediate(s, t, i));
        };

        #[cfg(not(feature = "concurrent"))]
        for (s, homotopy) in self.homotopies.iter_enum().skip(1) {
            let s = s as u32;
            for t in homotopy.composites.range() {
                for i in 0..homotopy.source.number_of_gens_in_degree(t) {
                    f(s, t, i)
                }
            }
        }

        #[cfg(feature = "concurrent")]
        self.homotopies
            .par_iter_enum()
            .skip(1)
            .for_each(|(s, homotopy)| {
                let s = s as u32;

                homotopy.composites.range().into_par_iter().for_each(|t| {
                    (0..homotopy.source.number_of_gens_in_degree(t))
                        .into_par_iter()
                        .for_each(|i| f(s, t, i))
                })
            })
    }

    pub fn compute_homotopy_step(&self, s: u32, t: i32) {
        match self.homotopies[s as i32].homotopies.next_degree().cmp(&t) {
            std::cmp::Ordering::Less => panic!("Not yet ready to compute {t}"),
            std::cmp::Ordering::Equal => (),
            std::cmp::Ordering::Greater => return,
        };

        let p = self.chain_complex.prime();
        let source = self.chain_complex.module(s);
        let d = self.chain_complex.differential(s);
        let num_gens = source.number_of_gens_in_degree(t);
        let target_dim = self.chain_complex.module(s as u32 - 2).dimension(t - 1);

        if let Some(dir) = self.chain_complex.save_dir() {
            let save_file = self
                .chain_complex
                .save_file(SaveKind::SecondaryHomotopy, s, t);
            if let Some(mut f) = save_file.open_file(dir.to_owned()) {
                let mut results = Vec::with_capacity(num_gens);
                for _ in 0..num_gens {
                    results.push(FpVector::from_bytes(p, target_dim, &mut f).unwrap());
                }
                self.homotopies[s as i32]
                    .homotopies
                    .add_generators_from_rows(t, results);
                return;
            }
        }

        let intermediates: Vec<FpVector> = (0..num_gens)
            .map(|i| {
                let mut v = self.get_intermediate(s, t, i);
                if s > 3 {
                    self.homotopies[s as i32 - 1].homotopies.apply(
                        v.as_slice_mut(),
                        1,
                        t,
                        d.output(t, i).as_slice(),
                    );
                }
                v
            })
            .collect();
        let mut results = vec![FpVector::new(p, target_dim); num_gens];

        assert!(self.chain_complex.apply_quasi_inverse(
            &mut results,
            s as u32 - 2,
            t - 1,
            &intermediates,
        ));

        if let Some(dir) = self.chain_complex.save_dir() {
            let save_file = self
                .chain_complex
                .save_file(SaveKind::SecondaryHomotopy, s, t);

            let mut f = save_file.create_file(dir.to_owned());
            for row in &results {
                row.to_bytes(&mut f).unwrap();
            }
            drop(f);

            let mut save_file = SaveFile {
                algebra: self.chain_complex.algebra(),
                kind: SaveKind::SecondaryIntermediate,
                s,
                t,
                idx: None,
            };

            for i in 0..num_gens {
                save_file.idx = Some(i);
                save_file.delete_file(dir.to_owned()).unwrap();
            }
        }

        self.homotopies[s as i32]
            .homotopies
            .add_generators_from_rows(t, results);
    }

    pub fn compute_homotopies(&self) {
        // When s = 2, the homotopies are just zero
        {
            let h2 = &self.homotopies[2];
            h2.homotopies.extend_by_zero(h2.composites.max_degree());
        }

        for (s, homotopy) in self.homotopies.iter_enum().skip(1) {
            for t in homotopy.homotopies.next_degree()..homotopy.composites.len() {
                self.compute_homotopy_step(s as u32, t);
            }
        }
    }

    #[cfg(feature = "concurrent")]
    pub fn compute_homotopies_concurrent(&self, bucket: &TokenBucket) {
        // When s = 2, the homotopies are just zero
        {
            let h2 = &self.homotopies[2];
            h2.homotopies.extend_by_zero(h2.composites.max_degree());
        }

        let min_t = self.homotopies[2].homotopies.min_degree();
        let max_t = |s| self.homotopies[s as i32].composites.len();

        let s_range = self.homotopies.range();
        bucket.iter_s_t(
            s_range.start as u32 + 1..s_range.end as u32,
            min_t,
            max_t,
            (),
            |s, t, _| self.compute_homotopy_step(s, t),
        )
    }

    pub fn homotopy(&self, s: u32) -> &SecondaryHomotopy<A> {
        &self.homotopies[s as i32]
    }
}

// Rustdoc ICE's when trying to document this struct. See
// https://github.com/rust-lang/rust/issues/91380
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
    intermediates: DashMap<(u32, i32, usize), FpVector>,
}

impl<
        A: PairAlgebra + Send + Sync,
        CC1: FreeChainComplex<Algebra = A>,
        CC2: FreeChainComplex<Algebra = A> + AugmentedChainComplex,
    > SecondaryResolutionHomomorphism<A, CC1, CC2>
{
    pub fn new(
        source: Arc<SecondaryLift<A, CC1>>,
        target: Arc<SecondaryLift<A, CC2>>,
        underlying: Arc<ResolutionHomomorphism<CC1, CC2>>,
    ) -> Self {
        assert!(Arc::ptr_eq(&underlying.source, &source.chain_complex));
        assert!(Arc::ptr_eq(&underlying.target, &target.chain_complex));

        if let Some(p) = underlying.save_dir() {
            let mut p = p.to_owned();

            for subdir in SaveKind::secondary_data() {
                p.push(format!("{}s", subdir.name()));
                if !p.exists() {
                    std::fs::create_dir_all(&p)
                        .with_context(|| format!("Failed to create directory {p:?}"))
                        .unwrap();
                } else if !p.is_dir() {
                    panic!("{p:?} is not a directory");
                }
                p.pop();
            }
        }

        Self {
            source,
            target,
            homotopies: OnceBiVec::new(underlying.shift_s as i32 + 1),
            underlying,
            intermediates: DashMap::new(),
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
        self.homotopies.extend(max_s as i32 - 1, |s| {
            let s = s as u32;
            SecondaryHomotopy::new(
                self.source.chain_complex.module(s),
                self.target.chain_complex.module(s - shift_s - 1),
                shift_t,
            )
        });
    }

    fn max_t(&self, s: u32) -> i32 {
        let shift_s = self.shift_s();
        let shift_t = self.shift_t();
        std::cmp::min(
            self.underlying.get_map(s).next_degree(),
            std::cmp::min(
                self.source.homotopies[s as i32].homotopies.next_degree(),
                if s == shift_s + 1 {
                    i32::MAX
                } else {
                    self.target.homotopies[(s - shift_s) as i32]
                        .composites
                        .max_degree()
                        + shift_t
                        + 1
                },
            ),
        )
    }

    pub fn compute_composites(&self) {
        let shift_s = self.shift_s();
        let p = *self.underlying.source.prime();
        // This is -1 mod p^2
        let neg_1 = p * p - 1;

        let mut range = self.homotopies.range();
        range.end -= 1;

        let f = |s| {
            let s = s as u32;
            let d_source = &*self.source.chain_complex.differential(s);
            let d_target = &*self.target.chain_complex.differential(s - shift_s);

            let c1 = &*self.underlying.get_map(s);
            let c0 = &*self.underlying.get_map(s - 1);

            self.homotopies[s as i32].add_composite(
                s,
                self.max_t(s) - 1,
                &[(neg_1, d_source, c0), (1, c1, d_target)],
                self.underlying.save_dir(),
            );
        };

        #[cfg(feature = "concurrent")]
        range.into_par_iter().for_each(f);

        #[cfg(not(feature = "concurrent"))]
        range.into_iter().for_each(f)
    }

    pub fn get_intermediate(&self, s: u32, t: i32, idx: usize) -> FpVector {
        if let Some((_, v)) = self.intermediates.remove(&(s, t, idx)) {
            return v;
        }
        let shift_s = self.shift_s();
        let shift_t = self.shift_t();

        let p = self.target.chain_complex.prime();
        let target = self.target.chain_complex.module(s as u32 - shift_s - 2);

        let save_file = SaveFile {
            algebra: self.underlying.algebra(),
            kind: SaveKind::SecondaryIntermediate,
            s,
            t,
            idx: Some(idx),
        };

        if let Some(dir) = self.underlying.save_dir() {
            if let Some(mut f) = save_file.open_file(dir.to_owned()) {
                // The target dimension can depend on whether we resolved to stem
                let dim = f.read_u64::<LittleEndian>().unwrap() as usize;
                return FpVector::from_bytes(p, dim, &mut f).unwrap();
            }
        }

        let mut result = FpVector::new(p, Module::dimension(&*target, t - 1 - shift_t));
        let d = self.source.chain_complex.differential(s);

        self.homotopies[s as i32 - 1].act(
            result.as_slice_mut(),
            1,
            t,
            d.output(t, idx).as_slice(),
            false,
        );
        self.target.homotopy(s - shift_s).act(
            result.as_slice_mut(),
            1,
            t - shift_t,
            self.underlying.get_map(s).output(t, idx).as_slice(),
            true,
        );

        self.underlying.get_map(s - 2).apply(
            result.as_slice_mut(),
            1,
            t - 1,
            self.source.homotopy(s).homotopies.output(t, idx).as_slice(),
        );

        if let Some(dir) = self.underlying.save_dir() {
            let mut f = save_file.create_file(dir.to_owned());
            f.write_u64::<LittleEndian>(result.len() as u64).unwrap();
            result.to_bytes(&mut f).unwrap();
        }

        result
    }

    pub fn compute_intermediates(&self) {
        let f = |s, t, i| {
            // If we already have homotopies, we don't need to compute intermediate
            if self.homotopies[s as i32].homotopies.next_degree() >= t {
                return;
            }
            // Check if we have a saved homotopy
            if let Some(dir) = self.underlying.save_dir() {
                let save_file = self
                    .underlying
                    .source
                    .save_file(SaveKind::SecondaryHomotopy, s, t);
                if save_file.exists(dir.to_owned()) {
                    return;
                }
            }
            self.intermediates
                .insert((s, t, i), self.get_intermediate(s, t, i));
        };

        #[cfg(feature = "concurrent")]
        self.homotopies
            .par_iter_enum()
            .skip(1)
            .for_each(|(s, homotopy)| {
                let s = s as u32;
                homotopy
                    .composites
                    .range()
                    .into_par_iter()
                    .skip(1)
                    .for_each(|t| {
                        (0..homotopy.source.number_of_gens_in_degree(t))
                            .into_par_iter()
                            .for_each(|i| f(s, t, i))
                    })
            });

        #[cfg(not(feature = "concurrent"))]
        for (s, homotopy) in self.homotopies.iter_enum().skip(1) {
            let s = s as u32;
            for t in homotopy.composites.range().skip(1) {
                for i in 0..homotopy.source.number_of_gens_in_degree(t) {
                    f(s, t, i);
                }
            }
        }
    }

    pub fn compute_homotopy_step(&self, s: u32, t: i32) {
        let homotopy = &self.homotopies[s as i32];
        match homotopy.homotopies.next_degree().cmp(&t) {
            std::cmp::Ordering::Less => panic!("Not yet ready to compute {t}"),
            std::cmp::Ordering::Equal => (),
            std::cmp::Ordering::Greater => return,
        };
        let p = self.source.chain_complex.prime();
        let shift_s = self.shift_s();
        let shift_t = self.shift_t();
        let d = self.source.chain_complex.differential(s);
        let source = self.source.chain_complex.module(s);
        let num_gens = source.number_of_gens_in_degree(t);
        let target_dim = self
            .target
            .chain_complex
            .module(s as u32 - shift_s - 1)
            .dimension(t - shift_t - 1);

        if let Some(dir) = self.underlying.save_dir() {
            let save_file = self
                .underlying
                .source
                .save_file(SaveKind::SecondaryHomotopy, s, t);
            if let Some(mut f) = save_file.open_file(dir.to_owned()) {
                let mut results = Vec::with_capacity(num_gens);
                for _ in 0..num_gens {
                    results.push(FpVector::from_bytes(p, target_dim, &mut f).unwrap());
                }
                self.homotopies[s as i32]
                    .homotopies
                    .add_generators_from_rows(t, results);
                return;
            }
        }

        let intermediates: Vec<FpVector> = (0..num_gens)
            .map(|i| {
                let mut v = self.get_intermediate(s, t, i);
                if s > shift_s + 2 {
                    self.homotopies[s as i32 - 1].homotopies.apply(
                        v.as_slice_mut(),
                        1,
                        t,
                        d.output(t, i).as_slice(),
                    );
                }
                v
            })
            .collect();
        let mut results = vec![FpVector::new(p, target_dim); num_gens];

        assert!(self.target.chain_complex.apply_quasi_inverse(
            &mut results,
            s as u32 - shift_s - 1,
            t - shift_t - 1,
            &intermediates,
        ));

        if let Some(dir) = self.underlying.save_dir() {
            let save_file = self
                .underlying
                .source
                .save_file(SaveKind::SecondaryHomotopy, s, t);

            let mut f = save_file.create_file(dir.to_owned());
            for row in &results {
                row.to_bytes(&mut f).unwrap();
            }
            drop(f);

            let mut save_file = SaveFile {
                algebra: self.underlying.algebra(),
                kind: SaveKind::SecondaryIntermediate,
                s,
                t,
                idx: None,
            };

            for i in 0..num_gens {
                save_file.idx = Some(i);
                save_file.delete_file(dir.to_owned()).unwrap();
            }
        }

        homotopy.homotopies.add_generators_from_rows(t, results);
    }

    pub fn compute_homotopies(&self) {
        let shift_s = self.shift_s();

        // When s = shift_s + 1, the homotopies are just zero
        {
            let h = &self.homotopies[shift_s as i32 + 1];
            h.homotopies.extend_by_zero(h.composites.max_degree());
        }

        for (s, homotopy) in self.homotopies.iter_enum().skip(1) {
            let s = s as u32;

            for t in homotopy.homotopies.next_degree()..self.max_t(s) {
                self.compute_homotopy_step(s, t);
            }
        }
    }

    #[cfg(feature = "concurrent")]
    pub fn compute_homotopies_concurrent(&self, bucket: &TokenBucket) {
        let shift_s = self.shift_s();

        // When s = shift_s + 1, the homotopies are just zero
        {
            let h = &self.homotopies[shift_s as i32 + 1];
            h.homotopies.extend_by_zero(h.composites.max_degree());
        }

        let min_t = self.homotopies[shift_s as i32 + 1].homotopies.min_degree();

        let s_range = self.homotopies.range();
        bucket.iter_s_t(
            s_range.start as u32 + 1..s_range.end as u32,
            min_t,
            |s| self.max_t(s),
            (),
            |s, t, _| self.compute_homotopy_step(s, t),
        )
    }

    pub fn homotopy(&self, s: u32) -> &SecondaryHomotopy<A> {
        &self.homotopies[s as i32]
    }
}

/// Whether picking δ₂ = 0 gives a valid secondary refinement. This requires
///  1. The chain complex is concentrated in degree zero;
///  2. The module is finite dimensional; and
///  3. $\mathrm{Hom}(\mathrm{Ext}^{2, t}_A(H^*X, k), H^{t - 1} X) = 0$ for all $t$ or $\mathrm{Hom}(\mathrm{Ext}^{3, t}_A(H^*X, k), H^{t - 1} X) = 0$ for all $t$.
pub fn can_compute(res: &Resolution<CCC>) -> bool {
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
    use std::fmt::Write;

    #[test]
    fn test_compute_differentials() {
        let mut result = String::new();
        let resolution = construct("S_2@milnor", None).unwrap();

        let max_s = 7;
        let max_t = 30;

        resolution.compute_through_bidegree(max_s, max_t);

        let lift = SecondaryLift::new(Arc::new(resolution));
        lift.initialize_homotopies();
        lift.compute_composites();
        lift.compute_homotopies();

        // Iterate through the bidegree of the source of the differential.
        for (s, n, t) in lift.chain_complex.iter_stem() {
            if !lift.chain_complex.has_computed_bidegree(s + 2, t + 1) {
                continue;
            }
            let homotopy = lift.homotopy(s + 2);

            let source_num_gens = homotopy.source.number_of_gens_in_degree(t + 1);
            let target_num_gens = homotopy.target.number_of_gens_in_degree(t);
            if source_num_gens == 0 || target_num_gens == 0 {
                continue;
            }
            let entries = homotopy.homotopies.hom_k(t);

            for (k, row) in entries.iter().enumerate() {
                writeln!(&mut result, "d_2 x_({n}, {s}, {k}) = {row:?}",).unwrap();
            }
        }

        expect![[r#"
            d_2 x_(1, 1, 0) = [0]
            d_2 x_(8, 2, 0) = [0]
            d_2 x_(15, 1, 0) = [1]
            d_2 x_(15, 2, 0) = [0]
            d_2 x_(15, 3, 0) = [0]
            d_2 x_(15, 4, 0) = [0]
            d_2 x_(16, 2, 0) = [0]
            d_2 x_(17, 4, 0) = [1]
            d_2 x_(17, 5, 0) = [0]
            d_2 x_(18, 2, 0) = [0]
            d_2 x_(18, 3, 0) = [0]
            d_2 x_(18, 4, 0) = [0]
            d_2 x_(18, 4, 1) = [1]
            d_2 x_(18, 5, 0) = [1]
            d_2 x_(19, 3, 0) = [0]
            d_2 x_(21, 3, 0) = [0]
            d_2 x_(24, 5, 0) = [0]
        "#]]
        .assert_eq(&result);
    }
}
