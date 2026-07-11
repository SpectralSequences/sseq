use std::cell::Cell;

use fp::{
    prime::{Binomial, Prime, ValidPrime, factor_pk, iter::BitflagIterator},
    vector::{FpSlice, FpSliceMut, FpVector},
};
use itertools::Itertools;
use once::OnceVec;
use rustc_hash::FxHashMap as HashMap;
use serde::{Deserialize, Serialize};

use crate::algebra::{Algebra, Bialgebra, GeneratedAlgebra, UnstableAlgebra, combinatorics};

/// Wrap profiling-only statements. Expands to nothing unless the crate is built with the
/// `MILNOR_PROFILE` environment variable set (which turns on `cfg(milnor_profile)`; see `build.rs`),
/// so the hot-path hooks below cost *nothing* — not even argument evaluation — in normal builds.
macro_rules! profile {
    ($($body:tt)*) => {
        #[cfg(milnor_profile)]
        {
            $($body)*
        }
    };
}

/// Compile-time-gated counters for the Milnor multiplication hot path, to pinpoint what to
/// optimize (how sparse the operands are, how much matrix enumeration is wasted, how many index
/// lookups we pay, which path is taken, …).
///
/// Enable by building with `MILNOR_PROFILE=1` (e.g.
/// `MILNOR_PROFILE=1 cargo run --release --example nassau_e2e -- 80 42 1`); otherwise every counter
/// and hook is removed by `#[cfg]`. Call [`report`] to print a summary to stderr.
pub mod profile {
    #[cfg(milnor_profile)]
    pub use enabled::*;

    /// Print the collected multiply statistics to stderr (or a note if profiling is disabled).
    #[cfg(not(milnor_profile))]
    pub fn report() {
        eprintln!(
            "[milnor profile] disabled — rebuild with `MILNOR_PROFILE=1` to collect multiply stats"
        );
    }

    #[cfg(milnor_profile)]
    mod enabled {
        use std::sync::{
            LazyLock, Mutex,
            atomic::{AtomicU64, Ordering::Relaxed},
        };

        use rustc_hash::FxHashMap;

