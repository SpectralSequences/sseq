use std::mem::ManuallyDrop;

use crate::grove::TwoEndedGrove;

/// A multi-dimensional array that allows efficient storage and retrieval of values using
/// K-dimensional integer coordinates.
///
/// `MultiIndexed<K, V>` provides a thread-safe and wait-free way to store values indexed by
/// multi-dimensional coordinates. It is implemented using a K-dimensional trie structure that
/// efficiently handles sparse data, where each level of the trie corresponds to one dimension of
/// the coordinate space.
///
/// The `MultiIndexed` is created with a fixed number of dimensions `K` and can store values of any
/// type `V`. Each dimension can have both positive and negative indices.
///
/// # Thread Safety
///
/// `MultiIndexed` is designed to be thread-safe and wait-free, allowing concurrent insertions and
/// retrievals from multiple threads. This makes it suitable for parallel algorithms that need to
/// build up a shared data structure.
///
/// # Memory Efficiency
///
/// The underlying trie structure allocates memory only for coordinates that are actually used,
/// making it memory-efficient for sparse data. The implementation uses a series of
/// [`TwoEndedGrove`] instances to store the trie nodes, which themselves use a block-based
/// allocation strategy.
///
/// # Performance Characteristics
///
/// - **Insertion**: O(K) time complexity,
/// - **Retrieval**: O(K) time complexity,
/// - **Memory Usage**: amortized O(N) space complexity, where N is the number of inserted elements
///
/// # Warning
///
/// This data structure is designed for write-once semantics, meaning that once a value is inserted
/// at a specific coordinate, it cannot be changed directly. If you need to update values, you can
/// wrap them in a struct that allows for interior mutability (e.g., `RefCell`, `Mutex`, etc.).
///
/// Note that, for performance reasons, we do not allow `K = 0`.
///
/// # Examples
///
/// Correct usage:
///
/// ```
/// use once::MultiIndexed;
///
/// // Create a 3-dimensional array
/// let array = MultiIndexed::<3, i32>::new();
///
/// // Insert values at specific coordinates
/// array.insert([1, 2, 3], 42);
/// array.insert([5, 0, 2], 100);
/// array.insert([-1, -2, 3], 200); // Negative coordinates are supported
///
/// // Retrieve values
/// assert_eq!(array.get([1, 2, 3]), Some(&42));
/// assert_eq!(array.get([5, 0, 2]), Some(&100));
/// assert_eq!(array.get([-1, -2, 3]), Some(&200));
/// assert_eq!(array.get([0, 0, 0]), None); // No value at these coordinates
/// ```
///
/// Incorrect usage:
///
/// ```should_panic
/// use once::MultiIndexed;
///
/// let array = MultiIndexed::<2, i32>::new();
///
/// array.insert([1, 2], 42);
/// array.insert([1, 2], 43); // Panics because the value at [1, 2] is already set
/// ```
pub struct MultiIndexed<const K: usize, V>(KdTrie<V>);

impl<const K: usize, V> MultiIndexed<K, V> {
    /// Creates a new empty `MultiIndexed` array with K dimensions.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::MultiIndexed;
    ///
    /// // Create a 2D array for strings
    /// let array = MultiIndexed::<2, String>::new();
    ///
    /// // Create a 3D array for integers
    /// let array3d = MultiIndexed::<3, i32>::new();
    ///
    /// // Create a 4D array for custom types
    /// struct Point {
    ///     x: f64,
    ///     y: f64,
    /// }
    /// let array4d = MultiIndexed::<4, Point>::new();
    /// ```
    pub fn new() -> Self {
        Self(KdTrie::new(K))
    }

