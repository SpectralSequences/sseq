use std::mem::ManuallyDrop;

use crate::{
    grove::{TwoEndedGrove, ZeroedTwoEndedGrove},
    std_or_loom::Zeroable,
};

/// A node in the K-dimensional trie structure.
///
/// This is an internal implementation detail of `KdTrie`. It uses a union to represent either an
/// inner node, which contains child nodes, or a leaf node, which contains values. This allows us to
/// store values in contiguous memory instead of creating a separate node for each value.
///
/// # Node Types
///
/// There are two types of nodes in the trie:
///
/// 1. **Inner Nodes**: These nodes contain child nodes and are used for all dimensions except the
///    last one. Each inner node is a [`ZeroedTwoEndedGrove`] of child `Node`s, allowing for
///    efficient storage of sparse child nodes. A child exists, empty, from the moment its block is
///    allocated, so threads that reach the same child find the same node and none is ever created
///    twice.
///
/// 2. **Leaf Nodes**: These nodes contain the actual values and are used for the last dimension.
///    Each leaf node is a [`TwoEndedGrove`] of values of type `V`.
///
/// # Thread Safety
///
/// The `Node` struct is designed to be thread-safe and wait-free, allowing concurrent insertions
/// and retrievals from multiple threads. This is achieved through the use of atomic operations in
/// the underlying [`TwoEndedGrove`] data structure.
///
/// # Memory Management
///
/// The `Node` struct uses a Rust union to efficiently represent either an inner node or a leaf node
/// without the overhead of an enum discriminant. This approach saves memory but requires careful
/// manual memory management.
///
/// # Safety
///
/// It is the responsibility of the caller to know which variant is contained in the union. We only
/// use this in `KdTrie`, where we know which variant is contained based on the number of
/// dimensions.
///
/// The `inner` and `leaf` variants are wrapped in `ManuallyDrop`, because union fields can't have
/// drop glue. Unions don't know what they contain, and so can't know which drop implementation to
/// run. We delegate the responsibility of dropping the node to the `KdTrie`, using `drop_level`.
pub(super) union Node<V> {
    inner: ManuallyDrop<ZeroedTwoEndedGrove<Self>>,
    leaf: ManuallyDrop<TwoEndedGrove<V>>,
}

impl<V> Node<V> {
    /// Creates a new inner node.
    ///
    /// An inner node contains child nodes and is used for all dimensions except the last one.
    pub(super) fn new_inner() -> Self {
        Self {
            inner: ManuallyDrop::new(ZeroedTwoEndedGrove::new()),
        }
    }

    /// Creates a new leaf node.
    ///
    /// A leaf node contains values and is used for the last dimension.
    pub(super) fn new_leaf() -> Self {
        Self {
            leaf: ManuallyDrop::new(TwoEndedGrove::new()),
        }
    }

    /// The child node at `idx`, allocating its block if needed.
    ///
    /// A child nothing was put in yet is an empty node, which serves as either an inner node or a
    /// leaf, whichever the trie needs at the child's level.
    ///
    /// # Safety
    ///
    /// Can only be called on an inner node.
    pub(super) unsafe fn ensure_child(&self, idx: i32) -> &Self {
        // SAFETY: this is an inner node by assumption.
        unsafe { self.inner.entry(idx) }
    }

    /// Retrieves a reference to the child node at the specified index, if it exists.
    ///
    /// # Parameters
    ///
    /// * `idx`: The index of the child node to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(&Self)` if the block holding the child is allocated, even if the child is empty
    /// * `None` otherwise
    ///
    /// # Safety
    ///
    /// Can only be called on an inner node.
    pub(super) unsafe fn get_child(&self, idx: i32) -> Option<&Self> {
        // SAFETY: this is an inner node by assumption.
        unsafe { self.inner.get_entry(idx) }
    }

    /// Retrieves a mutable reference to the child node at the specified index, if it exists.
    ///
    /// # Parameters
    ///
    /// * `idx`: The index of the child node to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(&mut Self)` if the block holding the child is allocated, even if the child is empty
    /// * `None` otherwise
    ///
    /// # Safety
    ///
    /// Can only be called on an inner node.
    pub(super) unsafe fn get_child_mut(&mut self, idx: i32) -> Option<&mut Self> {
        // SAFETY: this is an inner node by assumption.
        unsafe { self.inner.get_entry_mut(idx) }
    }

    /// Retrieves a reference to the value at the specified index, if it exists.
    ///
    /// # Parameters
    ///
    /// * `idx`: The index of the value to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(&V)` if a value exists at the specified index
    /// * `None` if no value exists at the specified index
    ///
    /// # Safety
    ///
    /// Can only be called on a leaf node.
    pub(super) unsafe fn get_value(&self, idx: i32) -> Option<&V> {
        unsafe { self.leaf.get(idx) }
    }

