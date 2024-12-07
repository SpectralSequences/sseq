use std::{io, sync::Arc};

use algebra::{
    module::{
        homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism},
        FreeModule, Module,
    },
    pair_algebra::PairAlgebra,
    Algebra,
};
use bivec::BiVec;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use dashmap::DashMap;
use fp::{
    matrix::Matrix,
    prime::ValidPrime,
    vector::{FpSlice, FpSliceMut, FpVector},
};
use itertools::Itertools;
use maybe_rayon::prelude::*;
use once::OnceBiVec;
use sseq::coordinates::{Bidegree, BidegreeGenerator, BidegreeRange};
use tracing::Level;

use crate::{
    chain_complex::{ChainComplex, ChainHomotopy, FreeChainComplex},
    resolution_homomorphism::ResolutionHomomorphism,
    save::{SaveDirectory, SaveFile, SaveKind},
};

pub static LAMBDA_BIDEGREE: Bidegree = Bidegree::n_s(0, 1);

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

    pub fn new(target: Arc<FreeModule<A>>, degree: i32, hit_generator: bool) -> Self {
        let algebra = target.algebra();
        let min_degree = target.min_degree();

        let mut composite = BiVec::with_capacity(min_degree, degree);

        let end = if hit_generator { degree + 1 } else { degree };
        for t_ in min_degree..end {
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

    pub fn to_bytes(&self, buffer: &mut impl io::Write) -> io::Result<()> {
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
        hit_generator: bool,
        buffer: &mut impl io::Read,
    ) -> io::Result<Self> {
        let min_degree = target.min_degree();
        let algebra = target.algebra();
        let mut composite = BiVec::with_capacity(min_degree, degree);

        let end = if hit_generator { degree + 1 } else { degree };
        for t in min_degree..end {
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

    pub fn act(&self, mut result: FpSliceMut, coeff: u32, op_degree: i32, op: FpSlice) {
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
                let len = algebra.dimension(module_op_deg + op_degree - 1);

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

    hit_generator: bool,
}

impl<A: PairAlgebra + Send + Sync> SecondaryHomotopy<A> {
    pub fn new(
        source: Arc<FreeModule<A>>,
        target: Arc<FreeModule<A>>,
        shift_t: i32,
        hit_generator: bool,
    ) -> Self {
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
            hit_generator,
        }
    }

    /// Add composites up to and including the specified degree
    #[tracing::instrument(skip(self, maps, dir), fields(source = %self.source, target = %self.target))]
    pub fn add_composite(&self, s: u32, degree: i32, maps: CompositeData<A>, dir: &SaveDirectory) {
        for (_, d1, d0) in &maps {
            assert!(Arc::ptr_eq(&d1.target(), &d0.source()));
            assert!(Arc::ptr_eq(&d0.target(), &self.target));
            assert_eq!(d1.degree_shift() + d0.degree_shift(), self.shift_t);
        }

        let tracing_span = tracing::Span::current();
        let f = |t, idx| {
            let _tracing_guard = tracing_span.enter();
            let gen = BidegreeGenerator::s_t(s, t, idx);
            let save_file = SaveFile {
                algebra: self.target.algebra(),
                kind: SaveKind::SecondaryComposite,
                b: gen.degree(),
                idx: Some(gen.idx()),
            };
            if let Some(dir) = dir.read() {
                if let Some(mut f) = save_file.open_file(dir.to_owned()) {
                    return SecondaryComposite::from_bytes(
                        Arc::clone(&self.target),
                        gen.t() - self.shift_t,
                        self.hit_generator,
                        &mut f,
                    )
                    .unwrap();
                }
            }

            let mut composite = SecondaryComposite::new(
                Arc::clone(&self.target),
                gen.t() - self.shift_t,
                self.hit_generator,
            );

            tracing::info_span!("Computing composite", gen = %gen).in_scope(|| {
                for (coef, d1, d0) in &maps {
                    composite.add_composite(*coef, gen.t(), gen.idx(), d1, d0);
                }
                composite.finalize();
            });

            if let Some(dir) = dir.write() {
                let mut f = save_file.create_file(dir.to_owned(), false);
                composite.to_bytes(&mut f).unwrap();
            }

            composite
        };

        self.composites.maybe_par_extend(degree, |t| {
            (0..self.source.number_of_gens_in_degree(t))
                .into_maybe_par_iter()
                .map(|i| f(t, i))
                .collect()
        });
    }

    /// Compute the image of an element in the source under the homotopy, writing the result in
    /// `result`. It is assumed that the coefficients of generators are zero in `op`.
    ///
    /// # Arguments
    ///  - full: Whether to include the action of the homotopy part as well
    pub fn act(
        &self,
        mut result: FpSliceMut,
        coeff: u32,
        elt_degree: i32,
        elt: FpSlice,
        full: bool,
    ) {
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

/// Logic that is common to all secondary lifts.
///
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
/// The next is a compatibility equation, which restricts the λ part of the null-homotopy, and is
/// usually of the form
///
/// $$ dh = hd + \mathrm{stuff} $$
///
/// The λ part of $hd + \mathrm{stuff}$ is known as the intermediate data, and is what
/// [`SecondaryLift::compute_intermediate`] returns.
pub trait SecondaryLift: Sync + Sized {
    type Algebra: PairAlgebra;
    type Source: FreeChainComplex<Algebra = Self::Algebra>;
    type Target: FreeChainComplex<Algebra = Self::Algebra>;
    type Underlying;

    /// Whether the composite can hit generators. This is true for `SecondaryChainHomotopy` and
    /// false for the rest. This is important because for [`SecondaryResolution`], we don't
    /// actually know all the generators if we resolve up to a stem. So in composites for
    /// [`SecondaryResolution`], we need to ignore target generators of the same degree uniformly.
    const HIT_GENERATOR: bool = false;

    fn underlying(&self) -> Arc<Self::Underlying>;
    fn algebra(&self) -> Arc<Self::Algebra>;
    fn prime(&self) -> ValidPrime {
        self.algebra().prime()
    }

    fn source(&self) -> Arc<Self::Source>;
    fn target(&self) -> Arc<Self::Target>;
    fn shift(&self) -> Bidegree;

    fn max(&self) -> BidegreeRange<Self>;

    fn homotopies(&self) -> &OnceBiVec<SecondaryHomotopy<Self::Algebra>>;
    fn intermediates(&self) -> &DashMap<BidegreeGenerator, FpVector>;

    fn save_dir(&self) -> &SaveDirectory;

    fn compute_intermediate(&self, gen: BidegreeGenerator) -> FpVector;
    fn composite(&self, s: u32) -> CompositeData<Self::Algebra>;

    #[tracing::instrument(skip(self))]
    fn initialize_homotopies(&self) {
        let shift = self.shift();
        let max = self.max();

        self.homotopies().extend(max.s() as i32 - 1, |s| {
            let s = s as u32;
            SecondaryHomotopy::new(
                self.source().module(s),
                self.target().module(s - shift.s()),
                shift.t(),
                Self::HIT_GENERATOR,
            )
        });
    }

    #[tracing::instrument(skip(self))]
    fn compute_composites(&self) {
        let tracing_span = tracing::Span::current();
        let f = |s| {
            let _tracing_guard = tracing_span.enter();
            let s = s as u32;
            self.homotopies()[s as i32].add_composite(
                s,
                self.max().t(s) - 1,
                self.composite(s),
                self.save_dir(),
            );
        };

        self.homotopies().range().into_maybe_par_iter().for_each(f);
    }

    #[tracing::instrument(skip(self), ret(Display, level = Level::DEBUG), fields(gen = %gen))]
    fn get_intermediate(&self, gen: BidegreeGenerator) -> FpVector {
        if let Some((_, v)) = self.intermediates().remove(&gen) {
            return v;
        }

        let save_file = SaveFile {
            algebra: self.algebra(),
            kind: SaveKind::SecondaryIntermediate,
            b: gen.degree(),
            idx: Some(gen.idx()),
        };

        if let Some(dir) = self.save_dir().read() {
            if let Some(mut f) = save_file.open_file(dir.to_owned()) {
                // The target dimension can depend on whether we resolved to stem
                let dim = f.read_u64::<LittleEndian>().unwrap() as usize;
                return FpVector::from_bytes(self.prime(), dim, &mut f).unwrap();
            }
        }

        let result = self.compute_intermediate(gen);

        if let Some(dir) = self.save_dir().write() {
            let mut f = save_file.create_file(dir.to_owned(), false);
            f.write_u64::<LittleEndian>(result.len() as u64).unwrap();
            result.to_bytes(&mut f).unwrap();
        }

        result
    }

    #[tracing::instrument(skip(self))]
    fn compute_partial(&self, s: u32) {
        self.initialize_homotopies();
        let homotopies = self.homotopies();

        if (s as i32) < homotopies.min_degree() {
            eprintln!(
                "Computing partial for s = {s} when minimum degree is {}",
                homotopies.min_degree()
            );
            return;
        }

        homotopies[s as i32].add_composite(
            s,
            self.max().t(s) - 1,
            self.composite(s),
            self.save_dir(),
        );

        if let Some(homotopy) = homotopies.get(s as i32 + 1) {
            (0..self.max().t(s + 1))
                .into_maybe_par_iter()
                .for_each(|t| {
                    (0..homotopy.source.number_of_gens_in_degree(t))
                        .into_maybe_par_iter()
                        .for_each(|i| {
                            self.get_intermediate(BidegreeGenerator::s_t(s + 1, t, i));
                        })
                });
        }
    }

    #[tracing::instrument(skip(self))]
    fn compute_intermediates(&self) {
        let tracing_span = tracing::Span::current();
        let f = |gen: BidegreeGenerator| {
            let _tracing_guard = tracing_span.enter();

            // If we already have homotopies, we don't need to compute intermediate
            if self.homotopies()[gen.s() as i32].homotopies.next_degree() >= gen.t() {
                return;
            }
            // Check if we have a saved homotopy
            if let Some(dir) = self.save_dir().read() {
                let save_file = SaveFile {
                    algebra: self.algebra(),
                    kind: SaveKind::SecondaryHomotopy,
                    b: gen.degree(),
                    idx: None,
                };

                if save_file.exists(dir.to_owned()) {
                    return;
                }
            }
            self.intermediates().insert(gen, self.get_intermediate(gen));
        };

        self.homotopies()
            .maybe_par_iter_enum()
            .skip(1)
            .for_each(|(s, homotopy)| {
                let s = s as u32;

                homotopy
                    .composites
                    .range()
                    .into_maybe_par_iter()
                    .for_each(|t| {
                        (0..homotopy.source.number_of_gens_in_degree(t))
                            .into_maybe_par_iter()
                            .for_each(|i| f(BidegreeGenerator::s_t(s, t, i)))
                    })
            })
    }

    #[tracing::instrument(skip(self), fields(b = %b))]
    fn compute_homotopy_step(&self, b: Bidegree) -> std::ops::Range<i32> {
        let homotopy = &self.homotopies()[b.s() as i32];
        if homotopy.homotopies.next_degree() > b.t() {
            return b.t()..b.t() + 1;
        }
        let p = self.prime();
        let shift = self.shift();
        let target_b = b - shift - Bidegree::s_t(0, 1);

        let d = self.source().differential(b.s());
        let source = self.source().module(b.s());
        let target = self.target();
        let num_gens = source.number_of_gens_in_degree(b.t());
        let target_dim = target.module(target_b.s()).dimension(target_b.t());

        if let Some(dir) = self.save_dir().read() {
            let save_file = SaveFile {
                algebra: self.algebra(),
                kind: SaveKind::SecondaryHomotopy,
                b,
                idx: None,
            };

            if let Some(mut f) = save_file.open_file(dir.to_owned()) {
                let mut results = Vec::with_capacity(num_gens);
                for _ in 0..num_gens {
                    results.push(FpVector::from_bytes(p, target_dim, &mut f).unwrap());
                }
                return self.homotopies()[b.s() as i32]
                    .homotopies
                    .add_generators_from_rows_ooo(b.t(), results);
            }
        }

        let tracing_span = tracing::Span::current();
        let get_intermediate = |i| {
            let _tracing_guard = tracing_span.enter();

            let gen = BidegreeGenerator::new(b, i);
            let mut v = self.get_intermediate(gen);
            if gen.s() > shift.s() + 1 {
                self.homotopies()[gen.s() as i32 - 1].homotopies.apply(
                    v.as_slice_mut(),
                    1,
                    gen.t(),
                    d.output(gen.t(), gen.idx()).as_slice(),
                );
            }
            v
        };

        let mut intermediates: Vec<FpVector> = (0..num_gens)
            .into_maybe_par_iter()
            .map(get_intermediate)
            .collect();

        let mut results = vec![FpVector::new(p, target_dim); num_gens];

        assert!(target.apply_quasi_inverse(&mut results, target_b, &intermediates,));

        if b.s() == shift.s() + 1 {
            // Check that we indeed had a lift
            let d = target.differential(target_b.s());
            for (src, tgt) in std::iter::zip(&results, &mut intermediates) {
                d.apply(tgt.as_slice_mut(), p - 1, target_b.t(), src.as_slice());
                assert!(
                    tgt.is_zero(),
                    "secondary: Failed to lift at {b}. This likely indicates an invalid input."
                );
            }
        }

        if let Some(dir) = self.save_dir().write() {
            let save_file = SaveFile {
                algebra: self.algebra(),
                kind: SaveKind::SecondaryHomotopy,
                b,
                idx: None,
            };

            let mut f = save_file.create_file(dir.to_owned(), false);
            for row in &results {
                row.to_bytes(&mut f).unwrap();
            }
            drop(f);

            let mut save_file = SaveFile {
                algebra: self.algebra(),
                kind: SaveKind::SecondaryIntermediate,
                b,
                idx: None,
            };

            for i in 0..num_gens {
                save_file.idx = Some(i);
                save_file.delete_file(dir.to_owned()).unwrap();
            }
        }

        homotopy
            .homotopies
            .add_generators_from_rows_ooo(b.t(), results)
    }

    #[tracing::instrument(skip(self))]
    fn compute_homotopies(&self) {
        let shift = self.shift();

        // When s = shift_s, the homotopies are just zero
        {
            let h = &self.homotopies()[shift.s() as i32];
            h.homotopies.extend_by_zero(h.composites.max_degree());
        }

        let min_t = self.homotopies()[shift.s() as i32].homotopies.min_degree();
        let s_range = self.homotopies().range();
        let min = Bidegree::s_t(s_range.start as u32 + 1, min_t);
        let max = self.max().restrict(s_range.end as u32);
        sseq::coordinates::iter_s_t(&|b| self.compute_homotopy_step(b), min, max);
    }

    #[tracing::instrument(skip(self))]
    fn extend_all(&self) {
        self.initialize_homotopies();
        self.compute_composites();
        self.compute_intermediates();
        self.compute_homotopies();
    }
}

pub struct SecondaryResolution<CC: FreeChainComplex>
where
    CC::Algebra: PairAlgebra,
{
    underlying: Arc<CC>,
    /// s -> t -> idx -> homotopy
    pub(crate) homotopies: OnceBiVec<SecondaryHomotopy<CC::Algebra>>,
    intermediates: DashMap<BidegreeGenerator, FpVector>,
}

impl<CC: FreeChainComplex> SecondaryLift for SecondaryResolution<CC>
where
    CC::Algebra: PairAlgebra,
{
    type Algebra = CC::Algebra;
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

    fn shift(&self) -> Bidegree {
        Bidegree::s_t(2, 0)
    }

    fn max(&self) -> BidegreeRange<Self> {
        BidegreeRange::new(
            self,
            self.underlying.next_homological_degree(),
            &|selff, s| {
                std::cmp::min(
                    selff.underlying.module(s).max_computed_degree(),
                    selff.underlying.module(s - 2).max_computed_degree() + 1,
                ) + 1
            },
        )
    }

    fn homotopies(&self) -> &OnceBiVec<SecondaryHomotopy<CC::Algebra>> {
        &self.homotopies
    }

    fn intermediates(&self) -> &DashMap<BidegreeGenerator, FpVector> {
        &self.intermediates
    }

    fn save_dir(&self) -> &SaveDirectory {
        self.underlying.save_dir()
    }

    fn composite(&self, s: u32) -> CompositeData<CC::Algebra> {
        let d1 = self.underlying.differential(s);
        let d0 = self.underlying.differential(s - 1);
        vec![(1, d1, d0)]
    }

    fn compute_intermediate(&self, gen: BidegreeGenerator) -> FpVector {
        let p = self.prime();
        let target = self.underlying.module(gen.s() - 3);
        let mut result = FpVector::new(p, target.dimension(gen.t() - 1));
        let d = self.underlying.differential(gen.s());
        self.homotopies[gen.s() as i32 - 1].act(
            result.as_slice_mut(),
            1,
            gen.t(),
            d.output(gen.t(), gen.idx()).as_slice(),
            false,
        );
        result
    }
}

impl<CC: FreeChainComplex> SecondaryResolution<CC>
where
    CC::Algebra: PairAlgebra,
{
    pub fn new(cc: Arc<CC>) -> Self {
        if let Some(p) = cc.save_dir().write() {
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

    pub fn homotopy(&self, s: u32) -> &SecondaryHomotopy<CC::Algebra> {
        &self.homotopies[s as i32]
    }

    pub fn e3_page(&self) -> sseq::Sseq<sseq::Adams> {
        let p = self.prime();

        let mut sseq = self.underlying.to_sseq();

        let mut source_vec = FpVector::new(p, 0);
        let mut target_vec = FpVector::new(p, 0);

        for b in self.underlying.iter_stem() {
            if b.t() > 0
                && self
                    .underlying
                    .has_computed_bidegree(b + Bidegree::n_s(-1, 2))
            {
                let m = self.homotopy(b.s() + 2).homotopies.hom_k(b.t());
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
                        b.n(),
                        b.s() as i32,
                        source_vec.as_slice(),
                        target_vec.as_slice(),
                    );
                }
            }
        }

        for b in self.underlying.iter_stem() {
            if sseq.invalid(b.n(), b.s() as i32) {
                sseq.update_bidegree(b.n(), b.s() as i32);
            }
        }
        sseq
    }
}

// Rustdoc ICE's when trying to document this struct. See
// https://github.com/rust-lang/rust/issues/91380
#[doc(hidden)]
pub struct SecondaryResolutionHomomorphism<
    CC1: FreeChainComplex,
    CC2: FreeChainComplex<Algebra = CC1::Algebra>,
> where
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
        Arc::clone(&self.source.underlying)
    }

    fn target(&self) -> Arc<Self::Target> {
        Arc::clone(&self.target.underlying)
    }

    fn shift(&self) -> Bidegree {
        self.underlying.shift + Bidegree::s_t(1, 0)
    }

    fn max(&self) -> BidegreeRange<Self> {
        BidegreeRange::new(
            self,
            self.underlying.next_homological_degree() as u32,
            &|selff, s| {
                std::cmp::min(
                    selff.underlying.get_map(s).next_degree(),
                    std::cmp::min(
                        selff.source.homotopies[s as i32].homotopies.next_degree(),
                        if s == selff.shift().s() {
                            i32::MAX
                        } else {
                            selff.target.homotopies[(s + 1 - selff.shift().s()) as i32]
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

    fn composite(&self, s: u32) -> CompositeData<Self::Algebra> {
        let p = self.prime();
        // This is -1 mod p^2
        let neg_1 = p * p - 1;

        let d_source = self.source.underlying.differential(s);
        let d_target = self
            .target
            .underlying
            .differential(s + 1 - self.shift().s());

        let c1 = self.underlying.get_map(s);
        let c0 = self.underlying.get_map(s - 1);

        vec![(neg_1, d_source, c0), (1, c1, d_target)]
    }

    fn compute_intermediate(&self, gen: BidegreeGenerator) -> FpVector {
        let p = self.prime();
        let neg_1 = p - 1;
        let shifted_b = gen.degree() - self.shift();
        let target = self.target().module(shifted_b.s() - 1);

        let mut result = FpVector::new(p, target.dimension(shifted_b.t() - 1));
        let d = self.source().differential(gen.s());

        self.homotopies[gen.s() as i32 - 1].act(
            result.as_slice_mut(),
            neg_1,
            gen.t(),
            d.output(gen.t(), gen.idx()).as_slice(),
            false,
        );
        self.target.homotopy(shifted_b.s() + 1).act(
            result.as_slice_mut(),
            neg_1,
            shifted_b.t(),
            self.underlying
                .get_map(gen.s())
                .output(gen.t(), gen.idx())
                .as_slice(),
            true,
        );
        self.underlying.get_map(gen.s() - 2).apply(
            result.as_slice_mut(),
            1,
            gen.t() - 1,
            self.source
                .homotopy(gen.s())
                .homotopies
                .output(gen.t(), gen.idx())
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
        assert!(Arc::ptr_eq(&underlying.source, &source.underlying));
        assert!(Arc::ptr_eq(&underlying.target, &target.underlying));

        if let Some(p) = underlying.save_dir().write() {
            for subdir in SaveKind::secondary_data() {
                subdir.create_dir(p).unwrap();
            }
        }

        Self {
            source,
            target,
            homotopies: OnceBiVec::new(underlying.shift.s() as i32 + 1),
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

    pub fn homotopy(&self, s: u32) -> &SecondaryHomotopy<CC1::Algebra> {
        &self.homotopies[s as i32]
    }

    /// A version of [`hom_k`] but with a non-trivial λ part.
    pub fn hom_k_with<'a>(
        &self,
        lambda_part: Option<&ResolutionHomomorphism<CC1, CC2>>,
        sseq: Option<&sseq::Sseq>,
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
        let mut m1 = Matrix::from_vec(p, &self.homotopy(lambda_source.s()).homotopies.hom_k(b.t()));
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

        let sign = if (self.underlying.shift.s() as i32 * b.t()) % 2 == 1 {
            p * p - 1
        } else {
            1
        };
        let filtration_one_sign = if (b.t() % 2) == 1 { p - 1 } else { 1 };

        let page_data = sseq.map(|sseq| {
            let d = sseq.page_data(lambda_source.n(), lambda_source.s() as i32);
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
                    .add(m1[i].as_slice(), (v * sign) % p);
            }
            for (i, v) in scratch0.iter().enumerate() {
                out.add_basis_element(i, *v % p);

                let extra = *v / p;
                out.slice_mut(source_num_gens, source_num_gens + lambda_num_gens)
                    .add(mp[i].as_slice(), (extra * filtration_one_sign) % p);
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
        sseq: Option<&sseq::Sseq>,
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
        sseq: &sseq::Sseq,
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
            [class.slice(0, target_num_gens)].into_iter(),
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
            class.slice(target_num_gens, target_num_gens + target_lambda_num_gens),
        );

        let diff_source = b + shift - Bidegree::n_s(-1, 1);
        sseq.differentials(diff_source.n(), diff_source.s() as i32)[2].quasi_inverse(
            output_class.as_slice_mut(),
            prod_value.slice(lower_num_gens, lower_num_gens + lambda_num_gens),
        );

        output_class
    }
}

#[doc(hidden)]
pub struct SecondaryChainHomotopy<
    S: FreeChainComplex,
    T: FreeChainComplex<Algebra = S::Algebra> + Sync,
    U: FreeChainComplex<Algebra = S::Algebra> + Sync,
> where
    S::Algebra: PairAlgebra,
{
    underlying: Arc<ChainHomotopy<S, T, U>>,
    left: Arc<SecondaryResolutionHomomorphism<S, T>>,
    right: Arc<SecondaryResolutionHomomorphism<T, U>>,
    left_lambda: Option<Arc<ResolutionHomomorphism<S, T>>>,
    right_lambda: Option<Arc<ResolutionHomomorphism<T, U>>>,
    homotopies: OnceBiVec<SecondaryHomotopy<S::Algebra>>,
    intermediates: DashMap<BidegreeGenerator, FpVector>,
}

impl<
        S: FreeChainComplex,
        T: FreeChainComplex<Algebra = S::Algebra> + Sync,
        U: FreeChainComplex<Algebra = S::Algebra> + Sync,
    > SecondaryLift for SecondaryChainHomotopy<S, T, U>
where
    S::Algebra: PairAlgebra,
{
    type Algebra = S::Algebra;
    type Source = S;
    type Target = U;
    type Underlying = ChainHomotopy<S, T, U>;

    const HIT_GENERATOR: bool = true;

    fn underlying(&self) -> Arc<Self::Underlying> {
        Arc::clone(&self.underlying)
    }

    fn algebra(&self) -> Arc<Self::Algebra> {
        self.left.algebra()
    }

    fn source(&self) -> Arc<Self::Source> {
        self.left.source()
    }

    fn target(&self) -> Arc<Self::Target> {
        self.right.target()
    }

    fn shift(&self) -> Bidegree {
        Bidegree::s_t(
            self.underlying.shift().s(),
            self.left.shift().t() + self.right.shift().t(),
        )
    }

    fn max(&self) -> BidegreeRange<Self> {
        BidegreeRange::new(
            self,
            std::cmp::min(
                self.right.target.max().s() + self.shift().s() - 1,
                self.left.source.max().s(),
            ),
            &|selff, s| {
                std::cmp::min(
                    selff.left.source.max().t(s),
                    if s == selff.shift().s() {
                        i32::MAX
                    } else {
                        selff.right.target.max().t(s - selff.shift().s() + 1) + selff.shift().t()
                    },
                )
            },
        )
    }

    fn homotopies(&self) -> &OnceBiVec<SecondaryHomotopy<S::Algebra>> {
        &self.homotopies
    }

    fn intermediates(&self) -> &DashMap<BidegreeGenerator, FpVector> {
        &self.intermediates
    }

    fn save_dir(&self) -> &SaveDirectory {
        self.underlying.save_dir()
    }

    fn compute_intermediate(&self, gen: BidegreeGenerator) -> FpVector {
        let p = self.prime();
        let neg_1 = p - 1;
        let shifted_b = gen.degree() - self.shift();

        let target = self.target().module(shifted_b.s() - 1);

        let mut result = FpVector::new(p, target.dimension(shifted_b.t() - 1));

        self.homotopies[gen.s() as i32 - 1].act(
            result.as_slice_mut(),
            1,
            gen.t(),
            self.source()
                .differential(gen.s())
                .output(gen.t(), gen.idx())
                .as_slice(),
            false,
        );

        self.right.target.homotopies()[(shifted_b.s() + 1) as i32].act(
            result.as_slice_mut(),
            1,
            shifted_b.t(),
            self.underlying
                .homotopy(gen.s())
                .output(gen.t(), gen.idx())
                .as_slice(),
            true,
        );

        self.underlying.homotopy(gen.s() - 2).apply(
            result.as_slice_mut(),
            neg_1,
            gen.t() - 1,
            self.left.source.homotopies()[gen.s() as i32]
                .homotopies
                .output(gen.t(), gen.idx())
                .as_slice(),
        );

        let left_shifted_b = gen.degree() - self.left.underlying.shift;
        self.right.homotopies()[left_shifted_b.s() as i32].act(
            result.as_slice_mut(),
            neg_1,
            left_shifted_b.t(),
            self.left
                .underlying
                .get_map(gen.s())
                .output(gen.t(), gen.idx())
                .as_slice(),
            true,
        );

        // This is inefficient if both right_lambda and right are non-zero, but this is not needed atm
        // and the change would not be user-facing.
        if let Some(right_lambda) = &self.right_lambda {
            right_lambda.get_map(left_shifted_b.s()).apply(
                result.as_slice_mut(),
                neg_1,
                left_shifted_b.t(),
                self.left
                    .underlying
                    .get_map(gen.s())
                    .output(gen.t(), gen.idx())
                    .as_slice(),
            );
        }

        self.right.underlying.get_map(left_shifted_b.s() - 1).apply(
            result.as_slice_mut(),
            neg_1,
            left_shifted_b.t() - 1,
            self.left.homotopies()[gen.s() as i32]
                .homotopies
                .output(gen.t(), gen.idx())
                .as_slice(),
        );

        if let Some(left_lambda) = &self.left_lambda {
            self.right.underlying.get_map(left_shifted_b.s() - 1).apply(
                result.as_slice_mut(),
                neg_1,
                left_shifted_b.t() - 1,
                left_lambda
                    .get_map(gen.s())
                    .output(gen.t(), gen.idx())
                    .as_slice(),
            );
        }
        result
    }

    fn composite(&self, s: u32) -> CompositeData<S::Algebra> {
        let p = self.prime();
        // This is -1 mod p^2
        let neg_1 = p * p - 1;

        vec![
            (
                neg_1,
                self.underlying.left().get_map(s),
                self.underlying
                    .right()
                    .get_map(s - self.left.underlying.shift.s()),
            ),
            (
                1,
                self.underlying.homotopy(s),
                self.target().differential(s - self.shift().s() + 1),
            ),
            (
                1,
                self.source().differential(s),
                self.underlying.homotopy(s - 1),
            ),
        ]
    }
}

impl<
        S: FreeChainComplex,
        T: FreeChainComplex<Algebra = S::Algebra> + Sync,
        U: FreeChainComplex<Algebra = S::Algebra> + Sync,
    > SecondaryChainHomotopy<S, T, U>
where
    S::Algebra: PairAlgebra,
{
    pub fn new(
        left: Arc<SecondaryResolutionHomomorphism<S, T>>,
        right: Arc<SecondaryResolutionHomomorphism<T, U>>,
        left_lambda: Option<Arc<ResolutionHomomorphism<S, T>>>,
        right_lambda: Option<Arc<ResolutionHomomorphism<T, U>>>,
        underlying: Arc<ChainHomotopy<S, T, U>>,
    ) -> Self {
        assert!(Arc::ptr_eq(&underlying.left(), &left.underlying));
        assert!(Arc::ptr_eq(&underlying.right(), &right.underlying));

        if let Some(left_lambda) = &left_lambda {
            assert!(Arc::ptr_eq(&left_lambda.source, &underlying.left().source));
            assert!(Arc::ptr_eq(&left_lambda.target, &underlying.left().target));

            assert_eq!(left_lambda.shift, underlying.left().shift + LAMBDA_BIDEGREE);
        }

        if let Some(right_lambda) = &right_lambda {
            assert!(Arc::ptr_eq(
                &right_lambda.source,
                &underlying.right().source
            ));
            assert!(Arc::ptr_eq(
                &right_lambda.target,
                &underlying.right().target
            ));

            assert_eq!(
                right_lambda.shift,
                underlying.right().shift + LAMBDA_BIDEGREE
            );
        }

        if let Some(p) = underlying.save_dir().write() {
            for subdir in SaveKind::secondary_data() {
                subdir.create_dir(p).unwrap();
            }
        }

        Self {
            left,
            right,
            left_lambda,
            right_lambda,
            homotopies: OnceBiVec::new(underlying.shift().s() as i32),
            underlying,
            intermediates: DashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::utils;

    #[test]
    #[should_panic(
        expected = "secondary: Failed to lift at (14, 3). This likely indicates an invalid input."
    )]
    fn cofib_h4() {
        let module = json!({
            "type": "finite dimensional module",
            "p": 2,
            "gens": {
                "x0": 0,
                "x16": 16,
            },
            "actions": ["Sq16 x0 = x16"]
        });
        let resolution = utils::construct((module, algebra::AlgebraType::Milnor), None).unwrap();
        resolution.compute_through_stem(Bidegree::n_s(20, 5));
        let lift = SecondaryResolution::new(Arc::new(resolution));
        lift.extend_all();
    }
}
