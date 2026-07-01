use std::sync::{Arc, Mutex};

use algebra::module::{
    Module,
    homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism},
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
            save_dir.push(format!("massey/{},{}/", left.name(), right.name()));

            SaveKind::ChainHomotopy
                .create_dir(save_dir.write().unwrap())
                .unwrap();

            save_dir
        } else {
            SaveDirectory::None
        };

        assert!(Arc::ptr_eq(&left.target, &right.source));
        Self {
            homotopies: OnceBiVec::new((left.shift + right.shift).s() - 1),
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
            max_source.t() - max_source.s() + s + 1
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
    pub fn initialize_homotopies(&self, max_source_s: i32) {
        self.homotopies.extend(max_source_s - 1, |s| {
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

        if self.homotopies[source.s()].next_degree() > source.t() {
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
            return self.homotopies[source.s()].add_generators_from_rows_ooo(source.t(), outputs);
        }

        if let Some(dir) = self.save_dir.read()
            && let Some(mut f) = self
                .left
                .source
                .save_file(SaveKind::ChainHomotopy, source)
                .open_file(dir.to_owned())
        {
            let mut outputs = Vec::with_capacity(num_gens);
            for _ in 0..num_gens {
                outputs.push(FpVector::from_bytes(p, target_dim, &mut f).unwrap());
            }
            return self.homotopies[source.s()].add_generators_from_rows_ooo(source.t(), outputs);
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

            self.homotopies[source.s() - 1].apply(
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
        self.homotopies[source.s()].add_generators_from_rows_ooo(source.t(), outputs)
    }

    pub fn homotopy(&self, source_s: i32) -> Arc<FreeModuleHomomorphism<U::Module>> {
        Arc::clone(&self.homotopies[source_s])
    }

    /// Like [`homotopy`](Self::homotopy), but returns `None` for any homological degree outside the
    /// currently defined range (see [`defined_range`](Self::defined_range)) instead of panicking.
    ///
    /// [`homotopy`](Self::homotopy) indexes the internal `homotopies` `OnceBiVec` and panics out of
    /// range; this is the non-panicking sibling used to guard such an access (e.g. from the Python
    /// bindings).
    pub fn try_homotopy(&self, source_s: i32) -> Option<Arc<FreeModuleHomomorphism<U::Module>>> {
        let range = self.defined_range();
        if source_s < range.start || source_s >= range.end {
            None
        } else {
            Some(self.homotopy(source_s))
        }
    }

    /// The range of homological degrees `s` for which [`Self::homotopy`] is
    /// currently defined (i.e. the populated range of the internal homotopy
    /// table). Used by external callers (e.g. the Python bindings) to guard
    /// [`Self::homotopy`] against an out-of-range index without panicking.
    pub fn defined_range(&self) -> std::ops::Range<i32> {
        self.homotopies.range()
    }

    pub fn save_dir(&self) -> &SaveDirectory {
        &self.save_dir
    }
}

// The secondary lift of a `ChainHomotopy` lives here, beside the primary object it lifts, rather
// than in the monolithic `secondary` module. This keeps `secondary.rs` to the shared lift machinery
// and pairs each variant with its primary for locality. The module is `pub(crate)`;
// `SecondaryChainHomotopy` is re-exported from `crate::secondary` so the public API path is
// unchanged.
pub(crate) mod secondary {
    use std::sync::Arc;

    use algebra::{
        module::{Module, homomorphism::ModuleHomomorphism},
        pair_algebra::PairAlgebra,
    };
    use dashmap::DashMap;
    use fp::vector::FpVector;
    use once::OnceBiVec;
    use sseq::coordinates::{Bidegree, BidegreeGenerator, BidegreeRange};

    use super::ChainHomotopy;
    use crate::{
        chain_complex::FreeChainComplex,
        resolution_homomorphism::ResolutionHomomorphism,
        save::{SaveDirectory, SaveKind},
        secondary::{
            CompositeData, LAMBDA_BIDEGREE, SecondaryHomotopy, SecondaryLift,
            SecondaryResolutionHomomorphism,
        },
    };

    #[doc(hidden)]
    pub struct SecondaryChainHomotopy<
        S: FreeChainComplex,
        T: FreeChainComplex<Algebra = S::Algebra> + Sync,
        U: FreeChainComplex<Algebra = S::Algebra> + Sync,
    >
    where
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

        fn max(&self) -> BidegreeRange<'_, Self> {
            BidegreeRange::new(
                self,
                std::cmp::min(
                    self.right.secondary_target().max().s() + self.shift().s() - 1,
                    self.left.secondary_source().max().s(),
                ),
                &|selff, s| {
                    std::cmp::min(
                        selff.left.secondary_source().max().t(s),
                        if s == selff.shift().s() {
                            i32::MAX
                        } else {
                            selff
                                .right
                                .secondary_target()
                                .max()
                                .t(s - selff.shift().s() + 1)
                                + selff.shift().t()
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

        fn compute_intermediate(&self, g: BidegreeGenerator) -> FpVector {
            let p = self.prime();
            let neg_1 = p - 1;
            let shifted_b = g.degree() - self.shift();

            let target = self.target().module(shifted_b.s() - 1);

            let mut result = FpVector::new(p, target.dimension(shifted_b.t() - 1));

            self.homotopies[g.s() - 1].act(
                result.as_slice_mut(),
                1,
                g.t(),
                self.source()
                    .differential(g.s())
                    .output(g.t(), g.idx())
                    .as_slice(),
                false,
            );

            self.right.secondary_target().homotopies()[shifted_b.s() + 1].act(
                result.as_slice_mut(),
                1,
                shifted_b.t(),
                self.underlying
                    .homotopy(g.s())
                    .output(g.t(), g.idx())
                    .as_slice(),
                true,
            );

            self.underlying.homotopy(g.s() - 2).apply(
                result.as_slice_mut(),
                neg_1,
                g.t() - 1,
                self.left.secondary_source().homotopies()[g.s()]
                    .homotopies
                    .output(g.t(), g.idx())
                    .as_slice(),
            );

            let left_shifted_b = g.degree() - self.left.underlying().shift;
            self.right.homotopies()[left_shifted_b.s()].act(
                result.as_slice_mut(),
                neg_1,
                left_shifted_b.t(),
                self.left
                    .underlying()
                    .get_map(g.s())
                    .output(g.t(), g.idx())
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
                        .underlying()
                        .get_map(g.s())
                        .output(g.t(), g.idx())
                        .as_slice(),
                );
            }

            self.right
                .underlying()
                .get_map(left_shifted_b.s() - 1)
                .apply(
                    result.as_slice_mut(),
                    neg_1,
                    left_shifted_b.t() - 1,
                    self.left.homotopies()[g.s()]
                        .homotopies
                        .output(g.t(), g.idx())
                        .as_slice(),
                );

            if let Some(left_lambda) = &self.left_lambda {
                self.right
                    .underlying()
                    .get_map(left_shifted_b.s() - 1)
                    .apply(
                        result.as_slice_mut(),
                        neg_1,
                        left_shifted_b.t() - 1,
                        left_lambda.get_map(g.s()).output(g.t(), g.idx()).as_slice(),
                    );
            }
            result
        }

        fn composite(&self, s: i32) -> CompositeData<S::Algebra> {
            let p = self.prime();
            // This is -1 mod p^2
            let neg_1 = p * p - 1;

            vec![
                (
                    neg_1,
                    self.underlying.left().get_map(s),
                    self.underlying
                        .right()
                        .get_map(s - self.left.underlying().shift.s()),
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
            assert!(Arc::ptr_eq(&underlying.left(), &left.underlying()));
            assert!(Arc::ptr_eq(&underlying.right(), &right.underlying()));

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
                homotopies: OnceBiVec::new(underlying.shift().s()),
                underlying,
                intermediates: DashMap::new(),
            }
        }
    }
}
