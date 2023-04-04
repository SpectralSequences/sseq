use std::sync::{Arc, Mutex};

use algebra::module::{
    homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism},
    Module,
};
use fp::{prime::ValidPrime, vector::FpVector};
use maybe_rayon::prelude::*;
use once::OnceBiVec;
use sseq::coordinates::{Bidegree, BidegreeRange};

use crate::{
    chain_complex::{ChainComplex, FreeChainComplex},
    resolution_homomorphism::ResolutionHomomorphism,
    save::{SaveDirectory, SaveKind},
};

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
    save_dir: SaveDirectory,
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
            let mut save_dir = left.source.save_dir().clone();
            save_dir.push(format!(
                "massey/{left_s}/{right_s}/{},{}/",
                left.name(),
                right.name(),
                left_s = left.shift.s(),
                right_s = right.shift.s(),
            ));

            SaveKind::ChainHomotopy
                .create_dir(save_dir.write().unwrap())
                .unwrap();

            save_dir
        } else {
            SaveDirectory::None
        };

        assert!(Arc::ptr_eq(&left.target, &right.source));
        Self {
            homotopies: OnceBiVec::new((left.shift + right.shift).s() as i32 - 1),
            left,
            right,
            lock: Mutex::new(()),
            save_dir,
        }
    }

    pub fn prime(&self) -> ValidPrime {
        self.left.source.prime()
    }

    pub fn shift(&self) -> Bidegree {
        self.left.shift + self.right.shift
    }

    pub fn left(&self) -> Arc<ResolutionHomomorphism<S, T>> {
        Arc::clone(&self.left)
    }

    pub fn right(&self) -> Arc<ResolutionHomomorphism<T, U>> {
        Arc::clone(&self.right)
    }

    /// Lift maps so that the chain *homotopy* is defined on `max_source`.
    pub fn extend(&self, max_source: Bidegree) {
        self.extend_profile(BidegreeRange::new(&(), max_source.s() + 1, &|_, s| {
            max_source.t() - (max_source.s() - s) as i32 + 1
        }));
    }

    /// Lift maps so that the chain homotopy is defined on as many bidegrees as possible
    pub fn extend_all(&self) {
        self.extend_profile(BidegreeRange::new(
            &self,
            std::cmp::min(
                self.left.source.next_homological_degree(),
                self.right.target.next_homological_degree() + self.shift().s(),
            ),
            &|selff, s| {
                std::cmp::min(
                    selff.left.source.module(s).max_computed_degree() + 1,
                    selff
                        .right
                        .target
                        .module(s + 1 - selff.shift().s())
                        .max_computed_degree()
                        + selff.shift().t()
                        + 1,
                )
            },
        ));
    }

    /// Initialize self.homotopies to contain [`FreeModuleHomomorphisms`]s up to but excluding
    /// `max_source_s`, which can be returned by [`Self::homotopy`]. This does not actually lift
    /// the maps, which is done by [`Self::extend_all`] and [`Self::extend`].
    pub fn initialize_homotopies(&self, max_source_s: u32) {
        self.homotopies.extend(max_source_s as i32 - 1, |s| {
            let s = s as u32;
            Arc::new(FreeModuleHomomorphism::new(
                self.left.source.module(s),
                self.right.target.module(s + 1 - self.shift().s()),
                self.shift().t(),
            ))
        });
    }

    /// Exclusive bounds
    fn extend_profile<AUX: Sync>(&self, max_source: BidegreeRange<AUX>) {
        let shift = self.shift();

        if max_source.s() == shift.s() - 1 {
            return;
        }

        let _lock = self.lock.lock();

        self.initialize_homotopies(max_source.s());

        let min = Bidegree::s_t(
            shift.s() - 1,
            std::cmp::min(
                self.left.source.min_degree(),
                self.right.target.min_degree() + shift.t(),
            ),
        );

        sseq::coordinates::iter_s_t(&|b| self.extend_step(b), min, max_source);
    }

    fn extend_step(&self, source: Bidegree) -> std::ops::Range<i32> {
        let p = self.prime();
        let shift = self.shift();
        let target = source + Bidegree::s_t(1, 0) - shift;

        if self.homotopies[source.s() as i32].next_degree() > source.t() {
            return source.t()..source.t() + 1;
        }

        let num_gens = self
            .left
            .source
            .module(source.s())
            .number_of_gens_in_degree(source.t());

        let target_dim = self.right.target.module(target.s()).dimension(target.t());

        // Default to the zero homotopy for the bottom-most homotopy. For computing normal Massey
        // products, any choice works, and it is conventional to choose zero. For secondary Massey
        // products, this may have to be non-zero, in which case the user should manually set up
        // these values.
        if target.s() == 0 || target_dim == 0 || num_gens == 0 {
            let outputs = vec![FpVector::new(p, target_dim); num_gens];
            return self.homotopies[source.s() as i32]
                .add_generators_from_rows_ooo(source.t(), outputs);
        }

        if let Some(dir) = self.save_dir.read() {
            if let Some(mut f) = self
                .left
                .source
                .save_file(SaveKind::ChainHomotopy, source)
                .open_file(dir.to_owned())
            {
                let mut outputs = Vec::with_capacity(num_gens);
                for _ in 0..num_gens {
                    outputs.push(FpVector::from_bytes(p, target_dim, &mut f).unwrap());
                }
                return self.homotopies[source.s() as i32]
                    .add_generators_from_rows_ooo(source.t(), outputs);
            }
        }

        let mut outputs = vec![FpVector::new(p, target_dim); num_gens];

        let f = |i| {
            let mut scratch = FpVector::new(
                p,
                self.right
                    .target
                    .module(target.s() - 1)
                    .dimension(target.t()),
            );
            let left_shifted_b = source - self.left.shift;
            self.right.get_map(left_shifted_b.s()).apply(
                scratch.as_slice_mut(),
                1,
                left_shifted_b.t(),
                self.left
                    .get_map(source.s())
                    .output(source.t(), i)
                    .as_slice(),
            );

            self.homotopies[source.s() as i32 - 1].apply(
                scratch.as_slice_mut(),
                p - 1,
                source.t(),
                self.left
                    .source
                    .differential(source.s())
                    .output(source.t(), i)
                    .as_slice(),
            );

            #[cfg(debug_assertions)]
            if target.s() > 1
                && self
                    .right
                    .target
                    .has_computed_bidegree(target - Bidegree::s_t(2, 0))
            {
                let mut r = FpVector::new(
                    p,
                    self.right
                        .target
                        .module(target.s() - 2)
                        .dimension(target.t()),
                );
                self.right.target.differential(target.s() - 1).apply(
                    r.as_slice_mut(),
                    1,
                    target.t(),
                    scratch.as_slice(),
                );
                assert!(
                    r.is_zero(),
                    "Failed to lift at {target_prev}",
                    target_prev = target - Bidegree::s_t(1, 0)
                );
            }

            scratch
        };

        let scratches: Vec<FpVector> = (0..num_gens).into_maybe_par_iter().map(f).collect();

        assert!(U::apply_quasi_inverse(
            &*self.right.target,
            &mut outputs,
            target,
            &scratches,
        ));

        if let Some(dir) = self.save_dir.write() {
            let mut f = self
                .left
                .source
                .save_file(SaveKind::ChainHomotopy, source)
                .create_file(dir.to_owned(), false);
            for row in &outputs {
                row.to_bytes(&mut f).unwrap();
            }
        }
        self.homotopies[source.s() as i32].add_generators_from_rows_ooo(source.t(), outputs)
    }

    pub fn homotopy(&self, source_s: u32) -> Arc<FreeModuleHomomorphism<U::Module>> {
        Arc::clone(&self.homotopies[source_s as i32])
    }

    pub fn save_dir(&self) -> &SaveDirectory {
        &self.save_dir
    }
}
