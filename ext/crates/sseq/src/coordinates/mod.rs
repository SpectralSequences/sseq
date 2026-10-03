pub use degree::MultiDegree;
pub use element::MultiDegreeElement;
pub use generator::MultiDegreeGenerator;
use maybe_rayon::prelude::*;
use ordered::OrderedMultiDegree;
pub use range::BidegreeRange;

pub mod degree;
pub mod element;
pub mod generator;
pub mod ordered;
pub mod range;

pub type Bidegree = MultiDegree<2>;
pub type BidegreeElement = MultiDegreeElement<2>;
pub type BidegreeGenerator = MultiDegreeGenerator<2>;
pub type OrderedBidegree<O> = OrderedMultiDegree<2, O>;

impl Bidegree {
    pub const fn n_s(n: i32, s: i32) -> Self {
        Self::new([n, s])
    }

    pub const fn s_t(s: i32, t: i32) -> Self {
        Self::n_s(t - s, s)
    }

    pub const fn x_y(x: i32, y: i32) -> Self {
        Self::n_s(x, y)
    }
}

impl BidegreeGenerator {
    pub fn s_t(s: i32, t: i32, idx: usize) -> Self {
        Self::new(Bidegree::s_t(s, t), idx)
    }

    pub fn n_s(n: i32, s: i32, idx: usize) -> Self {
        Self::new(Bidegree::n_s(n, s), idx)
    }
}

/// Execute a function on a range of bidegrees, possibly in parallel.
///
/// Given a function `f(s, t)`, compute it for every `s` in `[min_s, max_s]` and every `t` in
/// `[min_t, max_t(s)]`.  Further, we only compute `f(s, t)` when `f(s - 1, t')` has been computed
/// for all `t' < t`.
///
/// The function `f` should return a range starting from t and ending at the largest `T` such that
/// `f(s, t')` has already been computed for every `t' < T`.
///
/// While `iter_s_t` could have had kept track of that data, it is usually the case that `f` would
/// compute something and write it to a `OnceBiVec`, and
/// [`OnceBiVec::push_ooo`](once::OnceBiVec::push_ooo) would return this range for us.
///
/// This uses [`maybe_rayon`] under the hood, and `f` should feel free to use further parallelism.
///
/// # Arguments:
///  - `max_s`: This is exclusive
///  - `max_t`: This is exclusive
pub fn iter_s_t<T: Sync>(
    f: &(impl Fn(Bidegree) -> std::ops::Range<i32> + Sync),
    min: Bidegree,
    max: BidegreeRange<T>,
) {
    iter_s_t_with_lag(f, min, max, 1);
}

/// [`iter_s_t`] for a complex whose dependency at `(s, t)` reaches `(s - 1, t)` at the **same**
/// `t`, rather than only `t' < t`.
///
/// [`iter_s_t`] runs `f(s, t)` once `f(s - 1, t')` is done for every `t' < t`. That is exactly
/// right for a **minimal** resolution: `d` lands in $\bar A \cdot P_{s-1}$, so the coefficient of
/// every component has positive degree and `f(s, t)` only ever reads generators of degree *below*
/// `t`. A **non-minimal** differential has an identity (degree-zero) component, so `f(s, t)` can
/// read the generator of degree exactly `t` in filtration `s - 1`, and needs `f(s - 1, t)` itself.
///
/// That is a shift of the wavefront by one column, *not* a reason to serialise filtrations. This
/// driver is [`iter_s_t`] with the recursion advanced one step less: having finished `f(s, t)`, it
/// releases `(s + 1, t')` for `t' < T` rather than `t' < T + 1`, where `T` is the extent of the
/// contiguous computed prefix of row `s` reported by `f`. Filtrations still overlap in time — row
/// `s + 1` works on low `t` while row `s` is still climbing — so the available parallelism is
/// [`iter_s_t`]'s, minus one diagonal.
///
/// The bottom row is the only seed. [`iter_s_t`] additionally seeds `(s, min_t)` for every `s` at
/// once, which is sound only because `f(s, min_t)`'s dependency is vacuous there; here it is not —
/// it is `f(s - 1, min_t)` — so those must be reached through the recursion.
///
/// # Arguments (matching [`iter_s_t`]):
///  - `max.s()`: exclusive
///  - `max.t(s)`: exclusive
pub fn iter_s_t_inclusive<T: Sync>(
    f: &(impl Fn(Bidegree) -> std::ops::Range<i32> + Sync),
    min: Bidegree,
    max: BidegreeRange<T>,
) {
    iter_s_t_with_lag(f, min, max, 0);
}

