mod block;

use std::num::NonZero;

use block::Block;

use crate::std_or_loom::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

const MAX_NUM_BLOCKS: usize = 32;

/// An insert-only sparse vector with pinned elements and geometrically growing capacity.
///
/// `Grove` (a pun on "grow vec") is a specialized data structure that provides efficient storage
/// for sparse data with the following key features:
///
/// - **Thread Safety**: Safe for concurrent reads and writes from multiple threads
/// - **Memory Efficiency**: Uses a block-based allocation strategy that grows geometrically
/// - **Pinned Elements**: Once inserted, elements never move in memory
/// - **Sparse Storage**: Efficiently handles sparse data with large gaps between indices
///
/// This data structure is primarily used as the backing store for other collections in this crate,
/// such as [`OnceVec`](crate::OnceVec) and [`MultiIndexed`](crate::MultiIndexed).
///
/// # Implementation Details
///
/// `Grove` uses a series of fixed-size blocks to store elements. Each block's size is a power of 2,
/// and the block number determines its size. This approach allows for efficient memory usage while
/// still supporting large indices without allocating memory for all intermediate elements. We use
/// distinct lazily-allocated blocks to avoid any reallocation, which would invalidate pointers held
/// by other threads.
///
/// # Examples
///
/// ```
/// use once::Grove;
///
/// let grove = Grove::<i32>::new();
///
/// // Insert elements at arbitrary indices
/// grove.insert(0, 10);
/// grove.insert(100, 20);
/// grove.insert(1000, 30);
///
/// // Retrieve elements
/// assert_eq!(grove.get(0), Some(&10));
/// assert_eq!(grove.get(100), Some(&20));
/// assert_eq!(grove.get(1000), Some(&30));
/// assert_eq!(grove.get(50), None); // No element at this index
/// ```
pub struct Grove<T> {
    blocks: [Block<T>; MAX_NUM_BLOCKS],
    /// The maximum index that has been inserted into the `Grove` (exclusive).
    max: AtomicUsize,
}

impl<T> Grove<T> {
    /// Creates a new empty `Grove`.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::Grove;
    ///
    /// let grove = Grove::<i32>::new();
    /// ```
    pub fn new() -> Self {
        Self {
            blocks: std::array::from_fn(|_| Block::new()),
            max: AtomicUsize::new(0),
        }
    }

    /// Finds the block and offset within the block for the given index.
    ///
    /// This is an internal method used to determine which block contains the element
    /// at the specified index and the offset within that block.
    ///
    /// # Parameters
    ///
    /// * `index`: The index to locate
    ///
    /// # Returns
    ///
    /// A tuple containing the block number and the offset within that block.
    fn locate(&self, index: usize) -> (usize, usize) {
        let block_num = (usize::BITS - 1 - (index + 1).leading_zeros()) as usize;
        let block_offset = (index + 1) - (1 << block_num);
        (block_num, block_offset)
    }

    /// Ensures that the specified block is initialized.
    ///
    /// This is an internal method that initializes a block if it hasn't been initialized yet.
    /// It is called before any operation that needs to access a block.
    ///
    /// # Parameters
    ///
    /// * `block_num`: The block number to initialize
    fn ensure_init(&self, block_num: usize) {
        // Safety: `Block::init` is only ever called through this method, and every block has a
        // well-defined `block_num`, and therefore a well-defined size.
        unsafe { self.blocks[block_num].init(NonZero::new(1 << block_num).unwrap()) };
    }

    /// Inserts a value at the specified index.
    ///
    /// This operation is thread-safe and can be called from multiple threads. However, this method
    /// panics if a value already exists at the specified index. Therefore, it should only be called
    /// at most once for any given `index`.
    ///
    /// # Parameters
    ///
    /// * `index`: The index at which to insert the value
    /// * `value`: The value to insert
    ///
    /// # Examples
    ///
    /// ```
    /// use once::Grove;
    ///
    /// let grove = Grove::<i32>::new();
    /// grove.insert(0, 10);
    /// grove.insert(100, 20);
    /// ```
    pub fn insert(&self, index: usize, value: T) {
        let (block_num, block_offset) = self.locate(index);
        self.ensure_init(block_num);
        // Safety: We just initialized the block, and `locate` only returns valid indices
        unsafe { self.blocks[block_num].insert(block_offset, value) };
        self.max.fetch_max(index + 1, Ordering::Release);
    }

