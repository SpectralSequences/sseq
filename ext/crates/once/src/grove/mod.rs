mod block;
mod zeroed;

use std::ops::{Index, IndexMut};

pub(crate) use zeroed::{ZeroedGrove, ZeroedTwoEndedGrove};

use crate::write_once::WriteOnce;

/// A sparse vector with pinned elements and geometrically growing capacity.
///
/// `Grove` (a pun on "grow vec") is a specialized data structure that provides efficient storage
/// for data with the following key features:
///
/// - **Thread Safety**: Safe for concurrent reads and writes from multiple threads
/// - **Memory Efficiency**: Uses a block-based allocation strategy that grows geometrically
/// - **Pinned Elements**: Once inserted, elements never move in memory
/// - **Sparse Storage**: Efficiently handles sparse data with large gaps between indices
///
/// This data structure is primarily used as the backing store for other collections in this crate,
/// such as [`OnceVec`](crate::OnceVec) and [`MultiIndexed`](crate::MultiIndexed).
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
///
/// A `Grove` can be shared across threads only when its values can be sent across them too:
///
/// ```compile_fail
/// fn assert_sync<T: Sync>() {}
/// assert_sync::<once::Grove<std::sync::MutexGuard<'static, i32>>>();
/// ```
#[repr(transparent)]
pub struct Grove<T>(ZeroedGrove<WriteOnce<T>>);

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
        Self(ZeroedGrove::new())
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
        let grove = Self::default();
        for (i, value) in v.into_iter().enumerate() {
            grove.insert(i, value);
        }
        grove
    }
}

impl<T> Grove<T> {
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
        self.0.entry(index).set(value);
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
        self.0.entry(index).try_set(value)
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
        self.0.get_entry(index)?.get()
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
        self.0.get_entry_mut(index)?.get_mut()
    }

    /// Retrieves a reference to the value at the specified index without checking that a value
    /// exists.
    ///
    /// # Safety
    ///
    /// A value must have been previously inserted at the specified index.
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        // SAFETY: a value was inserted at `index`, so its block is allocated and its cell is set.
        unsafe { self.0.get_entry_unchecked(index).get_unchecked() }
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
        self.0.get_entry(index).is_some_and(WriteOnce::is_set)
    }

    /// Returns the length of the `Grove`.
    ///
    /// The length is defined as the maximum index that has been inserted into the `Grove` plus one.
    /// Note that this does not necessarily reflect the number of elements in the `Grove`, as there
    /// may be gaps. Also, since this is a wait-free data structure, a concurrent reader may see a
    /// stale length, or one that counts an index whose value another thread is still inserting.
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
        self.0.len()
    }

    /// Whether no value has been inserted.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the index-value pairs in the `Grove`.
    ///
    /// The iterator yields pairs in order of their indices, from 0 to `len() - 1`.
    /// Indices that don't have a value are skipped.
    ///
    /// # Returns
    ///
    /// An iterator over `(usize, &T)` pairs in the `Grove`.
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
    /// let entries: Vec<_> = grove.iter().collect();
    /// assert_eq!(entries, vec![(0, &10), (2, &30)]);
    /// ```
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            grove: self,
            pos: 0,
            len: self.len(),
        }
    }

    /// Returns an iterator over the index-value pairs in the `Grove`, with mutable values.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, &mut T)> {
        self.0
            .entries_mut()
            .filter_map(|(i, cell)| cell.get_mut().map(|value| (i, value)))
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
    /// let values: Vec<_> = grove.values().collect();
    /// assert_eq!(values, vec![&10, &30]);
    /// ```
    pub fn values(&self) -> Values<'_, T> {
        Values(self.iter())
    }
}

/// An iterator over the index-value pairs in a [`Grove`].
///
/// Created by [`Grove::iter`].
pub struct Iter<'a, T> {
    grove: &'a Grove<T>,
    pos: usize,
    len: usize,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.len {
            let i = self.pos;
            self.pos += 1;
            if let Some(value) = self.grove.get(i) {
                return Some((i, value));
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Grove is sparse, so we can't know the exact count without scanning.
        (0, Some(self.len - self.pos))
    }
}

/// An iterator over the values in a [`Grove`].
///
/// Created by [`Grove::values`].
pub struct Values<'a, T>(Iter<'a, T>);

impl<'a, T> Iterator for Values<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<T> std::iter::FusedIterator for Iter<'_, T> {}

impl<T> std::iter::FusedIterator for Values<'_, T> {}

impl<'a, T> IntoIterator for &'a Grove<T> {
    type IntoIter = Iter<'a, T>;
    type Item = (usize, &'a T);

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T> Default for Grove<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Grove<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_map = f.debug_map();
        for (i, val) in self.iter() {
            debug_map.entry(&i, val);
        }
        debug_map.finish()
    }
}

impl<T: Clone> Clone for Grove<T> {
    fn clone(&self) -> Self {
        let new_grove = Self::default();
        for (i, value) in self.iter() {
            new_grove.insert(i, value.clone());
        }
        new_grove
    }
}

impl<T: PartialEq> PartialEq for Grove<T> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<T: Eq> Eq for Grove<T> {}

impl<T: std::hash::Hash> std::hash::Hash for Grove<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for (i, value) in self.iter() {
            i.hash(state);
            value.hash(state);
        }
    }
}

impl<T> Index<usize> for Grove<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
            .unwrap_or_else(|| panic!("no value at index {index:?}"))
    }
}

impl<T> IndexMut<usize> for Grove<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index)
            .unwrap_or_else(|| panic!("no value at index {index:?}"))
    }
}

