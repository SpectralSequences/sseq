pub mod bidegree;
pub mod element;
pub mod generator;
pub mod range;

pub use bidegree::Bidegree;
pub use element::BidegreeElement;
pub use generator::BidegreeGenerator;
pub use range::BidegreeRange;

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
/// This uses [`rayon`] under the hood, and `f` should feel free to use further rayon parallelism.
///
/// # Arguments:
///  - `max_s`: This is exclusive
///  - `max_t`: This is exclusive
#[cfg(feature = "concurrent")]
pub fn iter_s_t<T: Sync>(
    f: &(impl Fn(Bidegree) -> std::ops::Range<i32> + Sync),
    min: Bidegree,
    max: BidegreeRange<T>,
) {
    use rayon::prelude::*;

    rayon::scope(|scope| {
        // Rust does not support recursive closures, so we have to pass everything along as
        // arguments.
        fn run<'a, S: Sync>(
            scope: &rayon::Scope<'a>,
            f: &'a (impl Fn(Bidegree) -> std::ops::Range<i32> + Sync + 'a),
            max: BidegreeRange<'a, S>,
            current: Bidegree,
        ) {
            let mut ret = f(current);
            if current.s() + 1 < max.s() {
                ret.start += 1;
                ret.end = std::cmp::min(ret.end + 1, max.t(current.s() + 1));

                if !ret.is_empty() {
                    // We spawn a new scope to avoid recursion, which may blow the stack
                    scope.spawn(move |scope| {
                        ret.into_par_iter()
                            .for_each(|t| run(scope, f, max, Bidegree::s_t(current.s() + 1, t)));
                    });
                }
            }
        }

        rayon::join(
            || {
                (min.t()..max.t(min.s()))
                    .into_par_iter()
                    .for_each(|t| run(scope, f, max, Bidegree::s_t(min.s(), t)))
            },
            || {
                (min.s() + 1..max.s())
                    .into_par_iter()
                    .for_each(|s| run(scope, f, max, Bidegree::s_t(s, min.t())))
            },
        );
    });
}

#[cfg(test)]
mod test {
    use fp::{prime::ValidPrime, vector::FpVector};

    use super::{Bidegree, BidegreeElement, BidegreeGenerator};

    #[test]
    fn test_bidegree_generator_try_from_element() {
        let b = Bidegree::n_s(23, 9);
        let mut vec = FpVector::new(ValidPrime::new(2), 2);
        vec.set_entry(1, 1);
        let h1_pd0 = BidegreeElement::new(b, vec.as_slice());
        assert_eq!(Ok(BidegreeGenerator::new(b, 1)), h1_pd0.try_into());
        vec.set_entry(0, 1);
        let h0_squared_i = BidegreeElement::new(b, vec.as_slice());
        assert_eq!(
            Result::<BidegreeGenerator, ()>::Err(()),
            h0_squared_i.try_into()
        );
    }
}