    /// Retrieves a mutable reference to the value at the specified index, if it exists.
    ///
    /// # Parameters
    ///
    /// * `idx`: The index of the value to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(&mut V)` if a value exists at the specified index
    /// * `None` if no value exists at the specified index
    ///
    /// # Safety
    ///
    /// Can only be called on a leaf node.
    pub(super) unsafe fn get_value_mut(&mut self, idx: i32) -> Option<&mut V> {
        unsafe { self.leaf.get_mut(idx) }
    }

    /// Sets the value at the specified index.
    ///
    /// # Parameters
    ///
    /// * `idx`: The index at which to set the value
    /// * `value`: The value to set
    ///
    /// # Safety
    ///
    /// Can only be called on a leaf node.
    pub(super) unsafe fn set_value(&self, idx: i32, value: V) {
        unsafe { self.leaf.insert(idx, value) }
    }

    /// Attempts to set the value at the specified index.
    ///
    /// # Parameters
    ///
    /// * `idx`: The index at which to set the value
    /// * `value`: The value to set
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the value was successfully set
    /// * `Err(value)` if the index was already occupied, returning the value that we tried to
    ///   insert
    ///
    /// # Safety
    ///
    /// Can only be called on a leaf node.
    pub(super) unsafe fn try_set_value(&self, idx: i32, value: V) -> Result<(), V> {
        unsafe { self.leaf.try_insert(idx, value) }
    }

    /// Returns a reference to the inner grove.
    ///
    /// # Safety
    ///
    /// Can only be called on an inner node.
    pub(super) unsafe fn inner(&self) -> &ZeroedTwoEndedGrove<Self> {
        unsafe { &self.inner }
    }

    /// The range of indices this node holds children or values at.
    ///
    /// # Safety
    ///
    /// `is_leaf` must say whether this is a leaf node.
    pub(super) unsafe fn range(&self, is_leaf: bool) -> std::ops::Range<i32> {
        // SAFETY: `is_leaf` names the variant this node is, by assumption.
        if is_leaf {
            unsafe { self.leaf.range() }
        } else {
            unsafe { self.inner.range() }
        }
    }

    /// Returns a reference to the leaf grove.
    ///
    /// # Safety
    ///
    /// Can only be called on a leaf node.
    pub(super) unsafe fn leaf(&self) -> &TwoEndedGrove<V> {
        unsafe { &self.leaf }
    }

    /// Recursively drops the node and all its children.
    ///
    /// This method is called by the `Drop` implementation of `KdTrie` to ensure
    /// proper cleanup of all allocated memory.
    ///
    /// # Parameters
    ///
    /// * `dimensions`: The total number of dimensions in the trie
    /// * `level`: The current level in the trie (the root node has level 0)
    pub(super) fn drop_level(&mut self, dimensions: usize, level: usize) {
        if level == dimensions {
            return;
        }

        if level == dimensions - 1 {
            // This is a leaf node
            unsafe { ManuallyDrop::drop(&mut self.leaf) };
        } else {
            // This is an inner node
            let child_is_leaf = level + 1 == dimensions - 1;
            // SAFETY: a node above the last level is an inner node, and its children are leaves
            // exactly when they sit at the last level. The inner grove is dropped once, after its
            // children.
            unsafe {
                for (_, child) in self.inner.entries_mut() {
                    // A child with an empty range was never touched, so it owns no memory.
                    if !child.range(child_is_leaf).is_empty() {
                        child.drop_level(dimensions, level + 1);
                    }
                }
                ManuallyDrop::drop(&mut self.inner);
            }
        }
    }
}

// SAFETY: the bounds require zero bytes to be valid for both variants, so an all-zero node is an
// empty inner node and an empty leaf at once (`TwoEndedGrove` is a transparent
// `ZeroedTwoEndedGrove`).
#[cfg(not(loom))]
unsafe impl<V> Zeroable for Node<V>
where
    ZeroedTwoEndedGrove<Self>: Zeroable,
    ZeroedTwoEndedGrove<crate::write_once::WriteOnce<V>>: Zeroable,
{
}

// SAFETY: `zeroed` builds the inner variant, which reads as an empty leaf too: `Block`,
// `ZeroedGrove` and `ZeroedTwoEndedGrove` are `repr(C)`, `TwoEndedGrove` is a transparent
// `ZeroedTwoEndedGrove`, and no field's layout depends on the element type.
#[cfg(loom)]
unsafe impl<V> Zeroable for Node<V> {
    fn zeroed() -> Self {
        Self::new_inner()
    }
}
