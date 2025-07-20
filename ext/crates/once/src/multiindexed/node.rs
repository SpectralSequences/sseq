use std::mem::ManuallyDrop;

use crate::grove::TwoEndedGrove;

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
///    last one. Each inner node is a [`TwoEndedGrove`] of other `Node` instances, allowing for
///    efficient storage of sparse child nodes.
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
    inner: ManuallyDrop<TwoEndedGrove<Node<V>>>,
    leaf: ManuallyDrop<TwoEndedGrove<V>>,
}

impl<V> Node<V> {
    /// Creates a new inner node.
    ///
    /// An inner node contains child nodes and is used for all dimensions except the last one.
    pub(super) fn new_inner() -> Self {
        Self {
            inner: ManuallyDrop::new(TwoEndedGrove::new()),
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

    /// Ensures that a child node exists at the specified index, creating it if necessary.
    ///
    /// # Parameters
    ///
    /// * `idx`: The index at which to ensure a child node exists
    /// * `to_insert`: The node to insert if no child exists at the specified index
    ///
    /// # Returns
    ///
    /// A reference to the child node at the specified index
    ///
    /// # Safety
    ///
    /// Can only be called on an inner node.
    pub(super) unsafe fn ensure_child(&self, idx: i32, to_insert: Self) -> &Self {
        // Safety: this is an inner node by assumption
        if let Some(child) = unsafe { self.get_child(idx) } {
            child
        } else {
            let _ = unsafe { self.inner.try_insert(idx, to_insert) };
            // Safety: either we inserted the node or another thread did. In either case, we can now
            // unwrap the child node.
            unsafe { self.get_child(idx).unwrap() }
        }
    }

    /// Retrieves a reference to the child node at the specified index, if it exists.
    ///
    /// # Parameters
    ///
    /// * `idx`: The index of the child node to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(&Self)` if a child node exists at the specified index
    /// * `None` if no child node exists at the specified index
    ///
    /// # Safety
    ///
    /// Can only be called on an inner node.
    pub(super) unsafe fn get_child(&self, idx: i32) -> Option<&Self> {
        unsafe { self.inner.get(idx) }
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
            unsafe {
                self.inner.for_each_mut(|node| {
                    node.drop_level(dimensions, level + 1);
                });
                ManuallyDrop::drop(&mut self.inner);
            }
        }
    }
}
