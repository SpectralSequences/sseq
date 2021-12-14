use crate::chain_complex::{BoundedChainComplex, ChainComplex, FreeChainComplex};
use crate::resolution::Resolution;
use crate::resolution_homomorphism::ResolutionHomomorphism;
use crate::save::{SaveFile, SaveKind};

use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::{BoundedModule, FreeModule, Module};
use algebra::pair_algebra::PairAlgebra;
use algebra::Algebra;
use bivec::BiVec;
use fp::matrix::Matrix;
use fp::prime::ValidPrime;
use fp::vector::{FpVector, Slice, SliceMut};
use once::OnceBiVec;

use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

use crate::CCC;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use dashmap::DashMap;
#[cfg(feature = "concurrent")]
use rayon::prelude::*;

pub type CompositeData<A> = Vec<(
    u32,
    Arc<FreeModuleHomomorphism<FreeModule<A>>>,
    Arc<FreeModuleHomomorphism<FreeModule<A>>>,
)>;

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
    pub fn add_composite(&self, s: u32, degree: i32, maps: CompositeData<A>, dir: Option<&Path>) {
        for (_, d1, d0) in &maps {
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
            for (coef, d1, d0) in &maps {
                composite.add_composite(*coef, t, idx, &*d1, &*d0);
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

/// When lifting a thing to its secondary version, often what we have to do is to specify an
/// explicit homotopy to witnesses that some equation holds. For example, to lift a chain complex,
/// we need a homotopy witnessing the fact that $d^2 \simeq 0$. This homotopy in turn is required
/// to satisfy certain recursive relations.
///
/// To specify this lifting problem, one needs to supply two pieces of data. First is the equation
/// that we are trying to witness, which is usually of the form
///
/// $$ \sum_i c_i f_i g_i = 0, $$
///
/// where $f_i$ and $g_i$ are free module homomorphisms and $c_i$ are constants. This is specified
/// by [`SecondaryLift::composite`].
///
/// The next is a compatibility equation, which restricts the τ part of the null-homotopy, and is
/// usually of the form
///
/// $$ dh = hd + \mathrm{stuff} $$
///
/// The τ part of $hd + \mathrm{stuff}$ is known as the intermediate data, and is what
/// [`SecondaryLift::compute_intermediate`] returns.
pub trait SecondaryLift: Sync {
    type Algebra: PairAlgebra;
    type Source: FreeChainComplex<Algebra = Self::Algebra>;
    type Target: FreeChainComplex<Algebra = Self::Algebra>;
    type Underlying;

    fn underlying(&self) -> Arc<Self::Underlying>;
    fn algebra(&self) -> Arc<Self::Algebra>;
    fn prime(&self) -> ValidPrime {
        self.algebra().prime()
    }

    fn source(&self) -> Arc<Self::Source>;
    fn target(&self) -> Arc<Self::Target>;
    fn shift_t(&self) -> i32;
    fn shift_s(&self) -> u32;

    /// Exclusive max s
    fn max_s(&self) -> u32;

    /// Exclusive max t
    fn max_t(&self, s: u32) -> i32;

    fn homotopies(&self) -> &OnceBiVec<SecondaryHomotopy<Self::Algebra>>;
    fn intermediates(&self) -> &DashMap<(u32, i32, usize), FpVector>;

    fn save_dir(&self) -> Option<&Path>;

    fn compute_intermediate(&self, s: u32, t: i32, idx: usize) -> FpVector;
    fn composite(&self, s: u32) -> CompositeData<Self::Algebra>;

    fn initialize_homotopies(&self) {
        let shift_s = self.shift_s();
        let shift_t = self.shift_t();

        let max_s = self.max_s();
        self.homotopies().extend(max_s as i32 - 1, |s| {
            let s = s as u32;
            SecondaryHomotopy::new(
                self.source().module(s),
                self.target().module(s - shift_s),
                shift_t,
            )
        });
    }

    fn compute_composites(&self) {
        let f = |s| {
            let s = s as u32;
            self.homotopies()[s as i32].add_composite(
                s,
                self.max_t(s) - 1,
                self.composite(s),
                self.save_dir(),
            );
        };

        #[cfg(not(feature = "concurrent"))]
        self.homotopies().range().for_each(f);

        #[cfg(feature = "concurrent")]
        self.homotopies().range().into_par_iter().for_each(f);
    }

    fn get_intermediate(&self, s: u32, t: i32, idx: usize) -> FpVector {
        if let Some((_, v)) = self.intermediates().remove(&(s, t, idx)) {
            return v;
        }

        let save_file = SaveFile {
            algebra: self.algebra(),
            kind: SaveKind::SecondaryIntermediate,
            s,
            t,
            idx: Some(idx),
        };

        if let Some(dir) = self.save_dir() {
            if let Some(mut f) = save_file.open_file(dir.to_owned()) {
                // The target dimension can depend on whether we resolved to stem
                let dim = f.read_u64::<LittleEndian>().unwrap() as usize;
                return FpVector::from_bytes(self.prime(), dim, &mut f).unwrap();
            }
        }

        let result = self.compute_intermediate(s, t, idx);

        if let Some(dir) = self.save_dir() {
            let mut f = save_file.create_file(dir.to_owned());
            f.write_u64::<LittleEndian>(result.len() as u64).unwrap();
            result.to_bytes(&mut f).unwrap();
        }

        result
    }

    fn compute_intermediates(&self) {
        let f = |s, t, i| {
            // If we already have homotopies, we don't need to compute intermediate
            if self.homotopies()[s as i32].homotopies.next_degree() >= t {
                return;
            }
            // Check if we have a saved homotopy
            if let Some(dir) = self.save_dir() {
                let save_file = SaveFile {
                    algebra: self.algebra(),
                    kind: SaveKind::SecondaryHomotopy,
                    s,
                    t,
                    idx: None,
                };

                if save_file.exists(dir.to_owned()) {
                    return;
                }
            }
            self.intermediates()
                .insert((s, t, i), self.get_intermediate(s, t, i));
        };

        #[cfg(not(feature = "concurrent"))]
        for (s, homotopy) in self.homotopies().iter_enum().skip(1) {
            let s = s as u32;
            for t in homotopy.composites.range() {
                for i in 0..homotopy.source.number_of_gens_in_degree(t) {
                    f(s, t, i)
                }
            }
        }

        #[cfg(feature = "concurrent")]
        self.homotopies()
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

    fn compute_homotopy_step(&self, s: u32, t: i32) -> std::ops::Range<i32> {
        let homotopy = &self.homotopies()[s as i32];
        if homotopy.homotopies.next_degree() > t {
            return t..t + 1;
        }
        let p = self.prime();
        let shift_s = self.shift_s();
        let shift_t = self.shift_t();

        let d = self.source().differential(s);
        let source = self.source().module(s);
        let num_gens = source.number_of_gens_in_degree(t);
        let target_dim = self
            .target()
            .module(s as u32 - shift_s)
            .dimension(t - shift_t - 1);

        if let Some(dir) = self.save_dir() {
            let save_file = SaveFile {
                algebra: self.algebra(),
                kind: SaveKind::SecondaryHomotopy,
                s,
                t,
                idx: None,
            };

            if let Some(mut f) = save_file.open_file(dir.to_owned()) {
                let mut results = Vec::with_capacity(num_gens);
                for _ in 0..num_gens {
                    results.push(FpVector::from_bytes(p, target_dim, &mut f).unwrap());
                }
                return self.homotopies()[s as i32]
                    .homotopies
                    .add_generators_from_rows_ooo(t, results);
            }
        }

        let get_intermediate = |i| {
            let mut v = self.get_intermediate(s, t, i);
            if s > shift_s + 1 {
                self.homotopies()[s as i32 - 1].homotopies.apply(
                    v.as_slice_mut(),
                    1,
                    t,
                    d.output(t, i).as_slice(),
                );
            }
            v
        };

        #[cfg(feature = "concurrent")]
        let intermediates: Vec<FpVector> = (0..num_gens)
            .into_par_iter()
            .map(get_intermediate)
            .collect();

        #[cfg(not(feature = "concurrent"))]
        let intermediates: Vec<FpVector> =
            (0..num_gens).into_iter().map(get_intermediate).collect();

        let mut results = vec![FpVector::new(p, target_dim); num_gens];

        assert!(self.target().apply_quasi_inverse(
            &mut results,
            s as u32 - shift_s,
            t - shift_t - 1,
            &intermediates,
        ));

        if let Some(dir) = self.save_dir() {
            let save_file = SaveFile {
                algebra: self.algebra(),
                kind: SaveKind::SecondaryHomotopy,
                s,
                t,
                idx: None,
            };

            let mut f = save_file.create_file(dir.to_owned());
            for row in &results {
                row.to_bytes(&mut f).unwrap();
            }
            drop(f);

            let mut save_file = SaveFile {
                algebra: self.algebra(),
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

        homotopy.homotopies.add_generators_from_rows_ooo(t, results)
    }

    fn compute_homotopies(&self) {
        let shift_s = self.shift_s();

        // When s = shift_s, the homotopies are just zero
        {
            let h = &self.homotopies()[shift_s as i32];
            h.homotopies.extend_by_zero(h.composites.max_degree());
        }

        #[cfg(not(feature = "concurrent"))]
        for (s, homotopy) in self.homotopies().iter_enum().skip(1) {
            let s = s as u32;

            for t in homotopy.homotopies.next_degree()..self.max_t(s) {
                self.compute_homotopy_step(s, t);
            }
        }

        #[cfg(feature = "concurrent")]
        {
            let min_t = self.homotopies()[shift_s as i32].homotopies.min_degree();
            let s_range = self.homotopies().range();
            crate::utils::iter_s_t(
                &|s, t| self.compute_homotopy_step(s, t),
                s_range.start as u32 + 1,
                min_t,
                s_range.end as u32,
                &|s| self.max_t(s),
            );
        }
    }

    fn extend_all(&self) {
        self.initialize_homotopies();
        self.compute_composites();
        #[cfg(feature = "concurrent")]
        self.compute_intermediates();
        self.compute_homotopies();
    }
}

impl<A: PairAlgebra + Send + Sync, CC: FreeChainComplex<Algebra = A>> SecondaryLift
    for SecondaryResolution<A, CC>
{
    type Algebra = A;
    type Source = CC;
    type Target = CC;
    type Underlying = CC;

    fn underlying(&self) -> Arc<CC> {
        Arc::clone(&self.underlying)
    }

    fn algebra(&self) -> Arc<Self::Algebra> {
        self.underlying.algebra()
    }

    fn source(&self) -> Arc<Self::Source> {
        Arc::clone(&self.underlying)
    }

    fn target(&self) -> Arc<Self::Target> {
        Arc::clone(&self.underlying)
    }

    fn shift_s(&self) -> u32 {
        2
    }

    fn shift_t(&self) -> i32 {
        0
    }

    fn max_s(&self) -> u32 {
        self.underlying.next_homological_degree() as u32
    }

    fn max_t(&self, s: u32) -> i32 {
        std::cmp::min(
            self.underlying.module(s).max_computed_degree(),
            self.underlying.module(s - 2).max_computed_degree() + 1,
        ) + 1
    }

    fn homotopies(&self) -> &OnceBiVec<SecondaryHomotopy<Self::Algebra>> {
        &self.homotopies
    }

    fn intermediates(&self) -> &DashMap<(u32, i32, usize), FpVector> {
        &self.intermediates
    }

    fn save_dir(&self) -> Option<&Path> {
        self.underlying.save_dir()
    }

    fn composite(&self, s: u32) -> CompositeData<Self::Algebra> {
        let d1 = self.underlying.differential(s);
        let d0 = self.underlying.differential(s - 1);
        vec![(1, d1, d0)]
    }

    fn compute_intermediate(&self, s: u32, t: i32, idx: usize) -> FpVector {
        let p = self.prime();
        let target = self.underlying.module(s - 3);
        let mut result = FpVector::new(p, target.dimension(t - 1));
        let d = self.underlying.differential(s);
        self.homotopies[s as i32 - 1].act(
            result.as_slice_mut(),
            1,
            t,
            d.output(t, idx).as_slice(),
            false,
        );
        result
    }
}

pub struct SecondaryResolution<A: PairAlgebra, CC: FreeChainComplex<Algebra = A>> {
    underlying: Arc<CC>,
    /// s -> t -> idx -> homotopy
    pub(crate) homotopies: OnceBiVec<SecondaryHomotopy<A>>,
    intermediates: DashMap<(u32, i32, usize), FpVector>,
}

impl<A: PairAlgebra + Send + Sync, CC: FreeChainComplex<Algebra = A>> SecondaryResolution<A, CC> {
    pub fn new(cc: Arc<CC>) -> Self {
        if let Some(p) = cc.save_dir() {
            for subdir in SaveKind::secondary_data() {
                subdir.create_dir(p).unwrap();
            }
        }

        Self {
            underlying: cc,
            homotopies: OnceBiVec::new(2),
            intermediates: DashMap::new(),
        }
    }

    pub fn homotopy(&self, s: u32) -> &SecondaryHomotopy<A> {
        &self.homotopies[s as i32]
    }

    pub fn e3_page(&self) -> sseq::Sseq<sseq::Adams> {
        let p = self.prime();

        let mut sseq = sseq::Sseq::<sseq::Adams>::new(p, 0, 0);

        let mut source_vec = FpVector::new(p, 0);
        let mut target_vec = FpVector::new(p, 0);

        for (s, n, t) in self.underlying.iter_stem() {
            let num_gens = self.underlying.module(s).number_of_gens_in_degree(t);
            sseq.set_dimension(n, s as i32, num_gens);

            if t > 0 && self.underlying.has_computed_bidegree(s + 2, t + 1) {
                let m = self.homotopy(s + 2).homotopies.hom_k(t);
                if m.is_empty() || m[0].is_empty() {
                    continue;
                }

                source_vec.set_scratch_vector_size(m.len());
                target_vec.set_scratch_vector_size(m[0].len());

                for (i, row) in m.into_iter().enumerate() {
                    source_vec.set_to_zero();
                    source_vec.set_entry(i, 1);
                    target_vec.copy_from_slice(&row);

                    sseq.add_differential(
                        2,
                        n,
                        s as i32,
                        source_vec.as_slice(),
                        target_vec.as_slice(),
                    );
                }
            }
        }

        for (s, n, _) in self.underlying.iter_stem() {
            if sseq.invalid(n, s as i32) {
                sseq.update_bidegree(n, s as i32);
            }
        }
        sseq
    }
}

// Rustdoc ICE's when trying to document this struct. See
// https://github.com/rust-lang/rust/issues/91380
#[doc(hidden)]
pub struct SecondaryResolutionHomomorphism<
    A: PairAlgebra,
    CC1: FreeChainComplex<Algebra = A>,
    CC2: FreeChainComplex<Algebra = A>,
> {
    source: Arc<SecondaryResolution<A, CC1>>,
    target: Arc<SecondaryResolution<A, CC2>>,
    underlying: Arc<ResolutionHomomorphism<CC1, CC2>>,
    /// input s -> homotopy
    homotopies: OnceBiVec<SecondaryHomotopy<A>>,
    intermediates: DashMap<(u32, i32, usize), FpVector>,
}

impl<
        A: PairAlgebra + Send + Sync,
        CC1: FreeChainComplex<Algebra = A>,
        CC2: FreeChainComplex<Algebra = A>,
    > SecondaryLift for SecondaryResolutionHomomorphism<A, CC1, CC2>
{
    type Algebra = A;
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
        Arc::clone(&self.source.underlying)
    }

    fn target(&self) -> Arc<Self::Target> {
        Arc::clone(&self.target.underlying)
    }

    fn shift_s(&self) -> u32 {
        self.underlying.shift_s + 1
    }

    fn shift_t(&self) -> i32 {
        self.underlying.shift_t
    }

    fn max_s(&self) -> u32 {
        self.underlying.next_homological_degree() as u32
    }

    fn max_t(&self, s: u32) -> i32 {
        let shift_s = self.shift_s();
        let shift_t = self.shift_t();
        std::cmp::min(
            self.underlying.get_map(s).next_degree(),
            std::cmp::min(
                self.source.homotopies[s as i32].homotopies.next_degree(),
                if s == shift_s {
                    i32::MAX
                } else {
                    self.target.homotopies[(s + 1 - shift_s) as i32]
                        .composites
                        .max_degree()
                        + shift_t
                        + 1
                },
            ),
        )
    }

    fn homotopies(&self) -> &OnceBiVec<SecondaryHomotopy<Self::Algebra>> {
        &self.homotopies
    }

    fn intermediates(&self) -> &DashMap<(u32, i32, usize), FpVector> {
        &self.intermediates
    }

    fn save_dir(&self) -> Option<&Path> {
        self.underlying.save_dir()
    }

    fn composite(&self, s: u32) -> CompositeData<Self::Algebra> {
        let shift_s = self.shift_s();
        let p = *self.prime();
        // This is -1 mod p^2
        let neg_1 = p * p - 1;

        let d_source = self.source.underlying.differential(s);
        let d_target = self.target.underlying.differential(s + 1 - shift_s);

        let c1 = self.underlying.get_map(s);
        let c0 = self.underlying.get_map(s - 1);

        vec![(neg_1, d_source, c0), (1, c1, d_target)]
    }

    fn compute_intermediate(&self, s: u32, t: i32, idx: usize) -> FpVector {
        let shift_s = self.shift_s();
        let shift_t = self.shift_t();

        let p = self.prime();
        let neg_1 = *p - 1;
        let target = self.target().module(s - shift_s - 1);

        let mut result = FpVector::new(p, Module::dimension(&*target, t - 1 - shift_t));
        let d = self.source().differential(s);

        self.homotopies[s as i32 - 1].act(
            result.as_slice_mut(),
            neg_1,
            t,
            d.output(t, idx).as_slice(),
            false,
        );
        self.target.homotopy(s + 1 - shift_s).act(
            result.as_slice_mut(),
            neg_1,
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

        result
    }
}

impl<
        A: PairAlgebra + Send + Sync,
        CC1: FreeChainComplex<Algebra = A>,
        CC2: FreeChainComplex<Algebra = A>,
    > SecondaryResolutionHomomorphism<A, CC1, CC2>
{
    pub fn new(
        source: Arc<SecondaryResolution<A, CC1>>,
        target: Arc<SecondaryResolution<A, CC2>>,
        underlying: Arc<ResolutionHomomorphism<CC1, CC2>>,
    ) -> Self {
        assert!(Arc::ptr_eq(&underlying.source, &source.underlying));
        assert!(Arc::ptr_eq(&underlying.target, &target.underlying));

        if let Some(p) = underlying.save_dir() {
            for subdir in SaveKind::secondary_data() {
                subdir.create_dir(p).unwrap();
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

    pub fn homotopy(&self, s: u32) -> &SecondaryHomotopy<A> {
        &self.homotopies[s as i32]
    }

    /// Compute the induced map on Mod_{C\tau^2} homotopy groups. This only computes it on
    /// standard lifts on elements in Ext. `outputs` is an iterator of `SliceMut`s whose lengths
    /// are equal to the total dimension of `(s + shift_s, t + shift_t)` and `(s + shift_s + 1, t +
    /// shift_t + 1)`. The first chunk records the Ext part of the result, and the second chunk
    /// records the τ part of the result.
    ///
    /// This reduces the τ part of the result by the image of d₂.
    pub fn hom_k<'a>(
        &self,
        sseq: &sseq::Sseq,
        s: u32,
        t: i32,
        inputs: impl Iterator<Item = Slice<'a>>,
        outputs: impl Iterator<Item = SliceMut<'a>>,
    ) {
        let source_s = s + self.underlying.shift_s;
        let source_t = t + self.underlying.shift_t;

        let p = self.prime();
        let h_0 = self.algebra().p_tilde();

        let source_num_gens = self
            .source
            .underlying
            .number_of_gens_in_bidegree(source_s, source_t);
        let tau_num_gens = self
            .source
            .underlying
            .number_of_gens_in_bidegree(source_s + 1, source_t + 1);

        let m0 = self.underlying.get_map(source_s).hom_k(t);
        let m1 = Matrix::from_vec(p, &self.homotopy(source_s + 1).homotopies.hom_k(t));
        // The multiplication by p map
        let mp = Matrix::from_vec(
            p,
            &self
                .source
                .underlying()
                .filtration_one_product(1, h_0, source_s + 1, source_t + 1)
                .unwrap(),
        );

        let sign = if (self.underlying.shift_s as i32 * t) % 2 == 1 {
            *p * *p - 1
        } else {
            1
        };
        let filtration_one_sign = if (t as i32 % 2) == 1 { *p - 1 } else { 1 };

        let page_data = {
            let d = sseq.page_data(source_t - source_s as i32, source_s as i32 + 1);
            &d[std::cmp::min(3, d.len() - 1)]
        };

        let mut scratch0: Vec<u32> = Vec::new();
        for (input, mut out) in inputs.zip(outputs) {
            scratch0.clear();
            scratch0.resize(source_num_gens, 0);
            for (i, v) in input.iter_nonzero() {
                scratch0
                    .iter_mut()
                    .zip(&m0[i])
                    .for_each(|(a, b)| *a += v * b * sign);
                out.slice_mut(source_num_gens, source_num_gens + tau_num_gens)
                    .add(m1[i].as_slice(), (v * sign) % *p);
            }
            for (i, v) in scratch0.iter().enumerate() {
                out.add_basis_element(i, *v % *p);

                let extra = *v / *p;
                out.slice_mut(source_num_gens, source_num_gens + tau_num_gens)
                    .add(mp[i].as_slice(), (extra * filtration_one_sign) % *p);
            }
            page_data
                .reduce_by_quotient(out.slice_mut(source_num_gens, source_num_gens + tau_num_gens));
        }
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