        macro_rules! counters {
            ($($(#[$m:meta])* $name:ident),* $(,)?) => {
                $($(#[$m])* pub static $name: AtomicU64 = AtomicU64::new(0);)*
            };
        }
        counters! {
            /// `multiply_basis_element_by_element` (p=2) calls that do work.
            MBE_CALLS,
            /// … that took the admissible-matrix sweep.
            MBE_ADMISSIBLE,
            /// … that fell back to the per-term `PPartMultiplier` path.
            MBE_PERTERM,
            /// … where the operation was the identity `Sq(∅)`.
            MBE_IDENTITY,
            /// Basis×basis `multiply_with_allocation` invocations.
            KERNEL_CALLS,
            /// `PPartMultiplier` candidate terms yielded (before the index/bound checks).
            PPART_TERMS,
            /// `basis_element_to_index` lookups on the multiply hot path.
            INDEX_LOOKUPS,
            /// `add_basis_element` calls that actually contribute an output term.
            OUTPUT_ADDS,
            /// Admissible matrices enumerated (admissible path only).
            ADM_MATRICES,
            /// (matrix, term) compatibility tests (admissible path only).
            ADM_TESTS,
        }

        /// Histogram of `nnz(s)` — the number of non-zero terms of the element being acted on.
        pub static NNZ_HIST: LazyLock<Mutex<FxHashMap<usize, u64>>> =
            LazyLock::new(|| Mutex::new(FxHashMap::default()));
        /// Histogram of `(operation_degree, element_degree)` per call.
        pub static DEG_HIST: LazyLock<Mutex<FxHashMap<(i32, i32), u64>>> =
            LazyLock::new(|| Mutex::new(FxHashMap::default()));
        /// Total element terms multiplied by each distinct operation `R = (degree, index)`. This is
        /// the GPU-occupancy metric for Nassau's `Sq(R) · Σ Sq(Sⱼ)` kernel: batching all work sharing
        /// one `R` (uniform matrix enumeration) gives this many threads' worth of uniform work, so
        /// the distribution says how well workgroups would fill.
        pub static R_TERMS: LazyLock<Mutex<FxHashMap<(i32, usize), u64>>> =
            LazyLock::new(|| Mutex::new(FxHashMap::default()));

        pub fn record_call(nnz: usize, r_degree: i32, r_index: usize, s_degree: i32) {
            MBE_CALLS.fetch_add(1, Relaxed);
            *NNZ_HIST.lock().unwrap().entry(nnz).or_default() += 1;
            *DEG_HIST
                .lock()
                .unwrap()
                .entry((r_degree, s_degree))
                .or_default() += 1;
            *R_TERMS
                .lock()
                .unwrap()
                .entry((r_degree, r_index))
                .or_default() += nnz as u64;
        }

        /// Per–GPU-launch accumulation of `R → element-terms`, where one launch = one
        /// `get_partial_matrix` (matrix build). Unlike [`R_TERMS`], which sums an operation `R`
        /// across the *whole* resolution, this asks how much same-`R` work is co-located within a
        /// single matrix build — the largest unit a kernel could batch without buffering across the
        /// streaming algorithm. `depth` keeps [`scope_begin`]/[`scope_end`] reentrancy-safe: only the
        /// outermost pair clears and folds, so any nested matrix build is attributed to the outer
        /// launch rather than corrupting it. (Measurement must be run serially — the `concurrent`
        /// feature would let two launches interleave into one scope and inflate the batch sizes.)
        struct ScopeState {
            depth: u32,
            /// `(homomorphism id, degree)` of the currently open launch — its merged-scope key.
            key: (usize, i32),
            map: FxHashMap<(i32, usize), u64>,
        }
        static SCOPE: LazyLock<Mutex<ScopeState>> = LazyLock::new(|| {
            Mutex::new(ScopeState {
                depth: 0,
                key: (0, 0),
                map: FxHashMap::default(),
            })
        });

        /// Coarser accumulation: `R → element-terms` merged across every launch sharing a
        /// `(homomorphism id, degree)` key — i.e. all the per-signature `get_partial_matrix` builds
        /// of one differential at one bidegree, which all read the same matrix and so *could* be
        /// fused into a single kernel launch (computing the full matrix once and slicing it). Held to
        /// report time and folded there, to measure how much the realizable occupancy improves when
        /// the launch scope is widened from one masked build to one whole bidegree.
        #[allow(clippy::type_complexity)]
        static MERGED: LazyLock<Mutex<FxHashMap<(usize, i32), FxHashMap<(i32, usize), u64>>>> =
            LazyLock::new(|| Mutex::new(FxHashMap::default()));

        /// Workgroup sizes for the realizable-coverage report (mirrors the global one).
        const SCOPE_WS: [u64; 6] = [32, 64, 128, 256, 512, 1024];
        /// Number of launch scopes (matrix builds) that did any work.
        static SCOPE_COUNT: AtomicU64 = AtomicU64::new(0);
        /// Sum over scopes of each scope's total element-terms (equals the global term-work).
        static SCOPE_TERMS: AtomicU64 = AtomicU64::new(0);
        /// Largest single-scope total element-terms.
        static SCOPE_TERMS_MAX: AtomicU64 = AtomicU64::new(0);
        /// Per `W`, term-work in `R`s that reach ≥ `W` terms *within their own launch*.
        static SCOPE_COVER: [AtomicU64; 6] = [
            AtomicU64::new(0),
            AtomicU64::new(0),
            AtomicU64::new(0),
            AtomicU64::new(0),
            AtomicU64::new(0),
            AtomicU64::new(0),
        ];

        /// Open a launch scope around a matrix build of homomorphism `hom_id` at `degree`.
        /// Reentrancy-safe (see [`ScopeState`]); the key is set by the outermost open.
        pub fn scope_begin(hom_id: usize, degree: i32) {
            let mut s = SCOPE.lock().unwrap();
            if s.depth == 0 {
                s.map.clear();
                s.key = (hom_id, degree);
            }
            s.depth += 1;
        }

        /// Attribute `nnz` element-terms of operation `R = (r_degree, r_index)` to the open launch,
        /// both to its per-launch histogram and to its `(hom, degree)` merged bucket.
        pub fn scope_record(r_degree: i32, r_index: usize, nnz: usize) {
            if nnz == 0 {
                return;
            }
            let key = {
                let mut s = SCOPE.lock().unwrap();
                if s.depth == 0 {
                    return;
                }
                *s.map.entry((r_degree, r_index)).or_default() += nnz as u64;
                s.key
            };
            *MERGED
                .lock()
                .unwrap()
                .entry(key)
                .or_default()
                .entry((r_degree, r_index))
                .or_default() += nnz as u64;
        }

        /// Close a launch scope; the outermost close folds the scope's `R`-histogram into the
        /// realizable-coverage accumulators.
        pub fn scope_end() {
            let mut s = SCOPE.lock().unwrap();
            if s.depth == 0 {
                return;
            }
            s.depth -= 1;
            if s.depth > 0 {
                return;
            }
            let total: u64 = s.map.values().sum();
            if total == 0 {
                return;
            }
            SCOPE_COUNT.fetch_add(1, Relaxed);
            SCOPE_TERMS.fetch_add(total, Relaxed);
            SCOPE_TERMS_MAX.fetch_max(total, Relaxed);
            for (i, &w) in SCOPE_WS.iter().enumerate() {
                let work: u64 = s.map.values().filter(|&&t| t >= w).sum();
                SCOPE_COVER[i].fetch_add(work, Relaxed);
            }
            s.map.clear();
        }

        pub fn admissible() {
            MBE_ADMISSIBLE.fetch_add(1, Relaxed);
        }
        pub fn perterm() {
            MBE_PERTERM.fetch_add(1, Relaxed);
        }
        pub fn identity() {
            MBE_IDENTITY.fetch_add(1, Relaxed);
        }
        pub fn kernel_call() {
            KERNEL_CALLS.fetch_add(1, Relaxed);
        }
        pub fn ppart_term() {
            PPART_TERMS.fetch_add(1, Relaxed);
        }
        pub fn index_lookup() {
            INDEX_LOOKUPS.fetch_add(1, Relaxed);
        }
        pub fn output_add() {
            OUTPUT_ADDS.fetch_add(1, Relaxed);
        }
        pub fn adm_matrix() {
            ADM_MATRICES.fetch_add(1, Relaxed);
        }
        pub fn adm_test() {
            ADM_TESTS.fetch_add(1, Relaxed);
        }

        pub fn report() {
            let calls = MBE_CALLS.load(Relaxed);
            if calls == 0 {
                eprintln!("[milnor profile] no multiply_basis_element_by_element (p=2) calls seen");
                return;
            }
            let (adm, per, ident) = (
                MBE_ADMISSIBLE.load(Relaxed),
                MBE_PERTERM.load(Relaxed),
                MBE_IDENTITY.load(Relaxed),
            );
            let zero = calls - adm - per - ident;
            let adds = OUTPUT_ADDS.load(Relaxed);
            let pct = |x: u64| 100.0 * x as f64 / calls as f64;
            let per_add = |x: u64| {
                if adds > 0 {
                    x as f64 / adds as f64
                } else {
                    0.0
                }
            };

            eprintln!("================= Milnor multiply profile =================");
            eprintln!("multiply_basis_element_by_element (p=2) calls : {calls}");
            eprintln!("  admissible-matrix path : {adm:>12} ({:5.1}%)", pct(adm));
            eprintln!("  per-term path          : {per:>12} ({:5.1}%)", pct(per));
            eprintln!(
                "  identity Sq(∅)         : {ident:>12} ({:5.1}%)",
                pct(ident)
            );
            eprintln!("  zero element (nnz=0)   : {zero:>12} ({:5.1}%)", pct(zero));

            let hist = NNZ_HIST.lock().unwrap();
            let n: u64 = hist.values().sum();
            let weighted: u64 = hist.iter().map(|(k, v)| *k as u64 * v).sum();
            eprintln!(
                "element term-count nnz : mean {:.2} over {n} calls",
                weighted as f64 / n.max(1) as f64
            );
            let mut keys: Vec<usize> = hist.keys().copied().collect();
            keys.sort_unstable();
            let mut cum = 0u64;
            for k in keys {
                let v = hist[&k];
                cum += v;
                if k <= 6 || k.is_multiple_of(10) {
                    eprintln!(
                        "  nnz={k:<3} {:>6.2}%  cum {:>6.2}%",
                        100.0 * v as f64 / n as f64,
                        100.0 * cum as f64 / n as f64
                    );
                }
            }

            eprintln!(
                "kernel basis×basis multiply_with_allocation : {}",
                KERNEL_CALLS.load(Relaxed)
            );
            eprintln!("output basis-element adds : {adds}");
            eprintln!(
                "PPartMultiplier candidate terms : {} ({:.2} per output add)",
                PPART_TERMS.load(Relaxed),
                per_add(PPART_TERMS.load(Relaxed))
            );
            eprintln!(
                "basis_element_to_index lookups  : {} ({:.2} per output add)",
                INDEX_LOOKUPS.load(Relaxed),
                per_add(INDEX_LOOKUPS.load(Relaxed))
            );
            let matrices = ADM_MATRICES.load(Relaxed);
            if matrices > 0 {
                eprintln!(
                    "admissible matrices enumerated  : {matrices} ({:.1} term-tests each)",
                    ADM_TESTS.load(Relaxed) as f64 / matrices as f64
                );
            }

            let deg = DEG_HIST.lock().unwrap();
            let mut pairs: Vec<((i32, i32), u64)> = deg.iter().map(|(k, v)| (*k, *v)).collect();
            pairs.sort_by_key(|(_, v)| std::cmp::Reverse(*v));
            eprintln!("top (operation_degree, element_degree) call sites:");
            for ((r, s), v) in pairs.iter().take(10) {
                eprintln!(
                    "  op={r:<3} elt={s:<3} : {v} ({:.1}%)",
                    100.0 * *v as f64 / calls as f64
                );
            }

            // GPU-occupancy view: group all element terms by the operation `R` they are multiplied
            // by. Nassau's kernel (`Sq(R) · Σ Sq(Sⱼ)`) parallelizes over terms sharing one `R` with
            // uniform matrix enumeration, so a workgroup of size `W` is well-filled only by `R`s that
            // accumulate ≥ `W` terms. We report what fraction of all term-work lives in such `R`s.
            let r_terms = R_TERMS.lock().unwrap();
            let distinct = r_terms.len() as u64;
            let total_terms: u64 = r_terms.values().sum();
            let max_terms = r_terms.values().copied().max().unwrap_or(0);
            eprintln!(
                "GPU occupancy — distinct operations R: {distinct}, total element terms: \
                 {total_terms}, mean terms/R: {:.1}, max: {max_terms}",
                total_terms as f64 / distinct.max(1) as f64
            );
            eprintln!("  fraction of term-work in R's with ≥ W terms (W = workgroup size):");
            for w in [32u64, 64, 128, 256, 512, 1024] {
                let (n_r, work): (u64, u64) =
                    r_terms.values().fold(
                        (0, 0),
                        |(n, s), &t| {
                            if t >= w { (n + 1, s + t) } else { (n, s) }
                        },
                    );
                eprintln!(
                    "    W={w:<4} : {n_r:>6} R's ({:>5.1}% of R's) cover {:>5.1}% of term-work",
                    100.0 * n_r as f64 / distinct.max(1) as f64,
                    100.0 * work as f64 / total_terms.max(1) as f64
                );
            }

            // The number above aggregates each R across the *whole* resolution. A kernel can only
            // batch work that is co-located in one launch, so this is the realizable version: R-work
            // is re-counted per `get_partial_matrix` (matrix build), the largest unit batchable
            // without buffering across the streaming algorithm. If these percentages collapse
            // relative to the global ones, the amortization-by-R shape does not survive at launch
            // granularity, and a GPU kernel must lean on raw parallel width (many independent
            // products per launch) rather than on many terms sharing one R.
            let scopes = SCOPE_COUNT.load(Relaxed);
            if scopes > 0 {
                let scope_terms = SCOPE_TERMS.load(Relaxed);
                eprintln!(
                    "GPU occupancy — REALIZABLE per matrix-build launch ({scopes} launches, mean \
                     {:.1} terms/launch, max {}):",
                    scope_terms as f64 / scopes as f64,
                    SCOPE_TERMS_MAX.load(Relaxed)
                );
                eprintln!("  fraction of term-work in R's reaching ≥ W terms *within one launch*:");
                for (i, w) in SCOPE_WS.iter().enumerate() {
                    let work = SCOPE_COVER[i].load(Relaxed);
                    eprintln!(
                        "    W={w:<4} : {:>5.1}% of term-work",
                        100.0 * work as f64 / scope_terms.max(1) as f64
                    );
                }
            }

            // Coarser scope: merge the per-signature builds of one differential at one bidegree into
            // a single launch (they read the same matrix, so fusing them is free of extra multiply
            // work). This upper-bounds the realizable occupancy for a kernel that computes the full
            // bidegree matrix once instead of one masked slice per signature.
            let merged = MERGED.lock().unwrap();
            if !merged.is_empty() {
                let n_launches = merged.len() as u64;
                let mut total = 0u64;
                let mut max_terms = 0u64;
                let mut cover = [0u64; 6];
                for hist in merged.values() {
                    let t: u64 = hist.values().sum();
                    total += t;
                    max_terms = max_terms.max(t);
                    for (i, &w) in SCOPE_WS.iter().enumerate() {
                        cover[i] += hist.values().filter(|&&x| x >= w).sum::<u64>();
                    }
                }
                eprintln!(
                    "GPU occupancy — MERGED per (differential, bidegree) launch ({n_launches} \
                     launches, mean {:.1} terms/launch, max {max_terms}):",
                    total as f64 / n_launches.max(1) as f64
                );
                eprintln!("  fraction of term-work in R's reaching ≥ W terms *within one launch*:");
                for (i, w) in SCOPE_WS.iter().enumerate() {
                    eprintln!(
                        "    W={w:<4} : {:>5.1}% of term-work",
                        100.0 * cover[i] as f64 / total.max(1) as f64
                    );
                }
            }
            eprintln!("===========================================================");
        }
    }
}

fn q_part_default() -> u32 {
    !0
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MilnorProfile {
    /// If `true`, unspecified p_part entries will be 0. Otherwise they will be infinity.
    pub truncated: bool,
    /// A bitmask indicating which of the Q_k we want to include (1 = include). Defaults to `!0`.
    /// This is only relevant at odd primes.
    #[serde(default = "q_part_default")]
    pub q_part: u32,
    /// The profile function for the Q part.
    #[serde(default)]
    pub p_part: PPart,
}

impl MilnorProfile {
    pub fn is_trivial(&self) -> bool {
        !self.truncated && self.q_part == !0 && self.p_part.is_empty()
    }

    pub fn get_p_part(&self, i: usize) -> PPartEntry {
        self.p_part
            .get(i)
            .copied()
            .unwrap_or(if self.truncated { 0 } else { PPartEntry::MAX })
    }

    /// Checks whether the profile function is valid
    pub fn is_valid(&self) -> bool {
        for (i, &hi) in self.p_part.iter().enumerate() {
            for (j, &hj) in self.p_part.iter().enumerate().skip(i + 1) {
                if hi > (j - i) as PPartEntry + hj && self.p_part[j - i - 1] > hj {
                    return false;
                }
            }
        }
        if self.truncated {
            let len = self.p_part.len();
            for (i, &hi) in self.p_part.iter().enumerate() {
                if hi > (len - i) as PPartEntry {
                    return false;
                }
            }
        }
        if self.q_part != !0 {
            for i in BitflagIterator::set_bit_iterator(!self.q_part as u64) {
                for j in 0..i {
                    if (self.q_part >> j) & 1 == 1 && self.get_p_part(i - j - 1) > j as PPartEntry {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Whether the profile is that of A(n). This is relevant since A(n) is generated by P(p^n) and
    /// β but a general subalgebra is not.
    pub fn is_an(&self, generic: bool) -> bool {
        if *self == Self::default() {
            return true;
        }
        if !self.truncated {
            return false;
        }
        if self.p_part.len() != self.p_part[0] as usize {
            return false;
        }
        if generic && self.q_part != (1 << (self.p_part.len() + 1)) - 1 {
            return false;
        }
        true
    }
}

impl Default for MilnorProfile {
    fn default() -> Self {
        Self {
            truncated: false,
            q_part: !0,
            p_part: Vec::new(),
        }
    }
}

pub type PPartEntry = u32;
pub type PPart = Vec<PPartEntry>;

#[derive(Debug, Clone, Default)]
pub struct MilnorBasisElement {
    pub q_part: u32,
    pub p_part: PPart,
    pub degree: i32,
}

impl MilnorBasisElement {
    fn from_p(p_part: PPart, degree: i32) -> Self {
        Self {
            q_part: 0,
            p_part,
            degree,
        }
    }

    fn excess(&self, p: ValidPrime) -> u32 {
        if p == 2 {
            self.p_part.iter().sum::<PPartEntry>()
        } else {
            self.q_part.count_ones() + 2 * self.p_part.iter().sum::<PPartEntry>()
        }
    }

    pub fn clone_into(&self, other: &mut Self) {
        other.q_part = self.q_part;
        other.degree = self.degree;
        other.p_part.clear();
        other.p_part.extend_from_slice(&self.p_part);
    }

    /// Update the degree component to the correct degree
    pub fn compute_degree(&mut self, p: ValidPrime) {
        let q = if p == 2 { 1 } else { 2 * (p.as_i32() - 1) };
        let xi_degrees = combinatorics::xi_degrees(p);
        let tau_degrees = combinatorics::tau_degrees(p);

        self.degree = q * std::iter::zip(xi_degrees, &self.p_part)
            .map(|(&a, &b)| a * b as i32)
            .sum::<i32>()
            + BitflagIterator::set_bit_iterator(self.q_part as u64)
                .map(|k| tau_degrees[k])
                .sum::<i32>();
    }
}

impl std::cmp::PartialEq for MilnorBasisElement {
    fn eq(&self, other: &Self) -> bool {
        #[cfg(feature = "odd-primes")]
        return self.p_part == other.p_part && self.q_part == other.q_part;

        #[cfg(not(feature = "odd-primes"))]
        return self.p_part == other.p_part;
    }
}

impl std::cmp::Eq for MilnorBasisElement {}

impl std::hash::Hash for MilnorBasisElement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.p_part.hash(state);
        #[cfg(feature = "odd-primes")]
        self.q_part.hash(state);
    }
}

impl std::fmt::Display for MilnorBasisElement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.degree == 0 {
            write!(f, "1")?;
            return Ok(());
        }
        if self.q_part != 0 {
            let q_part = BitflagIterator::set_bit_iterator(self.q_part as u64)
                .map(|idx| format!("Q_{idx}"))
                .format(" ");
            write!(f, "{q_part}")?;
        }
        if !self.p_part.is_empty() {
            if self.q_part != 0 {
                write!(f, " ")?;
            }
            write!(f, "P({})", self.p_part.iter().format(", "))?;
        }
        Ok(())
    }
}

/// A version of `HashMap<MilnorBasisElement, T>` that is more efficient at the prime 2.
#[cfg(feature = "odd-primes")]
type MilnorHashMap<V> = HashMap<MilnorBasisElement, V>;

#[cfg(not(feature = "odd-primes"))]
struct MilnorHashMap<V> {
    degree: i32,
    inner: HashMap<u64, V>,
}

#[cfg(not(feature = "odd-primes"))]
impl<V> Default for MilnorHashMap<V> {
    fn default() -> Self {
        Self {
            degree: -1,
            inner: HashMap::default(),
        }
    }
}

#[cfg(not(feature = "odd-primes"))]
impl<V> MilnorHashMap<V> {
    /// Encode a [`MilnorBasisElement`] of a known degree into a `u64`. This is achieved by packing
    /// the PPart into a single `u64`, where we omit the first entry since it can be derived from
    /// the degree. This currently supports elements up to degree 2^9 * 3 = 1536.
    fn code(x: &MilnorBasisElement) -> u64 {
        let mut counter = 0;
        let mut shift = 0;
        for (idx, &entry) in x.p_part.iter().skip(1).enumerate() {
            counter += (entry as u64) << shift;
            shift += 9 - idx;
        }
        counter
    }

    fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }

    fn insert(&mut self, k: MilnorBasisElement, v: V) {
        if self.degree == -1 {
            self.degree = k.degree;
        }
        assert_eq!(k.degree, self.degree);
        assert!(
            self.inner.insert(Self::code(&k), v).is_none(),
            "Duplicate entry for {k}"
        );
    }

    fn get(&self, k: &MilnorBasisElement) -> Option<&V> {
        assert_eq!(k.degree, self.degree);
        self.inner.get(&Self::code(k))
    }
}

/// Flat, contiguous storage for the "seqno" (hash-free index) computation. See
/// [`MilnorAlgebra::compute_seqno_tables`] for how `g` is derived and [`MilnorAlgebra::seqno`] for
/// how it is read. Row-major with a fixed `width` (the number of ξ-degrees), so entry `(e, h)` lives
/// at `g[e * width + h]`; degrees `0..=max_degree` are populated.
struct SeqnoTables {
    max_degree: i32,
    width: usize,
    g: Vec<usize>,
}

pub struct MilnorAlgebra {
    profile: MilnorProfile,
    p: ValidPrime,
    #[cfg(feature = "odd-primes")]
    generic: bool,

    unstable_enabled: bool,

    /// This is a list of possible P(R) of each degree, where `ppart_table[i]` contains elements of
    /// degree `q * i`.
    ppart_table: OnceVec<Vec<PPart>>,

    /// A list of all basis elements of each degree, constructed from [`Self::ppart_table`]
    basis_table: OnceVec<Vec<MilnorBasisElement>>,

    excess_table: OnceVec<Vec<usize>>,

    /// degree -> MilnorBasisElement -> index
    basis_element_to_index_map: OnceVec<MilnorHashMap<usize>>,

    /// Table backing the "seqno" (hash-free index) computation, populated only when
    /// [`Self::seqno_applicable`] holds (p = 2, trivial profile, stable). It holds the flat,
    /// row-major `g` array described in [`Self::compute_seqno_tables`]; [`Self::seqno`] ranks a
    /// `p_part` from it with plain array indexing and no hash lookup. Stored behind an
    /// [`arc_swap::ArcSwapOption`] rather than a [`OnceVec`] so that reads on the hot path are a
    /// single guard load followed by direct indexing — the earlier `OnceVec<Vec<_>>` layout paid two
    /// atomics *per table access*, which is what made the table lose to the hashmap.
    seqno_tables: arc_swap::ArcSwapOption<SeqnoTables>,

    #[cfg(feature = "cache-multiplication")]
    /// source_deg -> target_deg -> source_op -> target_op
    multiplication_table: OnceVec<OnceVec<Vec<Vec<FpVector>>>>,
}

impl std::fmt::Display for MilnorAlgebra {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "MilnorAlgebra(p={})", self.prime())
    }
}

impl MilnorAlgebra {
    pub fn new(p: ValidPrime, unstable_enabled: bool) -> Self {
        Self::new_with_profile(p, MilnorProfile::default(), unstable_enabled)
    }

    pub fn new_with_profile(p: ValidPrime, profile: MilnorProfile, unstable_enabled: bool) -> Self {
        assert!(profile.is_valid());
        Self {
            p,
            #[cfg(feature = "odd-primes")]
            generic: p != 2,
            unstable_enabled,
            profile,
            ppart_table: OnceVec::new(),
            basis_table: OnceVec::new(),
            excess_table: OnceVec::new(),
            basis_element_to_index_map: OnceVec::new(),
            seqno_tables: arc_swap::ArcSwapOption::empty(),
            #[cfg(feature = "cache-multiplication")]
            multiplication_table: OnceVec::new(),
        }
    }

    #[inline]
    pub fn generic(&self) -> bool {
        #[cfg(feature = "odd-primes")]
        {
            self.generic
        }

        #[cfg(not(feature = "odd-primes"))]
        {
            false
        }
    }

    pub fn q(&self) -> i32 {
        if self.generic() {
            2 * (self.prime().as_i32() - 1)
        } else {
            1
        }
    }

    pub fn profile(&self) -> &MilnorProfile {
        &self.profile
    }

    pub fn basis_element_from_index(&self, degree: i32, idx: usize) -> &MilnorBasisElement {
        &self.basis_table[degree as usize][idx]
    }

    pub fn try_basis_element_to_index(&self, elt: &MilnorBasisElement) -> Option<usize> {
        // NB: the table-based [`Self::seqno`] computes this same index without a hash, but it loses
        // to this hashmap on the CPU. Even after moving its tables to flat, contiguous
        // `arc_swap`-backed storage (removing the earlier `OnceVec` per-access atomics), the
        // `benches/seqno` A/B still measures raw lookups at ~50 Melem/s for `seqno` vs ~115 Melem/s
        // for this hashmap — a ~2.3× gap that is flat across degree: computing the rank (a degree
        // sum plus two indexed table reads per populated ξ-position) is simply more work than one
        // hash and probe. `seqno` is therefore kept as the GPU-oriented index (a GPU kernel cannot
        // carry a hashmap, and the flat table uploads directly), not for the CPU hot path.
        self.basis_element_to_index_map[elt.degree as usize]
            .get(elt)
            .copied()
    }

    pub fn basis_element_to_index(&self, elt: &MilnorBasisElement) -> usize {
        self.try_basis_element_to_index(elt)
            .unwrap_or_else(|| panic!("Didn't find element: {elt:?}"))
    }

    /// Gives a list of PPart's in degree `t`.
    pub fn ppart_table(&self, t: i32) -> &[PPart] {
        &self.ppart_table[t as usize]
    }
}

impl Algebra for MilnorAlgebra {
    fn prefix(&self) -> &str {
        "milnor"
    }

    fn magic(&self) -> u32 {
        (self.p << 16)
            + if self.profile.is_trivial() {
                0x8000
            } else {
                0x8001
            }
    }

    fn prime(&self) -> ValidPrime {
        self.p
    }

    fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
        let mut products = Vec::with_capacity(4);
        let max_degree = if self.generic() {
            if self.profile.q_part & 1 != 0 {
                products.push((
                    "a_0".to_string(),
                    MilnorBasisElement {
                        degree: 1,
                        q_part: 1,
                        p_part: vec![],
                    },
                ));
            }
            if (self.profile.p_part.is_empty() && !self.profile.truncated)
                || (!self.profile.p_part.is_empty() && self.profile.p_part[0] > 0)
            {
                products.push((
                    "h_0".to_string(),
                    MilnorBasisElement {
                        degree: (2 * self.prime() - 2) as i32,
                        q_part: 0,
                        p_part: vec![1],
                    },
                ));
            }
            (2 * self.prime() - 2) as i32
        } else {
            let mut max = 4;
            if !self.profile.p_part.is_empty() {
                max = std::cmp::min(4, self.profile.p_part[0]);
            } else if self.profile.truncated {
                max = 0;
            }
            for i in 0..max {
                let degree = 1 << i; // degree is 2^hi
                products.push((
                    format!("h_{i}"),
                    MilnorBasisElement {
                        degree,
                        q_part: 0,
                        p_part: vec![1 << i],
                    },
                ));
            }
            1 << 3
        };
        self.compute_basis(max_degree + 1);

        products
            .into_iter()
            .map(|(name, b)| (name, b.degree, self.basis_element_to_index(&b)))
            .collect()
    }

    fn compute_basis(&self, max_degree: i32) {
        self.compute_ppart(max_degree);

        if self.generic() {
            self.generate_basis_generic(max_degree);
        } else {
            self.generate_basis_2(max_degree);
        }

        // The `seqno` tables are *not* built here: `seqno` lost to the hashmap on the CPU (see
        // `try_basis_element_to_index`), so a normal resolution should not pay to build tables it
        // won't use. A GPU backend that needs the hash-free index calls `compute_seqno_tables`.

        // Populate hash map
        self.basis_element_to_index_map
            .extend(max_degree as usize, |d| {
                let basis = &self.basis_table[d];
                let mut map = MilnorHashMap::default();
                map.reserve(basis.len());
                for (i, b) in basis.iter().enumerate() {
                    map.insert(b.clone(), i);
                }
                map
            });

        #[cfg(feature = "cache-multiplication")]
        {
            self.multiplication_table
                .extend(max_degree as usize, |_| OnceVec::new());

            for d in 0..=max_degree as usize {
                self.multiplication_table[d].extend(max_degree as usize - d, |e| {
                    (0..self.dimension(d as i32))
                        .map(|i| {
                            (0..self.dimension(e as i32))
                                .map(|j| {
                                    let mut res =
                                        FpVector::new(self.prime(), self.dimension((d + e) as i32));
                                    self.multiply(
                                        res.as_slice_mut(),
                                        1,
                                        &self.basis_table[d][i],
                                        &self.basis_table[e][j],
                                    );
                                    res
                                })
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>()
                });
            }
        }

        if self.unstable_enabled {
            self.generate_excess_table(max_degree);
        }
    }

    fn dimension(&self, degree: i32) -> usize {
        if degree < 0 {
            return 0;
        }
        self.basis_table[degree as usize].len()
    }

    #[cfg(not(feature = "cache-multiplication"))]
    fn multiply_basis_elements(
        &self,
        result: FpSliceMut,
        coef: u32,
        r_degree: i32,
        r_idx: usize,
        s_degree: i32,
        s_idx: usize,
    ) {
        self.multiply(
            result,
            coef,
            self.basis_element_from_index(r_degree, r_idx),
            self.basis_element_from_index(s_degree, s_idx),
        );
    }

    #[cfg(feature = "cache-multiplication")]
    fn multiply_basis_elements(
        &self,
        mut result: FpSliceMut,
        coef: u32,
        r_degree: i32,
        r_idx: usize,
        s_degree: i32,
        s_idx: usize,
    ) {
        result.add(
            self.multiplication_table[r_degree as usize][s_degree as usize][r_idx][s_idx]
                .as_slice(),
            coef,
        );
    }

    fn multiply_basis_element_by_element(
        &self,
        mut result: FpSliceMut,
        coeff: u32,
        r_degree: i32,
        r_idx: usize,
        s_degree: i32,
        s: FpSlice,
    ) {
        // Per-term reference sweep: run the `PPartMultiplier` multiply once for each term of `s`,
        // reusing one `PPartAllocation`. At p = 2 the admissible-matrix algorithm
        // ([`Self::multiply_basis_element_by_element_2`]) computes the same product by enumerating
        // `Sq(R)`'s admissible matrices once and amortizing over the terms of `s`, but end-to-end
        // A/Bs of Nassau's `S_2` regime measured it a consistent net regression on the CPU (~8% at
        // stem 80, ~3% at stem 100): the regime is dominated by sparse elements (≈31% single-term),
        // for which the up-front enumeration cannot be amortized. It is retained as the reference
        // model for a future GPU kernel (where the enumerate-once/test-all-terms shape is ideal),
        // not wired here. See the commit history for the measurements.
        profile!({
            let nnz = s.iter_nonzero().count();
            profile::record_call(nnz, r_degree, r_idx, s_degree);
            if r_degree == 0 {
                profile::identity();
            } else if nnz > 0 {
                profile::perterm();
            }
            profile::scope_record(r_degree, r_idx, nnz);
        });
        let p = self.prime();
        let r = self.basis_element_from_index(r_degree, r_idx);
        PPartAllocation::with_local(|mut allocation| {
            for (i, v) in s.iter_nonzero() {
                allocation = self.multiply_with_allocation(
                    result.copy(),
                    (coeff * v) % p,
                    r,
                    self.basis_element_from_index(s_degree, i),
                    i32::MAX,
                    allocation,
                );
            }
            allocation
        });
    }

    fn multiply_element_by_element(
        &self,
        mut res: FpSliceMut,
        coef: u32,
        r_deg: i32,
        r: FpSlice,
        s_deg: i32,
        s: FpSlice,
    ) {
        PPartAllocation::with_local(|mut allocation| {
            for (i, c) in r.iter_nonzero() {
                allocation = self.multiply_basis_by_element_with_allocation(
                    res.copy(),
                    coef * c,
                    self.basis_element_from_index(r_deg, i),
                    s_deg,
                    s,
                    allocation,
                );
            }
            allocation
        })
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        format!("{}", self.basis_element_from_index(degree, idx))
    }

    fn basis_element_from_string(&self, elt: &str) -> Option<(i32, usize)> {
        use nom::{
            Parser,
            branch::alt,
            bytes::complete::tag,
            character::complete::char,
            combinator::{map, opt},
            multi::{many0, separated_list1},
            sequence::preceded,
        };

        use crate::steenrod_parser::{brackets, digits, p_or_sq};

        let p = self.prime();

        let mut parser = alt((
            map(char('1'), |_| Some((0, 0))),
            map(char('b'), |_| Some((1, 0))),
            map(preceded(p_or_sq, digits), |i| self.try_beps_pn(0, i)),
            map((tag("P^"), digits, char('_'), digits), |(_, s, _, t)| {
                let entry = p.pow(s);
                let degree = entry as i32 * self.q() * combinatorics::xi_degrees(p)[t];
                let mut elt = MilnorBasisElement {
                    degree,
                    q_part: 0,
                    p_part: vec![0; t],
                };
                elt.p_part[t - 1] = entry as PPartEntry;
                self.compute_basis(degree);
                self.try_basis_element_to_index(&elt)
                    .map(|idx| (degree, idx))
            }),
            map(
                (
                    many0(preceded(tag("Q_"), digits::<u32>)),
                    opt(preceded(
                        char('P'),
                        brackets(separated_list1(char(','), digits)),
                    )),
                ),
                |(q_list, p_list)| {
                    let q_part = q_list.into_iter().fold(0, |acc, q| acc + (1 << q));
                    let mut elt = MilnorBasisElement {
                        degree: 0,
                        q_part,
                        p_part: p_list.unwrap_or_default(),
                    };
                    elt.compute_degree(p);
                    self.compute_basis(elt.degree);

                    self.try_basis_element_to_index(&elt)
                        .map(|idx| (elt.degree, idx))
                },
            ),
        ));

        if let Ok(("", res)) = parser.parse(elt) {
            res
        } else {
            None
        }
    }
}

impl UnstableAlgebra for MilnorAlgebra {
    fn dimension_unstable(&self, degree: i32, excess: i32) -> usize {
        if degree < 0 || excess < 0 {
            0
        } else if excess < degree {
            self.excess_table[degree as usize][excess as usize]
        } else {
            self.basis_table[degree as usize].len()
        }
    }

    fn multiply_basis_elements_unstable(
        &self,
        result: FpSliceMut,
        coeff: u32,
        r_degree: i32,
        r_index: usize,
        s_degree: i32,
        s_index: usize,
        excess: i32,
    ) {
        let m1 = self.basis_element_from_index(r_degree, r_index);
        let m2 = self.basis_element_from_index(s_degree, s_index);
        PPartAllocation::with_local(|allocation| {
            self.multiply_with_allocation(result, coeff, m1, m2, excess, allocation)
        });
    }
}

impl GeneratedAlgebra for MilnorAlgebra {
    fn generator_to_string(&self, degree: i32, idx: usize) -> String {
        if self.generic() {
            if degree == 1 {
                return "b".to_string();
            }
            let elt = self.basis_element_from_index(degree, idx);
            let len = elt.p_part.len();
            if elt.q_part != 0 {
                elt.to_string()
            } else if len == 1 {
                format!("P{}", degree / self.q())
            } else {
                format!(
                    "P^{}_{}",
                    degree / (self.q() * combinatorics::xi_degrees(self.prime())[len - 1]),
                    len
                )
            }
        } else {
            let elt = self.basis_element_from_index(degree, idx);
            let len = elt.p_part.len();
            if len == 1 {
                format!("Sq{degree}")
            } else {
                format!(
                    "P^{}_{}",
                    degree / (combinatorics::xi_degrees(self.prime())[len - 1]),
                    len
                )
            }
        }
    }

    fn generators(&self, degree: i32) -> Vec<usize> {
        if degree <= 0 {
            return vec![];
        } else if degree == 1 {
            return vec![0]; // Q_0
        }

        let p = self.prime();

        // Check for the Q_k
        if self.generic() && degree % 2 == 1 {
            if self.profile.is_an(true) {
                return vec![];
            }

            // If this is 2p^k - 1, then return Q_k
            if let (k, 2) = factor_pk(p, degree as u32 + 1) {
                let q_part = 1 << k;
                if self.profile.q_part & q_part != 0 {
                    return vec![self.basis_element_to_index(&MilnorBasisElement {
                        degree,
                        q_part,
                        p_part: vec![],
                    })];
                }
            }
            return vec![];
        }

        if self.profile.is_an(self.generic()) {
            // Look for P(p^k), which has degree p^k q.
            let q = self.q() as u32;
            if !(degree as u32).is_multiple_of(q) {
                return vec![];
            }
            if let (k, 1) = factor_pk(p, degree as u32 / q)
                && (k as PPartEntry) < self.profile.get_p_part(0)
            {
                return vec![self.basis_element_to_index(&MilnorBasisElement {
                    degree,
                    q_part: 0,
                    p_part: vec![(degree as u32 / q) as PPartEntry],
                })];
            }
            vec![]
        } else {
            // Look for P(0, ..., 0, p^k), which has degree (2p^j - 2) p^k.
            let (k, rem) = factor_pk(p, degree as u32);

            let reduced = if self.generic() {
                // rem must be even because degree is even
                (rem + 2) / 2
            } else {
                rem + 1
            };

            if let (j, 1) = factor_pk(p, reduced) {
                if self.profile.get_p_part(j as usize - 1) <= k as PPartEntry {
                    return vec![];
                }
                let mut p_part = vec![0; j as usize];
                p_part[j as usize - 1] = p.pow(k) as PPartEntry;
                return vec![self.basis_element_to_index(&MilnorBasisElement {
                    degree,
                    q_part: 0,
                    p_part,
                })];
            }
            vec![]
        }
    }

    fn decompose_basis_element(
        &self,
        degree: i32,
        idx: usize,
    ) -> Vec<(u32, (i32, usize), (i32, usize))> {
        let basis = self.basis_element_from_index(degree, idx);
        // If qpart = 0, return self
        if basis.q_part == 0 {
            self.decompose_basis_element_ppart(degree, idx)
        } else {
            self.decompose_basis_element_qpart(degree, idx)
        }
    }

    fn generating_relations(&self, degree: i32) -> Vec<Vec<(u32, (i32, usize), (i32, usize))>> {
        if self.generic() && degree == 2 {
            // beta^2 = 0 is an edge case
            return vec![vec![(1, (1, 0), (1, 0))]];
        }
        let p = self.prime();
        let inadmissible_pairs = combinatorics::inadmissible_pairs(p, self.generic(), degree);
        let mut result = Vec::new();
        for (x, b, y) in inadmissible_pairs {
            let mut relation = Vec::new();
            // Adem relation. Sometimes these don't exist because of profiles. Then just ignore it.
            (|| {
                let (first_degree, first_index) = self.try_beps_pn(0, x as PPartEntry)?;
                let (second_degree, second_index) = self.try_beps_pn(b, y as PPartEntry)?;
                relation.push((
                    p - 1,
                    (first_degree, first_index),
                    (second_degree, second_index),
                ));
                for e1 in 0..=b {
                    let e2 = b - e1;
                    // e1 and e2 determine where a bockstein shows up.
                    // e1 determines whether a bockstein shows up in front
                    // e2 determines whether a bockstein shows up in middle
                    // So our output term looks like b^{e1} P^{x+y-j} b^{e2} P^{j}
                    for j in 0..=x / p {
                        let c = combinatorics::adem_relation_coefficient(p, x, y, j, e1, e2);
                        if c == 0 {
                            continue;
                        }
                        if j == 0 {
                            relation.push((
                                c,
                                self.try_beps_pn(e1, (x + y) as PPartEntry)?,
                                (e2 as i32, 0),
                            ));
                            continue;
                        }
                        let first_sq = self.try_beps_pn(e1, (x + y - j) as PPartEntry)?;
                        let second_sq = self.try_beps_pn(e2, j as PPartEntry)?;
                        relation.push((c, first_sq, second_sq));
                    }
                }
                result.push(relation);
                Some(())
            })();
        }
        result
    }
}

// Compute basis functions
impl MilnorAlgebra {
    fn compute_ppart(&self, max_degree: i32) {
        self.ppart_table.extend(0, |_| vec![Vec::new()]);

        let p = self.prime().as_i32();
        let q = if p == 2 { 1 } else { 2 * p - 2 };
        let new_deg = max_degree / q;

        let xi_degrees = combinatorics::xi_degrees(self.prime());
        let mut profile_list = Vec::with_capacity(xi_degrees.len());
        for i in 0..xi_degrees.len() {
            if i < self.profile.p_part.len() {
                profile_list.push((self.prime().pow(self.profile.p_part[i]) - 1) as PPartEntry);
            } else if self.profile.truncated {
                profile_list.push(0);
            } else {
                profile_list.push(PPartEntry::MAX);
            }
        }

        self.ppart_table.extend(new_deg as usize, |d| {
            let d = d as i32;
            let mut new_row = Vec::new(); // Improve this
            for i in 0..xi_degrees.len() {
                if xi_degrees[i] > d {
                    break;
                }
                if profile_list[i] == 0 {
                    continue;
                }

                let rem = (d - xi_degrees[i]) as usize;
                for old in &self.ppart_table[rem] {
                    // ppart_table[rem] is arranged in increasing order of highest
                    // xi_i. If we get something too large, we may abort;
                    if old.len() > i + 1 {
                        break;
                    }
                    if old.len() == i + 1 && old[i] == profile_list[i] {
                        continue;
                    }
                    let mut new = old.clone();
                    if new.len() < i + 1 {
                        new.resize(i + 1, 0);
                    }
                    new[i] += 1;
                    new_row.push(new);
                }
            }
            new_row
        });
    }

    /// Whether the fast table-based [`Self::seqno`] can be used instead of the hashmap. It requires
    /// `p = 2` (single-generator Milnor basis), a trivial profile (so *every* `P(R)` of a degree is
    /// a basis element, matching the partition counts), and the stable ordering (unstable sorts the
    /// basis by excess, breaking the enumeration-order = index correspondence).
    fn seqno_applicable(&self) -> bool {
        !self.generic() && !self.unstable_enabled && self.profile.is_trivial()
    }

    /// Build the flat [`SeqnoTables`] up to `max_degree`, so that [`Self::seqno`] can be used.
    /// Requires [`Self::seqno_applicable`]. Idempotent: if the stored tables already reach
    /// `max_degree` this returns immediately; otherwise it rebuilds the whole (cheap,
    /// `O(max_degree · width)`) table from scratch and atomically swaps it in, so readers always see
    /// either the old complete table or the new one.
    ///
    /// The `n[e][m]` intermediate — the number of `P(R)` of degree `e` using only `ξ₁ … ξ_{m+1}` —
    /// is built locally and discarded; only the `g` row-progression it feeds is stored, since that
    /// is all [`Self::seqno`] reads. `g[e][h]` sums `n[·][h−1]` along the arithmetic progression of
    /// step `ξ_{h+1}`, letting `seqno` rank a `p_part` without a hash lookup.
    pub fn compute_seqno_tables(&self, max_degree: i32) {
        assert!(self.seqno_applicable());
        if let Some(t) = &*self.seqno_tables.load() {
            if t.max_degree >= max_degree {
                return;
            }
        }

        let xi = combinatorics::xi_degrees(self.prime());
        let width = xi.len();
        let rows = max_degree as usize + 1;

        // n[e * width + m] = #{ P(R) of degree e using only ξ₁ … ξ_{m+1} }
        //                  = n[e][m-1] + [ξ_{m+1} ≤ e] · n[e − ξ_{m+1}][m]
        let mut n = vec![0usize; rows * width];
        for e in 0..=max_degree {
            let base = e as usize * width;
            for m in 0..width {
                // m = 0: partitions into {1} — always exactly one, P(e), for e ≥ 0.
                let without = if m == 0 {
                    (e == 0) as usize
                } else {
                    n[base + m - 1]
                };
                let with = if xi[m] <= e {
                    n[(e - xi[m]) as usize * width + m]
                } else {
                    0
                };
                n[base + m] = without + with;
            }
        }

        // g[e * width + h] = Σ_{j ≥ 0} n[e − j·ξ_{h+1}][h−1]   (h ≥ 1; g[·][0] unused)
        //                  = n[e][h−1] + [ξ_{h+1} ≤ e] · g[e − ξ_{h+1}][h]
        let mut g = vec![0usize; rows * width];
        for e in 0..=max_degree {
            let base = e as usize * width;
            for h in 1..width {
                let head = n[base + h - 1];
                let tail = if xi[h] <= e {
                    g[(e - xi[h]) as usize * width + h]
                } else {
                    0
                };
                g[base + h] = head + tail;
            }
        }

        self.seqno_tables
            .store(Some(std::sync::Arc::new(SeqnoTables {
                max_degree,
                width,
                g,
            })));
    }

    /// The index ("sequence number") of `P(p_part)` in the Milnor basis of its degree, computed in
    /// O(number of `p_part` entries) from the precomputed tables — no hash lookup. Assumes
    /// [`Self::seqno_applicable`] and that `p_part` is a genuine basis element (trimmed, in range).
    ///
    /// The basis is enumerated by increasing highest ξ-index, so the rank of `P` accumulates, for
    /// each populated position `h`, the number of basis elements whose highest index is `< h`
    /// together with `h` — which is exactly the `g_table` difference across the degree consumed at
    /// that position.
    pub fn seqno(&self, p_part: &[PPartEntry]) -> usize {
        let xi = combinatorics::xi_degrees(self.prime());
        let guard = self.seqno_tables.load();
        let t = guard
            .as_ref()
            .expect("seqno tables not built; call compute_seqno_tables first");
        let w = t.width;
        let mut cur_d: i32 = p_part.iter().zip(xi).map(|(&r, &x)| r as i32 * x).sum();
        let mut rank = 0;
        // Consume positions from the highest down; position 0 contributes nothing.
        for h in (1..p_part.len()).rev() {
            let r = p_part[h] as i32;
            if r == 0 {
                continue;
            }
            let below = cur_d - r * xi[h];
            rank += t.g[cur_d as usize * w + h] - t.g[below as usize * w + h];
            cur_d = below;
        }
        rank
    }

    fn generate_basis_generic(&self, max_degree: i32) {
        let q = 2 * self.prime() - 2;
        let tau_degrees = combinatorics::tau_degrees(self.prime());

        self.basis_table.extend(max_degree as usize, |d| {
            let mut table = Vec::new();
            let residue = d as u32 % q;

            for q_part in 0u32.. {
                if q_part.count_ones() % q != residue {
                    continue;
                }

                let mut q_degree = 0;
                let mut bs = q_part;
                for &entry in tau_degrees {
                    q_degree += entry * (bs & 1) as i32;
                    bs >>= 1;
                    if bs == 0 {
                        break;
                    }
                }

                if q_degree > d as i32 {
                    break;
                }

                if q_part & !self.profile.q_part != 0 {
                    continue;
                }

                table.extend(
                    self.ppart_table[(d - q_degree as usize) / q as usize]
                        .iter()
                        .map(|p_part| MilnorBasisElement {
                            p_part: p_part.clone(),
                            q_part,
                            degree: d as i32,
                        }),
                );
            }
            if self.unstable_enabled {
                table.sort_by_cached_key(|e| e.excess(self.p));
            }
            table
        });
    }

    fn generate_basis_2(&self, max_degree: i32) {
        self.basis_table.extend(max_degree as usize, |d| {
            let mut table: Vec<_> = self.ppart_table[d]
                .iter()
                .map(|p| MilnorBasisElement::from_p(p.clone(), d as i32))
                .collect();
            if self.unstable_enabled {
                table.sort_by_cached_key(|e| e.excess(fp::prime::TWO));
            }
            table
        });
    }

    fn generate_excess_table(&self, max_degree: i32) {
        let p = self.prime();
        self.excess_table.extend(max_degree as usize, |n| {
            let mut new_entry = Vec::with_capacity(n);
            let mut cur_excess = 0;
            for (i, elt) in self.basis_table[n].iter().enumerate() {
                let excess = elt.excess(p);
                for _ in cur_excess..excess {
                    new_entry.push(i);
                }
                cur_excess = excess;
            }
            let dim = self.dimension(n as i32);
            for _ in cur_excess..n as u32 {
                new_entry.push(dim);
            }
            new_entry
        });
    }
}

// Multiplication logic
impl MilnorAlgebra {
    /// Return the degree and index of $Q_1^e P(x)$, or `None` if the element is not present
    /// (e.g. out of range or excluded by the profile).
    pub fn try_beps_pn(&self, e: u32, x: PPartEntry) -> Option<(i32, usize)> {
        let q = self.q() as u32;
        let degree = (q * x + e) as i32;
        self.compute_basis(degree);
        self.try_basis_element_to_index(&MilnorBasisElement {
            degree,
            q_part: e,
            p_part: vec![x as PPartEntry],
        })
        .map(|index| (degree, index))
    }

    /// Return the degree and index of $Q_1^e P(x)$.
    pub fn beps_pn(&self, e: u32, x: PPartEntry) -> (i32, usize) {
        self.try_beps_pn(e, x).unwrap()
    }

    fn multiply_qpart(&self, m1: &MilnorBasisElement, f: u32) -> Vec<(u32, MilnorBasisElement)> {
        let mut new_result: Vec<(u32, MilnorBasisElement)> = vec![(1, m1.clone())];
        let mut old_result: Vec<(u32, MilnorBasisElement)> = Vec::new();

        for k in BitflagIterator::set_bit_iterator(f as u64) {
            let k = k as u32;
            let pk = self.p.pow(k) as PPartEntry;
            std::mem::swap(&mut new_result, &mut old_result);
            new_result.clear();

            // We implement the formula
            // P(R) Q_k = Q_k P^R + Q_{k+1} P(R - p^k e_1) + Q_{k+2} P(R - p^k e_2) +
            // ... + Q_{k + i} P(R - p^k e_i) + ...
            // where e_i is the vector with value 1 in entry i and 0 otherwise (in the above
            // formula, the first xi is xi_1, hence the offset below). If R - p^k e_i has a
            // negative entry, the term is 0.
            //
            // We also use the fact that Q_k Q_j = -Q_j Q_k
            for (coef, term) in &old_result {
                for i in 0..=term.p_part.len() {
                    // If there is already Q_{k+i} on the other side, the result is 0
                    if term.q_part & (1 << (k + i as u32)) != 0 {
                        continue;
                    }
                    // Check if R - p^k e_i < 0. Only do this from the first term onwards.
                    if i > 0 && term.p_part[i - 1] < pk {
                        continue;
                    }

                    let mut new_p = term.p_part.clone();
                    if i > 0 {
                        new_p[i - 1] -= pk;
                    }

                    // Now calculate the number of Q's we are moving past
                    let larger_q = (term.q_part >> (k + i as u32 + 1)).count_ones();

                    // If new_p ends with 0, drop them
                    while let Some(0) = new_p.last() {
                        new_p.pop();
                    }
                    // Now put everything together
                    let m = MilnorBasisElement {
                        p_part: new_p,
                        q_part: term.q_part | (1 << (k + i as u32)),
                        degree: 0, // we don't really care about the degree here. The final degree of the whole calculation is known a priori
                    };
                    let c = if larger_q.is_multiple_of(2) {
                        *coef
                    } else {
                        *coef * (self.prime() - 1)
                    };

                    new_result.push((c, m));
                }
            }
        }
        new_result
    }

    pub fn multiply(
        &self,
        res: FpSliceMut,
        coef: u32,
        m1: &MilnorBasisElement,
        m2: &MilnorBasisElement,
    ) {
        PPartAllocation::with_local(|allocation| {
            self.multiply_with_allocation(res, coef, m1, m2, i32::MAX, allocation)
        });
    }

    /// Compute `Sq(R) * s` for a fixed operation `Sq(R)` and a general element `s`, adding the
    /// result to `result`. Only valid at `p = 2`.
    ///
    /// Algorithm due to Christian Nassau (ported from the previously disabled
    /// `FreeModule::custom_milnor_act`). To compute `Sq(R) * (Sq(S₁) + Sq(S₂) + ⋯)` we build the
    /// admissible matrices for `Sq(R)` once and, for each matrix, test every `Sq(Sₖ)` against it:
    /// a matrix contributes iff each column sum is at most the corresponding entry of `Sₖ` and the
    /// relevant bits are disjoint. This amortizes the (expensive) matrix enumeration over the whole
    /// element, whereas [`Self::multiply_with_allocation`] re-runs it per term of `s`.
    ///
    /// **Not on the CPU hot path.** End-to-end A/Bs of Nassau's `S_2` regime measured this a net
    /// regression versus the per-term sweep in [`Self::multiply_basis_element_by_element`] (the
    /// regime is too sparse for the up-front enumeration to pay off). It is kept as the reference
    /// model for a future GPU kernel: enumerate `Sq(R)`'s matrices once per operation and test every
    /// element term against them in parallel — a shape that batches extremely well on a GPU (a real
    /// resolution presents tens of thousands of terms per distinct `R`). Exercised by the
    /// `admissible_multiply_agrees_with_reference` test.
    // The `working`-building loops below legitimately index `basis`, `col_sums`, and `masks` by the
    // same `j`, so a range loop is clearer than zipping three slices.
    #[allow(clippy::needless_range_loop)]
    pub fn multiply_basis_element_by_element_2(
        &self,
        mut result: FpSliceMut,
        coeff: u32,
        r_degree: i32,
        r_idx: usize,
        s_degree: i32,
        s: FpSlice,
    ) {
        debug_assert!(
            !self.generic(),
            "multiply_basis_element_by_element_2 is p = 2 only"
        );
        // Coefficients live in F₂, so an even coefficient kills the whole product, and every
        // non-zero term of `s` has coefficient 1.
        if coeff.is_multiple_of(2) {
            return;
        }
        profile!(profile::record_call(
            s.iter_nonzero().count(),
            r_degree,
            r_idx,
            s_degree
        ));

        let r = self.basis_element_from_index(r_degree, r_idx);
        // `Sq(∅) = 1`, so `Sq(R) * s = s`. (Also avoids an empty `AdmissibleMatrix`.) The output
        // degree equals `s_degree`, so basis indices are unchanged.
        if r.p_part.is_empty() {
            profile!(profile::identity());
            for (i, _) in s.iter_nonzero() {
                result.add_basis_element(i, 1);
            }
            return;
        }

        // The admissible-matrix sweep enumerates *all* matrices of `Sq(R)` up front and amortizes
        // that over the terms of `s`. With a single term there is nothing to amortize, and for a
        // large operation the wasted enumeration makes it several times slower than the
        // `PPartMultiplier` path (which is constrained by `S` and so enumerates far fewer matrices).
        // So peek the first two terms in one pass: with fewer than two, fall back to the per-term
        // path — byte-for-byte the generic multiply, so that case never regresses.
        let mut nonzero = s.iter_nonzero();
        let (Some((i0, _)), second) = (nonzero.next(), nonzero.next()) else {
            return; // s = 0
        };
        let Some((i1, _)) = second else {
            profile!(profile::perterm());
            PPartAllocation::with_local(|allocation| {
                self.multiply_with_allocation(
                    result,
                    1,
                    r,
                    self.basis_element_from_index(s_degree, i0),
                    i32::MAX,
                    allocation,
                )
            });
            return;
        };

        // Two or more terms: use the admissible-matrix sweep. Cache the (already-peeked) input
        // basis elements once; they are reused across every admissible matrix.
        let mut terms: Vec<&MilnorBasisElement> = Vec::with_capacity(s.len());
        terms.push(self.basis_element_from_index(s_degree, i0));
        terms.push(self.basis_element_from_index(s_degree, i1));
        terms.extend(nonzero.map(|(i, _)| self.basis_element_from_index(s_degree, i)));
        profile!(profile::admissible());

        let out_degree = r_degree + s_degree;
        let mut matrix = AdmissibleMatrix::new(&r.p_part);
        let mut working = MilnorBasisElement {
            q_part: 0,
            p_part: PPart::new(),
            degree: out_degree,
        };

        loop {
            profile!(profile::adm_matrix());
            'outer: for term in &terms {
                profile!(profile::adm_test());
                let basis = &term.p_part;
                working.p_part.clear();

                for j in 0..std::cmp::min(basis.len(), matrix.col_sums.len()) {
                    if matrix.col_sums[j] > basis[j] {
                        continue 'outer;
                    }
                    if (basis[j] - matrix.col_sums[j]) & matrix.masks[j] != 0 {
                        continue 'outer;
                    }
                    // We should add the diagonal sum, but that equals the mask, and there are no
                    // bit conflicts, so a bitwise-or is the same thing.
                    working
                        .p_part
                        .push((basis[j] - matrix.col_sums[j]) | matrix.masks[j]);
                }

                if basis.len() < matrix.col_sums.len() {
                    for &col_sum in &matrix.col_sums[basis.len()..] {
                        if col_sum > 0 {
                            continue 'outer;
                        }
                    }
                    for &mask in &matrix.masks[basis.len()..] {
                        working.p_part.push(mask);
                    }
                } else {
                    for j in matrix.col_sums.len()..std::cmp::min(basis.len(), matrix.masks.len()) {
                        if basis[j] & matrix.masks[j] != 0 {
                            continue 'outer;
                        }
                        working.p_part.push(basis[j] | matrix.masks[j]);
                    }
                    if basis.len() < matrix.masks.len() {
                        for &mask in &matrix.masks[basis.len()..] {
                            working.p_part.push(mask);
                        }
                    } else {
                        for &entry in &basis[matrix.masks.len()..] {
                            working.p_part.push(entry);
                        }
                    }
                }

                while let Some(0) = working.p_part.last() {
                    working.p_part.pop();
                }

                let idx = self.basis_element_to_index(&working);
                profile!(profile::index_lookup());
                result.add_basis_element(idx, 1);
                profile!(profile::output_add());
            }
            if !matrix.next() {
                break;
            }
        }
    }

    pub fn multiply_with_allocation(
        &self,
        mut res: FpSliceMut,
        coef: u32,
        m1: &MilnorBasisElement,
        m2: &MilnorBasisElement,
        excess: i32,
        mut allocation: PPartAllocation,
    ) -> PPartAllocation {
        let target_deg = m1.degree + m2.degree;
        // The unstable dimension only depends on `target_deg` and `excess`, both loop-invariant, so
        // compute the truncation bound once instead of per output term. In Nassau's stable path
        // (`excess = i32::MAX`) this is the full dimension and the check never fires, but keeping it
        // hoisted is correct for the unstable callers too.
        let dim = self.dimension_unstable(target_deg, excess);
        if self.generic() {
            let m1f = self.multiply_qpart(m1, m2.q_part);
            for (cc, basis) in m1f {
                let mut multiplier = PPartMultiplier::<false>::new_from_allocation(
                    self.prime(),
                    &basis.p_part,
                    &m2.p_part,
                    allocation,
                    basis.q_part,
                    target_deg,
                );

                while let Some(c) = multiplier.next() {
                    let idx = self.basis_element_to_index(&multiplier.ans);
                    if idx < dim {
                        res.add_basis_element(idx, c * cc * coef);
                    }
                }
                allocation = multiplier.into_allocation()
            }
        } else {
            profile!(profile::kernel_call());
            let mut multiplier = PPartMultiplier::<false>::new_from_allocation(
                self.prime(),
                &m1.p_part,
                &m2.p_part,
                allocation,
                0,
                target_deg,
            );

            while let Some(c) = multiplier.next() {
                profile!(profile::ppart_term());
                let idx = self.basis_element_to_index(&multiplier.ans);
                profile!(profile::index_lookup());
                if idx < dim {
                    res.add_basis_element(idx, c * coef);
                    profile!(profile::output_add());
                }
            }
            allocation = multiplier.into_allocation()
        }
        allocation
    }

    pub fn multiply_basis_by_element(
        &self,
        res: FpSliceMut,
        coef: u32,
        m1: &MilnorBasisElement,
        s_deg: i32,
        s: FpSlice,
    ) {
        PPartAllocation::with_local(|allocation| {
            self.multiply_basis_by_element_with_allocation(res, coef, m1, s_deg, s, allocation)
        });
    }

    fn multiply_basis_by_element_with_allocation(
        &self,
        mut res: FpSliceMut,
        coef: u32,
        m1: &MilnorBasisElement,
        s_deg: i32,
        s: FpSlice,
        mut allocation: PPartAllocation,
    ) -> PPartAllocation {
        for (i, c) in s.iter_nonzero() {
            allocation = self.multiply_with_allocation(
                res.copy(),
                coef * c,
                m1,
                self.basis_element_from_index(s_deg, i),
                i32::MAX,
                allocation,
            );
        }
        allocation
    }
}

/// The state for enumerating the admissible matrices of a fixed operation `Sq(R)` at `p = 2`, used
/// by [`MilnorAlgebra::multiply_basis_element_by_element_2`]. See that method (and the original
/// `FreeModule::custom_milnor_act`) for the algorithm. Rows are indexed by the entries of `R`; the
/// stored `matrix` is row-major with `cols` columns.
struct AdmissibleMatrix {
    cols: usize,
    rows: usize,
    matrix: Vec<PPartEntry>,
    totals: Vec<PPartEntry>,
    col_sums: Vec<PPartEntry>,
    masks: Vec<PPartEntry>,
}

impl AdmissibleMatrix {
    fn new(ps: &[PPartEntry]) -> Self {
        let rows = ps.len();
        let cols = ps
            .iter()
            .map(|x| (PPartEntry::BITS - x.leading_zeros()) as usize)
            .max()
            .unwrap();
        let mut matrix = vec![0; rows * cols];
        for (i, &x) in ps.iter().enumerate() {
            matrix[i * cols] = x;
        }

        let mut masks = Vec::with_capacity(rows + cols - 1);
        masks.extend_from_slice(ps);
        masks.resize(rows + cols - 1, 0);

        Self {
            rows,
            cols,
            totals: vec![0; rows], // only used by `next`; no need to initialize
            col_sums: vec![0; cols - 1],
            matrix,
            masks,
        }
    }

    #[inline]
    fn get(&self, row: usize, col: usize) -> PPartEntry {
        self.matrix[row * self.cols + col]
    }

    #[inline]
    fn set(&mut self, row: usize, col: usize, val: PPartEntry) {
        self.matrix[row * self.cols + col] = val;
    }

    /// Advance to the next admissible matrix, returning `false` when the enumeration is exhausted.
    fn next(&mut self) -> bool {
        for row in 0..self.rows {
            let mut p_to_the_j: PPartEntry = 1;
            self.totals[row] = self.get(row, 0);
            'mid: for col in 1..self.cols {
                p_to_the_j *= 2;
                // Quick check before computing the bitsums.
                if p_to_the_j <= self.totals[row] {
                    // Compute the bitsum along the anti-diagonal to the bottom-left.
                    let mut d = 0;
                    for c in (row + col + 1).saturating_sub(self.rows)..col {
                        d |= self.get(row + col - c, c);
                    }
                    // Magic: the next number greater than `self[row][col]` whose bitwise-and with
                    // `d` is 0.
                    let new_entry = ((self.get(row, col) | d) + 1) & !d;
                    let inc = new_entry - self.get(row, col);
                    let sub = inc * p_to_the_j;
                    if self.totals[row] < sub {
                        self.totals[row] += p_to_the_j * self.get(row, col);
                        continue 'mid;
                    }
                    self.set(row, 0, self.totals[row] - sub);
                    self.masks[row] = self.get(row, 0);
                    self.col_sums[col - 1] += inc;
                    for j in 1..col {
                        self.masks[row + j] &= !self.get(row, j);
                        self.col_sums[j - 1] -= self.get(row, j);
                        self.set(row, j, 0);
                    }
                    self.set(row, col, new_entry);

                    for i in 0..row {
                        self.set(i, 0, self.totals[i]);
                        self.masks[i] = self.totals[i];
                        for j in 1..self.cols {
                            if i + j > row {
                                self.masks[i + j] &= !self.get(i, j);
                            }
                            self.col_sums[j - 1] -= self.get(i, j);
                            self.set(i, j, 0);
                        }
                    }
                    self.masks[row + col] = d | new_entry;
                    return true;
                }
                self.totals[row] += p_to_the_j * self.get(row, col);
            }
        }
        false
    }
}

#[derive(Debug, Default)]
struct Matrix2D {
    cols: usize,
    inner: PPart,
}

impl std::fmt::Display for Matrix2D {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for i in 0..self.inner.len() / self.cols {
            writeln!(f, "{:?}", &self[i][0..self.cols])?;
        }
        Ok(())
    }
}

impl Matrix2D {
    fn reset(&mut self, rows: usize, cols: usize) {
        self.cols = cols;
        self.inner.clear();
        self.inner.resize(rows * cols, 0);
    }
}

impl Matrix2D {
    fn with_capacity(rows: usize, cols: usize) -> Self {
        Self {
            cols: 0,
            inner: Vec::with_capacity(rows * cols),
        }
    }
}

impl std::ops::Index<usize> for Matrix2D {
    type Output = [PPartEntry];

