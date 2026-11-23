use std::{
    collections::HashMap,
    io,
    sync::{Arc, Mutex},
};

use algebra::{
    Algebra,
    module::{
        FreeModule, Module,
        homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism},
    },
    pair_algebra::PairAlgebra,
};
use anyhow::{Context, anyhow};
use bivec::BiVec;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use dashmap::DashMap;
use fp::{
    matrix::{AugmentedMatrix, QuasiInverse},
    prime::ValidPrime,
    vector::{FpSlice, FpSliceMut, FpVector},
};
use maybe_rayon::prelude::*;
use once::OnceBiVec;
use sseq::coordinates::{Bidegree, BidegreeGenerator, BidegreeRange};
use tracing::Level;

// This module holds the shared machinery for secondary lifts (the `SecondaryLift` trait,
// `SecondaryHomotopy`, etc.). The concrete lift types used to live here too, but they were moved
// next to their primary counterparts to keep this file from growing without bound and to improve
// locality: each secondary variant now sits beside the primary object it lifts (e.g.
// `SecondaryResolution` next to `Resolution`). Those variants live in private `secondary`
// submodules and are re-exported here so that the public API is unchanged — callers still find them
// under `crate::secondary`.
pub use crate::{
    chain_complex::chain_homotopy::secondary::SecondaryChainHomotopy,
    resolution::secondary::SecondaryResolution,
    resolution_homomorphism::secondary::SecondaryResolutionHomomorphism,
};
use crate::{
    chain_complex::{ChainComplex, FreeChainComplex},
    save::{SaveDirectory, SaveKind},
};

pub static LAMBDA_BIDEGREE: Bidegree = Bidegree::n_s(0, 1);

pub type CompositeData<A> = Vec<(
    u32,
    Arc<FreeModuleHomomorphism<FreeModule<A>>>,
    Arc<FreeModuleHomomorphism<FreeModule<A>>>,
)>;

/// The `secondary_actions` of a module spec: a normalized bar 2-cochain `F: Ā ⊗ Ā ⊗ M → M` of
/// degree -1, written `"[a|b] m = n"` for `F(a, b; m) = n`.
///
/// `F` sets `ε H_2` on `C_2` of a resolution `ε: C → M` to `F(f_2 g) + r(g)`:
///  - `f_2 g = Σ [a|b] ε x`, over the terms `a e` of `d g` and `b x` of `d e` with `|a|, |b| > 0`,
///    is the comparison map from `C` to the two-sided bar resolution `B(A, A, M)`.
///  - `r(g)` is what that comparison map fails to commute with the differentials by in the pair
///    algebra; see [`Self::correction`].
///
/// A solution of the bar equation gives the same secondary structure on every resolution. A list
/// that is not a full solution is read on the resolution at hand, which only reads the entries its
/// `d ∘ d` reaches. An invalid one makes the lift fail at `s = 3`.
pub struct SecondaryActions<A: Algebra> {
    module: Arc<dyn Module<Algebra = A>>,
    /// `[a, b, m] -> F(a, b; m)`, each entry a `(degree, index)` basis element.
    values: HashMap<[(i32, usize); 3], FpVector>,
    /// degree -> quasi-inverse of ε on `C_0`, built on first use.
    lifts: Mutex<HashMap<i32, Arc<QuasiInverse>>>,
}

impl<A: Algebra> std::fmt::Debug for SecondaryActions<A> {
    /// The number of entries, as the module and its cached lifts are not printable.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecondaryActions")
            .field("entries", &self.values.len())
            .finish_non_exhaustive()
    }
}

/// What [`SecondaryResolution`] needs from a resolution whose module has secondary actions.
pub struct SecondaryInput<'a, A: Algebra> {
    pub actions: &'a SecondaryActions<A>,
    /// ε on the basis element `(degree, index)` of `C_0`, in the basis of `M`.
    pub augmentation: Box<dyn Fn(i32, usize) -> FpVector + 'a>,
}

/// The comparison composite `Γ(e)` of one generator `e` of `C_1`: for each basis element `m` of
/// `M`, a pair algebra element of degree `|e| - |m|` on the generator `[] m` of the bar
/// resolution.
pub type ComparisonComposite<A> = Vec<((i32, usize), <A as PairAlgebra>::Element)>;

