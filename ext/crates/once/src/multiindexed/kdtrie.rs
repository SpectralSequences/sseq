use super::node::Node;

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
/// thread-safe properties of the underlying [`TwoEndedGrove`](crate::TwoEndedGrove) data structure.
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
