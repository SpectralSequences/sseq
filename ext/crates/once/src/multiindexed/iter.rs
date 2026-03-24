use std::{marker::PhantomData, ops::Range};

use super::{KdTrie, node::Node};
use crate::MultiIndexed;

// --- Iterator ---

/// A stack frame in the depth-first traversal of a [`KdTrie`].
///
/// Each frame records the current node, its depth in the trie (i.e. which coordinate dimension it
/// indexes), and the remaining range of indices to visit at this node.
struct IterFrame<R> {
    depth: usize,
    current_node: R,
    range: Range<i32>,
}

/// A depth-first iterator over a [`KdTrie`], generic over:
///
/// - `R: NodeRef` — the node handle type, either shared (`&Node<V>`) or exclusive
///   (`NodePtrMut<'_, V>`), determining whether values are yielded as `&V` or `&mut V`.
/// - `C: Coordinates` — the coordinate accumulator, either `[i32; K]` (fixed-size, for
///   [`MultiIndexed`]) or `Vec<i32>` (dynamic, for [`KdTrie`]).
///
/// The iterator walks the trie in lexicographic order of coordinates, yielding `(C, R::Value)` for
/// each stored entry.
struct KdIterator<R, C> {
    dimensions: usize,
    stack: Vec<IterFrame<R>>,
    coordinates: C,
}

impl<R: NodeRef, C> KdIterator<R, C> {
    fn new(dimensions: usize, root: R, coordinates: C) -> Self {
        let root_range = unsafe { root.range(dimensions == 1) };
        Self {
            dimensions,
            stack: vec![IterFrame {
                depth: 0,
                current_node: root,
                range: root_range,
            }],
            coordinates,
        }
    }
}

impl<R: NodeRef, C: Coordinates> Iterator for KdIterator<R, C> {
    type Item = (C, R::Value);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(IterFrame {
            depth,
            current_node,
            mut range,
        }) = self.stack.pop()
        {
            self.coordinates.truncate_to(depth);

            // Find the next index in the current range that has a value
            while let Some(idx) = range.next() {
                if depth == self.dimensions - 1 {
                    // This is a leaf node, check if there's a value at this index
                    if let Some(value) = unsafe { current_node.value(idx) } {
                        // Push back the remaining range for this node
                        if !range.is_empty() {
                            self.stack.push(IterFrame {
                                depth,
                                current_node,
                                range,
                            });
                        }

                        self.coordinates.set_coord(depth, idx);
                        return Some((self.coordinates.get(), value));
                    }
                } else if let Some(child_node) = unsafe { current_node.child(idx) } {
                    // This is an inner node, check if there's a child at this index

                    // Push back the remaining range for this node
                    if !range.is_empty() {
                        self.stack.push(IterFrame {
                            depth,
                            current_node,
                            range,
                        });
                    }

                    // Add the current index to coordinates and push the child
                    self.coordinates.set_coord(depth, idx);
                    let child_range = unsafe { child_node.range(depth + 1 == self.dimensions - 1) };
                    self.stack.push(IterFrame {
                        depth: depth + 1,
                        current_node: child_node,
                        range: child_range,
                    });

                    // Go to the next iteration of the outer loop, which will process the child
                    break;
                }
            }
        }

        None
    }
}

impl<R: NodeRef, C: Coordinates> std::iter::FusedIterator for KdIterator<R, C> {}

// --- NodeRef ---

/// Abstraction over shared (`&Node<V>`) and exclusive (`*mut Node<V>`) node access.
///
/// This trait allows [`KdIterator`] to be generic over the borrowing mode, so a single iterator
/// implementation drives both `iter` (shared) and `iter_mut` (exclusive).
///
/// # Safety
///
/// Implementations must ensure that:
/// - `range`, `child`, and `value` uphold the safety preconditions of the underlying [`Node`]
///   methods (i.e. leaf methods are only called on leaf nodes, and inner methods on inner nodes).
/// - For mutable implementations, the returned value references do not alias.
unsafe trait NodeRef: Copy {
    type Value;

