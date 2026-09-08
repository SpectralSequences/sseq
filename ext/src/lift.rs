//! Batched lifting of many maps through one target complex, one bidegree at a time.
use std::{any::Any, ops::Range, sync::Arc};

use algebra::module::Module;
use fp::vector::FpVector;
use once::OnceVec;
use sseq::coordinates::{Bidegree, BidegreeRange, iter_s_t};

use crate::chain_complex::ChainComplex;

/// Outcome of the `prepare` half of a lift step, generic over the pending state each implementor
/// carries between `prepare` and `finish`.
///
/// The split exists so that [`MultiLift`] can gather the vectors of many steps at a bidegree before
/// any of them is solved; a single-map caller matches on this immediately and does its own solve.
pub enum LiftPrep<P> {
    /// The step needed no quasi-inverse and is already finished; carries the range of
    /// newly-contiguous input degrees.
    Done(Range<i32>),
    /// The step needs a quasi-inverse solve; complete it with the implementor's `finish`.
    NeedsLift(P),
}

/// Something built by a sequence of quasi-inverse solves against a common target complex, one
/// target bidegree at a time.
///
/// Implemented by chain-map extension
/// ([`MuResolutionHomomorphism`](crate::resolution_homomorphism::MuResolutionHomomorphism)), the
/// chain-homotopy lifts of the Massey machinery
/// ([`ChainHomotopy`](crate::chain_complex::ChainHomotopy)), and the secondary lifts (via
/// [`batch_extend_secondary`](crate::secondary::batch_extend_secondary)). They all lift through the
/// *same* target quasi-inverse at a given bidegree, so [`MultiLift`] can gather a batch across
/// implementors of different kinds and solve it once.
///
/// The interface is deliberately free of the target/source type parameters: `prepare` returns plain
/// vectors to lift plus a boxed continuation, so the driver never needs to name a liftable's
/// internal state.
pub trait Liftable: Sync + Send {
    /// Prepare the lift at target bidegree `b`.
    ///
    /// Returns `None` if this liftable has no quasi-inverse work at `b` — out of range, already
    /// computed, or a step (augmentation, zero-dimensional) it finished itself. Otherwise returns
    /// the vectors to lift at `b` and a continuation that finishes the step once their lifts are
    /// known.
    fn prepare(&self, b: Bidegree) -> Option<LiftRequest<'_>>;

    /// Whether this lifts through `target`, so [`MultiLift::new`] can check that every participant
    /// shares one. Implementors compare it against their own target by identity, not by value: two
    /// equal-looking complexes have unrelated quasi-inverse stores.
    fn lifts_through(&self, target: &dyn Any) -> bool;
}

/// The inputs to lift at one bidegree together with a continuation to finish the step; see
/// [`Liftable::prepare`].
pub struct LiftRequest<'a> {
    /// Vectors to lift, i.e. the `inputs` passed to `apply_quasi_inverse` at this bidegree.
    pub inputs: Vec<FpVector>,
    /// Called exactly once with the lifted `results` (`results[i]` lifts `inputs[i]`) to complete
    /// the step (scatter the results, record generators).
    pub finish: Box<dyn FnOnce(&[FpVector]) + 'a>,
}

/// Extends several [`Liftable`]s that share one target complex, together, in bidegree-major order.
///
/// At each output bidegree the inputs of every participating liftable are gathered and lifted with
/// a single [`ChainComplex::apply_quasi_inverse`], so the target's quasi-inverse there is solved
/// once and shared across all of them rather than recomputed once per liftable. Concurrency across
/// the plane is provided by [`iter_s_t`], exactly as for a single map.
///
/// Single-map callers do not need this — one map hits each bidegree once, so it already recomputes
/// each quasi-inverse once. Those should use
/// [`MuResolutionHomomorphism`](crate::resolution_homomorphism::MuResolutionHomomorphism)`::extend_all`.
pub struct MultiLift<CC> {
    target: Arc<CC>,
    liftables: Vec<Arc<dyn Liftable>>,
}

impl<CC: ChainComplex + Sync> MultiLift<CC> {
    /// Build a driver over `liftables`.
    ///
    /// # Panics
    ///
    /// If any liftable lifts through a complex other than `target`. Batching them would solve their
    /// steps against the wrong quasi-inverse, silently producing invalid lifts.
    pub fn new(target: Arc<CC>, liftables: Vec<Arc<dyn Liftable>>) -> Self
    where
        CC: 'static,
    {
        for liftable in &liftables {
            assert!(
                liftable.lifts_through(&*target),
                "every liftable in a MultiLift must lift through the same target complex"
            );
        }
        Self { target, liftables }
    }

    /// Extend every liftable as far as the shared target is resolved, batching the quasi-inverse
    /// solve at each output bidegree.
    pub fn extend_all(&self) {
        self.extend_bounded(None);
    }

