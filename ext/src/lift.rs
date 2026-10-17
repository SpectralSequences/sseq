//! Batched lifting of many maps through one target complex, one bidegree at a time.
use std::{
    any::Any,
    ops::Range,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

use algebra::module::Module;
use fp::vector::FpVector;
use once::OnceVec;
use sseq::coordinates::Bidegree;

use crate::{chain_complex::ChainComplex, utils::parallel};

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
    /// the step (scatter the results, record generators). `Send` because [`MultiLift`] prepares the
    /// liftables of a bidegree in parallel.
    pub finish: Box<dyn FnOnce(&[FpVector]) + Send + 'a>,
}

/// A [`MultiLift`] sweep stopped because the shared target could not lift at a bidegree.
///
/// [`ChainComplex::apply_quasi_inverse`] returns `false` when the quasi-inverse it needs is not
/// available, e.g. a save directory that is missing quasi-inverses. The sweep then stops scheduling
/// new bidegrees, lets the ones already running finish, and reports the first failure here. The
/// liftables are left extended up to (not including) the failed bidegree and whatever else had
/// already finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiftError {
    /// The output (target) bidegree at which the quasi-inverse was unavailable.
    pub b: Bidegree,
}

impl std::fmt::Display for LiftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to lift at target bidegree (n, s) = ({}, {}): the target's quasi-inverse is \
             unavailable there",
            self.b.n(),
            self.b.s()
        )
    }
}

impl std::error::Error for LiftError {}

