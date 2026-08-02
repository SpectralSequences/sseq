//! Process-wide arbitration between the two CUDA consumers that share this GPU: the cubecl Milnor
//! multiply (`algebra::algebra::milnor_gpu`) and the `fp-cuda` row reduction ([`crate::blas::cuda`]).
//!
//! # Why this exists
//!
//! The two runtimes have opposite performance shapes. The multiply is *throughput* work: large,
//! long-running kernels that saturate the SMs. The composable (non-cooperative) row reduction is
//! *latency* work: thousands of tiny, strictly sequential per-column relaunches. Run them at the
//! same time and every one of those thousands of launches queues behind a saturating multiply
//! kernel, so a reduction that takes single-digit milliseconds on an unshared GPU takes tens of
//! seconds — measured 1.8–9.7 ms standalone versus 8.6–96.8 s co-running, a ~10 000× loss, with
//! `nvidia-smi` showing 99 % SM and 9 % memory utilisation (queueing, not compute).
//!
//! Being *composable* (no cooperative launch, so no co-residency requirement) means the reduction
//! **can** run alongside other GPU work without deadlocking. It does not mean it should: overlap is
//! precisely what destroys it.
//!
//! # The trade
//!
//! Giving a large reduction brief exclusive use of the device costs almost nothing: a whole
//! stem-200 resolution runs ~440 GPU reductions of ~10 ms each, so the multiply pauses for ~5 s in
//! total. Multiplies still overlap freely with each other — they take the shared side.
//!
//! Writer preference is deliberate. Multiplies are continuous and readers are many; a plain
//! `RwLock` would let the reduction starve indefinitely behind the stream of multiplies, which is
//! the failure this lock exists to prevent.

use std::{
    sync::{Condvar, Mutex},
    time::{Duration, Instant},
};

#[derive(Default)]
struct State {
    /// Multiplies currently submitting.
    readers: usize,
    /// A reduction currently holds the device.
    writer: bool,
    /// Reductions blocked waiting; new multiplies yield to them (writer preference).
    writers_waiting: usize,
}

/// Whether the two CUDA runtimes share one device. Arbitration is only needed when they do:
/// measured at ~47% of multiply time (`[batch-stats] lock=`), which is pure waste once the row
/// reduction has its own GPU. Defaults to `true` (single-device, the safe assumption) until
/// [`crate::blas::cuda`] resolves the device ids on first GPU use.
static SHARED_DEVICE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Record whether the multiply and the row reduction target the same GPU.
pub fn set_devices_shared(shared: bool) {
    SHARED_DEVICE.store(shared, std::sync::atomic::Ordering::Relaxed);
}

fn arbitration_needed() -> bool {
    SHARED_DEVICE.load(std::sync::atomic::Ordering::Relaxed)
}

fn state() -> &'static (Mutex<State>, Condvar) {
    use std::sync::OnceLock;
    static STATE: OnceLock<(Mutex<State>, Condvar)> = OnceLock::new();
    STATE.get_or_init(|| (Mutex::new(State::default()), Condvar::new()))
}

/// Shared access, held by the Milnor multiply while it submits and reads back. Many may be held at
/// once; all are excluded by an [`exclusive`] holder.
pub struct SharedGuard(());

/// Exclusive access, held by a large GPU row reduction for the duration of its launch chain.
pub struct ExclusiveGuard(());

impl Drop for SharedGuard {
    fn drop(&mut self) {
        if !arbitration_needed() {
            return;
        }
        let (lock, cv) = state();
        let mut s = lock.lock().unwrap_or_else(|e| e.into_inner());
        s.readers -= 1;
        if s.readers == 0 {
            cv.notify_all();
        }
    }
}

impl Drop for ExclusiveGuard {
    fn drop(&mut self) {
        if !arbitration_needed() {
            return;
        }
        let (lock, cv) = state();
        let mut s = lock.lock().unwrap_or_else(|e| e.into_inner());
        s.writer = false;
        cv.notify_all();
    }
}

/// How long a multiply defers to a waiting reduction before going ahead anyway.
///
/// This is a **safety valve, not the mechanism**. It must exceed the time a reduction holds the
/// device, or exclusivity evaporates precisely when it matters: at 25 ms, multiplies barged back in
/// partway through every multi-second reduction, which both kept reductions ~1000× slow and put the
/// overlap back that crashes the run. Correctness against deadlock comes from *where* the shared
/// guard is taken (`milnor_gpu.rs`, past every rayon section), not from this timeout firing.
const SHARED_YIELD: Duration = Duration::from_secs(60);
/// How long a reduction waits for in-flight multiplies to drain before going ahead anyway.
const EXCLUSIVE_DRAIN: Duration = Duration::from_secs(10);