    /// Attempts to insert a value at the specified index.
    ///
    /// This method will only insert the value if the index is not already occupied. If the index is
    /// already occupied, the value is returned in the `Err` variant.
    ///
    /// # Parameters
    ///
    /// * `index`: The index at which to insert the value
    /// * `value`: The value to insert
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the value was successfully inserted
    /// * `Err(value)` if the index was already occupied, returning the value that we tried to
    ///   insert
    ///
    /// # Examples
    ///
    /// ```
    /// use once::Grove;
    ///
    /// let grove = Grove::<i32>::new();
    /// assert_eq!(grove.try_insert(0, 10), Ok(()));
    /// assert_eq!(grove.try_insert(0, 20), Err(20)); // Index already occupied
    /// ```
    pub fn try_insert(&self, index: usize, value: T) -> Result<(), T> {
        let (block_num, block_offset) = self.locate(index);
        self.ensure_init(block_num);
        // Safety: We just initialized the block, and `locate` only returns valid indices
        let ret = unsafe { self.blocks[block_num].try_insert(block_offset, value) };
        if ret.is_ok() {
            self.max.fetch_max(index + 1, Ordering::Release);
        }
        ret
    }

    /// Retrieves a reference to the value at the specified index, if it exists.
    ///
    /// # Parameters
    ///
    /// * `index`: The index of the value to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(&T)` if a value exists at the specified index
    /// * `None` if no value exists at the specified index
    ///
    /// # Examples
    ///
    /// ```
    /// use once::Grove;
    ///
    /// let grove = Grove::<i32>::new();
    /// grove.insert(0, 10);
    ///
    /// assert_eq!(grove.get(0), Some(&10));
    /// assert_eq!(grove.get(1), None);
    /// ```
    pub fn get(&self, index: usize) -> Option<&T> {
        let (block_num, block_offset) = self.locate(index);
        if self.blocks[block_num].is_init() {
            // Safety: We just observed that the block is initialized
            unsafe { self.blocks[block_num].get(block_offset) }
        } else {
            None
        }
    }