/// The wavefront shared by [`iter_s_t`] and [`iter_s_t_inclusive`].
///
/// `lag` is how far filtration `s + 1` may run ahead of the contiguous computed prefix of
/// filtration `s`, and so encodes the dependency the caller has: `1` when `f(s, t)` reads only
/// `f(s - 1, t')` for `t' < t`, `0` when it also reads `t' = t`.
///
/// It also settles the seeding, which is not a free choice. With `lag == 1`, `f(s, min_t)` has a
/// vacuous dependency, so every filtration can be seeded at `min_t` simultaneously; with
/// `lag == 0` it depends on `f(s - 1, min_t)`, so only the bottom filtration may be seeded and the
/// rest must be reached through the recursion.
fn iter_s_t_with_lag<T: Sync>(
    f: &(impl Fn(Bidegree) -> std::ops::Range<i32> + Sync),
    min: Bidegree,
    max: BidegreeRange<T>,
    lag: i32,
) {
    // Track `tracing` spans correctly
    let tracing_span = tracing::Span::current();
    let f = &|b| {
        let _tracing_guard = tracing_span.enter();
        f(b)
    };

    maybe_rayon::scope(|scope| {
        // Rust does not support recursive closures, so we have to pass everything along as
        // arguments.
        fn run<'a, S: Sync>(
            scope: &maybe_rayon::Scope<'a>,
            f: &'a (impl Fn(Bidegree) -> std::ops::Range<i32> + Sync + 'a),
            max: BidegreeRange<'a, S>,
            current: Bidegree,
            lag: i32,
        ) {
            let mut ret = f(current);
            if current.s() + 1 < max.s() {
                // `ret` is `[t, T)` with `f(current.s(), t')` computed for every `t' < T`, so
                // filtration `s + 1` is released up to `T - 1 + lag`.
                ret.start += lag;
                ret.end = std::cmp::min(ret.end + lag, max.t(current.s() + 1));

                if !ret.is_empty() {
                    // We spawn a new scope to avoid recursion, which may blow the stack
                    scope.spawn(move |scope| {
                        ret.into_maybe_par_iter().for_each(|t| {
                            run(scope, f, max, Bidegree::s_t(current.s() + 1, t), lag)
                        });
                    });
                }
            }
        }

        let seed_bottom_row = || {
            (min.t()..max.t(min.s()))
                .into_maybe_par_iter()
                .for_each(|t| run(scope, f, max, Bidegree::s_t(min.s(), t), lag))
        };

        if lag > 0 {
            maybe_rayon::join(seed_bottom_row, || {
                (min.s() + 1..max.s())
                    .into_maybe_par_iter()
                    .for_each(|s| run(scope, f, max, Bidegree::s_t(s, min.t()), lag))
            });
        } else if min.s() < max.s() {
            // `max.s()` is exclusive, so an empty filtration range visits nothing at all.
            seed_bottom_row();
        }
    });
}

#[cfg(test)]
mod tests {
    use fp::{prime::ValidPrime, vector::FpVector};

    use super::{Bidegree, BidegreeElement, BidegreeGenerator, BidegreeRange};

