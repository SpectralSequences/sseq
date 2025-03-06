use std::{
    fmt::Debug,
    hash::Hash,
    ops::{Deref, DerefMut},
};

use super::Bidegree;

/// A variant of `Bidegree` that has a defined order.
///
/// There is no canonical way to order bidegrees, so we can't have `Bidegree: PartialOrd`. Instead,
/// we wrap a bidegree using this struct and use a marker type `O` to specify the ordering. This is
/// useful if we want to use bidegrees as keys in a `BTreeMap`, iterate through a collection in a
/// specific order, or sort a list so we have faster lookup times.
///
/// Note that `BidegreeOrdering` is not sealed, so users can define their own ordering.
pub struct OrderedBidegree<O> {
    bidegree: Bidegree,
    ordering: std::marker::PhantomData<O>,
}

// We do the derives manually to avoid constraining the `O` parameter.

impl<O> Debug for OrderedBidegree<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.bidegree)
    }
}

impl<O> Copy for OrderedBidegree<O> {}

impl<O> Clone for OrderedBidegree<O> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<O> PartialEq for OrderedBidegree<O> {
    fn eq(&self, other: &Self) -> bool {
        self.bidegree == other.bidegree
    }
}

impl<O> Eq for OrderedBidegree<O> {}

impl<O> Hash for OrderedBidegree<O> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bidegree.hash(state)
    }
}

impl<O> Deref for OrderedBidegree<O> {
    type Target = Bidegree;

    fn deref(&self) -> &Self::Target {
        &self.bidegree
    }
}

impl<O> DerefMut for OrderedBidegree<O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bidegree
    }
}

impl<O: BidegreeOrdering> Ord for OrderedBidegree<O> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        O::cmp(self.bidegree, other.bidegree)
    }
}

impl<O: BidegreeOrdering> PartialOrd for OrderedBidegree<O> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<O> From<Bidegree> for OrderedBidegree<O> {
    fn from(bidegree: Bidegree) -> Self {
        Self {
            bidegree,
            ordering: std::marker::PhantomData,
        }
    }
}

/// A trait for ordering bidegrees.
///
/// This trait is used to specify the ordering of bidegrees in `OrderedBidegree`.
pub trait BidegreeOrdering {
    fn cmp(a: Bidegree, b: Bidegree) -> std::cmp::Ordering;
}

pub struct ByStem;

impl BidegreeOrdering for ByStem {
    fn cmp(a: Bidegree, b: Bidegree) -> std::cmp::Ordering {
        a.n().cmp(&b.n()).then(a.s().cmp(&b.s()))
    }
}

pub struct ByInternalDegree;

impl BidegreeOrdering for ByInternalDegree {
    fn cmp(a: Bidegree, b: Bidegree) -> std::cmp::Ordering {
        a.t().cmp(&b.t()).then(a.s().cmp(&b.s()))
    }
}

pub struct ByHomologicalDegree;

impl BidegreeOrdering for ByHomologicalDegree {
    fn cmp(a: Bidegree, b: Bidegree) -> std::cmp::Ordering {
        a.s().cmp(&b.s()).then(a.t().cmp(&b.t()))
    }
}

pub struct ByReverseHomologicalDegree;

impl BidegreeOrdering for ByReverseHomologicalDegree {
    fn cmp(a: Bidegree, b: Bidegree) -> std::cmp::Ordering {
        a.s().cmp(&b.s()).reverse().then(a.t().cmp(&b.t()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NUM_TEST_BIDEGREES: usize = 9;

    /// Small grid of bidegrees for testing.
    ///
    /// Graphically, it looks like
    /// ```text
    /// 0 1 2
    /// 3 4 5
    /// 6 7 8
    /// ```
    const TEST_BIDEGREES: [Bidegree; NUM_TEST_BIDEGREES] = [
        Bidegree::n_s(-1, 2),
        Bidegree::n_s(0, 2),
        Bidegree::n_s(1, 2),
        Bidegree::n_s(-1, 1),
        Bidegree::n_s(0, 1),
        Bidegree::n_s(1, 1),
        Bidegree::n_s(-1, 0),
        Bidegree::n_s(0, 0),
        Bidegree::n_s(1, 0),
    ];

    fn get_ordered<const N: usize, O: BidegreeOrdering>(
        order: [usize; N],
    ) -> [OrderedBidegree<O>; N] {
        order.map(|idx| TEST_BIDEGREES[idx].into())
    }

    macro_rules! test_ordering {
        ($o:ident, $v:expr) => {
            let mut ordered_bidegrees: [OrderedBidegree<$o>; NUM_TEST_BIDEGREES] =
                get_ordered([0, 1, 2, 3, 4, 5, 6, 7, 8]);
            ordered_bidegrees.sort();
            assert_eq!(ordered_bidegrees, get_ordered($v));
        };
    }

    #[test]
    fn test_stem_ordered() {
        test_ordering!(ByStem, [6, 3, 0, 7, 4, 1, 8, 5, 2]);
    }

    #[test]
    fn test_internal_ordered() {
        test_ordering!(ByInternalDegree, [6, 7, 3, 8, 4, 0, 5, 1, 2]);
    }

    #[test]
    fn test_homological_ordered() {
        test_ordering!(ByHomologicalDegree, [6, 7, 8, 3, 4, 5, 0, 1, 2]);
    }

    #[test]
    fn test_reverse_homological_ordered() {
        test_ordering!(ByReverseHomologicalDegree, [0, 1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