    /// Retrieves a mutable reference to the value at the specified index, if it exists.
    ///
    /// This method can only be called if we have an exclusive reference to self, which may not be
    /// very common in practice. However, it is useful for `Drop` implementations.
    ///
    /// # Parameters
    ///
    /// * `index`: The index of the value to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(&mut T)` if a value exists at the specified index
    /// * `None` if no value exists at the specified index
    ///
    /// # Examples
    ///
    /// ```
    /// use once::Grove;
    ///
    /// let mut grove = Grove::<i32>::new();
    /// grove.insert(0, 10);
    ///
    /// if let Some(value) = grove.get_mut(0) {
    ///     *value = 20;
    /// }
    ///
    /// assert_eq!(grove.get(0), Some(&20));
    /// ```
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let (block_num, block_offset) = self.locate(index);
        self.blocks[block_num].get_mut(block_offset)
    }

    /// Retrieves a reference to the value at the specified index without checking that a value
    /// exists.
    ///
    /// # Safety
    ///
    /// A value must have been previously inserted at the specified index.
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        let (block_num, block_offset) = self.locate(index);
        // We know that we already observed the block to be initialized, so we can get away with a
        // relaxed load.
        let data_ptr = self.blocks[block_num].data().load(Ordering::Relaxed);
        unsafe { (*data_ptr.add(block_offset)).get_unchecked() }
    }

    /// Checks if a value exists at the specified index.
    ///
    /// # Parameters
    ///
    /// * `index`: The index to check
    ///
    /// # Returns
    ///
    /// `true` if a value exists at the specified index, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::Grove;
    ///
    /// let grove = Grove::<i32>::new();
    /// grove.insert(0, 10);
    ///
    /// assert!(grove.is_set(0));
    /// assert!(!grove.is_set(1));
    /// ```
    pub fn is_set(&self, index: usize) -> bool {
        let (block_num, block_offset) = self.locate(index);
        if self.blocks[block_num].is_init() {
            // Safety: We just observed that the block is initialized
            unsafe { self.blocks[block_num].is_set(block_offset) }
        } else {
            false
        }
    }

    /// Returns the length of the `Grove`.
    ///
    /// The length is defined as the maximum index that has been inserted into the `Grove` plus one.
    /// Note that this does not necessarily reflect the number of elements in the `Grove`, as there
    /// may be gaps. Also, since this is a wait-free data structure, the return value may not be
    /// accurate, but it will always be a lower bound for the true length from the perspective of
    /// any thread.
    ///
    /// # Returns
    ///
    /// The length of the `Grove`.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::Grove;
    ///
    /// let grove = Grove::<i32>::new();
    /// grove.insert(0, 10);
    /// grove.insert(5, 20);
    ///
    /// assert_eq!(grove.len(), 6); // 5 + 1
    /// ```
    pub fn len(&self) -> usize {
        self.max.load(Ordering::Acquire)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Creates a `Grove` from a `Vec`.
    ///
    /// The elements in the vector will be inserted into the `Grove` at their respective indices.
    ///
    /// # Parameters
    ///
    /// * `v`: The vector to convert
    ///
    /// # Returns
    ///
    /// A new `Grove` containing the elements from the vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::Grove;
    ///
    /// let vec = vec![10, 20, 30];
    /// let grove = Grove::from_vec(vec);
    ///
    /// assert_eq!(grove.get(0), Some(&10));
    /// assert_eq!(grove.get(1), Some(&20));
    /// assert_eq!(grove.get(2), Some(&30));
    /// ```
    pub fn from_vec(v: Vec<T>) -> Self {
        let grove = Self::new();
        for (i, value) in v.into_iter().enumerate() {
            grove.insert(i, value);
        }
        grove
    }

    /// Returns an iterator over the values in the `Grove`.
    ///
    /// The iterator yields values in order of their indices, from 0 to `len() - 1`.
    /// Indices that don't have a value are skipped.
    ///
    /// # Returns
    ///
    /// An iterator over the values in the `Grove`.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::Grove;
    ///
    /// let grove = Grove::<i32>::new();
    /// grove.insert(0, 10);
    /// grove.insert(2, 30);
    ///
    /// let values: Vec<_> = grove.iter().collect();
    /// assert_eq!(values, vec![&10, &30]);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.len()).filter_map(move |i| self.get(i))
    }
}

impl<T> Default for Grove<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A bidirectional sparse vector that supports both positive and negative indices.
///
/// `TwoEndedGrove` extends the functionality of [`Grove`] by allowing elements to be indexed using
/// both positive and negative integers. It maintains two separate `Grove` instances: one for
/// non-negative indices and another for negative indices.
///
/// This data structure is primarily used as the backing store for the nodes in
/// [`KdTrie`](crate::multiindexed::KdTrie).
///
/// # Examples
///
/// ```
/// use once::TwoEndedGrove;
///
/// let grove = TwoEndedGrove::<i32>::new();
///
/// // Insert elements at both positive and negative indices
/// grove.insert(-5, 10);
/// grove.insert(0, 20);
/// grove.insert(5, 30);
///
/// // Retrieve elements
/// assert_eq!(grove.get(-5), Some(&10));
/// assert_eq!(grove.get(0), Some(&20));
/// assert_eq!(grove.get(5), Some(&30));
/// assert_eq!(grove.get(-2), None); // No element at this index
///
/// // Get the range of valid indices
/// assert_eq!(grove.range(), -5..6);
/// ```
pub struct TwoEndedGrove<T> {
    non_neg: Grove<T>,
    neg: Grove<T>,
    min: AtomicI32,
    max: AtomicI32,
}