    fn index(&self, row: usize) -> &Self::Output {
        // Computing the end point is fairly expensive and only serves as a safety check...
        &self.inner[row * self.cols..]
    }
}

impl std::ops::IndexMut<usize> for Matrix2D {
    fn index_mut(&mut self, row: usize) -> &mut Self::Output {
        &mut self.inner[row * self.cols..]
    }
}

/// The parts of a PPartMultiplier that involve heap allocation.
///
/// This lets us reuse the allocation across multiple different multipliers. Reusing the whole
/// PPartMultiplier is finicky but doable due to lifetime issues. However, it appears to be less
/// performant.
#[derive(Default)]
pub struct PPartAllocation {
    m: Matrix2D,
    #[cfg(feature = "odd-primes")]
    diagonal: PPart,
    p_part: PPart,
}

thread_local! {
    static ALLOCATION: Cell<PPartAllocation> = Cell::new(PPartAllocation::with_capacity(9));
}

impl PPartAllocation {
    /// This creates a PPartAllocation with enough capacity to handle mulitiply elements with
    /// of total degree < 2^n - ε at p = 2.
    pub fn with_capacity(n: usize) -> Self {
        Self {
            m: Matrix2D::with_capacity(n + 1, n),
            #[cfg(feature = "odd-primes")]
            diagonal: Vec::with_capacity(n),
            // This size should be the number of diagonals. Even though the answer cannot be that
            // long, we still insert zeros then pop them out later.
            p_part: Vec::with_capacity(2 * n),
        }
    }