    /// Retrieves a reference to the value at the specified coordinates, if it exists.
    ///
    /// # Parameters
    ///
    /// * `coords`: An array of K integer coordinates
    ///
    /// # Returns
    ///
    /// * `Some(&V)` if a value exists at the specified coordinates
    /// * `None` if no value exists at the specified coordinates
    ///
    /// # Examples
    ///
    /// ```
    /// use once::MultiIndexed;
    ///
    /// // Basic retrieval in a 3D array
    /// let array = MultiIndexed::<3, i32>::new();
    /// array.insert([1, 2, 3], 42);
    ///
    /// assert_eq!(array.get([1, 2, 3]), Some(&42));
    /// assert_eq!(array.get([0, 0, 0]), None);
    ///
    /// // Retrieval with negative coordinates
    /// array.insert([-5, -10, 15], 100);
    /// assert_eq!(array.get([-5, -10, 15]), Some(&100));
    ///
    /// // Retrieval in a 2D array
    /// let array2d = MultiIndexed::<2, String>::new();
    /// array2d.insert([0, 0], "Origin".to_string());
    /// array2d.insert([10, -5], "Far point".to_string());
    ///
    /// assert_eq!(array2d.get([0, 0]), Some(&"Origin".to_string()));
    /// assert_eq!(array2d.get([10, -5]), Some(&"Far point".to_string()));
    /// assert_eq!(array2d.get([1, 1]), None);
    ///
    /// // Retrieval in a 1D array
    /// let array1d = MultiIndexed::<1, f64>::new();
    /// array1d.insert([0], 3.14);
    /// array1d.insert([-10], -2.71);
    ///
    /// assert_eq!(array1d.get([0]), Some(&3.14));
    /// assert_eq!(array1d.get([-10]), Some(&-2.71));
    /// assert_eq!(array1d.get([5]), None);
    /// ```
    pub fn get(&self, coords: [i32; K]) -> Option<&V> {
        self.0.get(&coords)
    }

    /// Inserts a value at the specified coordinates.
    ///
    /// This operation is thread-safe and can be called from multiple threads. However, this method
    /// panics if a value already exists at the specified coordinates. Therefore, it should only be
    /// called at most once for any given set of coordinates.
    ///
    /// # Parameters
    ///
    /// * `coords`: An array of K integer coordinates
    /// * `value`: The value to insert at the specified coordinates
    ///
    /// # Examples
    ///
    /// ```
    /// use once::MultiIndexed;
    ///
    /// // Basic insertion in a 3D array
    /// let array = MultiIndexed::<3, i32>::new();
    /// array.insert([1, 2, 3], 42);
    ///
    /// // Insertion with negative coordinates
    /// array.insert([-5, 0, 10], 100);
    /// array.insert([0, -3, -7], 200);
    ///
    /// // Insertion in a 2D array
    /// let array2d = MultiIndexed::<2, String>::new();
    /// array2d.insert([0, 0], "Origin".to_string());
    /// array2d.insert([10, -5], "Far point".to_string());
    ///
    /// // Insertion in a 1D array
    /// let array1d = MultiIndexed::<1, f64>::new();
    /// array1d.insert([0], 3.14);
    /// array1d.insert([-10], -2.71);
    /// ```
    ///
    /// # Panics
    ///
    /// This method will panic if a value already exists at the specified coordinates:
    ///
    /// ```should_panic
    /// use once::MultiIndexed;
    ///
    /// let array = MultiIndexed::<2, i32>::new();
    /// array.insert([1, 2], 42);
    /// array.insert([1, 2], 43); // Panics
    /// ```
    pub fn insert(&self, coords: [i32; K], value: V) {
        self.0.insert(&coords, value);
    }

    pub fn try_insert(&self, coords: [i32; K], value: V) -> Result<(), V> {
        self.0.try_insert(&coords, value)
    }
}

impl<const K: usize, V> Default for MultiIndexed<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

/// A K-dimensional trie data structure that efficiently stores values indexed by multi-dimensional
/// coordinates.
///
/// The difference between `KdTrie` and `MultiIndexed` is that the latter has the dimension as part
/// of the type, which allows for more type safety. The `KdTrie` itself may be useful in a situation
/// where the dimension is not known at compile time.
///
/// The trie is structured as a tree where each level corresponds to one dimension of the coordinate
/// space. For example, in a 3D trie, the first level corresponds to the x-coordinate, the second
/// level to the y-coordinate, and the third level to the z-coordinate. This structure allows for
/// efficient storage and retrieval of values in a sparse coordinate space.
///
/// # Thread Safety
///
/// `KdTrie` is designed to be thread-safe and wait-free, allowing concurrent insertions and
/// retrievals from multiple threads. This is achieved through the use of atomic operations and the
/// thread-safe properties of the underlying [`TwoEndedGrove`] data structure.
///
/// # Memory Efficiency
///
/// The trie only allocates memory for coordinates that are actually used, making it
/// memory-efficient for sparse data. Each node in the trie is either an inner node (which contains
/// child nodes) or a leaf node (which contains values).
///
/// # Type Parameters
///
/// * `V`: The type of values stored in the trie
pub struct KdTrie<V> {
    root: Node<V>,
    dimensions: usize,
}