impl<T> TwoEndedGrove<T> {
    /// Creates a new empty `TwoEndedGrove`.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::TwoEndedGrove;
    ///
    /// let grove = TwoEndedGrove::<i32>::new();
    /// ```
    pub fn new() -> Self {
        Self {
            non_neg: Grove::new(),
            neg: Grove::new(),
            min: AtomicI32::new(i32::MAX),
            max: AtomicI32::new(i32::MIN + 1),
        }
    }

    /// Inserts a value at the specified index.
    ///
    /// This operation is thread-safe and can be called from multiple threads. This method panics if
    /// a value already exists at the specified index.
    ///
    /// # Parameters
    ///
    /// * `idx`: The index at which to insert the value
    /// * `value`: The value to insert
    ///
    /// # Examples
    ///
    /// ```
    /// use once::TwoEndedGrove;
    ///
    /// let grove = TwoEndedGrove::<i32>::new();
    /// grove.insert(-5, 10);
    /// grove.insert(5, 20);
    /// ```
    pub fn insert(&self, idx: i32, value: T) {
        if idx >= 0 {
            self.non_neg.insert(idx as usize, value);
        } else {
            self.neg.insert((-idx) as usize, value);
        }
        self.max.fetch_max(idx + 1, Ordering::Relaxed);
        self.min.fetch_min(idx, Ordering::Release);
    }

    /// Attempts to insert a value at the specified index.
    ///
    /// This method will only insert the value if the index is not already occupied. If the index is
    /// already occupied, the value is returned in the `Err` variant.
    ///
    /// # Parameters
    ///
    /// * `idx`: The index at which to insert the value
    /// * `value`: The value to insert
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the value was successfully inserted
    /// * `Err(value)` if the index was already occupied, returning the value that we tried to
    ///   insert
    ///
    /// # Examples
    ///
    /// ```
    /// use once::TwoEndedGrove;
    ///
    /// let grove = TwoEndedGrove::<i32>::new();
    /// assert_eq!(grove.try_insert(-5, 10), Ok(()));
    /// assert_eq!(grove.try_insert(-5, 20), Err(20)); // Index already occupied
    /// ```
    pub fn try_insert(&self, idx: i32, value: T) -> Result<(), T> {
        let ret = if idx >= 0 {
            self.non_neg.try_insert(idx as usize, value)
        } else {
            self.neg.try_insert((-idx) as usize, value)
        };
        if ret.is_ok() {
            self.max.fetch_max(idx + 1, Ordering::Relaxed);
            self.min.fetch_min(idx, Ordering::Release);
        }
        ret
    }

    /// Retrieves a reference to the value at the specified index, if it exists.
    ///
    /// # Parameters
    ///
    /// * `idx`: The index of the value to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(&T)` if a value exists at the specified index
    /// * `None` if no value exists at the specified index
    ///
    /// # Examples
    ///
    /// ```
    /// use once::TwoEndedGrove;
    ///
    /// let grove = TwoEndedGrove::<i32>::new();
    /// grove.insert(-5, 10);
    ///
    /// assert_eq!(grove.get(-5), Some(&10));
    /// assert_eq!(grove.get(0), None);
    /// ```
    pub fn get(&self, idx: i32) -> Option<&T> {
        if idx >= 0 {
            self.non_neg.get(idx as usize)
        } else {
            self.neg.get((-idx) as usize)
        }
    }