    pub fn with_local(f: impl FnOnce(Self) -> Self) {
        ALLOCATION.with(|alloc| {
            alloc.set(f(alloc.take()));
        });
    }
}

#[allow(non_snake_case)]
pub struct PPartMultiplier<'a, const MOD4: bool> {
    p: ValidPrime,
    M: Matrix2D,
    r: &'a PPart,
    rows: usize,
    cols: usize,
    diag_num: usize,
    init: bool,
    pub ans: MilnorBasisElement,
    #[cfg(feature = "odd-primes")]
    diagonal: PPart,
}

#[allow(non_snake_case)]
impl<'a, const MOD4: bool> PPartMultiplier<'a, MOD4> {
    fn prime(&self) -> ValidPrime {
        self.p
    }

    #[allow(unused_mut)] // Mut is only used with odd primes
    pub fn new_from_allocation(
        p: ValidPrime,
        r: &'a PPart,
        s: &'a PPart,
        mut allocation: PPartAllocation,
        q_part: u32,
        degree: i32,
    ) -> Self {
        if MOD4 {
            assert_eq!(p, 2);
        }
        let rows = r.len() + 1;
        let cols = s.len() + 1;
        let diag_num = r.len() + s.len();
        #[cfg(feature = "odd-primes")]
        {
            allocation.diagonal.clear();
            allocation.diagonal.reserve_exact(std::cmp::max(rows, cols));
        }

        let mut M = allocation.m;
        M.reset(rows, cols);

        for i in 1..rows {
            M[i][0] = r[i - 1];
        }
        // This is somehow quite significantly faster than copy_from_slice
        #[allow(clippy::manual_memcpy)]
        for k in 1..cols {
            M[0][k] = s[k - 1];
        }

        let ans = MilnorBasisElement {
            q_part,
            p_part: allocation.p_part,
            degree,
        };
        PPartMultiplier {
            #[cfg(feature = "odd-primes")]
            diagonal: allocation.diagonal,
            p,
            M,
            r,
            rows,
            cols,
            diag_num,
            ans,
            init: true,
        }
    }

