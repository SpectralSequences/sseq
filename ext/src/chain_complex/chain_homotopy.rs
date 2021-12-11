use crate::chain_complex::{ChainComplex, FreeChainComplex};
use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::Module;
use fp::prime::ValidPrime;
use fp::vector::FpVector;
use once::OnceVec;
use std::sync::Mutex;

/// A chain homotopy from $f to g$, or equivalently a null-homotopy of $h = f - g$. A chain map is
/// a priori a collection of free module homomorphisms. However, instead of providing
/// FreeModuleHomomorphism objects, the user is expected to give a function that computes the value
/// of $h$ on each generator.
pub struct ChainHomotopy<
    'a,
    S: FreeChainComplex,
    T: ChainComplex + Sync,
    F: Fn(u32, i32, usize, &mut FpVector) + Sync,
> {
    source: &'a S,
    target: &'a T,
    /// The $s$ shift of the original chain map $f - g$.
    shift_s: u32,
    /// The $t$ shift of the original chain map $f - g$.
    shift_t: i32,
    /// A function that given (s, t, idx, result), adds (f - g)(x_{s, t, i}), to `result`.
    map: F,
    lock: Mutex<()>,
    /// Homotopies, indexed by the filtration of the target of f - g.
    homotopies: OnceVec<FreeModuleHomomorphism<T::Module>>,
}

impl<
        'a,
        S: FreeChainComplex,
        T: ChainComplex<Algebra = S::Algebra> + Sync,
        F: Fn(u32, i32, usize, &mut FpVector) + Sync,
    > ChainHomotopy<'a, S, T, F>
{
    pub fn new(source: &'a S, target: &'a T, shift_s: u32, shift_t: i32, map: F) -> Self {
        Self {
            source,
            target,
            shift_s,
            shift_t,
            map,
            lock: Mutex::new(()),
            homotopies: OnceVec::new(),
        }
    }

    pub fn prime(&self) -> ValidPrime {
        self.source.prime()
    }

    /// Lift maps so that the chain *homotopy* is defined on `(max_source_s, max_source_t)`.
    pub fn extend(&self, max_source_s: u32, max_source_t: i32) {
        let _lock = self.lock.lock();

        let p = self.source.prime();

        let mut scratch = FpVector::new(p, 0);

        self.homotopies
            .extend((max_source_s - self.shift_s) as usize, |s| {
                let s = s as u32;
                FreeModuleHomomorphism::new(
                    self.source.module(s + self.shift_s),
                    self.target.module(s + 1),
                    self.shift_t,
                )
            });

        for source_s in self.shift_s..=max_source_s {
            let target_s = source_s - self.shift_s;
            for source_t in self.homotopies[target_s as usize].next_degree()
                ..=max_source_t - (max_source_s - source_s) as i32
            {
                self.extend_step(source_s, source_t, &mut scratch);
            }
        }
    }

    /// Lift maps so that the chain homotopy is defined on as many bidegrees as possible
    pub fn extend_all(&self) {
        let max_source_s = std::cmp::min(
            self.source.next_homological_degree(),
            self.target.next_homological_degree() + self.shift_s,
        );
        if max_source_s == 0 {
            return;
        }
        let max_source_s = max_source_s - 1;

        let _lock = self.lock.lock();

        let p = self.source.prime();

        self.homotopies
            .extend((max_source_s - self.shift_s) as usize, |s| {
                let s = s as u32;
                FreeModuleHomomorphism::new(
                    self.source.module(s + self.shift_s),
                    self.target.module(s + 1),
                    self.shift_t,
                )
            });

        let max_source_t = |s| {
            std::cmp::min(
                self.source.module(s).max_computed_degree(),
                self.target
                    .module(s - self.shift_s + 1)
                    .max_computed_degree()
                    + self.shift_t,
            )
        };

        #[cfg(not(feature = "concurrent"))]
        {
            let mut scratch = FpVector::new(p, 0);
            for source_s in self.shift_s..=max_source_s {
                for source_t in self.homotopies[(source_s - self.shift_s) as usize].next_degree()
                    ..=max_source_t(source_s)
                {
                    self.extend_step(source_s, source_t, &mut scratch);
                }
            }
        }

        #[cfg(feature = "concurrent")]
        {
            let min_source_t = std::cmp::min(
                self.source.min_degree(),
                self.target.min_degree() + self.shift_t,
            );

            crate::utils::iter_s_t(
                &|s, t| self.extend_step(s, t, &mut FpVector::new(p, 0)),
                self.shift_s,
                min_source_t,
                max_source_s,
                &max_source_t,
            );
        }
    }

    fn extend_step(
        &self,
        source_s: u32,
        source_t: i32,
        scratch: &mut FpVector,
    ) -> std::ops::Range<i32> {
        let p = self.prime();
        let target_s = source_s - self.shift_s;
        let target_t = source_t - self.shift_t;

        if self.homotopies[target_s].next_degree() > source_t {
            return source_t..source_t + 1;
        }

        let num_gens = self
            .source
            .module(source_s)
            .number_of_gens_in_degree(source_t);

        let target_dim = self.target.module(target_s + 1).dimension(target_t);
        let mut outputs = vec![FpVector::new(p, target_dim); num_gens];

        scratch.set_scratch_vector_size(self.target.module(target_s).dimension(target_t));

        for (i, row) in outputs.iter_mut().enumerate() {
            (self.map)(source_s, source_t, i, scratch);

            if target_s > 0 {
                self.homotopies[target_s as usize - 1].apply(
                    scratch.as_slice_mut(),
                    *p - 1,
                    source_t,
                    self.source
                        .differential(source_s)
                        .output(source_t, i)
                        .as_slice(),
                );
            }

            #[cfg(debug_assertions)]
            if target_s > 0 && self.target.has_computed_bidegree(target_s - 1, target_t) {
                let mut r = FpVector::new(p, self.target.module(target_s - 1).dimension(target_t));
                self.target.differential(target_s).apply(
                    r.as_slice_mut(),
                    1,
                    target_t,
                    scratch.as_slice(),
                );
                assert!(
                    r.is_zero(),
                    "Failed to lift at (target_s, target_t) = ({}, {})",
                    target_s,
                    target_t
                );
            }

            assert!(self.target.differential(target_s + 1).apply_quasi_inverse(
                row.as_slice_mut(),
                target_t,
                scratch.as_slice(),
            ));
            scratch.set_to_zero();
        }
        self.homotopies[target_s as usize].add_generators_from_rows_ooo(source_t, outputs)
    }

    pub fn homotopy(&self, source_s: u32) -> &FreeModuleHomomorphism<T::Module> {
        &self.homotopies[(source_s - self.shift_s) as usize]
    }

    /// Into the vec of homotopies. This Vec is indexed by the homological degree of the target of
    /// `f - g`.
    pub fn into_homotopies(self) -> OnceVec<FreeModuleHomomorphism<T::Module>> {
        self.homotopies
    }
}
