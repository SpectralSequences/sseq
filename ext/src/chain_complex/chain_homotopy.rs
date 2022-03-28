use crate::chain_complex::{ChainComplex, FreeChainComplex};
use crate::resolution_homomorphism::ResolutionHomomorphism;
use crate::save::SaveKind;
use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::Module;
use fp::prime::ValidPrime;
use fp::vector::FpVector;
use once::OnceBiVec;

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
    homotopies: OnceBiVec<Arc<FreeModuleHomomorphism<U::Module>>>,
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
            path.push(format!("massey/{},{}/", left.name(), right.name(),));

            SaveKind::ChainHomotopy.create_dir(&path).unwrap();

            Some(path)
        } else {
            None
        };

        assert!(Arc::ptr_eq(&left.target, &right.source));
        Self {
            homotopies: OnceBiVec::new((left.shift_s + right.shift_s) as i32 - 1),
            left,
            right,
            lock: Mutex::new(()),
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
                    .module(s + 1 - self.shift_s())
                    .max_computed_degree()
                    + self.shift_t()
                    + 1,
            )
        };

        self.extend_profile(max_source_s, &max_source_t);
    }

    /// Initialize self.homotopies to contain [`FreeModuleHomomorphisms`]s up to but excluding
    /// `max_source_s`, which can be returned by [`Self::homotopy`]. This does not actually lift
    /// the maps, which is done by [`Self::extend_all`] and [`Self::extend`].
    pub fn initialize_homotopies(&self, max_source_s: u32) {
        self.homotopies.extend(max_source_s as i32 - 1, |s| {
            let s = s as u32;
            Arc::new(FreeModuleHomomorphism::new(
                self.left.source.module(s),
                self.right.target.module(s + 1 - self.shift_s()),
                self.shift_t(),
            ))
        });
    }

    /// Exclusive bounds
    fn extend_profile(&self, max_source_s: u32, max_source_t: &(impl Fn(u32) -> i32 + Sync)) {
        let shift_s = self.shift_s();

        if max_source_s == shift_s - 1 {
            return;
        }

        let _lock = self.lock.lock();

        self.initialize_homotopies(max_source_s);

        #[cfg(not(feature = "concurrent"))]
        {
            for source_s in shift_s - 1..max_source_s {
                for source_t in
                    self.homotopies[source_s as i32].next_degree()..max_source_t(source_s)
                {
                    self.extend_step(source_s, source_t);
                }
            }
        }

        #[cfg(feature = "concurrent")]
        {
            let min_source_t = std::cmp::min(
                self.left.source.min_degree(),
                self.right.target.min_degree() + self.shift_t(),
            );

            crate::utils::iter_s_t(
                &|s, t| self.extend_step(s, t),
                shift_s - 1,
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
        let target_s = source_s + 1 - shift_s;
        let target_t = source_t - shift_t;

        if self.homotopies[source_s as i32].next_degree() > source_t {
            return source_t..source_t + 1;
        }

        let num_gens = self
            .left
            .source
            .module(source_s)
            .number_of_gens_in_degree(source_t);

        let target_dim = self.right.target.module(target_s).dimension(target_t);

        // Default to the zero homotopy for the bottom-most homotopy. For computing normal Massey
        // products, any choice works, and it is conventional to choose zero. For secondary Massey
        // products, this may have to be non-zero, in which case the user should manually set up
        // these values.
        if target_s == 0 || target_dim == 0 || num_gens == 0 {
            let outputs = vec![FpVector::new(p, target_dim); num_gens];
            return self.homotopies[source_s as i32]
                .add_generators_from_rows_ooo(source_t, outputs);
        }

        if let Some(dir) = &self.save_dir {
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
                return self.homotopies[source_s as i32]
                    .add_generators_from_rows_ooo(source_t, outputs);
            }
        }

        let mut outputs = vec![FpVector::new(p, target_dim); num_gens];

        let f = |i| {
            let mut scratch = FpVector::new(
                p,
                self.right.target.module(target_s - 1).dimension(target_t),
            );
            self.right.get_map(source_s - self.left.shift_s).apply(
                scratch.as_slice_mut(),
                1,
                source_t - self.left.shift_t,
                self.left.get_map(source_s).output(source_t, i).as_slice(),
            );

            self.homotopies[source_s as i32 - 1].apply(
                scratch.as_slice_mut(),
                *p - 1,
                source_t,
                self.left
                    .source
                    .differential(source_s)
                    .output(source_t, i)
                    .as_slice(),
            );

            #[cfg(debug_assertions)]
            if target_s > 1
                && self
                    .right
                    .target
                    .has_computed_bidegree(target_s - 2, target_t)
            {
                let mut r = FpVector::new(
                    p,
                    self.right.target.module(target_s - 2).dimension(target_t),
                );
                self.right.target.differential(target_s - 1).apply(
                    r.as_slice_mut(),
                    1,
                    target_t,
                    scratch.as_slice(),
                );
                assert!(
                    r.is_zero(),
                    "Failed to lift at (target_s, target_t) = ({}, {})",
                    target_s - 1,
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
            target_s,
            target_t,
            &scratches,
        ));

        if let Some(dir) = &self.save_dir {
            let mut f = self
                .left
                .source
                .save_file(SaveKind::ChainHomotopy, source_s, source_t)
                .create_file(dir.to_owned(), false);
            for row in &outputs {
                row.to_bytes(&mut f).unwrap();
            }
        }
        self.homotopies[source_s as i32].add_generators_from_rows_ooo(source_t, outputs)
    }

    pub fn homotopy(&self, source_s: u32) -> Arc<FreeModuleHomomorphism<U::Module>> {
        Arc::clone(&self.homotopies[source_s as i32])
    }

    pub fn save_dir(&self) -> Option<&Path> {
        self.save_dir.as_deref()
    }
}
