use std::ops::Range;

use super::{KdTrie, node::Node};
use crate::MultiIndexed;

/// A single frame in the iteration stack, representing the current state of traversal.
struct IterFrame<'a, V> {
    /// The current depth in the multi-indexed structure
    /// (0 for the root, 1 for the first level, etc.)
    depth: usize,

    /// The current node being processed
    current_node: &'a Node<V>,

    /// The range of indices left to iterate over in the current node
    range: Range<i32>,
}

impl<'a, V> IterFrame<'a, V> {
    /// Creates the initial iteration frame.
    fn new(dimensions: usize, root: &'a Node<V>) -> Self {
        // Safety: This function is only called by KdIterator::new, which is only called by the iter
        // methods of KdTrie and MultiIndexed. Therefore, by definition, the number of dimensions
        // can be trusted. There can not be any other caller because of the pub(self) visibility.
        let root_range = if dimensions == 1 {
            unsafe { root.leaf() }.range()
        } else {
            unsafe { root.inner() }.range()
        };

        Self {
            depth: 0,
            current_node: root,
            range: root_range,
        }
    }
}

/// Trait for managing coordinates during iteration
trait Coordinates {
    fn set_coord(&mut self, depth: usize, value: i32);
    fn truncate_to(&mut self, depth: usize);
    fn get(&self) -> Self;
}

/// Iterator implementation for multi-dimensional structures
///
/// This abstracts over both dynamic and fixed-size coordinates, which allows us to iterate over
/// `KdTrie`s with vector coordinates and `MultiIndexed`s with fixed-size arrays. It's important to
/// allow fixed-size arrays to be used as coordinates, as they are `Copy` and can avoid the
/// expensive `clone`s. Empirically, this gives a 3x speedup.
pub(super) struct KdIterator<'a, V, C> {
    dimensions: usize,
    stack: Vec<IterFrame<'a, V>>,
    coordinates: C,
}

impl<'a, V, C> KdIterator<'a, V, C> {
    fn new(dimensions: usize, root: &'a Node<V>, coordinates: C) -> Self {
        Self {
            dimensions,
            stack: vec![IterFrame::new(dimensions, root)],
            coordinates,
        }
    }
}

impl<V> KdTrie<V> {
    pub fn iter(&self) -> impl Iterator<Item = (Vec<i32>, &V)> + '_ {
        let dimensions = self.dimensions();
        KdIterator::new(dimensions, self.root(), Vec::with_capacity(dimensions))
    }
}

impl<const K: usize, V> MultiIndexed<K, V> {
    /// Returns an iterator over all coordinate-value pairs in the array.
    ///
    /// The iterator yields tuples of `([i32; K], &V)` where the first element is the coordinate
    /// array and the second is a reference to the value.
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
    pub fn iter(&self) -> impl Iterator<Item = ([i32; K], &V)> {
        KdIterator::new(K, self.0.root(), [0; K])
    }
}

impl<'a, V, C: Coordinates> Iterator for KdIterator<'a, V, C> {
    type Item = (C, &'a V);

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
                    let current_leaf = unsafe { current_node.leaf() };
                    if let Some(value) = current_leaf.get(idx) {
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
                } else {
                    // This is an inner node, check if there's a child at this index
                    let current_inner = unsafe { current_node.inner() };
                    if let Some(child_node) = current_inner.get(idx) {
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
                        let child_range = if depth + 1 == self.dimensions - 1 {
                            unsafe { child_node.leaf() }.range()
                        } else {
                            unsafe { child_node.inner() }.range()
                        };
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
        }

        None
    }
}

impl<const K: usize> Coordinates for [i32; K] {
    fn set_coord(&mut self, depth: usize, value: i32) {
        self[depth] = value;
    }

    fn truncate_to(&mut self, _depth: usize) {
        // Array doesn't need truncation
    }

    fn get(&self) -> Self {
        *self
    }
}

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