    /// Returns the range of indices for this node.
    ///
    /// # Safety
    ///
    /// `is_leaf` must correctly indicate whether this is a leaf node.
    unsafe fn range(self, is_leaf: bool) -> Range<i32>;

    /// Returns a handle to the child node at `idx`, or `None` if no child exists.
    ///
    /// # Safety
    ///
    /// Must only be called on inner nodes.
    unsafe fn child(self, idx: i32) -> Option<Self>;

    /// Returns a reference to the value at `idx`, or `None` if the slot is empty.
    ///
    /// # Safety
    ///
    /// Must only be called on leaf nodes.
    unsafe fn value(self, idx: i32) -> Option<Self::Value>;
}

/// Shared node reference. Yields `&V` values.
unsafe impl<'a, V> NodeRef for &'a Node<V> {
    type Value = &'a V;

    unsafe fn range(self, is_leaf: bool) -> Range<i32> {
        if is_leaf {
            unsafe { self.leaf() }.range()
        } else {
            unsafe { self.inner() }.range()
        }
    }

    unsafe fn child(self, idx: i32) -> Option<Self> {
        unsafe { self.inner().get(idx) }
    }

    unsafe fn value(self, idx: i32) -> Option<Self::Value> {
        unsafe { self.leaf().get(idx) }
    }
}

/// A `Copy` wrapper around `*mut Node<V>` that serves as the exclusive counterpart to
/// `&Node<V>` in the [`NodeRef`] trait.
///
/// The phantom lifetime `'a` ties the yielded `&'a mut V` references back to the original
/// `&'a mut MultiIndexed` (or `&'a mut KdTrie`), ensuring soundness.
///
/// This is safe to use because:
/// - It is only constructed from `&mut MultiIndexed` / `&mut KdTrie`, guaranteeing exclusive access
///   to the entire tree.
/// - The tree structure ensures that nodes at different positions are disjoint in memory.
/// - The iterator yields each value at most once.
struct NodePtrMut<'a, V>(*mut Node<V>, PhantomData<&'a mut V>);

impl<V> Copy for NodePtrMut<'_, V> {}

impl<V> Clone for NodePtrMut<'_, V> {
    fn clone(&self) -> Self {
        *self
    }
}

/// Exclusive node reference. Yields `&mut V` values.
unsafe impl<'a, V> NodeRef for NodePtrMut<'a, V> {
    type Value = &'a mut V;

    unsafe fn range(self, is_leaf: bool) -> Range<i32> {
        if is_leaf {
            unsafe { (*self.0).leaf() }.range()
        } else {
            unsafe { (*self.0).inner() }.range()
        }
    }

    unsafe fn child(self, idx: i32) -> Option<Self> {
        let child = unsafe { (*self.0).get_child_mut(idx) }?;
        Some(Self(child as *mut Node<V>, PhantomData))
    }

    unsafe fn value(self, idx: i32) -> Option<Self::Value> {
        unsafe { (*self.0).get_value_mut(idx) }
    }
}

// --- Coordinates ---

/// Trait for managing coordinates during iteration.
///
/// The iterator accumulates coordinates dimension-by-dimension as it descends. When it backtracks,
/// it calls [`truncate_to`](Coordinates::truncate_to) to discard coordinates from deeper
/// dimensions. When it yields an entry, it calls [`get`](Coordinates::get) to snapshot the current
/// coordinates.
trait Coordinates {
    /// Sets the coordinate at the given `depth` (dimension index) to `value`.
    fn set_coord(&mut self, depth: usize, value: i32);

    /// Discards any coordinate data beyond `depth`, preparing for backtracking.
    fn truncate_to(&mut self, depth: usize);

    /// Returns a snapshot of the current coordinates.
    fn get(&self) -> Self;
}