    pub fn into_allocation(self) -> PPartAllocation {
        PPartAllocation {
            m: self.M,
            #[cfg(feature = "odd-primes")]
            diagonal: self.diagonal,
            p_part: self.ans.p_part,
        }
    }

    /// This compute the first l > k such that (sum + l) choose l != 0 mod p, stopping if we reach
    /// max + 1. This is useful for incrementing the matrix.
    ///
    /// TODO: Improve odd prime performance
    fn next_val(&self, sum: PPartEntry, k: PPartEntry, max: PPartEntry) -> PPartEntry {
        match self.prime().as_u32() {
            2 => {
                if MOD4 {
                    // x.count_ones() + y.count_ones() - (x + y).count_ones() is the number of
                    // carries when adding x to y.
                    //
                    // The p-adic valuation of (n + r) choose r is the number of carries when
                    // adding r to n in base p.
                    (k + 1..max + 1)
                        .find(|&l| {
                            sum & l == 0
                                || (sum.count_ones() + l.count_ones()) - (sum + l).count_ones() == 1
                        })
                        .unwrap_or(max + 1)
                } else {
                    ((k | sum) + 1) & !sum
                }
            }
            _ => (k + 1..max + 1)
                .find(|&l| !PPartEntry::binomial_odd_is_zero(self.prime(), sum + l, l))
                .unwrap_or(max + 1),
        }
    }