    /// Retrieves a mutable reference to the value at the specified index, if it exists.
    ///
    /// This method can only be called if we have an exclusive reference to self, which may not be
    /// very common in practice. It is mainly used for the `Drop` implementation of the
    /// `MultiIndexed`, which allows to use non-atomic operations for performance.
    ///
    /// # Parameters
    ///
    /// * `idx`: The index of the value to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(&mut T)` if a value exists at the specified index
    /// * `None` if no value exists at the specified index
    ///
    /// # Examples
    ///
    /// ```
    /// use once::TwoEndedGrove;
    ///
    /// let mut grove = TwoEndedGrove::<i32>::new();
    /// grove.insert(-5, 10);
    ///
    /// if let Some(value) = grove.get_mut(-5) {
    ///     *value = 20;
    /// }
    ///
    /// assert_eq!(grove.get(-5), Some(&20));
    /// ```
    pub fn get_mut(&mut self, idx: i32) -> Option<&mut T> {
        if idx >= 0 {
            self.non_neg.get_mut(idx as usize)
        } else {
            self.neg.get_mut((-idx) as usize)
        }
    }

    /// Returns the range of indices that have values in the `TwoEndedGrove`.
    ///
    /// We return both the minimum and maximum indices at the same time to optimize memory
    /// orderings.
    ///
    /// # Returns
    ///
    /// A [`Range`](std::ops::Range) representing the range of indices that have values in the
    /// `TwoEndedGrove`. It is not guaranteed that all indices in the range have values. However, it
    /// is guaranteed that only the indices in the range have values *if* the `TwoEndedGrove` is not
    /// being concurrently modified.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::TwoEndedGrove;
    ///
    /// let grove = TwoEndedGrove::<i32>::new();
    /// grove.insert(-5, 10);
    /// grove.insert(0, 20);
    /// grove.insert(5, 30);
    ///
    /// assert_eq!(grove.range(), -5..6);
    /// ```
    pub fn range(&self) -> std::ops::Range<i32> {
        // We use a Relaxed max load because all insertion operations end with a release-store of
        // the min. We have a chain of happens-before relationships:
        // load max <- acquire-load min <- release-store min <- store max
        self.min.load(Ordering::Acquire)..self.max.load(Ordering::Relaxed)
    }

    /// Checks if a value exists at the specified index.
    ///
    /// # Parameters
    ///
    /// * `idx`: The index to check
    ///
    /// # Returns
    ///
    /// `true` if a value exists at the specified index, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::TwoEndedGrove;
    ///
    /// let grove = TwoEndedGrove::<i32>::new();
    /// grove.insert(-5, 10);
    ///
    /// assert!(grove.is_set(-5));
    /// assert!(!grove.is_set(0));
    /// ```
    pub fn is_set(&self, idx: i32) -> bool {
        if idx >= 0 {
            self.non_neg.is_set(idx as usize)
        } else {
            self.neg.is_set((-idx) as usize)
        }
    }

    /// Applies a function to each value in the `TwoEndedGrove`.
    ///
    /// This method iterates over all values in the `TwoEndedGrove` and applies the provided
    /// function to each one.
    ///
    /// This requires a mutable reference to the `TwoEndedGrove`, which may not be very common in
    /// practice. It is mainly used for the `Drop` implementation of `MultiIndexed`, which requires
    /// iterating over mutable references to all values. Simply returning an iterator over the
    /// entries that contain values would not be sufficient, as it would hold a reference to `self`
    /// and not allow mutable access to the internal values.
    ///
    /// # Parameters
    ///
    /// * `f`: The function to apply to each value
    ///
    /// # Examples
    ///
    /// ```
    /// use once::TwoEndedGrove;
    ///
    /// let mut grove = TwoEndedGrove::<i32>::new();
    /// grove.insert(-5, 10);
    /// grove.insert(0, 20);
    /// grove.insert(5, 30);
    ///
    /// // Double each value
    /// grove.for_each_mut(|value| *value *= 2);
    ///
    /// assert_eq!(grove.get(-5), Some(&20));
    /// assert_eq!(grove.get(0), Some(&40));
    /// assert_eq!(grove.get(5), Some(&60));
    /// ```
    pub fn for_each_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T),
    {
        // I would have liked to use `.filter_map(...).for_each(f)` but that gives me issues with
        // returning lifetimes from closures.
        for idx in self.range() {
            if let Some(value) = self.get_mut(idx) {
                f(value);
            }
        }
    }
}

