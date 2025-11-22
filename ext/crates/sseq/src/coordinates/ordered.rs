use std::{
    fmt::Debug,
    hash::Hash,
    ops::{Deref, DerefMut},
};

use super::MultiDegree;

/// A variant of `MultiDegree` that has a defined order.
///
/// There is no canonical way to order gradings with at least two dimensions, so we can't have
/// `MultiDegree: PartialOrd`. Instead, we wrap a multidegree using this struct and use a marker
/// type `O` to specify the ordering. This is useful if we want to use multidegrees as keys in a
/// `BTreeMap`, iterate through a collection in a specific order, or sort a list so we have faster
/// lookup times.
///
/// Note that `MultiDegreeOrdering` is not sealed, so users can define their own ordering.
pub struct OrderedMultiDegree<const N: usize, O> {
    degree: MultiDegree<N>,
    ordering: std::marker::PhantomData<O>,
}

// We do the derives manually to avoid constraining the `O` parameter.

impl<const N: usize, O> Debug for OrderedMultiDegree<N, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.degree)
    }
}

impl<const N: usize, O> Copy for OrderedMultiDegree<N, O> {}

impl<const N: usize, O> Clone for OrderedMultiDegree<N, O> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const N: usize, O> PartialEq for OrderedMultiDegree<N, O> {
    fn eq(&self, other: &Self) -> bool {
        self.degree == other.degree
    }
}

impl<const N: usize, O> Eq for OrderedMultiDegree<N, O> {}

impl<const N: usize, O> Hash for OrderedMultiDegree<N, O> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.degree.hash(state)
    }
}

impl<const N: usize, O> Deref for OrderedMultiDegree<N, O> {
    type Target = MultiDegree<N>;

    fn deref(&self) -> &Self::Target {
        &self.degree
    }
}

impl<const N: usize, O> DerefMut for OrderedMultiDegree<N, O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.degree
    }
}

impl<const N: usize, O: MultiDegreeOrdering<N>> Ord for OrderedMultiDegree<N, O> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        O::cmp(self.degree, other.degree)
    }
}

impl<const N: usize, O: MultiDegreeOrdering<N>> PartialOrd for OrderedMultiDegree<N, O> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<const N: usize, O> From<MultiDegree<N>> for OrderedMultiDegree<N, O> {
    fn from(degree: MultiDegree<N>) -> Self {
        Self {
            degree,
            ordering: std::marker::PhantomData,
        }
    }
}

/// A trait for ordering multidegrees.
///
/// This trait is used to specify the ordering of multidegrees in `OrderedMultiDegree`.
pub trait MultiDegreeOrdering<const N: usize> {
    fn cmp(a: MultiDegree<N>, b: MultiDegree<N>) -> std::cmp::Ordering;
}

pub struct ByStem;

impl<const N: usize> MultiDegreeOrdering<N> for ByStem {
    fn cmp(a: MultiDegree<N>, b: MultiDegree<N>) -> std::cmp::Ordering {
        a.coords().cmp(&b.coords())
    }
}

pub struct ByInternalDegree;

impl<const N: usize> MultiDegreeOrdering<N> for ByInternalDegree {
    fn cmp(a: MultiDegree<N>, b: MultiDegree<N>) -> std::cmp::Ordering {
        let mut a_coords = a.coords();
        let mut b_coords = b.coords();
        if N > 1 {
            a_coords[0] += a_coords[1];
            b_coords[0] += b_coords[1];
        }
        a_coords.cmp(&b_coords)
    }
}

pub struct ByHomologicalDegree;

impl<const N: usize> MultiDegreeOrdering<N> for ByHomologicalDegree {
    fn cmp(a: MultiDegree<N>, b: MultiDegree<N>) -> std::cmp::Ordering {
        let mut a_coords = a.coords();
        let mut b_coords = b.coords();
        if N > 1 {
            a_coords.swap(0, 1);
            b_coords.swap(0, 1);
        }
        a_coords.cmp(&b_coords)
    }
}

pub struct ByReverseHomologicalDegree;

impl<const N: usize> MultiDegreeOrdering<N> for ByReverseHomologicalDegree {
    fn cmp(a: MultiDegree<N>, b: MultiDegree<N>) -> std::cmp::Ordering {
        let mut a_coords = a.coords();
        let mut b_coords = b.coords();
        if N > 1 {
            a_coords.swap(0, 1);
            a_coords[0] = -a_coords[0];
            b_coords.swap(0, 1);
            b_coords[0] = -b_coords[0];
        }
        a_coords.cmp(&b_coords)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NUM_TEST_DEGREES: usize = 18;

    /// Small grid of bidegrees for testing.
    ///
    /// Graphically, it looks like a rectangular prism, where the front face is
    /// ```text
    ///  0  2  4
    ///  6  8 10
    /// 12 14 16
    /// ```
    /// and the back face is
    /// ```text
    ///  1  3  5
    ///  7  9 11
    /// 13 15 17
    const TEST_DEGREES: [MultiDegree<3>; NUM_TEST_DEGREES] = [
        MultiDegree::new([-1, 2, 0]),
        MultiDegree::new([-1, 2, 1]),
        MultiDegree::new([0, 2, 0]),
        MultiDegree::new([0, 2, 1]),
        MultiDegree::new([1, 2, 0]),
        MultiDegree::new([1, 2, 1]),
        MultiDegree::new([-1, 1, 0]),
        MultiDegree::new([-1, 1, 1]),
        MultiDegree::new([0, 1, 0]),
        MultiDegree::new([0, 1, 1]),
        MultiDegree::new([1, 1, 0]),
        MultiDegree::new([1, 1, 1]),
        MultiDegree::new([-1, 0, 0]),
        MultiDegree::new([-1, 0, 1]),
        MultiDegree::new([0, 0, 0]),
        MultiDegree::new([0, 0, 1]),
        MultiDegree::new([1, 0, 0]),
        MultiDegree::new([1, 0, 1]),
    ];

    fn get_ordered<const N: usize, O: MultiDegreeOrdering<3>>(
        order: [usize; N],
    ) -> [OrderedMultiDegree<3, O>; N] {
        order.map(|idx| TEST_DEGREES[idx].into())
    }

    fn test_ordering<O: MultiDegreeOrdering<3>>(v: [usize; NUM_TEST_DEGREES]) {
        let mut ordered_degrees: [OrderedMultiDegree<3, O>; _] =
            get_ordered([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17]);
        ordered_degrees.sort();
        assert_eq!(ordered_degrees, get_ordered(v));
    }

    #[test]
    fn test_stem_ordered() {
        test_ordering::<ByStem>([12, 13, 6, 7, 0, 1, 14, 15, 8, 9, 2, 3, 16, 17, 10, 11, 4, 5])
    }

    #[test]
    fn test_internal_ordered() {
        test_ordering::<ByInternalDegree>([
            12, 13, 14, 15, 6, 7, 16, 17, 8, 9, 0, 1, 10, 11, 2, 3, 4, 5,
        ])
    }

    #[test]
    fn test_homological_ordered() {
        test_ordering::<ByHomologicalDegree>([
            12, 13, 14, 15, 16, 17, 6, 7, 8, 9, 10, 11, 0, 1, 2, 3, 4, 5,
        ])
    }

    #[test]
    fn test_reverse_homological_ordered() {
        test_ordering::<ByReverseHomologicalDegree>([
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
        ])
    }
}