    /// We have a matrix of the form
    ///    | s₁  s₂  s₃ ...
    /// --------------------
    /// r₁ |
    /// r₂ |     x_{ij}
    /// r₃ |
    ///
    /// We think of ourselves as modifiying the center pieces x_{ij}, while the r_i's and s_j's are
    /// only there to ensure the x_{ij}'s don't get too big. The idea is to sweep through the
    /// matrix row by row, from top-to-bottom, and left-to-right. In each pass, we find the first
    /// entry that can be incremented. We then increment it and zero out all the entries that
    /// appear before it. This will give us all valid entries.
    fn update(&mut self) -> bool {
        for i in 1..self.rows {
            // total is sum x_{ij} p^j up to the jth column
            let mut total = self.M[i][0];
            let mut p_to_the_j = 1;
            for j in 1..self.cols {
                p_to_the_j *= self.prime().as_u32() as PPartEntry;
                if total < p_to_the_j {
                    // We don't have enough weight left in the entries above this one in the column to increment this cell.
                    // Add the weight from this cell to the total, we can use it to increment a cell lower down.
                    total += self.M[i][j] * p_to_the_j;
                    continue;
                }
                let col_sum: PPartEntry = (0..i).map(|k| self.M[k][j]).sum();
                if col_sum == 0 {
                    total += self.M[i][j] * p_to_the_j;
                    continue;
                }

                let max_inc = std::cmp::min(col_sum, total / p_to_the_j);

                // Compute the sum of entries along the diagonal to the bottom-left
                let mut sum = 0;
                for c in (i + j + 1).saturating_sub(self.rows)..j {
                    sum += self.M[i + j - c][c];
                }

                // Find the next possible value we can increment M[i][j] to without setting the
                // coefficient to 0. The coefficient is the multinomial coefficient of the
                // diagonal, and if the multinomial coefficient of any subset is zero, so is the
                // coefficient of the whole diagonal.
                let next_val = self.next_val(sum, self.M[i][j], max_inc + self.M[i][j]);
                let inc = next_val - self.M[i][j];

                // The remaining obstacle to incrementing this entry is the column sum condition.
                // For this, we only need a non-zero entry in the column j above row i.
                if inc <= max_inc {
                    // If so, we found our next matrix.
                    for row in 1..i {
                        self.M[row][0] = self.r[row - 1];
                        for col in 1..self.cols {
                            self.M[0][col] += self.M[row][col];
                            self.M[row][col] = 0;
                        }
                    }
                    for col in 1..j {
                        self.M[0][col] += self.M[i][col];
                        self.M[i][col] = 0;
                    }
                    self.M[0][j] -= inc;
                    self.M[i][j] += inc;
                    self.M[i][0] = total - p_to_the_j * inc;
                    return true;
                }
                // All the cells above this one are zero so we didn't find our next matrix.
                // Add the weight from this cell to the total, we can use it to increment a cell lower down.
                total += self.M[i][j] * p_to_the_j;
            }
        }
        false
    }
}