impl<V> KdTrie<V> {
    /// Creates a new `KdTrie` with the specified number of dimensions.
    ///
    /// # Parameters
    ///
    /// * `dimensions`: The number of dimensions for the trie (must be greater than 0)
    ///
    /// # Panics
    ///
    /// Panics if `dimensions` is 0.
    pub fn new(dimensions: usize) -> Self {
        assert!(dimensions > 0);

        let root = if dimensions == 1 {
            Node::new_leaf()
        } else {
            Node::new_inner()
        };

        Self { root, dimensions }
    }

    /// Retrieves a reference to the value at the specified coordinates, if it exists.
    ///
    /// # Parameters
    ///
    /// * `coords`: A slice of coordinates with length equal to `self.dimensions`
    ///
    /// # Returns
    ///
    /// * `Some(&V)` if a value exists at the specified coordinates
    /// * `None` if no value exists at the specified coordinates
    ///
    /// # Panics
    ///
    /// Panics if the length of `coords` does not match the number of dimensions.
    pub fn get(&self, coords: &[i32]) -> Option<&V> {
        assert!(coords.len() == self.dimensions);

        // When's the last time you saw a mutable shared reference?
        let mut node = &self.root;

        for &coord in coords.iter().take(self.dimensions - 1) {
            node = unsafe { node.get_child(coord)? };
        }

        unsafe { node.get_value(coords[self.dimensions - 1]) }
    }

    /// Inserts a value at the specified coordinates.
    ///
    /// This method traverses the trie structure to find the appropriate location
    /// for the value, creating nodes as needed along the way.
    ///
    /// # Parameters
    ///
    /// * `coords`: A slice of coordinates with length equal to `self.dimensions`
    /// * `value`: The value to insert at the specified coordinates
    ///
    /// # Panics
    ///
    /// Panics if the length of `coords` does not match the number of dimensions.
    pub fn insert(&self, coords: &[i32], value: V) {
        assert!(coords.len() == self.dimensions);

        let mut node = &self.root;

        for &coord in coords.iter().take(self.dimensions.saturating_sub(2)) {
            node = unsafe { node.ensure_child(coord, Node::new_inner()) };
        }
        if self.dimensions > 1 {
            node = unsafe { node.ensure_child(coords[self.dimensions - 2], Node::new_leaf()) };
        }

        unsafe { node.set_value(coords[self.dimensions - 1], value) };
    }

    /// Attempts to insert a value at the specified coordinates.
    ///
    /// This method traverses the trie structure to find the appropriate location for the value,
    /// creating nodes as needed along the way.
    ///
    /// This method will only insert the value if the coordinate is not already occupied. If the
    /// coordinate is already occupied, the value is returned in the `Err` variant.
    ///
    /// # Parameters
    ///
    /// * `coords`: A slice of coordinates with length equal to `self.dimensions`
    /// * `value`: The value to insert at the specified coordinates
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the value was successfully inserted
    /// * `Err(value)` if the coordinate was already occupied, returning the value that we tried to
    ///   insert
    ///
    /// # Panics
    ///
    /// Panics if the length of `coords` does not match the number of dimensions.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::multiindexed::KdTrie;
    ///
    /// let trie = KdTrie::<i32>::new(2);
    ///
    /// assert_eq!(trie.try_insert(&[-3, 1], 10), Ok(()));
    /// assert_eq!(trie.try_insert(&[-3, 1], 10), Err(10)); // Coordinate already occupied
    /// ```
    pub fn try_insert(&self, coords: &[i32], value: V) -> Result<(), V> {
        assert!(coords.len() == self.dimensions);

        let mut node = &self.root;

        for &coord in coords.iter().take(self.dimensions.saturating_sub(2)) {
            node = unsafe { node.ensure_child(coord, Node::new_inner()) };
        }
        if self.dimensions > 1 {
            node = unsafe { node.ensure_child(coords[self.dimensions - 2], Node::new_leaf()) };
        }

        unsafe { node.try_set_value(coords[self.dimensions - 1], value) }
    }
}