impl<A: Algebra> SecondaryActions<A> {
    /// Parses the `secondary_actions` field of a module spec. Returns `None` when the field is
    /// absent.
    pub fn from_json(
        module: Arc<dyn Module<Algebra = A>>,
        json: &serde_json::Value,
    ) -> anyhow::Result<Option<Self>> {
        let entries = &json["secondary_actions"];
        if entries.is_null() {
            return Ok(None);
        }
        let algebra = module.algebra();
        anyhow::ensure!(
            algebra.prime() == 2,
            "`secondary_actions` are only supported at p = 2"
        );
        let entries = entries
            .as_array()
            .context("`secondary_actions` must be a list of strings")?;
        let max_degree = module
            .max_degree()
            .context("`secondary_actions` requires a bounded module")?;

        let mut names = HashMap::new();
        for deg in module.min_degree()..=max_degree {
            for idx in 0..module.dimension(deg) {
                names.insert(module.basis_element_to_string(deg, idx), (deg, idx));
            }
        }
        let element = |name: &str| {
            names
                .get(name.trim())
                .copied()
                .ok_or_else(|| anyhow!("Unknown module element: {name}"))
        };
        let operation = |name: &str| {
            let (deg, idx) = algebra
                .basis_element_from_string(name.trim())
                .ok_or_else(|| anyhow!("Invalid algebra element: {name}"))?;
            anyhow::ensure!(
                deg > 0,
                "Operations in secondary actions have positive degree"
            );
            Ok((deg, idx))
        };

        let p = algebra.prime();
        let mut values: HashMap<[(i32, usize); 3], FpVector> = HashMap::new();
        for entry in entries {
            let entry = entry
                .as_str()
                .context("`secondary_actions` must be a list of strings")?;
            let mut parse = || -> anyhow::Result<()> {
                let (lhs, rhs) = entry.split_once(" = ").context("Missing ` = `")?;
                let (ops, m) = lhs
                    .trim()
                    .strip_prefix('[')
                    .and_then(|s| s.split_once(']'))
                    .context("Expected `[a|b] m`")?;
                let (a, b) = ops.split_once('|').context("Expected `[a|b] m`")?;
                let key = [operation(a)?, operation(b)?, element(m)?];
                let out_deg = key.iter().map(|x| x.0).sum::<i32>() - 1;

                let value = values
                    .entry(key)
                    .or_insert_with(|| FpVector::new(p, module.dimension(out_deg)));
                if rhs.trim() == "0" {
                    return Ok(());
                }
                for item in rhs.split(" + ") {
                    let (coef, n) = match item.trim().split_once(' ') {
                        Some((coef, n)) => (
                            coef.parse::<u32>()
                                .map_err(|_| anyhow!("Invalid coefficient in {item}"))?,
                            n,
                        ),
                        None => (1, item),
                    };
                    let (deg, idx) = element(n)?;
                    anyhow::ensure!(
                        deg == out_deg,
                        "Degree of {n} is {deg} but the value has degree {out_deg}"
                    );
                    value.add_basis_element(idx, coef % p);
                }
                Ok(())
            };
            parse().with_context(|| format!("Failed to parse secondary action: {entry}"))?;
        }
        Ok(Some(Self {
            module,
            values,
            lifts: Mutex::default(),
        }))
    }