/// Fixed-size coordinate accumulator for [`MultiIndexed`].
///
/// `truncate_to` is a no-op since all dimensions are always present in the array; stale values at
/// deeper indices are simply overwritten by `set_coord` before they are ever read.
impl<const K: usize> Coordinates for [i32; K] {
    fn set_coord(&mut self, depth: usize, value: i32) {
        self[depth] = value;
    }

    fn truncate_to(&mut self, _depth: usize) {}

    fn get(&self) -> Self {
        *self
    }
}

/// Dynamic coordinate accumulator for [`KdTrie`].
///
/// `set_coord` pushes a new coordinate (asserting that `depth == len`, i.e. coordinates are always
/// built in order), and `truncate_to` pops coordinates back to the given depth.
impl Coordinates for Vec<i32> {
    fn set_coord(&mut self, depth: usize, value: i32) {
        assert_eq!(self.len(), depth);
        self.push(value);
    }

    fn truncate_to(&mut self, depth: usize) {
        self.truncate(depth);
    }

    fn get(&self) -> Self {
        self.clone()
    }
}

// --- Public API ---

/// An iterator over the entries of a [`KdTrie`] or [`MultiIndexed`].
pub struct Iter<'a, V, C>(KdIterator<&'a Node<V>, C>);

impl<'a, V, C: Coordinates> Iterator for Iter<'a, V, C> {
    type Item = (C, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<V, C: Coordinates> std::iter::FusedIterator for Iter<'_, V, C> {}

/// A mutable iterator over the entries of a [`KdTrie`] or [`MultiIndexed`].
pub struct IterMut<'a, V, C>(KdIterator<NodePtrMut<'a, V>, C>);

impl<'a, V, C: Coordinates> Iterator for IterMut<'a, V, C> {
    type Item = (C, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<V, C: Coordinates> std::iter::FusedIterator for IterMut<'_, V, C> {}

impl<V> KdTrie<V> {
    pub fn iter(&self) -> Iter<'_, V, Vec<i32>> {
        let dimensions = self.dimensions();
        Iter(KdIterator::new(
            dimensions,
            self.root(),
            Vec::with_capacity(dimensions),
        ))
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, V, Vec<i32>> {
        let dimensions = self.dimensions();
        let root = NodePtrMut(self.root_mut() as *mut Node<V>, PhantomData);
        IterMut(KdIterator::new(
            dimensions,
            root,
            Vec::with_capacity(dimensions),
        ))
    }
}

impl<'a, V> IntoIterator for &'a KdTrie<V> {
    type IntoIter = Iter<'a, V, Vec<i32>>;
    type Item = (Vec<i32>, &'a V);

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, V> IntoIterator for &'a mut KdTrie<V> {
    type IntoIter = IterMut<'a, V, Vec<i32>>;
    type Item = (Vec<i32>, &'a mut V);

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<const K: usize, V> MultiIndexed<K, V> {
    /// Returns an iterator over all coordinate-value pairs in the array.
    ///
    /// The iterator yields `([i32; K], &V)` tuples in lexicographic order of coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::MultiIndexed;
    ///
    /// let array = MultiIndexed::<2, i32>::new();
    /// array.insert([3, 4], 10);
    /// array.insert([1, 2], 20);
    ///
    /// let mut items: Vec<_> = array.iter().collect();
    ///
    /// assert_eq!(items, vec![([1, 2], &20), ([3, 4], &10)]);
    /// ```
    pub fn iter(&self) -> Iter<'_, V, [i32; K]> {
        Iter(KdIterator::new(K, self.trie.root(), [0; K]))
    }

    /// Returns a mutable iterator over all coordinate-value pairs in the array.
    ///
    /// The iterator yields `([i32; K], &mut V)` tuples in lexicographic order of coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::MultiIndexed;
    ///
    /// let mut array = MultiIndexed::<2, i32>::new();
    /// array.insert([1, 2], 10);
    /// array.insert([3, 4], 20);
    ///
    /// for (_, v) in array.iter_mut() {
    ///     *v *= 2;
    /// }
    ///
    /// assert_eq!(array.get([1, 2]), Some(&20));
    /// assert_eq!(array.get([3, 4]), Some(&40));
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<'_, V, [i32; K]> {
        let root = NodePtrMut(self.trie.root_mut() as *mut Node<V>, PhantomData);
        IterMut(KdIterator::new(K, root, [0; K]))
    }
}

impl<'a, const K: usize, V> IntoIterator for &'a MultiIndexed<K, V> {
    type IntoIter = Iter<'a, V, [i32; K]>;
    type Item = ([i32; K], &'a V);

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, const K: usize, V> IntoIterator for &'a mut MultiIndexed<K, V> {
    type IntoIter = IterMut<'a, V, [i32; K]>;
    type Item = ([i32; K], &'a mut V);

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- MultiIndexed iteration tests ---

    #[test]
    fn test_multiindexed_iter_empty() {
        let arr = MultiIndexed::<2, i32>::new();
        let items: Vec<_> = arr.iter().collect();
        assert!(items.is_empty());
    }

    #[test]
    fn test_multiindexed_iter_multiple_calls() {
        let arr = MultiIndexed::<2, i32>::new();
        arr.insert([1, 2], 10);
        arr.insert([3, 4], 20);

        let items1: Vec<_> = arr.iter().collect();
        let items2: Vec<_> = arr.iter().collect();
        assert_eq!(items1, items2);
    }

    #[test]
    fn test_multiindexed_iter_mut_empty() {
        let mut arr = MultiIndexed::<2, i32>::new();
        let items: Vec<_> = arr.iter_mut().collect();
        assert!(items.is_empty());
    }

    #[test]
    fn test_multiindexed_iter_mut_basic() {
        let mut arr = MultiIndexed::<2, i32>::new();
        arr.insert([1, 2], 10);
        arr.insert([3, 4], 20);
        arr.insert([-5, 6], 30);

        for (_, v) in arr.iter_mut() {
            *v *= 3;
        }

        assert_eq!(arr.get([1, 2]), Some(&30));
        assert_eq!(arr.get([3, 4]), Some(&60));
        assert_eq!(arr.get([-5, 6]), Some(&90));
    }

    #[test]
    fn test_multiindexed_iter_and_iter_mut_agree() {
        let mut arr = MultiIndexed::<2, i32>::new();
        arr.insert([1, 2], 10);
        arr.insert([3, -4], 20);
        arr.insert([-5, 6], 30);
        arr.insert([0, 0], 40);

        let immutable: Vec<_> = arr.iter().map(|(c, &v)| (c, v)).collect();
        let mutable: Vec<_> = arr.iter_mut().map(|(c, &mut v)| (c, v)).collect();
        assert_eq!(immutable, mutable);
    }

    #[test]
    fn test_multiindexed_iter_mut_no_aliasing() {
        let mut arr = MultiIndexed::<3, i32>::new();
        arr.insert([0, 0, 0], 10);
        arr.insert([0, 0, 1], 20);
        arr.insert([0, 1, 0], 30);
        arr.insert([1, 0, 0], 40);

        let mut it = arr.iter_mut();
        let (_, a) = it.next().unwrap();
        let (_, b) = it.next().unwrap();
        let (_, c) = it.next().unwrap();
        let (_, d) = it.next().unwrap();

        // Miri detects borrow-model violations if any of the references alias, even before the
        // writes below.
        *a += 1;
        *b += 2;
        *c += 3;
        *d += 4;
        drop(it);

        assert_eq!(arr.get([0, 0, 0]), Some(&11));
        assert_eq!(arr.get([0, 0, 1]), Some(&22));
        assert_eq!(arr.get([0, 1, 0]), Some(&33));
        assert_eq!(arr.get([1, 0, 0]), Some(&44));
    }

    #[test]
    fn test_multiindexed_into_iterator() {
        let arr = MultiIndexed::<2, i32>::new();
        arr.insert([1, 2], 10);
        arr.insert([3, 4], 20);

        let items: Vec<_> = (&arr).into_iter().collect();
        assert_eq!(items.len(), 2);

        let mut arr = arr;
        let items: Vec<_> = (&mut arr).into_iter().map(|(c, &mut v)| (c, v)).collect();
        assert_eq!(items.len(), 2);
    }

    // --- KdTrie iteration tests ---

    #[test]
    fn test_kdtrie_iter_empty() {
        let trie = KdTrie::<i32>::new(2);
        let items: Vec<_> = trie.iter().collect();
        assert_eq!(items, vec![]);
    }

    #[test]
    fn test_kdtrie_iter_multiple_calls() {
        let trie = KdTrie::<i32>::new(2);
        trie.insert(&[1, 2], 10);
        trie.insert(&[3, 4], 20);

        let items1: Vec<_> = trie.iter().collect();
        let items2: Vec<_> = trie.iter().collect();

        assert_eq!(items1, items2);
    }

    #[test]
    fn test_kdtrie_iter_mut_empty() {
        let mut trie = KdTrie::<i32>::new(2);
        let items: Vec<_> = trie.iter_mut().collect();
        assert_eq!(items, vec![]);
    }

    #[test]
    fn test_kdtrie_iter_mut_basic() {
        let mut trie = KdTrie::<i32>::new(2);
        trie.insert(&[1, 2], 10);
        trie.insert(&[3, 4], 20);
        trie.insert(&[-5, 6], 30);

        for (_, v) in trie.iter_mut() {
            *v *= 3;
        }

        assert_eq!(trie.get(&[1, 2]), Some(&30));
        assert_eq!(trie.get(&[3, 4]), Some(&60));
        assert_eq!(trie.get(&[-5, 6]), Some(&90));
    }

    #[test]
    fn test_kdtrie_iter_and_iter_mut_agree() {
        let mut trie = KdTrie::<i32>::new(2);
        trie.insert(&[1, 2], 10);
        trie.insert(&[3, -4], 20);
        trie.insert(&[-5, 6], 30);
        trie.insert(&[0, 0], 40);

        let immutable: Vec<_> = trie.iter().map(|(c, &v)| (c, v)).collect();
        let mutable: Vec<_> = trie.iter_mut().map(|(c, &mut v)| (c, v)).collect();

        assert_eq!(immutable, mutable);
    }

    #[test]
    fn test_kdtrie_iter_mut_no_aliasing() {
        let mut trie = KdTrie::<i32>::new(3);
        trie.insert(&[0, 0, 0], 10);
        trie.insert(&[0, 0, 1], 20);
        trie.insert(&[0, 1, 0], 30);
        trie.insert(&[1, 0, 0], 40);

        let mut it = trie.iter_mut();
        let (_, a) = it.next().unwrap();
        let (_, b) = it.next().unwrap();
        let (_, c) = it.next().unwrap();
        let (_, d) = it.next().unwrap();

        // Miri detects borrow-model violations if any of the references alias, even before the
        // writes below.
        *a += 1;
        *b += 2;
        *c += 3;
        *d += 4;
        drop(it);

        assert_eq!(trie.get(&[0, 0, 0]), Some(&11));
        assert_eq!(trie.get(&[0, 0, 1]), Some(&22));
        assert_eq!(trie.get(&[0, 1, 0]), Some(&33));
        assert_eq!(trie.get(&[1, 0, 0]), Some(&44));
    }

    #[test]
    fn test_kdtrie_into_iterator() {
        let trie = KdTrie::<i32>::new(2);
        trie.insert(&[1, 2], 10);
        trie.insert(&[3, 4], 20);

        let items: Vec<_> = (&trie).into_iter().collect();
        assert_eq!(items.len(), 2);

        let mut trie = trie;
        let items: Vec<_> = (&mut trie).into_iter().map(|(c, &mut v)| (c, v)).collect();
        assert_eq!(items.len(), 2);
    }
}
