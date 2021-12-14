use crate::chain_complex::{ChainComplex, FreeChainComplex};
use crate::resolution_homomorphism::ResolutionHomomorphism;
use crate::save::SaveKind;
use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::Module;
use fp::prime::ValidPrime;
use fp::vector::FpVector;
use once::OnceVec;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;

#[cfg(feature = "concurrent")]
use rayon::prelude::*;

// Another instance of https://github.com/rust-lang/rust/issues/91380
/// A chain homotopy from $f to g$, or equivalently a null-homotopy of $h = f - g$. A chain map is
/// a priori a collection of free module homomorphisms. However, instead of providing
/// FreeModuleHomomorphism objects, the user is expected to give a function that computes the value
/// of $h$ on each generator.
#[doc(hidden)]
pub struct ChainHomotopy<
    S: FreeChainComplex,
    T: FreeChainComplex<Algebra = S::Algebra> + Sync,
    U: ChainComplex<Algebra = S::Algebra> + Sync,
> {
    left: Arc<ResolutionHomomorphism<S, T>>,
    right: Arc<ResolutionHomomorphism<T, U>>,
    lock: Mutex<()>,
    /// Homotopies, indexed by the filtration of the target of f - g.
    homotopies: OnceVec<Arc<FreeModuleHomomorphism<U::Module>>>,
    save_dir: Option<PathBuf>,
}