    /// `F(f_2 g) ∈ M_{t-1}` for the generator `g = (t, idx)` of `C_2`.
    pub fn pullback(
        &self,
        d2: &FreeModuleHomomorphism<FreeModule<A>>,
        d1: &FreeModuleHomomorphism<FreeModule<A>>,
        augmentation: &dyn Fn(i32, usize) -> FpVector,
        t: i32,
        idx: usize,
    ) -> FpVector {
        let p = self.module.prime();
        let c1 = d2.target();
        let c0 = d1.target();
        let mut value = FpVector::new(p, self.module.dimension(t - 1));
        if value.is_empty() || self.values.is_empty() {
            return value;
        }
        let dg = d2.output(t, idx);
        for (e_deg, e_idx, a_deg, a) in c1.iter_slices(t, dg.as_slice()) {
            if a_deg == 0 || a.is_zero() {
                continue;
            }
            let de = d1.output(e_deg, e_idx);
            for (x_deg, x_idx, b_deg, b) in c0.iter_slices(e_deg, de.as_slice()) {
                if b_deg == 0 || b.is_zero() {
                    continue;
                }
                let ex = augmentation(x_deg, c0.generator_offset(x_deg, x_deg, x_idx));
                for (ai, ac) in a.iter_nonzero() {
                    for (bi, bc) in b.iter_nonzero() {
                        for (mi, mc) in ex.iter_nonzero() {
                            let key = [(a_deg, ai), (b_deg, bi), (x_deg, mi)];
                            if let Some(v) = self.values.get(&key) {
                                value.add(v, (ac * bc % p) * mc % p);
                            }
                        }
                    }
                }
            }
        }
        value
    }

    /// An element of `C_0` in degree `d` whose image under ε is `v`.
    pub fn lift(
        &self,
        c0: &FreeModule<A>,
        augmentation: &dyn Fn(i32, usize) -> FpVector,
        d: i32,
        v: &FpVector,
    ) -> FpVector {
        let p = self.module.prime();
        let c0_dim = c0.dimension(d);
        let mut result = FpVector::new(p, c0_dim);
        if v.is_zero() {
            return result;
        }
        let qi = {
            let mut lifts = self.lifts.lock().unwrap();
            Arc::clone(lifts.entry(d).or_insert_with(|| {
                let m_dim = self.module.dimension(d);
                let mut matrix = AugmentedMatrix::<2>::new(p, c0_dim, [m_dim, c0_dim]);
                for i in 0..c0_dim {
                    matrix
                        .row_segment_mut(i, 0, 0)
                        .add(augmentation(d, i).as_slice(), 1);
                    matrix.row_segment_mut(i, 1, 1).set_entry(i, 1);
                }
                matrix.row_reduce();
                Arc::new(matrix.compute_quasi_inverse())
            }))
        };
        qi.apply(result.as_slice_mut(), 1, v.as_slice());
        result
    }
}

impl<A: PairAlgebra> SecondaryActions<A> {
    /// `H_2(g)` for the generator `g = (t, idx)` of `C_2`: the lift along ε of
    /// `F(f_2 g) + r(g)`. `cache` keeps the comparison composites of the generators of `C_1`
    /// between calls.
    pub fn base_homotopy(
        &self,
        d2: &FreeModuleHomomorphism<FreeModule<A>>,
        d1: &FreeModuleHomomorphism<FreeModule<A>>,
        augmentation: &dyn Fn(i32, usize) -> FpVector,
        t: i32,
        idx: usize,
        cache: &mut HashMap<(i32, usize), ComparisonComposite<A>>,
    ) -> FpVector {
        let mut value = self.pullback(d2, d1, augmentation, t, idx);
        if !value.is_empty() {
            value.add(&self.correction(d2, d1, augmentation, t, idx, cache), 1);
        }
        self.lift(&d1.target(), augmentation, t - 1, &value)
    }

    /// `r(g) ∈ M_{t-1}` for the generator `g = (t, idx)` of `C_2`: `ε(Σ A(a, Γ(e)))` over the
    /// terms `a e` of `d g`. `Γ(e)` is the composite `f_0 d + d f_1` on `e` in the pair algebra,
    /// where `f_0 x = [] ε x` and `f_1 e = Σ [b] ε x` over the terms `b x` of `d e` with `|b| > 0`
    /// are the comparison maps to the bar resolution.
    pub fn correction(
        &self,
        d2: &FreeModuleHomomorphism<FreeModule<A>>,
        d1: &FreeModuleHomomorphism<FreeModule<A>>,
        augmentation: &dyn Fn(i32, usize) -> FpVector,
        t: i32,
        idx: usize,
        cache: &mut HashMap<(i32, usize), ComparisonComposite<A>>,
    ) -> FpVector {
        let algebra = self.module.algebra();
        let p = algebra.prime();
        let mut out = FpVector::new(p, self.module.dimension(t - 1));
        let c1 = d2.target();
        let dg = d2.output(t, idx);
        for (e_deg, e_idx, a_deg, a) in c1.iter_slices(t, dg.as_slice()) {
            // A(1, -) = 0
            if a_deg == 0 || a.is_zero() {
                continue;
            }
            let gamma = cache
                .entry((e_deg, e_idx))
                .or_insert_with(|| self.comparison_composite(d1, augmentation, e_deg, e_idx));
            for ((m_deg, m_idx), elt) in gamma.iter() {
                let y_deg = a_deg + e_deg - m_deg - 1;
                if y_deg < 0 {
                    continue;
                }
                let mut y = FpVector::new(p, algebra.dimension(y_deg));
                algebra.a_multiply(y.as_slice_mut(), 1, a_deg, a, e_deg - m_deg, elt);
                for (yi, c) in y.iter_nonzero() {
                    self.module
                        .act_on_basis(out.as_slice_mut(), c, y_deg, yi, *m_deg, *m_idx);
                }
            }
        }
        out
    }