/// Extends several [`Liftable`]s that share one target complex, together, in bidegree-major order.
///
/// At each output bidegree the inputs of every participating liftable are gathered and lifted with
/// a single [`ChainComplex::apply_quasi_inverse`], so the target's quasi-inverse there is solved
/// once and shared across all of them rather than recomputed once per liftable.
///
/// Each output bidegree is a rayon task, started once the bidegrees it depends on are done, and its
/// liftables are prepared in parallel. This follows the resolution's own scheduler: every fan-out
/// inside a bidegree is a `ParallelGuard`ed region, and a bidegree task that starts on a thread
/// waiting inside one (i.e. stolen by it) hands itself back to be spawned again instead of running
/// there. Otherwise a cheap bidegree's fan-out could wait
/// for a whole unrelated, expensive bidegree that one of its threads stole.
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
    ///
    /// # Errors
    ///
    /// If the target cannot lift at some bidegree; see [`LiftError`].
    pub fn extend_all(&self) -> Result<(), LiftError> {
        self.extend_bounded(None)
    }

    /// Like [`extend_all`](Self::extend_all), but sweeping only the *output* bidegrees `(s, t)`
    /// with `s <= bound.s()` and `n <= bound.n()`. Use this when the batch's results are read only
    /// up to a known bidegree.
    ///
    /// Note the coordinates: `bound` is in the shared target's bidegrees, whereas the single-map
    /// [`MuResolutionHomomorphism`](crate::resolution_homomorphism::MuResolutionHomomorphism)`::extend_through_stem`
    /// bounds the map's *source*. A liftable of shift `d` swept to output `bound` is therefore
    /// defined up to source `bound + d`.
    ///
    /// # Errors
    ///
    /// If the target cannot lift at some bidegree; see [`LiftError`].
    pub fn extend_through_stem(&self, bound: Bidegree) -> Result<(), LiftError> {
        self.extend_bounded(Some(bound))
    }

    /// Shared driver for [`extend_all`](Self::extend_all) and
    /// [`extend_through_stem`](Self::extend_through_stem). `bound`, when present, caps the swept
    /// output bidegrees to its stem profile (intersected with the target's computed range).
    fn extend_bounded(&self, bound: Option<Bidegree>) -> Result<(), LiftError> {
        if self.liftables.is_empty() {
            return Ok(());
        }
        let mut max_s = self.target.next_homological_degree();
        if let Some(bound) = bound {
            max_s = std::cmp::min(max_s, bound.s() + 1);
        }
        if max_s <= 0 {
            return Ok(());
        }

        // Per-output-row completion frontier, so a finished cell can tell which cells of the next
        // row it unblocks. Cell (s, t) is recorded at index `t - min_t` of row `s`.
        let completion: OnceVec<OnceVec<()>> = OnceVec::new();
        for _ in 0..max_s {
            completion.push(OnceVec::new());
        }

        let sweep = Sweep {
            lift: self,
            completion,
            min_t: self.target.min_degree(),
            max_s,
            bound,
            failure: Failure::default(),
            span: tracing::Span::current(),
        };
        sweep.run();
        sweep.failure.into_result()
    }

    /// Process one output bidegree: gather every participating liftable's inputs, do one batched
    /// lift, then finish each. Returns the newly-contiguous frontier of this output row.
    ///
    /// If the batched lift fails, or another cell has already failed, the cell records the failure
    /// and returns an empty frontier, so nothing that depends on it is scheduled and the sweep
    /// drains instead of waiting on a cell that will never finish.
    fn step_cell(
        &self,
        b: Bidegree,
        completion: &OnceVec<OnceVec<()>>,
        min_t: i32,
        failure: &Failure,
    ) -> Range<i32> {
        if failure.is_set() {
            return b.t()..b.t();
        }
        let p = self.target.prime();

        let mut inputs: Vec<FpVector> = Vec::new();
        let mut finishers: Vec<(Box<dyn FnOnce(&[FpVector]) + Send + '_>, Range<usize>)> =
            Vec::new();

        // Each `prepare` reads only shared data and returns its own pending state, so the
        // liftables are independent; collecting them in order keeps the batch deterministic.
        let requests =
            parallel::guarded_map(self.liftables.len(), |i| self.liftables[i].prepare(b));
        for mut req in requests.into_iter().flatten() {
            let start = inputs.len();
            inputs.append(&mut req.inputs);
            finishers.push((req.finish, start..inputs.len()));
        }

        // Every liftable at output `b` lifts through the same quasi-inverse, so all results have
        // the target's module dimension at `b`.
        let fx_dim = self.target.module(b.s()).dimension(b.t());
        let mut results = vec![FpVector::new(p, fx_dim); inputs.len()];
        if !inputs.is_empty() && !self.target.apply_quasi_inverse(&mut results, b, &inputs) {
            // Drop the finishers unrun: finishing with unlifted (zero) results would record wrong
            // values as if they were computed.
            failure.set(LiftError { b });
            return b.t()..b.t();
        }
        // Run every finisher, even when a liftable contributed no inputs (a zero-dimensional step):
        // its finish still has to register/extend the step so later reads of that bidegree see it.
        // A no-input finisher receives an empty `results` slice.
        for (finish, range) in finishers {
            finish(&results[range]);
        }

        // `completion` tracks the contiguous frontier of finished degrees per output row (s-value).
        // `push_ooo()` marks position `(b.t() - min_t)` as done and returns the range of positions
        // that this made newly contiguous from 0 (empty if a gap remains below). Convert from
        // relative (0-indexed, relative to min_t) to absolute coordinates.
        let frontier = completion[b.s() as usize].push_ooo((), (b.t() - min_t) as usize);
        (frontier.start as i32 + min_t)..(frontier.end as i32 + min_t)
    }
}

/// One [`MultiLift`] sweep over the output plane, with every cell a rayon task.
///
/// Cell `(s, t)` depends on cells `(s - 1, t')` for all `t' < t` (a lift at `(s, t)` reads the
/// liftables' values one row down, in lower degrees). So the cells of row 0 and the first cell of
/// every row start at once, and a finished cell starts the cells of the next row that its row's
/// newly contiguous frontier unblocks.
struct Sweep<'a, CC> {
    lift: &'a MultiLift<CC>,
    completion: OnceVec<OnceVec<()>>,
    min_t: i32,
    /// Rows `0..max_s` are swept.
    max_s: i32,
    bound: Option<Bidegree>,
    failure: Failure,
    span: tracing::Span,
}

/// A cell that started on a thread inside a guarded fan-out, sent back to the sweep's own thread to
/// be spawned again, with the sender it should keep using.
struct Retry {
    b: Bidegree,
    sender: mpsc::Sender<Self>,
}