impl<
        S: FreeChainComplex,
        T: FreeChainComplex<Algebra = S::Algebra> + Sync,
        U: ChainComplex<Algebra = S::Algebra> + Sync,
    > ChainHomotopy<S, T, U>
{
    pub fn new(
        left: Arc<ResolutionHomomorphism<S, T>>,
        right: Arc<ResolutionHomomorphism<T, U>>,
    ) -> Self {
        let save_dir = if left.source.save_dir().is_some()
            && !left.name().is_empty()
            && !right.name().is_empty()
        {
            let mut path = left.source.save_dir().unwrap().to_owned();
            path.push(format!("products/{},{}/", left.name(), right.name(),));

            SaveKind::ChainHomotopy.create_dir(&path).unwrap();

            Some(path)
        } else {
            None
        };

        assert!(Arc::ptr_eq(&left.target, &right.source));
        Self {
            left,
            right,
            lock: Mutex::new(()),
            homotopies: OnceVec::new(),
            save_dir,
        }
    }

    pub fn prime(&self) -> ValidPrime {
        self.left.source.prime()
    }

    pub fn shift_s(&self) -> u32 {
        self.left.shift_s + self.right.shift_s
    }

    pub fn shift_t(&self) -> i32 {
        self.left.shift_t + self.right.shift_t
    }

    pub fn left(&self) -> Arc<ResolutionHomomorphism<S, T>> {
        Arc::clone(&self.left)
    }

    pub fn right(&self) -> Arc<ResolutionHomomorphism<T, U>> {
        Arc::clone(&self.right)
    }

    /// Lift maps so that the chain *homotopy* is defined on `(max_source_s, max_source_t)`.
    pub fn extend(&self, max_source_s: u32, max_source_t: i32) {
        self.extend_profile(max_source_s + 1, &|s| {
            max_source_t - (max_source_s - s) as i32 + 1
        });
    }

    /// Lift maps so that the chain homotopy is defined on as many bidegrees as possible
    pub fn extend_all(&self) {
        let max_source_s = std::cmp::min(
            self.left.source.next_homological_degree(),
            self.right.target.next_homological_degree() + self.shift_s(),
        );

        let max_source_t = |s| {
            std::cmp::min(
                self.left.source.module(s).max_computed_degree() + 1,
                self.right
                    .target
                    .module(s - self.shift_s() + 1)
                    .max_computed_degree()
                    + self.shift_t()
                    + 1,
            )
        };

        self.extend_profile(max_source_s, &max_source_t);
    }

    /// Exclusive bounds
    fn extend_profile(&self, max_source_s: u32, max_source_t: &(impl Fn(u32) -> i32 + Sync)) {
        let shift_s = self.shift_s();
        let shift_t = self.shift_t();

        if max_source_s == shift_s {
            return;
        }

        let _lock = self.lock.lock();

        self.homotopies
            .extend((max_source_s - shift_s - 1) as usize, |s| {
                let s = s as u32;
                Arc::new(FreeModuleHomomorphism::new(
                    self.left.source.module(s + shift_s),
                    self.right.target.module(s + 1),
                    shift_t,
                ))
            });

        #[cfg(not(feature = "concurrent"))]
        {
            for source_s in shift_s..max_source_s {
                for source_t in self.homotopies[(source_s - shift_s) as usize].next_degree()
                    ..max_source_t(source_s)
                {
                    self.extend_step(source_s, source_t);
                }
            }
        }

        #[cfg(feature = "concurrent")]
        {
            let min_source_t = std::cmp::min(
                self.left.source.min_degree(),
                self.right.target.min_degree() + shift_t,
            );

            crate::utils::iter_s_t(
                &|s, t| self.extend_step(s, t),
                shift_s,
                min_source_t,
                max_source_s,
                max_source_t,
            );
        }
    }

    fn extend_step(&self, source_s: u32, source_t: i32) -> std::ops::Range<i32> {
        let p = self.prime();
        let shift_s = self.shift_s();
        let shift_t = self.shift_t();
        let target_s = source_s - shift_s;
        let target_t = source_t - shift_t;

        if self.homotopies[target_s].next_degree() > source_t {
            return source_t..source_t + 1;
        }

        let num_gens = self
            .left
            .source
            .module(source_s)
            .number_of_gens_in_degree(source_t);

        let target_dim = self.right.target.module(target_s + 1).dimension(target_t);

        if target_dim == 0 || num_gens == 0 {
            let outputs = vec![FpVector::new(p, target_dim); num_gens];
            return self.homotopies[target_s as usize]
                .add_generators_from_rows_ooo(source_t, outputs);
        }

        if let Some(dir) = self.save_dir.as_ref() {
            if let Some(mut f) = self
                .left
                .source
                .save_file(SaveKind::ChainHomotopy, source_s, source_t)
                .open_file(dir.to_owned())
            {
                let mut outputs = Vec::with_capacity(num_gens);
                for _ in 0..num_gens {
                    outputs.push(FpVector::from_bytes(p, target_dim, &mut f).unwrap());
                }
                return self.homotopies[target_s].add_generators_from_rows_ooo(source_t, outputs);
            }
        }

        let mut outputs = vec![FpVector::new(p, target_dim); num_gens];

        let f = |i| {
            let mut scratch =
                FpVector::new(p, self.right.target.module(target_s).dimension(target_t));
            self.right.get_map(source_s - self.left.shift_s).apply(
                scratch.as_slice_mut(),
                1,
                source_t - self.left.shift_t,
                self.left.get_map(source_s).output(source_t, i).as_slice(),
            );

            if target_s > 0 {
                self.homotopies[target_s as usize - 1].apply(
                    scratch.as_slice_mut(),
                    *p - 1,
                    source_t,
                    self.left
                        .source
                        .differential(source_s)
                        .output(source_t, i)
                        .as_slice(),
                );
            }

            #[cfg(debug_assertions)]
            if target_s > 0
                && self
                    .right
                    .target
                    .has_computed_bidegree(target_s - 1, target_t)
            {
                let mut r = FpVector::new(
                    p,
                    self.right.target.module(target_s - 1).dimension(target_t),
                );
                self.right.target.differential(target_s).apply(
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

            scratch
        };

        #[cfg(not(feature = "concurrent"))]
        let scratches: Vec<FpVector> = (0..num_gens).into_iter().map(f).collect();

        #[cfg(feature = "concurrent")]
        let scratches: Vec<FpVector> = (0..num_gens).into_par_iter().map(f).collect();

        assert!(self.right.target.apply_quasi_inverse(
            &mut outputs,
            target_s + 1,
            target_t,
            &scratches,
        ));

        if let Some(dir) = self.save_dir.as_ref() {
            let mut f = self
                .left
                .source
                .save_file(SaveKind::ChainHomotopy, source_s, source_t)
                .create_file(dir.to_owned());
            for row in &outputs {
                row.to_bytes(&mut f).unwrap();
            }
        }
        self.homotopies[target_s as usize].add_generators_from_rows_ooo(source_t, outputs)
    }

    pub fn homotopy(&self, source_s: u32) -> Arc<FreeModuleHomomorphism<U::Module>> {
        Arc::clone(&self.homotopies[(source_s - self.shift_s()) as usize])
    }

    pub fn save_dir(&self) -> Option<&Path> {
        self.save_dir.as_deref()
    }
}