/// A bidirectional sparse vector that supports both positive and negative indices.
///
/// `TwoEndedGrove` extends the functionality of [`Grove`] by allowing elements to be indexed using
/// both positive and negative integers.
///
/// This data structure is primarily used as the backing store for the leaves of
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
#[repr(transparent)]
pub struct TwoEndedGrove<T>(ZeroedTwoEndedGrove<WriteOnce<T>>);

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
        Self(ZeroedTwoEndedGrove::new())
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
        self.0.entry(idx).set(value);
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
        self.0.entry(idx).try_set(value)
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
        self.0.get_entry(idx)?.get()
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
        self.0.get_entry_mut(idx)?.get_mut()
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
        self.0.range()
    }

    /// Returns an iterator over the index-value pairs, non-negative indices first.
    pub fn iter(&self) -> impl Iterator<Item = (i32, &T)> {
        self.0
            .entries()
            .filter_map(|(idx, cell)| cell.get().map(|value| (idx, value)))
    }

    /// Returns an iterator over the index-value pairs with mutable values, non-negative indices
    /// first.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (i32, &mut T)> {
        self.0
            .entries_mut()
            .filter_map(|(idx, cell)| cell.get_mut().map(|value| (idx, value)))
    }

    /// Returns an iterator over the values, non-negative indices first.
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.iter().map(move |(_, value)| value)
    }

    /// Returns an iterator over the values, mutably, non-negative indices first.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.iter_mut().map(move |(_, value)| value)
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
        self.0.get_entry(idx).is_some_and(WriteOnce::is_set)
    }
}

impl<T> Default for TwoEndedGrove<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for TwoEndedGrove<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_map = f.debug_map();
        for (i, val) in self.iter() {
            debug_map.entry(&i, val);
        }
        debug_map.finish()
    }
}

impl<T: Clone> Clone for TwoEndedGrove<T> {
    fn clone(&self) -> Self {
        let new_grove = Self::default();
        for (i, value) in self.iter() {
            new_grove.insert(i, value.clone());
        }
        new_grove
    }
}

impl<T: PartialEq> PartialEq for TwoEndedGrove<T> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<T: Eq> Eq for TwoEndedGrove<T> {}

impl<T: std::hash::Hash> std::hash::Hash for TwoEndedGrove<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for (i, value) in self.iter() {
            i.hash(state);
            value.hash(state);
        }
    }
}

impl<T> Index<i32> for TwoEndedGrove<T> {
    type Output = T;

    fn index(&self, index: i32) -> &Self::Output {
        self.get(index)
            .unwrap_or_else(|| panic!("no value at index {index:?}"))
    }
}

impl<T> IndexMut<i32> for TwoEndedGrove<T> {
    fn index_mut(&mut self, index: i32) -> &mut Self::Output {
        self.get_mut(index)
            .unwrap_or_else(|| panic!("no value at index {index:?}"))
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
    fn test_grove_insert_get() {
        let v = Grove::<i32>::new();
        assert!(v.get(42).is_none());
        v.insert(42, 42);
        assert_eq!(v.get(42), Some(&42));
    }

    #[test]
    fn test_grove_debug() {
        let v = Grove::<i32>::new();
        v.insert(0, 1);
        v.insert(100, 2);
        v.insert(1000, 3);

        expect_test::expect![[r#"
            {
                0: 1,
                100: 2,
                1000: 3,
            }
        "#]]
        .assert_debug_eq(&v);
    }

    #[test]
    fn test_grove_clone() {
        let v = Grove::<i32>::new();
        v.insert(0, 1);
        v.insert(100, 2);
        v.insert(1000, 3);

        let v2 = v.clone();

        assert_ne!(&raw const v, &raw const v2);
        assert_eq!(v, v2);
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
    fn test_grove_iter_mut_no_aliasing() {
        let mut grove = Grove::<i32>::new();
        grove.insert(0, 10);
        grove.insert(1, 20);
        grove.insert(3, 30);
        grove.insert(7, 40);

        let mut it = grove.iter_mut();
        let (_, a) = it.next().unwrap();
        let (_, b) = it.next().unwrap();
        let (_, c) = it.next().unwrap();
        let (_, d) = it.next().unwrap();

        // Miri detects borrow-model violations if any of the references alias.
        *a += 1;
        *b += 2;
        *c += 3;
        *d += 4;
        drop(it);

        assert_eq!(grove.get(0), Some(&11));
        assert_eq!(grove.get(1), Some(&22));
        assert_eq!(grove.get(3), Some(&33));
        assert_eq!(grove.get(7), Some(&44));
    }

    #[test]
    fn test_two_ended_grove_iter_mut_no_aliasing() {
        let mut grove = TwoEndedGrove::<i32>::new();
        grove.insert(-3, 10);
        grove.insert(-1, 20);
        grove.insert(0, 30);
        grove.insert(5, 40);

        // Hold all four &mut references simultaneously.
        let mut it = grove.iter_mut();
        let (i0, a) = it.next().unwrap();
        let (i1, b) = it.next().unwrap();
        let (i2, c) = it.next().unwrap();
        let (i3, d) = it.next().unwrap();

        *a += 1;
        *b += 2;
        *c += 3;
        *d += 4;
        drop(it);

        // Verify using the indices returned by the iterator (order is non-neg then neg).
        assert_eq!(grove.get(i0), Some(&31));
        assert_eq!(grove.get(i1), Some(&42));
        assert_eq!(grove.get(i2), Some(&23));
        assert_eq!(grove.get(i3), Some(&14));
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