    #[test]
    fn test_bidegree_generator_try_from_element() {
        let b = Bidegree::n_s(23, 9);
        let mut vec = FpVector::new(ValidPrime::new(2), 2);
        vec.set_entry(1, 1);
        let h1_pd0 = BidegreeElement::new(b, vec.clone());
        assert_eq!(Ok(BidegreeGenerator::new(b, 1)), h1_pd0.try_into());
        vec.set_entry(0, 1);
        let h0_squared_i = BidegreeElement::new(b, vec.clone());
        assert_eq!(
            Result::<BidegreeGenerator, ()>::Err(()),
            h0_squared_i.try_into()
        );
    }

    /// Run [`super::iter_s_t_inclusive`] over a rectangle, checking its contract *as it goes*:
    /// before `f(s, t)` runs, `f(s - 1, t')` must already have completed for every `t' ≤ t`. The
    /// check has to happen inside `f` rather than on a recorded order — under `concurrent` the
    /// bidegrees genuinely overlap in time, so a completion list says nothing about happens-before.
    ///
    /// `f` reports the contiguous computed prefix of its own row, as the real callers do via
    /// `OnceBiVec::push_ooo`. Returns every bidegree visited.
    fn run_checked(
        min: Bidegree,
        max_s: i32,
        max_t: &(dyn Fn(&(), i32) -> i32 + Sync),
    ) -> Vec<(i32, i32)> {
        use std::collections::{BTreeSet, HashMap};

        let done: std::sync::Mutex<HashMap<i32, BTreeSet<i32>>> =
            std::sync::Mutex::new(HashMap::new());
        super::iter_s_t_inclusive(
            &|b| {
                let mut done = done.lock().unwrap();
                if b.s() > min.s() {
                    let below = done.get(&(b.s() - 1)).cloned().unwrap_or_default();
                    for t in min.t()..=b.t() {
                        assert!(
                            below.contains(&t),
                            "({}, {}) ran before ({}, {})",
                            b.s(),
                            b.t(),
                            b.s() - 1,
                            t
                        );
                    }
                }
                let row = done.entry(b.s()).or_default();
                assert!(row.insert(b.t()), "({}, {}) ran twice", b.s(), b.t());
                // The contiguous prefix of this row, i.e. what `push_ooo` would report.
                let mut end = min.t();
                while row.contains(&end) {
                    end += 1;
                }
                b.t()..end
            },
            min,
            BidegreeRange::new(&(), max_s, max_t),
        );

        let done = done.into_inner().unwrap();
        let mut visited: Vec<(i32, i32)> = done
            .iter()
            .flat_map(|(s, ts)| ts.iter().map(move |t| (*s, *t)))
            .collect();
        visited.sort_unstable();
        visited
    }

    #[test]
    fn iter_s_t_inclusive_covers_the_rectangle_and_respects_the_same_t_dependency() {
        let visited = run_checked(Bidegree::s_t(0, 0), 4, &|(), _| 3);
        let expected: Vec<(i32, i32)> = (0..4).flat_map(|s| (0..3).map(move |t| (s, t))).collect();
        assert_eq!(visited, expected);
    }

    #[test]
    fn iter_s_t_inclusive_handles_degenerate_ranges() {
        // Empty `s` range, and an empty `t` range for every `s`.
        assert!(run_checked(Bidegree::s_t(2, 0), 2, &|(), _| 5).is_empty());
        assert!(run_checked(Bidegree::s_t(0, 0), 3, &|(), _| 0).is_empty());

        // A shrinking `t` bound is honoured per row, exactly as `iter_s_t` clamps it.
        let visited = run_checked(Bidegree::s_t(0, 1), 4, &|(), s| 4 - s);
        let mut per_row = [0usize; 4];
        for (s, t) in &visited {
            assert!(*t >= 1 && *t < 4 - *s, "({s}, {t}) out of range");
            per_row[*s as usize] += 1;
        }
        assert_eq!(per_row, [3, 2, 1, 0]);
    }
}