impl<T> Default for TwoEndedGrove<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        thread,
    };

    use super::*;

    #[test]
    fn test_locate() {
        let vec = Grove::<i32>::new();
        assert_eq!(vec.locate(0), (0, 0));
        assert_eq!(vec.locate(1), (1, 0));
        assert_eq!(vec.locate(2), (1, 1));
        assert_eq!(vec.locate(3), (2, 0));
        assert_eq!(vec.locate(4), (2, 1));
        assert_eq!(vec.locate(5), (2, 2));
        assert_eq!(vec.locate(6), (2, 3));
        assert_eq!(vec.locate(7), (3, 0));
        assert_eq!(vec.locate(8), (3, 1));
        assert_eq!(vec.locate(9), (3, 2));
        assert_eq!(vec.locate(10), (3, 3));
        assert_eq!(vec.locate(11), (3, 4));
        assert_eq!(vec.locate(12), (3, 5));
        assert_eq!(vec.locate(13), (3, 6));
        assert_eq!(vec.locate(14), (3, 7));
        assert_eq!(vec.locate(15), (4, 0));
        assert_eq!(vec.locate(16), (4, 1));
        assert_eq!(vec.locate(17), (4, 2));
        // This should be good enough
    }

    #[test]
    fn test_grove_insert_get() {
        let v = Grove::<i32>::new();
        assert!(v.get(42).is_none());
        v.insert(42, 42);
        assert_eq!(v.get(42), Some(&42));
    }

    #[test]
    fn test_grove_requires_drop() {
        use std::sync::Arc;

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

        let v = Arc::new(Grove::<DropCounter>::new());
        assert_eq!(ACTIVE_ALLOCS.load(Ordering::Relaxed), 0);

        let num_threads = crate::test_utils::num_threads();
        let inserts_per_thread = crate::test_utils::values_per_thread();

        thread::scope(|s| {
            for thread_id in 0..num_threads {
                let v = Arc::clone(&v);
                s.spawn(move || {
                    for i in 0..inserts_per_thread {
                        v.insert(thread_id * inserts_per_thread + i, DropCounter::new());
                    }
                });
            }
        });

        assert_eq!(
            ACTIVE_ALLOCS.load(Ordering::Relaxed),
            num_threads * inserts_per_thread
        );

        drop(v);

        assert_eq!(ACTIVE_ALLOCS.load(Ordering::Relaxed), 0);
    }

    fn grove_high_contention(num_threads: usize) {
        use crate::std_or_loom::{sync::Arc, thread};

        let vec = Arc::new(Grove::<usize>::new());

        let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
                let vec = Arc::clone(&vec);
                thread::spawn(move || {
                    for i in 0..10 {
                        vec.insert(thread_id * 10 + i, thread_id);
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        for thread_id in 0..num_threads {
            for i in 0..10 {
                assert_eq!(
                    vec.get(thread_id * 10 + i),
                    Some(&thread_id),
                    "Value mismatch at index {}",
                    thread_id * 10 + i
                );
            }
        }
    }

    #[test]
    fn test_grove_high_contention() {
        grove_high_contention(crate::test_utils::num_threads());
    }

    #[cfg(loom)]
    #[test]
    fn loom_grove_contention() {
        loom::model(|| grove_high_contention(2));
    }

    // TwoEndedGrove tests

    #[test]
    fn test_two_ended_grove_basic() {
        let grove = TwoEndedGrove::<i32>::new();

        // Insert values at positive and negative indices
        grove.insert(-5, 10);
        grove.insert(-2, 20);
        grove.insert(0, 30);
        grove.insert(3, 40);
        grove.insert(5, 50);

        // Check values
        assert_eq!(grove.get(-5), Some(&10));
        assert_eq!(grove.get(-2), Some(&20));
        assert_eq!(grove.get(0), Some(&30));
        assert_eq!(grove.get(3), Some(&40));
        assert_eq!(grove.get(5), Some(&50));

        // Check non-existent values
        assert_eq!(grove.get(-10), None);
        assert_eq!(grove.get(-3), None);
        assert_eq!(grove.get(1), None);
        assert_eq!(grove.get(10), None);

        // Check min and max
        assert_eq!(grove.range(), -5..6);
    }

    #[test]
    fn test_two_ended_grove_try_insert() {
        let grove = TwoEndedGrove::<i32>::new();

        // Insert values
        assert!(grove.try_insert(-5, 10).is_ok());
        assert!(grove.try_insert(5, 20).is_ok());

        // Try to insert at the same indices
        assert!(grove.try_insert(-5, 30).is_err());
        assert!(grove.try_insert(5, 40).is_err());

        // Check values
        assert_eq!(grove.get(-5), Some(&10));
        assert_eq!(grove.get(5), Some(&20));
    }

    #[test]
    fn test_two_ended_grove_is_set() {
        let grove = TwoEndedGrove::<i32>::new();

        // Insert values
        grove.insert(-5, 10);
        grove.insert(5, 20);

        // Check is_set
        assert!(grove.is_set(-5));
        assert!(grove.is_set(5));
        assert!(!grove.is_set(-10));
        assert!(!grove.is_set(0));
        assert!(!grove.is_set(10));
    }

    #[test]
    fn test_two_ended_grove_for_each_mut() {
        let mut grove = TwoEndedGrove::<i32>::new();

        // Insert values
        grove.insert(-5, 10);
        grove.insert(-2, 20);
        grove.insert(0, 30);
        grove.insert(3, 40);
        grove.insert(5, 50);

        // Double each value
        grove.for_each_mut(|value| *value *= 2);

        // Check values
        assert_eq!(grove.get(-5), Some(&20));
        assert_eq!(grove.get(-2), Some(&40));
        assert_eq!(grove.get(0), Some(&60));
        assert_eq!(grove.get(3), Some(&80));
        assert_eq!(grove.get(5), Some(&100));
    }

    #[test]
    fn test_two_ended_grove_get_mut() {
        let mut grove = TwoEndedGrove::<i32>::new();

        // Insert values
        grove.insert(-5, 10);
        grove.insert(5, 20);

        // Modify values
        if let Some(value) = grove.get_mut(-5) {
            *value = 30;
        }
        if let Some(value) = grove.get_mut(5) {
            *value = 40;
        }

        // Check values
        assert_eq!(grove.get(-5), Some(&30));
        assert_eq!(grove.get(5), Some(&40));
    }

    #[test]
    fn test_two_ended_grove_concurrent() {
        use std::sync::Arc;

        let grove = Arc::new(TwoEndedGrove::<i32>::new());
        let num_threads = crate::test_utils::num_threads() as i32;
        let values_per_thread = crate::test_utils::values_per_thread() as i32;

        let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
                let grove = Arc::clone(&grove);
                thread::spawn(move || {
                    for i in (-values_per_thread / 2)..(values_per_thread / 2) {
                        let value = thread_id * values_per_thread + i;
                        grove.insert(value, value);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify values
        for thread_id in 0..num_threads {
            for i in (-values_per_thread / 2)..(values_per_thread / 2) {
                let value = thread_id * values_per_thread + i;
                assert_eq!(grove.get(value), Some(&value));
            }
        }
    }

    #[test]
    fn test_two_ended_grove_positive_min() {
        let grove = TwoEndedGrove::<i32>::new();

        // Insert values
        grove.insert(3, 30);
        grove.insert(5, 50);

        // Check min
        assert_eq!(grove.range(), 3..6);
    }

    #[test]
    fn test_two_ended_grove_negative_max() {
        let grove = TwoEndedGrove::<i32>::new();

        // Insert values
        grove.insert(-3, 30);
        grove.insert(-5, 50);

        // Check max
        assert_eq!(grove.range(), -5..-2);
    }
}