/// Acquire shared (multiply) access, briefly yielding to any waiting reduction.
///
/// The yield is **bounded**, and that bound is load-bearing rather than a tuning choice. Callers
/// reach this from inside rayon parallel sections: a multiply that blocks here can be holding a
/// join that another worker's stolen multiply needs, so an unbounded yield deadlocks (observed on
/// H200 — a reduction waiting on `readers == 0` while every reader waited on a join that could only
/// finish once a blocked reader proceeded). Timing out costs the reduction some exclusivity; never
/// timing out costs the whole resolution.
pub fn shared() -> SharedGuard {
    if !arbitration_needed() {
        return SharedGuard(());
    }
    let (lock, cv) = state();
    let mut s = lock.lock().unwrap_or_else(|e| e.into_inner());
    let deadline = Instant::now() + SHARED_YIELD;
    while s.writer || s.writers_waiting > 0 {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        s = cv
            .wait_timeout(s, remaining)
            .unwrap_or_else(|e| e.into_inner())
            .0;
    }
    s.readers += 1;
    SharedGuard(())
}

/// Acquire exclusive (row-reduction) access, waiting for in-flight multiplies to drain.
///
/// Waiting out another *reduction* is unbounded and safe: reductions never block on rayon work, so
/// they always finish in finite time, and letting two run at once is what the concurrency cap was
/// added to prevent. Waiting for *multiplies* to drain is bounded for the reason in [`shared`] —
/// past the deadline this proceeds without full exclusivity, which is slow, not wrong.
pub fn exclusive() -> ExclusiveGuard {
    if !arbitration_needed() {
        return ExclusiveGuard(());
    }
    let (lock, cv) = state();
    let mut s = lock.lock().unwrap_or_else(|e| e.into_inner());
    s.writers_waiting += 1;
    while s.writer {
        s = cv.wait(s).unwrap_or_else(|e| e.into_inner());
    }
    // Claim the slot *before* draining readers. Both waits below release the mutex, so a writer
    // that only set this flag afterwards could race another writer through the check above and let
    // two reductions run at once (caught by the test in this module).
    s.writer = true;
    s.writers_waiting -= 1;
    let deadline = Instant::now() + EXCLUSIVE_DRAIN;
    while s.readers > 0 {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        s = cv
            .wait_timeout(s, remaining)
            .unwrap_or_else(|e| e.into_inner())
            .0;
    }
    ExclusiveGuard(())
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        thread,
    };

    use super::*;

    /// What the arbitration actually guarantees: every acquisition terminates under heavy
    /// contention (no deadlock — the property the first version got wrong), two reductions never
    /// overlap, and multiplies still overlap each other. Exclusion against multiplies is
    /// deliberately best-effort (see [`EXCLUSIVE_DRAIN`]), so it is not asserted here.
    #[test]
    fn contended_acquisition_terminates_and_writers_are_exclusive() {
        let live_shared = Arc::new(AtomicUsize::new(0));
        let live_exclusive = Arc::new(AtomicUsize::new(0));
        let violations = Arc::new(AtomicUsize::new(0));
        let max_shared = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..8 {
            let (live, bad, max) = (
                Arc::clone(&live_shared),
                Arc::clone(&violations),
                Arc::clone(&max_shared),
            );
            handles.push(thread::spawn(move || {
                for _ in 0..200 {
                    let _g = shared();
                    let n = live.fetch_add(1, Ordering::SeqCst) + 1;
                    max.fetch_max(n, Ordering::SeqCst);
                    thread::yield_now();
                    live.fetch_sub(1, Ordering::SeqCst);
                    let _ = &bad;
                }
            }));
        }
        for _ in 0..3 {
            let (live_w, bad) = (Arc::clone(&live_exclusive), Arc::clone(&violations));
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let _g = exclusive();
                    if live_w.fetch_add(1, Ordering::SeqCst) != 0 {
                        bad.fetch_add(1, Ordering::SeqCst);
                    }
                    thread::yield_now();
                    live_w.fetch_sub(1, Ordering::SeqCst);
                }
            }));
        }
        // Joining at all is the deadlock assertion: the previous design hung here forever.
        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(
            violations.load(Ordering::SeqCst),
            0,
            "two reductions held the device at once"
        );
        assert!(
            max_shared.load(Ordering::SeqCst) > 1,
            "multiplies never overlapped — the shared side is serialising, which defeats the point"
        );
    }
}