    /// Like [`extend_all`](Self::extend_all), but sweeping only the *output* bidegrees `(s, t)`
    /// with `s <= bound.s()` and `n <= bound.n()`. Use this when the batch's results are read only
    /// up to a known bidegree.
    ///
    /// Note the coordinates: `bound` is in the shared target's bidegrees, whereas the single-map
    /// [`MuResolutionHomomorphism`](crate::resolution_homomorphism::MuResolutionHomomorphism)`::extend_through_stem`
    /// bounds the map's *source*. A liftable of shift `d` swept to output `bound` is therefore
    /// defined up to source `bound + d`.
    pub fn extend_through_stem(&self, bound: Bidegree) {
        self.extend_bounded(Some(bound));
    }

    /// Shared driver for [`extend_all`](Self::extend_all) and
    /// [`extend_through_stem`](Self::extend_through_stem). `bound`, when present, caps the swept
    /// output bidegrees to its stem profile (intersected with the target's computed range).
    fn extend_bounded(&self, bound: Option<Bidegree>) {
        if self.liftables.is_empty() {
            return;
        }
        let mut max_s = self.target.next_homological_degree();
        if let Some(bound) = bound {
            max_s = std::cmp::min(max_s, bound.s() + 1);
        }
        if max_s <= 0 {
            return;
        }
        let min_t = self.target.min_degree();
        let min = Bidegree::s_t(0, min_t);

        // Per-output-row completion frontier, so `iter_s_t` can tell how far each row is done. Cell
        // (s, t) is recorded at index `t - min_t` of row `s`; `push_ooo` returns the contiguous
        // frontier that `iter_s_t` expects.
        let completion: OnceVec<OnceVec<()>> = OnceVec::new();
        for _ in 0..max_s {
            completion.push(OnceVec::new());
        }

        let max_t = move |slf: &Self, s: i32| {
            let mut t = slf.target.module(s).max_computed_degree() + 1;
            if let Some(bound) = bound {
                // Stem profile `n <= bound.n()`, i.e. `t <= bound.n() + s`, exclusive upper bound.
                t = std::cmp::min(t, bound.n() + s + 1);
            }
            t
        };
        let max = BidegreeRange::new(self, max_s, &max_t);

        iter_s_t(&|b| self.step_cell(b, &completion, min_t), min, max);
    }

    /// Process one output bidegree: gather every participating liftable's inputs, do one batched
    /// lift, then finish each. Returns the newly-contiguous frontier of this output row.
    fn step_cell(&self, b: Bidegree, completion: &OnceVec<OnceVec<()>>, min_t: i32) -> Range<i32> {
        let p = self.target.prime();

        let mut inputs: Vec<FpVector> = Vec::new();
        let mut finishers: Vec<(Box<dyn FnOnce(&[FpVector]) + '_>, Range<usize>)> = Vec::new();

        for liftable in &self.liftables {
            if let Some(mut req) = liftable.prepare(b) {
                let start = inputs.len();
                inputs.append(&mut req.inputs);
                finishers.push((req.finish, start..inputs.len()));
            }
        }

        // Every liftable at output `b` lifts through the same quasi-inverse, so all results have
        // the target's module dimension at `b`.
        let fx_dim = self.target.module(b.s()).dimension(b.t());
        let mut results = vec![FpVector::new(p, fx_dim); inputs.len()];
        if !inputs.is_empty() {
            assert!(self.target.apply_quasi_inverse(&mut results, b, &inputs));
        }
        // Run every finisher, even when a liftable contributed no inputs (a zero-dimensional step):
        // its finish still has to register/extend the step so later reads of that bidegree see it.
        // A no-input finisher receives an empty `results` slice.
        for (finish, range) in finishers {
            finish(&results[range]);
        }

        // `completion` tracks the contiguous frontier of finished degrees per output row (s-value).
        // `push_ooo()` marks position `(b.t() - min_t)` as done and returns the maximal contiguous
        // range from 0 up through this position (out-of-order inserts are allowed earlier). Convert
        // from relative (0-indexed, relative to min_t) to absolute coordinates and return it so
        // `iter_s_t` knows how far this row has progressed.
        let frontier = completion[b.s() as usize].push_ooo((), (b.t() - min_t) as usize);
        (frontier.start as i32 + min_t)..(frontier.end as i32 + min_t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{resolution_homomorphism::ResolutionHomomorphism, utils::construct_standard};

    /// Batching lifts through a target other than the one they were built against would solve them
    /// with the wrong quasi-inverse, so [`MultiLift::new`] rejects it up front.
    #[test]
    #[should_panic(expected = "must lift through the same target complex")]
    fn rejects_foreign_target() {
        let res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        let other = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        let bound = Bidegree::n_s(4, 4);
        res.compute_through_stem(bound);
        other.compute_through_stem(bound);

        let hom = Arc::new(ResolutionHomomorphism::from_class(
            String::new(),
            Arc::clone(&res),
            Arc::clone(&res),
            Bidegree::n_s(0, 1),
            &[1],
        ));
        MultiLift::new(other, vec![hom as Arc<dyn Liftable>]);
    }
}