impl<const MOD4: bool> Iterator for PPartMultiplier<'_, MOD4> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        let p = self.prime().as_u32() as PPartEntry;
        'outer: loop {
            self.ans.p_part.clear();
            let mut coef = 1;

            if self.init {
                self.init = false;
                for i in 1..std::cmp::min(self.cols, self.rows) {
                    if MOD4 {
                        coef *= PPartEntry::binomial4(self.M[i][0] + self.M[0][i], self.M[0][i]);
                        coef %= 4;
                    } else {
                        coef *= PPartEntry::binomial(
                            self.prime(),
                            self.M[i][0] + self.M[0][i],
                            self.M[0][i],
                        );
                        coef %= p;
                    }
                    if coef == 0 {
                        continue 'outer;
                    }
                }
                self.ans
                    .p_part
                    .reserve(std::cmp::max(self.cols, self.rows) - 1);
                self.ans.p_part.extend(&self.M[0][1..self.cols]);

                if self.rows > self.cols {
                    self.ans.p_part.resize(self.r.len(), 0);
                }
                self.ans
                    .p_part
                    .iter_mut()
                    .zip(self.r.iter())
                    .for_each(|(l, r)| *l += r);

                // If new_p ends with 0, drop them
                while let Some(0) = self.ans.p_part.last() {
                    self.ans.p_part.pop();
                }
                return Some(coef);
            } else if self.update() {
                self.ans.p_part.reserve(self.diag_num);
                for diag_idx in 1..=self.diag_num {
                    let i_min = (diag_idx + 1).saturating_sub(self.cols);
                    let i_max = std::cmp::min(diag_idx + 1, self.rows);
                    let mut sum = 0;

                    if self.prime() == 2 {
                        if MOD4 {
                            for i in i_min..i_max {
                                let entry = self.M[i][diag_idx - i];
                                sum += entry;
                                if coef.is_multiple_of(2) {
                                    coef *= PPartEntry::binomial2(sum, entry);
                                } else {
                                    coef *= PPartEntry::binomial4(sum, entry);
                                }
                                coef %= 4;
                                if coef == 0 {
                                    continue 'outer;
                                }
                            }
                        } else {
                            let mut or = 0;
                            for i in i_min..i_max {
                                sum += self.M[i][diag_idx - i];
                                or |= self.M[i][diag_idx - i];
                            }
                            if sum != or {
                                continue 'outer;
                            }
                        }
                    } else {
                        #[cfg(feature = "odd-primes")]
                        {
                            self.diagonal.clear();
                            for i in i_min..i_max {
                                self.diagonal.push(self.M[i][diag_idx - i]);
                                sum += self.M[i][diag_idx - i];
                            }

                            coef *= PPartEntry::multinomial_odd(self.prime(), &mut self.diagonal);
                            coef %= p;
                            if coef == 0 {
                                continue 'outer;
                            }
                        }
                    }
                    self.ans.p_part.push(sum);
                }
                // If new_p ends with 0, drop them
                while let Some(0) = self.ans.p_part.last() {
                    self.ans.p_part.pop();
                }

                return Some(coef);
            } else {
                return None;
            }
        }
    }
}

impl MilnorAlgebra {
    fn decompose_basis_element_qpart(
        &self,
        degree: i32,
        idx: usize,
    ) -> Vec<(u32, (i32, usize), (i32, usize))> {
        let basis = self.basis_element_from_index(degree, idx);
        // Look for left-most non-zero qpart
        let i = basis.q_part.trailing_zeros();
        // If the basis element is just Q_{k+1}, we decompose Q_{k+1} = P(p^k) Q_k - Q_k P(p^k).
        if basis.q_part == 1 << i && basis.p_part.is_empty() {
            let ppow = self.prime().pow(i - 1);

            let q_degree = (2 * ppow - 1) as i32;
            let p_degree = (ppow * (2 * self.prime() - 2)) as i32;

            let p_idx = self
                .basis_element_to_index(&MilnorBasisElement::from_p(
                    vec![ppow as PPartEntry],
                    p_degree,
                ))
                .to_owned();

            let q_idx = self
                .basis_element_to_index(&MilnorBasisElement {
                    q_part: 1 << (i - 1),
                    p_part: Vec::new(),
                    degree: q_degree,
                })
                .to_owned();

            vec![
                (1, (p_degree, p_idx), (q_degree, q_idx)),
                (self.prime() - 1, (q_degree, q_idx), (p_degree, p_idx)),
            ]
        } else {
            // Otherwise, separate out the first Q_k.
            let first_degree = combinatorics::tau_degrees(self.prime())[i as usize];
            let second_degree = degree - first_degree;

            let first_idx = self.basis_element_to_index(&MilnorBasisElement {
                q_part: 1 << i,
                p_part: Vec::new(),
                degree: first_degree,
            });

            let second_idx = self.basis_element_to_index(&MilnorBasisElement {
                q_part: basis.q_part ^ (1 << i),
                p_part: basis.p_part.clone(),
                degree: second_degree,
            });

            vec![(1, (first_degree, first_idx), (second_degree, second_idx))]
        }
    }

    fn decompose_basis_element_ppart(
        &self,
        degree: i32,
        idx: usize,
    ) -> Vec<(u32, (i32, usize), (i32, usize))> {
        let p = self.prime();

        // We define an ordering on the p parts as follows: we order each entry in reverse, and
        // then impose the reverse lexicographic ordering. Then for each non-generator P(R), `init`
        // is a partial decomposition such that the non-zero terms in the `init` product are all
        // greater than or equal to P(R) (and P(R) has non-zero coefficient in `init`). We can then
        // apply this algorithm recursively to decompose an element.

        // result is the products we have added so far
        let mut result = Vec::new();
        // buffer is the products we are adding in the current iteration
        let mut buffer = Vec::new();

        // out_vec is the remaining items we have to kill. We are done when this hits zero.
        let mut out_vec = FpVector::new(p, self.dimension(degree));
        out_vec.set_entry(idx, p - 1);

        while let Some((idx, c)) = out_vec.iter_nonzero().next() {
            let b = self.basis_element_from_index(degree, idx);
            let len = b.p_part.len();

            if b.p_part[0..len - 1].iter().all(|&x| x == 0) {
                // There is only one entry
                let entry = b.p_part[len - 1];
                let (k, m) = factor_pk(p, entry);

                // This is a power of p
                if m == 1 {
                    if len == 1 || !self.profile.is_an(self.generic()) {
                        buffer.extend([(p - c, (degree, idx), (0, 0))]);
                    } else {
                        // Write this as [P(p^(len + k - 1)), P(0, .., 0, P^k)] plus higher order
                        // terms.
                        let l_entry = p.pow(len as u32 + k - 1) as PPartEntry;
                        let r_entry = p.pow(k) as PPartEntry;

                        let l_degree = l_entry as i32 * self.q();
                        let l_index = self.basis_element_to_index(&MilnorBasisElement {
                            q_part: 0,
                            p_part: vec![l_entry],
                            degree: l_degree,
                        });

                        let mut r_p_part = vec![0; len - 1];
                        r_p_part[len - 2] = r_entry;
                        let r_degree =
                            r_entry as i32 * combinatorics::xi_degrees(p)[len - 2] * self.q();

                        let r_index = self.basis_element_to_index(&MilnorBasisElement {
                            q_part: 0,
                            p_part: r_p_part,
                            degree: r_degree,
                        });
                        buffer.extend(vec![
                            (p - c, (l_degree, l_index), (r_degree, r_index)),
                            (c, (r_degree, r_index), (l_degree, l_index)),
                        ])
                    }
                } else {
                    // This is not a power of p. Just subtract the lowest power of p.
                    let pk = p.pow(k) as PPartEntry;
                    let rem_entry = entry - pk;

                    let entry_deg = combinatorics::xi_degrees(p)[len - 1] * self.q();

                    let mut elt = MilnorBasisElement {
                        q_part: 0,
                        degree: 0,
                        p_part: vec![0; len],
                    };

                    elt.p_part[len - 1] = pk;
                    elt.degree = entry_deg * elt.p_part[len - 1] as i32;
                    let first = (elt.degree, self.basis_element_to_index(&elt));

                    elt.p_part[len - 1] = rem_entry;
                    elt.degree = entry_deg * elt.p_part[len - 1] as i32;
                    let second = (elt.degree, self.basis_element_to_index(&elt));

                    let coef =
                        p - fp::prime::inverse(p, PPartEntry::binomial(p, pk + rem_entry, pk));
                    buffer.extend([(coef, first, second)])
                }
            } else {
                // There is more than one entry. Just separate out the last entry.
                let last_entry = b.p_part[len - 1];
                let last_deg = combinatorics::xi_degrees(p)[len - 1] * self.q() * last_entry as i32;
                let mut elt = MilnorBasisElement {
                    q_part: 0,
                    p_part: vec![0; len],
                    degree: last_deg,
                };
                elt.p_part[len - 1] = last_entry;
                let first = (elt.degree, self.basis_element_to_index(&elt));

                elt.degree = degree - last_deg;
                elt.p_part.clear();
                elt.p_part.extend_from_slice(&b.p_part[0..len - 1]);
                while let Some(0) = elt.p_part.last() {
                    elt.p_part.pop();
                }
                let second = (elt.degree, self.basis_element_to_index(&elt));
                buffer.extend([(p - c, first, second)]);
            };
            for (c, first, second) in &buffer {
                self.multiply_basis_elements(
                    out_vec.as_slice_mut(),
                    *c,
                    first.0,
                    first.1,
                    second.0,
                    second.1,
                );
            }
            result.extend(&buffer);
            buffer.clear();
        }
        result
    }
}

impl MilnorAlgebra {
    /// Returns `true` if the new element is not within the bounds
    fn increment_p_part(element: &mut PPart, max: &[PPartEntry]) -> bool {
        element[0] += 1;
        for i in 0..element.len() - 1 {
            if element[i] > max[i] {
                element[i] = 0;
                element[i + 1] += 1;
            }
        }
        element.last().unwrap() > max.last().unwrap()
    }
}

impl Bialgebra for MilnorAlgebra {
    fn coproduct(&self, op_deg: i32, op_idx: usize) -> Vec<(i32, usize, i32, usize)> {
        assert_eq!(self.prime(), 2, "Coproduct at odd primes not supported");
        if op_deg == 0 {
            return vec![(0, 0, 0, 0)];
        }
        let xi_degrees = combinatorics::xi_degrees(self.prime());

        let mut len = 1;
        let p_part = &self.basis_element_from_index(op_deg, op_idx).p_part;

        for i in p_part.iter() {
            len *= i + 1;
        }
        let len = len as usize;
        let mut result = Vec::with_capacity(len);

        let mut cur_ppart: PPart = vec![0; p_part.len()];
        loop {
            let mut left_degree: i32 = 0;
            for i in 0..cur_ppart.len() {
                left_degree += cur_ppart[i] as i32 * xi_degrees[i];
            }
            let right_degree: i32 = op_deg - left_degree;

            let mut left_ppart = cur_ppart.clone();
            while let Some(0) = left_ppart.last() {
                left_ppart.pop();
            }

            let mut right_ppart = cur_ppart
                .iter()
                .enumerate()
                .map(|(i, v)| p_part[i] - *v)
                .collect::<Vec<_>>();
            while let Some(0) = right_ppart.last() {
                right_ppart.pop();
            }

            let left_idx = self.basis_element_to_index(&MilnorBasisElement {
                degree: left_degree,
                q_part: 0,
                p_part: left_ppart,
            });
            let right_idx = self.basis_element_to_index(&MilnorBasisElement {
                degree: right_degree,
                q_part: 0,
                p_part: right_ppart,
            });

            result.push((left_degree, left_idx, right_degree, right_idx));
            if Self::increment_p_part(&mut cur_ppart, p_part) {
                break;
            }
        }
        result
    }