impl<V> Drop for KdTrie<V> {
    fn drop(&mut self) {
        self.root.drop_level(self.dimensions, 0);
    }
}

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
union Node<V> {
    inner: ManuallyDrop<TwoEndedGrove<Node<V>>>,
    leaf: ManuallyDrop<TwoEndedGrove<V>>,
}

impl<V> Node<V> {
    /// Creates a new inner node.
    ///
    /// An inner node contains child nodes and is used for all dimensions except the last one.
    fn new_inner() -> Self {
        Self {
            inner: ManuallyDrop::new(TwoEndedGrove::new()),
        }
    }

    /// Creates a new leaf node.
    ///
    /// A leaf node contains values and is used for the last dimension.
    fn new_leaf() -> Self {
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
    unsafe fn ensure_child(&self, idx: i32, to_insert: Self) -> &Self {
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
    unsafe fn get_child(&self, idx: i32) -> Option<&Self> {
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
    unsafe fn get_value(&self, idx: i32) -> Option<&V> {
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
    unsafe fn set_value(&self, idx: i32, value: V) {
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
    unsafe fn try_set_value(&self, idx: i32, value: V) -> Result<(), V> {
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
    fn drop_level(&mut self, dimensions: usize, level: usize) {
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

#[cfg(test)]
mod tests {
    #![cfg_attr(miri, allow(dead_code))]
    use super::*;

    /// Generate all tuples of length K where the sum of coordinates equals n
    /// ```
    /// let result = once::get_nth_diagonal::<3>(4);
    ///
    /// assert_eq!(result.len(), 15);
    /// assert!(result.contains(&[0, 0, 4]));
    /// assert!(result.contains(&[0, 1, 3]));
    /// assert!(result.contains(&[0, 2, 2]));
    /// assert!(result.contains(&[0, 3, 1]));
    /// assert!(result.contains(&[0, 4, 0]));
    /// assert!(result.contains(&[1, 0, 3]));
    /// assert!(result.contains(&[1, 1, 2]));
    /// assert!(result.contains(&[1, 2, 1]));
    /// assert!(result.contains(&[1, 3, 0]));
    /// assert!(result.contains(&[2, 0, 2]));
    /// assert!(result.contains(&[2, 1, 1]));
    /// assert!(result.contains(&[2, 2, 0]));
    /// assert!(result.contains(&[3, 0, 1]));
    /// assert!(result.contains(&[3, 1, 0]));
    /// assert!(result.contains(&[4, 0, 0]));
    /// ```
    pub fn get_nth_diagonal<const K: usize>(n: usize) -> Vec<[i32; K]> {
        let mut result = Vec::new();
        let mut tuple = vec![0; K];

        // Generate all tuples where the sum of coordinates equals n
        generate_tuples::<K>(&mut tuple, 0, n, &mut result);

        result
    }

    /// Helper function to recursively generate the tuples
    fn generate_tuples<const K: usize>(
        tuple: &mut Vec<i32>,
        index: usize,
        sum: usize,
        result: &mut Vec<[i32; K]>,
    ) {
        if index == K - 1 {
            // The last element gets whatever is left to reach the sum
            tuple[index] = sum as i32;
            result.push(tuple.clone().try_into().unwrap()); // Convert to [i32; K]
            return;
        }

        for i in 0..=sum {
            tuple[index] = i as i32;
            generate_tuples::<K>(tuple, index + 1, sum - i, result);
        }
    }

    fn get_n_coords<const K: usize>(n: usize) -> Vec<[i32; K]> {
        (0..).flat_map(get_nth_diagonal).take(n).collect()
    }

    #[test]
    fn test_basic() {
        let arr = MultiIndexed::new();

        arr.insert([1, 2, 3], 42);
        arr.insert([1, 2, 4], 43);
        arr.insert([1, 3, 3], 44);
        arr.insert([1, 3, 4], 45);

        assert_eq!(arr.get([1, 2, 3]), Some(&42));
        assert_eq!(arr.get([1, 2, 4]), Some(&43));
        assert_eq!(arr.get([1, 3, 3]), Some(&44));
        assert_eq!(arr.get([1, 3, 4]), Some(&45));
    }

    // This is a bit too heavy for miri
    #[cfg_attr(not(miri), test)]
    fn test_large() {
        let arr = MultiIndexed::<8, _>::new();
        for (idx, coord) in get_n_coords(10_000).iter().enumerate() {
            arr.insert(*coord, idx);
        }
    }

    #[test]
    fn test_requires_drop() {
        use std::{
            sync::{
                atomic::{AtomicUsize, Ordering},
                Arc,
            },
            thread,
        };

        static ACTIVE_ALLOCS: AtomicUsize = AtomicUsize::new(0);

        struct DropCounter;

        impl DropCounter {
            fn new() -> Self {
                ACTIVE_ALLOCS.fetch_add(1, Ordering::Relaxed);
                Self
            }
        }

        impl Drop for DropCounter {
            fn drop(&mut self) {
                ACTIVE_ALLOCS.fetch_sub(1, Ordering::Relaxed);
            }
        }

        let v = Arc::new(MultiIndexed::<3, DropCounter>::new());
        assert_eq!(ACTIVE_ALLOCS.load(Ordering::Relaxed), 0);

        let num_threads = crate::test_utils::num_threads() as i32;
        let inserts_per_thread = crate::test_utils::values_per_thread() as i32;

        thread::scope(|s| {
            for thread_id in 0..num_threads {
                let v = Arc::clone(&v);
                s.spawn(move || {
                    for i in (-inserts_per_thread / 2)..(inserts_per_thread / 2) {
                        v.insert([thread_id, i, 4], DropCounter::new());
                    }
                });
            }
        });

        assert_eq!(
            ACTIVE_ALLOCS.load(Ordering::Relaxed),
            (num_threads * inserts_per_thread) as usize
        );

        drop(v);

        assert_eq!(ACTIVE_ALLOCS.load(Ordering::Relaxed), 0);
    }

    #[cfg(not(miri))]
    mod proptests {
        use std::collections::HashMap;

        use proptest::prelude::*;

        use super::*;

        /// Generate a strategy for a single i32 coordinate.
        fn coord_strategy(max: u32) -> impl Strategy<Value = i32> {
            -(max as i32)..=max as i32
        }

        // Generate a strategy for arrays of i32 with a specific dimension
        fn coords_strategy<const K: usize>(max: u32) -> impl Strategy<Value = [i32; K]> {
            proptest::collection::vec(coord_strategy(max), K)
                .prop_map(|v| std::array::from_fn(|i| v[i]))
        }

        #[derive(Debug, Clone, Copy)]
        enum Operation<const K: usize> {
            Insert([i32; K], i32),
            Get([i32; K]),
        }

        // Generate a strategy for a single operation (insert or get)
        fn operation_strategy<const K: usize>(max: u32) -> impl Strategy<Value = Operation<K>> {
            proptest::bool::ANY.prop_flat_map(move |is_insert| {
                coords_strategy::<K>(max).prop_flat_map(move |coords| {
                    if is_insert {
                        proptest::num::i32::ANY
                            .prop_map(move |value| Operation::Insert(coords, value))
                            .boxed()
                    } else {
                        proptest::strategy::Just(Operation::Get(coords)).boxed()
                    }
                })
            })
        }

        // Generate a strategy for a list of operations (insert or get)
        fn operations_strategy<const K: usize>(
            max_ops: usize,
        ) -> impl Strategy<Value = Vec<Operation<K>>> {
            // This is chosen so that the max number of operations is twice the number of elements in
            // the hypercube (-max..=max)^K
            let max = ((max_ops as f32 / 2.0).log(K as f32) / 2.0).ceil() as u32;
            proptest::collection::vec(operation_strategy::<K>(max), 1..max_ops)
        }

        fn proptest_multiindexed_kd<const K: usize>(ops: Vec<Operation<K>>) {
            let arr = MultiIndexed::<K, i32>::new();
            let mut reference = HashMap::new();

            for op in ops {
                match op {
                    Operation::Insert(coords, value) => {
                        // Only insert if the key doesn't exist yet (to avoid panics)
                        if let std::collections::hash_map::Entry::Vacant(e) =
                            reference.entry(coords)
                        {
                            arr.insert(coords, value);
                            e.insert(value);
                        } else {
                            // If the key already exists, test that try_insert returns an error
                            assert!(arr.try_insert(coords, value).is_err());
                        }
                    }
                    Operation::Get(coords) => {
                        // Check that get returns the same as our reference HashMap
                        let actual = arr.get(coords);
                        let expected = reference.get(&coords);
                        assert_eq!(actual, expected);
                    }
                }
            }
        }

        proptest! {
            #[test]
            fn proptest_multiindexed_2d(ops in operations_strategy::<2>(10000)) {
                proptest_multiindexed_kd::<2>(ops);
            }

            #[test]
            fn proptest_multiindexed_3d(ops in operations_strategy::<3>(10000)) {
                proptest_multiindexed_kd::<3>(ops);
            }
        }
    }

    #[cfg(loom)]
    mod loom_tests {
        use super::*;
        use crate::std_or_loom::{sync::Arc, thread};

        #[test]
        fn loom_concurrent_insert_get() {
            loom::model(|| {
                let arr = Arc::new(MultiIndexed::<2, i32>::new());

                // Thread 1: Insert values
                let arr1 = Arc::clone(&arr);
                let t1 = thread::spawn(move || {
                    arr1.insert([0, 0], 10);
                    arr1.insert([0, 1], 20);
                    arr1.insert([1, 0], 30);
                });

                // Thread 2: Insert different values
                let arr2 = Arc::clone(&arr);
                let t2 = thread::spawn(move || {
                    arr2.insert([1, 1], 40);
                    arr2.insert([2, 0], 50);
                    arr2.insert([0, 2], 60);
                });

                // Thread 3: Read values
                let arr3 = Arc::clone(&arr);
                let t3 = thread::spawn(move || {
                    // These may or may not be set yet
                    let _ = arr3.get([0, 0]);
                    let _ = arr3.get([1, 1]);
                    let _ = arr3.get([2, 2]); // This one is never set
                });

                t1.join().unwrap();
                t2.join().unwrap();
                t3.join().unwrap();

                // Verify final state
                assert_eq!(arr.get([0, 0]), Some(&10));
                assert_eq!(arr.get([0, 1]), Some(&20));
                assert_eq!(arr.get([1, 0]), Some(&30));
                assert_eq!(arr.get([1, 1]), Some(&40));
                assert_eq!(arr.get([2, 0]), Some(&50));
                assert_eq!(arr.get([0, 2]), Some(&60));
                assert_eq!(arr.get([2, 2]), None);
            });
        }

        #[test]
        fn loom_concurrent_with_negative_coords() {
            loom::model(|| {
                let arr = Arc::new(MultiIndexed::<3, i32>::new());

                // Thread 1: Insert values with negative coordinates
                let arr1 = Arc::clone(&arr);
                let t1 = thread::spawn(move || {
                    arr1.insert([-1, -2, -3], 10);
                    arr1.insert([-1, 0, 1], 20);
                    arr1.insert([0, -2, 3], 30);
                });

                // Thread 2: Insert values with mixed coordinates
                let arr2 = Arc::clone(&arr);
                let t2 = thread::spawn(move || {
                    arr2.insert([1, 1, 1], 40);
                    arr2.insert([2, -3, 0], 50);
                    arr2.insert([-5, 2, -1], 60);
                });

                // Thread 3: Read values
                let arr3 = Arc::clone(&arr);
                let t3 = thread::spawn(move || {
                    // These may or may not be set yet
                    let _ = arr3.get([-1, -2, -3]);
                    let _ = arr3.get([1, 1, 1]);
                    let _ = arr3.get([0, 0, 0]); // This one is never set
                });

                t1.join().unwrap();
                t2.join().unwrap();
                t3.join().unwrap();

                // Verify final state
                assert_eq!(arr.get([-1, -2, -3]), Some(&10));
                assert_eq!(arr.get([-1, 0, 1]), Some(&20));
                assert_eq!(arr.get([0, -2, 3]), Some(&30));
                assert_eq!(arr.get([1, 1, 1]), Some(&40));
                assert_eq!(arr.get([2, -3, 0]), Some(&50));
                assert_eq!(arr.get([-5, 2, -1]), Some(&60));
                assert_eq!(arr.get([0, 0, 0]), None);
            });
        }
    }
}