    /// `Γ(e)` for the generator `e = (e_deg, e_idx)` of `C_1`; see [`Self::correction`].
    fn comparison_composite(
        &self,
        d1: &FreeModuleHomomorphism<FreeModule<A>>,
        augmentation: &dyn Fn(i32, usize) -> FpVector,
        e_deg: i32,
        e_idx: usize,
    ) -> ComparisonComposite<A> {
        let algebra = self.module.algebra();
        let p = algebra.prime();
        let mut one = FpVector::new(p, 1);
        one.set_entry(0, 1);
        let mut elts: HashMap<(i32, usize), A::Element> = HashMap::new();
        let c0 = d1.target();
        let de = d1.output(e_deg, e_idx);
        for (x_deg, x_idx, b_deg, b) in c0.iter_slices(e_deg, de.as_slice()) {
            if b.is_zero() {
                continue;
            }
            let ex = augmentation(x_deg, c0.generator_offset(x_deg, x_deg, x_idx));
            for (mi, mc) in ex.iter_nonzero() {
                // f_0 d: b · [] m
                let elt = elts
                    .entry((x_deg, mi))
                    .or_insert_with(|| algebra.new_pair_element(e_deg - x_deg));
                algebra.sigma_multiply(elt, mc, b_deg, b, 0, one.as_slice());
                if b_deg == 0 {
                    continue;
                }
                // d f_1: [b_i] m ↦ b_i · [] m + [] (b_i m)
                for (bi, bc) in b.iter_nonzero() {
                    let coef = bc * mc % p;
                    let mut unit = FpVector::new(p, algebra.dimension(b_deg));
                    unit.set_entry(bi, 1);
                    let elt = elts
                        .entry((x_deg, mi))
                        .or_insert_with(|| algebra.new_pair_element(e_deg - x_deg));
                    algebra.sigma_multiply(elt, coef, 0, one.as_slice(), b_deg, unit.as_slice());
                    let mut bm = FpVector::new(p, self.module.dimension(e_deg));
                    if bm.is_empty() {
                        continue;
                    }
                    self.module
                        .act_on_basis(bm.as_slice_mut(), coef, b_deg, bi, x_deg, mi);
                    for (mj, c) in bm.iter_nonzero() {
                        let elt = elts
                            .entry((e_deg, mj))
                            .or_insert_with(|| algebra.new_pair_element(0));
                        algebra.sigma_multiply(elt, c, 0, one.as_slice(), 0, one.as_slice());
                    }
                }
            }
        }
        elts.into_iter()
            .map(|(k, mut elt)| {
                A::finalize_element(&mut elt);
                (k, elt)
            })
            .collect()
    }
}

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
    pub fn add_composite(&self, s: i32, degree: i32, maps: CompositeData<A>, dir: &SaveDirectory) {
        for (_, d1, d0) in &maps {
            assert!(Arc::ptr_eq(&d1.target(), &d0.source()));
            assert!(Arc::ptr_eq(&d0.target(), &self.target));
            assert_eq!(d1.degree_shift() + d0.degree_shift(), self.shift_t);
        }

        let tracing_span = tracing::Span::current();
        let f = |t, idx| {
            let _tracing_guard = tracing_span.enter();
            let g = BidegreeGenerator::s_t(s, t, idx);
            if let Some(store) = dir.store()
                && let Some(data) = store.read(SaveKind::SecondaryComposite, g).unwrap()
            {
                return SecondaryComposite::from_bytes(
                    Arc::clone(&self.target),
                    g.t() - self.shift_t,
                    self.hit_generator,
                    &mut &data[..],
                )
                .unwrap();
            }

            let mut composite = SecondaryComposite::new(
                Arc::clone(&self.target),
                g.t() - self.shift_t,
                self.hit_generator,
            );

            tracing::info_span!("Computing composite", %g).in_scope(|| {
                for (coef, d1, d0) in &maps {
                    composite.add_composite(*coef, g.t(), g.idx(), d1, d0);
                }
                composite.finalize();
            });

            if let Some(store) = dir.store() {
                let mut buf = Vec::new();
                composite.to_bytes(&mut buf).unwrap();
                store.write(SaveKind::SecondaryComposite, g, &buf).unwrap();
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

    fn max(&self) -> BidegreeRange<'_, Self>;

    fn homotopies(&self) -> &OnceBiVec<SecondaryHomotopy<Self::Algebra>>;
    fn intermediates(&self) -> &DashMap<BidegreeGenerator, FpVector>;

    fn save_dir(&self) -> &SaveDirectory;

    fn compute_intermediate(&self, g: BidegreeGenerator) -> FpVector;
    fn composite(&self, s: i32) -> CompositeData<Self::Algebra>;

    #[tracing::instrument(skip(self))]
    fn initialize_homotopies(&self) {
        let shift = self.shift();
        let max = self.max();

        self.homotopies().extend(max.s() - 1, |s| {
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
            self.homotopies()[s].add_composite(
                s,
                self.max().t(s) - 1,
                self.composite(s),
                self.save_dir(),
            );
        };

        self.homotopies().range().into_maybe_par_iter().for_each(f);
    }

    #[tracing::instrument(skip(self), ret(Display, level = Level::DEBUG), fields(%g))]
    fn get_intermediate(&self, g: BidegreeGenerator) -> FpVector {
        if let Some((_, v)) = self.intermediates().remove(&g) {
            return v;
        }

        if let Some(store) = self.save_dir().store()
            && let Some(data) = store.read(SaveKind::SecondaryIntermediate, g).unwrap()
        {
            // The target dimension can depend on whether we resolved to stem
            let mut cursor = &data[..];
            let dim = cursor.read_u64::<LittleEndian>().unwrap() as usize;
            return FpVector::from_bytes(self.prime(), dim, &mut cursor).unwrap();
        }

        let result = self.compute_intermediate(g);

        if let Some(store) = self.save_dir().store() {
            let mut buf = Vec::new();
            buf.write_u64::<LittleEndian>(result.len() as u64).unwrap();
            result.to_bytes(&mut buf).unwrap();
            store
                .write(SaveKind::SecondaryIntermediate, g, &buf)
                .unwrap();
        }

        result
    }

    #[tracing::instrument(skip(self))]
    fn compute_partial(&self, s: i32) {
        self.initialize_homotopies();
        let homotopies = self.homotopies();
        let tracing_span = tracing::Span::current();

        if s < homotopies.min_degree() {
            eprintln!(
                "Computing partial for s = {s} when minimum degree is {}",
                homotopies.min_degree()
            );
            return;
        }

        homotopies[s].add_composite(s, self.max().t(s) - 1, self.composite(s), self.save_dir());

        if let Some(homotopy) = homotopies.get(s + 1) {
            (0..self.max().t(s + 1))
                .into_maybe_par_iter()
                .for_each(|t| {
                    (0..homotopy.source.number_of_gens_in_degree(t))
                        .into_maybe_par_iter()
                        .for_each(|i| {
                            let _tracing_guard = tracing_span.enter();
                            self.get_intermediate(BidegreeGenerator::s_t(s + 1, t, i));
                        })
                });
        }
    }

    #[tracing::instrument(skip(self))]
    fn compute_intermediates(&self) {
        let tracing_span = tracing::Span::current();
        // Whether the bidegree needs no intermediates: its homotopy is computed or saved. The saved
        // homotopy is one payload per bidegree, so this is checked once, not per generator.
        let skip = |b: Bidegree| {
            let _tracing_guard = tracing_span.enter();
            self.homotopies()[b.s()].homotopies.next_degree() >= b.t()
                || self
                    .save_dir()
                    .store()
                    .is_some_and(|store| store.exists(SaveKind::SecondaryHomotopy, b))
        };
        let f = |g: BidegreeGenerator| {
            let _tracing_guard = tracing_span.enter();
            self.intermediates().insert(g, self.get_intermediate(g));
        };

        self.homotopies()
            .maybe_par_iter()
            .skip(1)
            .for_each(|(s, homotopy)| {
                homotopy
                    .composites
                    .range()
                    .into_maybe_par_iter()
                    .filter(|&t| {
                        homotopy.source.number_of_gens_in_degree(t) > 0
                            && !skip(Bidegree::s_t(s, t))
                    })
                    .for_each(|t| {
                        (0..homotopy.source.number_of_gens_in_degree(t))
                            .into_maybe_par_iter()
                            .for_each(|i| f(BidegreeGenerator::s_t(s, t, i)))
                    })
            })
    }

    fn compute_homotopy_step(&self, b: Bidegree) -> std::ops::Range<i32> {
        self.try_compute_homotopy_step(b).unwrap()
    }

    /// Fallible version of [`compute_homotopy_step`](Self::compute_homotopy_step).
    ///
    /// Returns `Err` when the input does not lift to a secondary homotopy, i.e. when the input
    /// is invalid. [`compute_homotopy_step`](Self::compute_homotopy_step) is simply
    /// `self.try_compute_homotopy_step(b).unwrap()`.
    #[tracing::instrument(skip(self), fields(%b))]
    fn try_compute_homotopy_step(&self, b: Bidegree) -> anyhow::Result<std::ops::Range<i32>> {
        let homotopy = &self.homotopies()[b.s()];
        if homotopy.homotopies.next_degree() > b.t() {
            return Ok(b.t()..b.t() + 1);
        }
        let p = self.prime();
        let shift = self.shift();
        let target_b = b - shift - Bidegree::s_t(0, 1);

        let d = self.source().differential(b.s());
        let source = self.source().module(b.s());
        let target = self.target();
        let num_gens = source.number_of_gens_in_degree(b.t());
        let target_dim = target.module(target_b.s()).dimension(target_b.t());

        if let Some(store) = self.save_dir().store()
            && let Some(data) = store.read(SaveKind::SecondaryHomotopy, b).unwrap()
        {
            let mut cursor = &data[..];
            let mut results = Vec::with_capacity(num_gens);
            for _ in 0..num_gens {
                results.push(FpVector::from_bytes(p, target_dim, &mut cursor).unwrap());
            }
            return Ok(self.homotopies()[b.s()]
                .homotopies
                .add_generators_from_rows_ooo(b.t(), results));
        }

        let tracing_span = tracing::Span::current();
        let get_intermediate = |i| {
            let _tracing_guard = tracing_span.enter();

            let g = BidegreeGenerator::new(b, i);
            let mut v = self.get_intermediate(g);
            if g.s() > shift.s() + 1 || (g.s() == shift.s() + 1 && !self.base_homotopy_is_zero()) {
                self.homotopies()[g.s() - 1].homotopies.apply(
                    v.as_slice_mut(),
                    1,
                    g.t(),
                    d.output(g.t(), g.idx()).as_slice(),
                );
            }
            v
        };

        let mut intermediates: Vec<FpVector> = (0..num_gens)
            .into_maybe_par_iter()
            .map(get_intermediate)
            .collect();

        let mut results = vec![FpVector::new(p, target_dim); num_gens];

        anyhow::ensure!(
            target.apply_quasi_inverse(&mut results, target_b, &intermediates,),
            "secondary: failed to apply quasi-inverse at {b}; the input likely does not lift"
        );

        if b.s() == shift.s() + 1 {
            // Check that we indeed had a lift
            let d = target.differential(target_b.s());
            for (src, tgt) in std::iter::zip(&results, &mut intermediates) {
                d.apply(tgt.as_slice_mut(), p - 1, target_b.t(), src.as_slice());
                anyhow::ensure!(
                    tgt.is_zero(),
                    "secondary: Failed to lift at {b}. This likely indicates an invalid input."
                );
            }
        }

        if let Some(store) = self.save_dir().store() {
            let mut buf = Vec::new();
            for row in &results {
                row.to_bytes(&mut buf).unwrap();
            }
            store.write(SaveKind::SecondaryHomotopy, b, &buf).unwrap();

            for i in 0..num_gens {
                store
                    .delete(
                        SaveKind::SecondaryIntermediate,
                        BidegreeGenerator::new(b, i),
                    )
                    .unwrap();
            }
        }

        Ok(homotopy
            .homotopies
            .add_generators_from_rows_ooo(b.t(), results))
    }

    /// Computes the homotopy at `s = shift.s()`, the input of the recursion. It is zero unless
    /// overridden.
    fn compute_base_homotopy(&self) {
        let h = &self.homotopies()[self.shift().s()];
        h.homotopies.extend_by_zero(h.composites.max_degree());
    }

    /// Whether [`compute_base_homotopy`](Self::compute_base_homotopy) produced the zero map, in
    /// which case the steps at `s = shift.s() + 1` skip applying it.
    fn base_homotopy_is_zero(&self) -> bool {
        true
    }

    #[tracing::instrument(skip(self))]
    fn compute_homotopies(&self) {
        let shift = self.shift();
        self.compute_base_homotopy();

        let min_t = self.homotopies()[shift.s()].homotopies.min_degree();
        let s_range = self.homotopies().range();
        let min = Bidegree::s_t(s_range.start + 1, min_t);
        let max = self.max().restrict(s_range.end);
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

#[cfg(test)]
pub(crate) mod tests {
    use serde_json::json;

    use super::*;
    use crate::utils;

    /// The Joker, whose zero secondary lift fails; `actions` are its `secondary_actions`.
    pub(crate) fn joker(actions: Option<&[&str]>) -> serde_json::Value {
        let mut json = json!({
            "type": "finite dimensional module",
            "p": 2,
            "gens": {"x0": 0, "x1": 1, "x2": 2, "x3": 3, "x4": 4},
            "actions": [
                "Sq1 x0 = x1",
                "Sq2 x0 = x2",
                "Sq2 x1 = x3",
                "Sq2 x2 = x4",
                "Sq1 x3 = x4"
            ]
        });
        if let Some(actions) = actions {
            json["secondary_actions"] = json!(actions);
        }
        json
    }

    /// The total rank of the d2 differentials of `lift`.
    pub(crate) fn d2_rank<CC: FreeChainComplex>(lift: &SecondaryResolution<CC>) -> usize
    where
        CC::Algebra: PairAlgebra,
    {
        let p = lift.prime();
        let underlying = lift.underlying();
        let mut rank = 0;
        for b in underlying.iter_stem() {
            if b.t() > 0 && underlying.has_computed_bidegree(b + Bidegree::n_s(-1, 2)) {
                let rows = lift.homotopy(b.s() + 2).homotopies.hom_k(b.t());
                if rows.first().is_some_and(|row| !row.is_empty()) {
                    rank += fp::matrix::Matrix::from_vec(p, &rows).row_reduce();
                }
            }
        }
        rank
    }

    /// Without secondary actions, the Joker's secondary lift fails at the first step it can.
    #[test]
    fn joker_zero_lift_fails() {
        let resolution = utils::construct_standard::<false, _, _>(
            (joker(None), algebra::AlgebraType::Milnor),
            None,
        )
        .unwrap();
        resolution.compute_through_stem(Bidegree::n_s(10, 5));
        let lift = SecondaryResolution::new(Arc::new(resolution));
        lift.initialize_homotopies();
        lift.compute_composites();
        lift.compute_base_homotopy();
        assert!(lift.try_compute_homotopy_step(Bidegree::n_s(2, 3)).is_err());
    }

    /// With its secondary actions, the Joker's secondary lift exists. The rank is that of the
    /// d2 the `secondary` example prints for the same range.
    #[test]
    fn joker_secondary_actions() {
        let resolution = utils::construct_standard::<false, _, _>(
            (
                joker(Some(&["[Sq1|Sq3] x0 = x3"])),
                algebra::AlgebraType::Milnor,
            ),
            None,
        )
        .unwrap();
        resolution.compute_through_stem(Bidegree::n_s(40, 10));
        let lift = SecondaryResolution::new(Arc::new(resolution));
        lift.extend_all();
        assert_eq!(d2_rank(&lift), 16);
    }

    /// `C4_secondary` and the split module `S^0 ∨ S^1` have the same cohomology and differ only
    /// in their secondary actions. Below the sphere's first d2 (from `h_4`), the split module has
    /// no d2, while `C4_secondary` has `d2(h_0^s ι_1) = h_0^{s+2} ι_0` among others.
    #[test]
    fn c4_secondary_actions() {
        let rank = |json: serde_json::Value| {
            let resolution = utils::construct_standard::<false, _, _>(
                (json, algebra::AlgebraType::Milnor),
                None,
            )
            .unwrap();
            resolution.compute_through_stem(Bidegree::n_s(13, 8));
            let lift = SecondaryResolution::new(Arc::new(resolution));
            lift.extend_all();
            d2_rank(&lift)
        };
        let c4 = utils::load_module_json("C4_secondary").unwrap();
        let mut split = c4.clone();
        split["secondary_actions"].take();
        assert_eq!(rank(split), 0);
        assert_eq!(rank(c4), 11);
    }

    /// Malformed `secondary_actions` entries are rejected with a message naming the problem.
    #[test]
    fn secondary_actions_parse_errors() {
        let module: Arc<dyn Module<Algebra = algebra::MilnorAlgebra>> = Arc::new(
            algebra::module::FDModule::from_json(
                Arc::new(algebra::MilnorAlgebra::new(fp::prime::TWO, false)),
                &joker(None),
            )
            .unwrap(),
        );
        let parse = |entry: &str| {
            SecondaryActions::from_json(Arc::clone(&module), &joker(Some(&[entry])))
                .map_err(|e| format!("{e:#}"))
        };
        assert!(parse("[Sq1|Sq3] x0 = x3").unwrap().is_some());
        assert!(parse("[Sq1|Sq3] x0 = 0").is_ok());
        assert!(parse("[Sq1|Sq3] x0 = x4").unwrap_err().contains("degree 3"));
        assert!(
            parse("[Sq1|Sq3] y0 = x3")
                .unwrap_err()
                .contains("Unknown module element")
        );
        assert!(
            parse("[Sq0|Sq4] x0 = x3")
                .unwrap_err()
                .contains("positive degree")
        );
        assert!(parse("Sq1 Sq3 x0 = x3").unwrap_err().contains("[a|b] m"));
        assert!(
            SecondaryActions::from_json(Arc::clone(&module), &joker(None))
                .unwrap()
                .is_none()
        );
    }

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

    #[test]
    fn cofib_h4_try_returns_err() {
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

        // The failing bidegree is (n, s) = (14, 3), i.e. s = 3, t = 17.
        let failing = Bidegree::n_s(14, 3);

        // Compute all prerequisite data, then drive the homotopy steps exactly as
        // `compute_homotopies` does, but skip the failing step (and everything that comes
        // after it) by returning a dummy "already computed" range. This computes all the
        // predecessors of `failing` without panicking, leaving `failing` to be computed
        // explicitly via the fallible path below.
        lift.initialize_homotopies();
        lift.compute_composites();
        lift.compute_intermediates();

        let shift = lift.shift();
        lift.compute_base_homotopy();
        let min_t = lift.homotopies()[shift.s()].homotopies.min_degree();
        let s_range = lift.homotopies().range();
        let min = Bidegree::s_t(s_range.start + 1, min_t);
        let max = lift.max().restrict(s_range.end);
        sseq::coordinates::iter_s_t(
            &|b| {
                if b.s() > failing.s() || (b.s() == failing.s() && b.t() >= failing.t()) {
                    // Skip the failing step and everything after it.
                    return b.t()..b.t() + 1;
                }
                lift.compute_homotopy_step(b)
            },
            min,
            max,
        );

        // The failing step should report an error rather than panicking.
        let result = lift.try_compute_homotopy_step(failing);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to lift"));
    }
}
