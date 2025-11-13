use self::iter::KdIterator;
pub use self::kdtrie::KdTrie;

mod iter;
pub mod kdtrie;
mod node;

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
/// [`TwoEndedGrove`](crate::TwoEndedGrove) instances to store the trie nodes, which themselves use
/// a block-based allocation strategy.
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

impl<const K: usize, V> Default for MultiIndexed<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const K: usize, V: std::fmt::Debug> std::fmt::Debug for MultiIndexed<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<const K: usize, V> Clone for MultiIndexed<K, V>
where
    V: Clone,
{
    fn clone(&self) -> Self {
        let new_mi = Self::new();
        for (coords, value) in self.iter() {
            new_mi.insert(coords, value.clone());
        }
        new_mi
    }
}

impl<const K: usize, V> PartialEq for MultiIndexed<K, V>
where
    V: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<const K: usize, V> Eq for MultiIndexed<K, V> where V: Eq {}

impl<const K: usize, V: std::hash::Hash> std::hash::Hash for MultiIndexed<K, V> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for (coords, value) in self.iter() {
            coords.hash(state);
            value.hash(state);
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
    fn test_iter_empty() {
        let arr = MultiIndexed::<2, i32>::new();
        let items: Vec<_> = arr.iter().collect();
        assert_eq!(items, vec![]);
    }

    #[test]
    fn test_iter_multiple_calls() {
        let arr = MultiIndexed::<2, i32>::new();
        arr.insert([1, 2], 10);
        arr.insert([3, 4], 20);

        // Multiple calls to iter should return the same items
        let items1: Vec<_> = arr.iter().collect();
        let items2: Vec<_> = arr.iter().collect();

        assert_eq!(items1, items2);
    }

    #[test]
    fn test_requires_drop() {
        use std::{
            sync::{
                Arc,
                atomic::{AtomicUsize, Ordering},
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

    #[test]
    fn test_debug() {
        let arr = MultiIndexed::<2, i32>::new();
        arr.insert([1, 2], 10);
        arr.insert([3, 4], 20);
        arr.insert([3, -4], 30);
        arr.insert([-5, 6], 40);

        expect_test::expect![[r#"
            {
                [
                    -5,
                    6,
                ]: 40,
                [
                    1,
                    2,
                ]: 10,
                [
                    3,
                    -4,
                ]: 30,
                [
                    3,
                    4,
                ]: 20,
            }
        "#]]
        .assert_debug_eq(&arr);
    }

    #[test]
    fn test_clone() {
        let arr = MultiIndexed::<2, i32>::new();
        arr.insert([1, 2], 10);
        arr.insert([3, 4], 20);

        let cloned_arr = arr.clone();

        assert_eq!(cloned_arr.get([1, 2]), Some(&10));
        assert_eq!(cloned_arr.get([3, 4]), Some(&20));
        assert_eq!(cloned_arr.get([5, 6]), None);
    }

    #[cfg(not(miri))]
    mod proptests {
        use std::collections::HashMap;

        use proptest::prelude::*;

        use super::*;

        // Return `max` such that the max length is twice the number of elements in the hypercube
        // (-max..=max)^K
        fn max_from_max_len<const K: usize>(max_len: usize) -> u32 {
            ((max_len as f32 / 2.0).powf(1.0 / K as f32) / 2.0).ceil() as u32
        }

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

        fn insert_strategy<const K: usize>(max: u32) -> impl Strategy<Value = Operation<K>> {
            coords_strategy::<K>(max).prop_flat_map(move |coords| {
                any::<i32>().prop_map(move |value| Operation::Insert(coords, value))
            })
        }

        fn get_strategy<const K: usize>(max: u32) -> impl Strategy<Value = Operation<K>> {
            coords_strategy::<K>(max).prop_map(Operation::Get)
        }

        // Generate a strategy for a single operation (insert or get)
        fn operation_strategy<const K: usize>(max: u32) -> impl Strategy<Value = Operation<K>> {
            prop_oneof![insert_strategy(max), get_strategy(max)]
        }

        // Generate a strategy for vectors of i32 coordinates
        fn coords_vec_strategy<const K: usize>(
            max_len: usize,
        ) -> impl Strategy<Value = Vec<[i32; K]>> {
            let size = max_from_max_len::<K>(max_len);
            proptest::collection::vec(coords_strategy::<K>(size), 1..=max_len)
        }

        // Generate a strategy for a list of operations (insert or get)
        fn operations_strategy<const K: usize>(
            max_ops: usize,
        ) -> impl Strategy<Value = Vec<Operation<K>>> {
            let max = max_from_max_len::<K>(max_ops);
            proptest::collection::vec(operation_strategy::<K>(max), 1..=max_ops)
        }

        fn proptest_multiindexed_ops_kd<const K: usize>(ops: Vec<Operation<K>>) {
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

        fn proptest_multiindexed_iter_kd<const K: usize>(coords: Vec<[i32; K]>) {
            let arr = MultiIndexed::<K, usize>::new();
            let mut tagged_coords = vec![];
            for (i, coord) in coords.iter().enumerate() {
                if arr.try_insert(*coord, i).is_ok() {
                    // Only insert if the coordinate was not already present
                    tagged_coords.push((*coord, i));
                };
            }

            let items: Vec<_> = arr.iter().map(|(coord, value)| (coord, *value)).collect();
            assert_eq!(items.len(), tagged_coords.len());

            tagged_coords.sort();
            assert_eq!(tagged_coords, items);
        }

        const MAX_LEN: usize = 10_000;

        proptest! {
            #[test]
            fn proptest_multiindexed_ops_2d(ops in operations_strategy::<2>(MAX_LEN)) {
                proptest_multiindexed_ops_kd::<2>(ops);
            }

            #[test]
            fn proptest_multiindexed_ops_3d(ops in operations_strategy::<3>(MAX_LEN)) {
                proptest_multiindexed_ops_kd::<3>(ops);
            }

            #[test]
            fn proptest_multiindexed_iter_2d(coords in coords_vec_strategy::<2>(MAX_LEN)) {
                proptest_multiindexed_iter_kd::<2>(coords);
            }

            #[test]
            fn proptest_multiindexed_iter_3d(coords in coords_vec_strategy::<3>(MAX_LEN)) {
                proptest_multiindexed_iter_kd::<3>(coords);
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