impl<CC: ChainComplex + Sync> Sweep<'_, CC> {
    /// One past the last `t` swept in row `s`.
    fn max_t(&self, s: i32) -> i32 {
        let mut t = self.lift.target.module(s).max_computed_degree() + 1;
        if let Some(bound) = self.bound {
            // Stem profile `n <= bound.n()`, i.e. `t <= bound.n() + s`, exclusive upper bound.
            t = std::cmp::min(t, bound.n() + s + 1);
        }
        t
    }

    fn run(&self) {
        maybe_rayon::in_place_scope(|scope| {
            let (sender, receiver) = mpsc::channel();
            for t in (self.min_t..self.max_t(0)).rev() {
                self.spawn(scope, Bidegree::s_t(0, t), sender.clone());
            }
            for s in 1..self.max_s {
                if self.min_t < self.max_t(s) {
                    self.spawn(scope, Bidegree::s_t(s, self.min_t), sender.clone());
                }
            }
            // Every cell task holds a sender, so this loop ends once no cell is left to run.
            drop(sender);
            while let Ok(Retry { b, sender }) = receiver.recv() {
                self.spawn(scope, b, sender);
            }
        });
    }

    fn spawn<'s>(
        &'s self,
        scope: &maybe_rayon::Scope<'s>,
        b: Bidegree,
        sender: mpsc::Sender<Retry>,
    ) {
        scope.spawn(move |scope| {
            let _span = self.span.enter();
            // This thread stole the cell while waiting inside a fan-out of another cell. Running it
            // here would hold that fan-out up until this whole cell is done, so hand it back to be
            // spawned again from outside: an idle thread will pick it up.
            if cfg!(feature = "concurrent") && parallel::is_in_parallel() {
                sender
                    .send(Retry {
                        b,
                        sender: sender.clone(),
                    })
                    .unwrap();
                return;
            }
            let frontier = self
                .lift
                .step_cell(b, &self.completion, self.min_t, &self.failure);

            // Cells `(s + 1, t')` need row `s` done below `t'`; the newly contiguous frontier
            // `[start, end)` of row `s` unblocks `t'` in `start + 1..=end`. Spawn them highest
            // first, so the lowest (most depended-on) runs first on this thread.
            let next_s = b.s() + 1;
            if next_s < self.max_s {
                let end = std::cmp::min(frontier.end + 1, self.max_t(next_s));
                for t in (frontier.start + 1..end).rev() {
                    self.spawn(scope, Bidegree::s_t(next_s, t), sender.clone());
                }
            }
        });
    }
}

/// The first [`LiftError`] of a sweep, plus a flag that tells the remaining cells to stop.
#[derive(Default)]
struct Failure {
    stop: AtomicBool,
    first: OnceLock<LiftError>,
}

impl Failure {
    fn is_set(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    fn set(&self, error: LiftError) {
        // Concurrent failures race for `first`; any of them is a correct report.
        let _ = self.first.set(error);
        self.stop.store(true, Ordering::Relaxed);
    }

    fn into_result(self) -> Result<(), LiftError> {
        match self.first.into_inner() {
            Some(error) => Err(error),
            None => Ok(()),
        }
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

    /// A target that cannot lift at some bidegree (here, a Nassau save with quasi-inverses deleted)
    /// makes the sweep return an error naming that bidegree, instead of panicking on a worker and
    /// leaving the caller waiting.
    #[test]
    fn missing_quasi_inverse_is_an_error() {
        let tempdir = tempfile::TempDir::new().unwrap();
        let res = Arc::new(
            crate::utils::construct_nassau("S_2", Some(tempdir.path().to_owned())).unwrap(),
        );
        res.compute_through_stem(Bidegree::n_s(12, 6));

        // Delete every saved quasi-inverse at `s = 2`.
        let qi_dir = tempdir
            .path()
            .join(format!("{}s", crate::save::SaveKind::NassauQi.name()));
        let mut deleted = 0;
        for entry in std::fs::read_dir(&qi_dir).unwrap() {
            let path = entry.unwrap().path();
            if path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("2_")
            {
                std::fs::remove_file(path).unwrap();
                deleted += 1;
            }
        }
        assert!(deleted > 0);

        let h0 = Arc::new(ResolutionHomomorphism::from_class(
            String::new(),
            Arc::clone(&res),
            Arc::clone(&res),
            Bidegree::n_s(0, 1),
            &[1],
        ));
        let error = MultiLift::new(res, vec![h0 as Arc<dyn Liftable>])
            .extend_all()
            .unwrap_err();
        assert_eq!(error.b.s(), 2, "{error}");
        assert!(error.to_string().contains("failed to lift"));
    }
}