    fn decompose(&self, op_deg: i32, op_idx: usize) -> Vec<(i32, usize)> {
        vec![(op_deg, op_idx)]
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _; // Needed for write! macro for String

    use expect_test::expect;
    use rstest::rstest;

    use super::*;

    /// The table-based [`MilnorAlgebra::seqno`] must return the position of every basis element in
    /// its degree — i.e. agree with the enumeration order that defines the index — for the stable
    /// `p = 2` full algebra, and reject non-basis elements via `try_`.
    #[test]
    fn seqno_matches_enumeration_order() {
        let algebra = MilnorAlgebra::new(ValidPrime::new(2), false);
        assert!(algebra.seqno_applicable());
        let max_degree = 100;
        algebra.compute_basis(max_degree);
        algebra.compute_seqno_tables(max_degree);
        for d in 0..=max_degree {
            let dim = algebra.dimension(d);
            for i in 0..dim {
                let elt = algebra.basis_element_from_index(d, i);
                assert_eq!(
                    algebra.seqno(&elt.p_part),
                    i,
                    "seqno mismatch at degree {d}, index {i}: {elt:?}"
                );
            }
        }
    }

    /// The `p = 2` admissible-matrix multiply ([`MilnorAlgebra::multiply_basis_element_by_element_2`],
    /// retained as the GPU reference model — no longer wired into the CPU path) must agree
    /// bit-for-bit with the reference `PPartMultiplier` path (`multiply_basis_elements`), both for
    /// single basis elements and for dense (multi-term) elements — the latter also exercising mod-2
    /// cancellation.
    #[test]
    fn admissible_multiply_agrees_with_reference() {
        let p = ValidPrime::new(2);
        let algebra = MilnorAlgebra::new(p, false);
        let max_degree = 32;
        algebra.compute_basis(max_degree);

        for r_degree in 0..=max_degree {
            let r_dim = algebra.dimension(r_degree);
            for s_degree in 0..=(max_degree - r_degree) {
                let s_dim = algebra.dimension(s_degree);
                let out_degree = r_degree + s_degree;
                let out_dim = algebra.dimension(out_degree);

                for i in 0..r_dim {
                    let mut expected_dense = FpVector::new(p, out_dim);

                    for j in 0..s_dim {
                        // Reference: R_i * S_j via the PPartMultiplier path.
                        let mut expected = FpVector::new(p, out_dim);
                        algebra.multiply_basis_elements(
                            expected.as_slice_mut(),
                            1,
                            r_degree,
                            i,
                            s_degree,
                            j,
                        );
                        expected_dense.add(&expected, 1);

                        // Admissible model: element `s = e_j`.
                        let mut s = FpVector::new(p, s_dim);
                        s.set_entry(j, 1);
                        let mut got = FpVector::new(p, out_dim);
                        algebra.multiply_basis_element_by_element_2(
                            got.as_slice_mut(),
                            1,
                            r_degree,
                            i,
                            s_degree,
                            s.as_slice(),
                        );
                        assert_eq!(
                            expected, got,
                            "single-term mismatch: R(deg {r_degree}, idx {i}) * S(deg {s_degree}, \
                             idx {j})",
                        );
                    }

                    // Dense element (all ones): multi-term handling and mod-2 cancellation.
                    if s_dim > 0 {
                        let mut s = FpVector::new(p, s_dim);
                        for j in 0..s_dim {
                            s.set_entry(j, 1);
                        }
                        let mut got = FpVector::new(p, out_dim);
                        algebra.multiply_basis_element_by_element_2(
                            got.as_slice_mut(),
                            1,
                            r_degree,
                            i,
                            s_degree,
                            s.as_slice(),
                        );
                        assert_eq!(
                            expected_dense, got,
                            "dense mismatch: R(deg {r_degree}, idx {i}) * (all of deg {s_degree})",
                        );
                    }
                }
            }
        }
    }

    #[rstest]
    #[trace]
    #[case(2, 32, None)]
    #[case(2, 32, Some(MilnorProfile { q_part: !0, p_part: vec!(3, 2, 1), truncated: true }))]
    #[case(2, 32, Some(MilnorProfile { q_part: !0, p_part: vec!(2, 2, 1), truncated: true }))]
    #[case(2, 32, Some(MilnorProfile { q_part: !0, p_part: vec!(0), truncated: false }))]
    #[case(3, 106, None)]
    #[case(3, 106, Some(MilnorProfile { q_part: 0b1111, p_part: vec!(3, 2, 1), truncated: true }))]
    #[case(3, 106, Some(MilnorProfile { q_part: 0b1111, p_part: vec!(2, 2, 1), truncated: true }))]
    fn test_milnor_decompose(
        #[case] p: u32,
        #[case] max_degree: i32,
        #[case] profile: Option<MilnorProfile>,
    ) {
        let p = ValidPrime::new(p);
        let algebra = MilnorAlgebra::new_with_profile(p, profile.unwrap_or_default(), false);
        algebra.compute_basis(max_degree);
        for i in 1..max_degree {
            let dim = algebra.dimension(i);
            let gens = algebra.generators(i);
            // println!("i : {}, gens : {:?}", i, gens);
            let mut out_vec = FpVector::new(p, dim);
            for j in 0..dim {
                if gens.contains(&j) {
                    continue;
                }
                for (coeff, (first_degree, first_idx), (second_degree, second_idx)) in
                    algebra.decompose_basis_element(i, j)
                {
                    // print!("{} * {} * {}  +  ", coeff, algebra.basis_element_to_string(first_degree,first_idx), algebra.basis_element_to_string(second_degree, second_idx));
                    algebra.multiply_basis_elements(
                        out_vec.as_slice_mut(),
                        coeff,
                        first_degree,
                        first_idx,
                        second_degree,
                        second_idx,
                    );
                }
                assert!(
                    out_vec.entry(j) == 1,
                    "{} != {}",
                    algebra.basis_element_to_string(i, j),
                    algebra.element_to_string(i, out_vec.as_slice())
                );
                out_vec.set_entry(j, 0);
                assert!(
                    out_vec.is_zero(),
                    "\n{} != {}",
                    algebra.basis_element_to_string(i, j),
                    algebra.element_to_string(i, out_vec.as_slice())
                );
            }
        }
    }

    #[rstest]
    #[trace]
    #[case(2, 32)]
    #[case(3, 106)]
    fn test_milnor_string(#[case] p: u32, #[case] max_degree: i32) {
        let p = ValidPrime::new(p);
        let algebra = MilnorAlgebra::new(p, false);
        algebra.compute_basis(max_degree);
        for t in 0..max_degree {
            for i in 0..algebra.dimension(t) {
                let elt = algebra.basis_element_to_string(t, i);
                assert_eq!(
                    Some((t, i)),
                    algebra.basis_element_from_string(&elt),
                    "Error parsing {elt}"
                );
            }
            for i in algebra.generators(t) {
                let elt = algebra.generator_to_string(t, i);
                assert_eq!(
                    Some((t, i)),
                    algebra.basis_element_from_string(&elt),
                    "Error parsing {elt}"
                );
            }
        }
    }

    #[test]
    fn try_beps_pn_milnor() {
        let p = ValidPrime::new(2);

        // On the full algebra, `try_beps_pn` agrees with the panicking `beps_pn`
        // for valid inputs and never returns `None` (every P(x), x >= 1, exists).
        let algebra = MilnorAlgebra::new(p, false);
        for x in 1..16 {
            assert_eq!(algebra.try_beps_pn(0, x), Some(algebra.beps_pn(0, x)));
            assert!(algebra.try_beps_pn(0, x).is_some());
        }

        // On A(2) (profile [3, 2, 1], truncated), the first xi exponent is bounded
        // by 2^3 - 1 = 7, so P(7) is present but P(8) is excluded by the profile.
        let a2 = MilnorAlgebra::new_with_profile(
            p,
            MilnorProfile {
                q_part: !0,
                p_part: vec![3, 2, 1],
                truncated: true,
            },
            false,
        );
        // Valid input: agrees with `beps_pn`.
        assert_eq!(a2.try_beps_pn(0, 7), Some(a2.beps_pn(0, 7)));
        // Invalid input: excluded by the profile, so `None` instead of a panic.
        assert_eq!(a2.try_beps_pn(0, 8), None);
    }

    #[test]
    fn basis_element_from_string_total_milnor() {
        let p = ValidPrime::new(2);
        let algebra = MilnorAlgebra::new(p, false);
        algebra.compute_basis(8);

        // Sanity: valid names round-trip through the canonical string form.
        for name in ["P(1)", "P(2)", "P(0, 1)"] {
            let (d, i) = algebra
                .basis_element_from_string(name)
                .unwrap_or_else(|| panic!("expected Some for {name}"));
            assert_eq!(algebra.basis_element_to_string(d, i), name);
        }

        // Syntactically-valid names that name no basis element must return `None`
        // (they previously panicked in `basis_element_to_index`).
        //
        // "P0"/"Sq0" parse via `try_beps_pn(0, 0)`, building the element
        // {q_part: 0, p_part: [0]} in degree 0, which is not a basis element.
        assert_eq!(algebra.basis_element_from_string("P0"), None);
        assert_eq!(algebra.basis_element_from_string("Sq0"), None);
        // "Q_5" parses via the Q/P branch into a candidate element (degree 63)
        // whose basis lookup finds nothing at p = 2.
        assert_eq!(algebra.basis_element_from_string("Q_5"), None);

        // On A(2) (profile [3, 2, 1], truncated) the first xi exponent is bounded
        // by 2^3 - 1 = 7. "P7" exists; the out-of-profile "P8" parses to a valid
        // candidate that is excluded by the profile, so it must return `None`.
        let a2 = MilnorAlgebra::new_with_profile(
            p,
            MilnorProfile {
                q_part: !0,
                p_part: vec![3, 2, 1],
                truncated: true,
            },
            false,
        );
        a2.compute_basis(16);
        assert!(a2.basis_element_from_string("P7").is_some());
        assert_eq!(a2.basis_element_from_string("P8"), None);
    }

    use crate::module::ModuleFailedRelationError;
    #[rstest(p, max_degree, case(2, 32), case(3, 106))]
    #[trace]
    fn test_adem_relations(p: u32, max_degree: i32) {
        let p = ValidPrime::new(p);
        let algebra = MilnorAlgebra::new(p, false);
        algebra.compute_basis(max_degree + 2);
        let mut output_vec = FpVector::new(p, 0);
        for i in 1..max_degree {
            let output_dim = algebra.dimension(i);
            output_vec.set_scratch_vector_size(output_dim);
            let relations = algebra.generating_relations(i);
            println!("{relations:?}");
            for relation in relations {
                for (coeff, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                    algebra.multiply_basis_elements(
                        output_vec.as_slice_mut(),
                        *coeff,
                        *deg_1,
                        *idx_1,
                        *deg_2,
                        *idx_2,
                    );
                }
                if !output_vec.is_zero() {
                    let mut relation_string = String::new();
                    for (coeff, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                        let _ = write!(
                            relation_string,
                            "{} * {} * {}  +  ",
                            coeff,
                            algebra.basis_element_to_string(*deg_1, *idx_1),
                            algebra.basis_element_to_string(*deg_2, *idx_2)
                        );
                    }
                    relation_string.pop();
                    relation_string.pop();
                    relation_string.pop();
                    relation_string.pop();
                    relation_string.pop();
                    let value_string = algebra.element_to_string(i, output_vec.as_slice());
                    panic!(
                        "{}",
                        ModuleFailedRelationError {
                            relation: relation_string,
                            value: value_string
                        }
                    );
                }
            }
        }
    }

    #[test]
    fn test_clone_into() {
        let mut other = MilnorBasisElement::default();

        let mut check = |a: &MilnorBasisElement| {
            a.clone_into(&mut other);
            assert_eq!(a, &other);
        };

        check(&MilnorBasisElement {
            q_part: 3,
            p_part: vec![3, 2],
            degree: 12,
        });
        check(&MilnorBasisElement {
            q_part: 1,
            p_part: vec![3],
            degree: 11,
        });
        check(&MilnorBasisElement {
            q_part: 5,
            p_part: vec![1, 3, 5, 2],
            degree: 7,
        });
        check(&MilnorBasisElement {
            q_part: 0,
            p_part: vec![],
            degree: 2,
        });
    }

    #[test]
    fn test_ppart_multiplier_2() {
        let r = vec![1, 4];
        let s = vec![2, 4];
        let mut m = PPartMultiplier::<false>::new_from_allocation(
            fp::prime::TWO,
            &r,
            &s,
            PPartAllocation::default(),
            0,
            0,
        );

        expect![[r#"
            [0, 2, 4]
            [1, 0, 0]
            [4, 0, 0]
        "#]]
        .assert_eq(&m.M.to_string());

        assert_eq!(m.next(), Some(1));

        expect![[r#"
            [0, 0, 4]
            [1, 0, 0]
            [0, 2, 0]
        "#]]
        .assert_eq(&m.M.to_string());

        assert_eq!(m.next(), Some(1));

        expect![[r#"
            [0, 2, 3]
            [1, 0, 0]
            [0, 0, 1]
        "#]]
        .assert_eq(&m.M.to_string());

        assert_eq!(m.next(), None);
    }

    #[test]
    fn test_ppart_multiplier_3() {
        let r = vec![3, 4];
        let s = vec![1, 4];
        let mut m = PPartMultiplier::<false>::new_from_allocation(
            ValidPrime::new(3),
            &r,
            &s,
            PPartAllocation::default(),
            0,
            0,
        );

        expect![[r#"
            [0, 1, 4]
            [3, 0, 0]
            [4, 0, 0]
        "#]]
        .assert_eq(&m.M.to_string());

        assert_eq!(m.next(), Some(1));

        expect![[r#"
            [0, 1, 4]
            [3, 0, 0]
            [4, 0, 0]
        "#]]
        .assert_eq(&m.M.to_string());

        assert_eq!(m.next(), Some(2));

        expect![[r#"
            [0, 0, 4]
            [3, 0, 0]
            [1, 1, 0]
        "#]]
        .assert_eq(&m.M.to_string());

        assert_eq!(m.next(), None);
    }

    #[test]
    fn test_valid_profile() {
        assert!(
            (MilnorProfile {
                p_part: vec![3, 2, 1],
                q_part: !0,
                truncated: true
            })
            .is_valid()
        );

        assert!(
            !(MilnorProfile {
                p_part: vec![3, 2],
                q_part: !0,
                truncated: true
            })
            .is_valid()
        );

        assert!(
            (MilnorProfile {
                p_part: vec![3, 2, 1],
                q_part: 0b1111,
                truncated: true
            })
            .is_valid()
        );

        assert!(
            (MilnorProfile {
                p_part: vec![2, 4, 2],
                q_part: 0,
                truncated: false
            })
            .is_valid()
        );

        assert!(
            !(MilnorProfile {
                p_part: vec![3, 2, 1],
                q_part: 0b111,
                truncated: true
            })
            .is_valid()
        );
    }
}
