use core::ops::{Index, IndexMut};
use std::{
    cmp::{Eq, PartialEq},
    collections::BTreeSet,
    fmt,
};

use maybe_rayon::prelude::*;

use crate::{
    grove::Grove,
    std_or_loom::sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, MutexGuard,
    },
};

/// A wrapper around a `BTreeSet` that tracks out-of-order element insertions.
///
/// This is an internal implementation detail of [`OnceVec`] that keeps track of
/// indices where elements have been inserted out of order. It's also used as the
/// target of the mutex lock to prevent concurrent modifications.
///
/// See [`OnceVec`] documentation for more details on how out-of-order insertions work.
#[derive(Clone, Default)]
pub struct OooTracker(BTreeSet<usize>);

/// A push-only vector which is thread-safe. To ensure thread-safety, we need to ensure three things
///
///  1. Never reallocate, since this would invalidate pointers held by other threads
///  2. Prevent simultaneous pushes
///  3. Avoid reading partially written data
///
/// To ensure (1), we use a [`Grove`] as the backing data structure.
///
/// To ensure (2), we use a mutex to lock when *writing* only. Note that data races are instant UB,
/// even with `UnsafeCell`. An earlier attempt sought to panic if such a data race is detected with
/// compare_exchange, but panicking after the fact is too late.
///
/// To ensure (3), we store the length of the vector in an `AtomicUsize`. We update this value
/// *after* writing to the vec, and check the value *before* reading the vec. The invariant to be
/// maintained is that at any point in time, the values up to `self.len` are always fully written.
///
/// # Key Features
///
/// - **Thread Safety**: Safe for concurrent reads and writes from multiple threads
/// - **Out-of-Order Insertion**: Supports inserting elements at arbitrary positions
/// - **Parallel Extension**: Can be extended in parallel using Rayon (with the `concurrent`
///   feature)
///
/// # Examples
///
/// ```
/// use std::{sync::Arc, thread};
///
/// use once::OnceVec;
///
/// // Create a shared vector
/// let vec = Arc::new(OnceVec::<i32>::new());
///
/// // Spawn multiple threads to push elements
/// let mut handles = vec![];
/// for i in 0..5 {
///     let vec_clone = Arc::clone(&vec);
///     let handle = thread::spawn(move || {
///         vec_clone.push(i);
///     });
///     handles.push(handle);
/// }
///
/// // Wait for all threads to complete
/// for handle in handles {
///     handle.join().unwrap();
/// }
///
/// // The vector now contains all elements (in some order)
/// assert_eq!(vec.len(), 5);
/// ```
///
/// # Out-of-Order Insertion
///
/// `OnceVec` supports inserting elements at arbitrary positions using `push_ooo`:
///
/// ```
/// use once::OnceVec;
///
/// let vec = OnceVec::<i32>::new();
///
/// // Insert at position 0
/// vec.push_ooo(10, 0);
///
/// // Insert at position 2 (leaving position 1 empty for now)
/// vec.push_ooo(30, 2);
///
/// // Fill in position 1
/// vec.push_ooo(20, 1);
///
/// assert_eq!(vec[0usize], 10);
/// assert_eq!(vec[1usize], 20);
/// assert_eq!(vec[2usize], 30);
/// ```

#[derive(Default)]
pub struct OnceVec<T> {
    len: AtomicUsize,
    /// [`BTreeSet`] of elements that have been added out of order. We also use this mutex to
    /// prevent conflicting concurrent pushes. We use a newtype to wrap the [`BTreeSet`] because
    /// we want [`OnceVec::lock`] to be public, but we don't want to let people mess with the
    /// internals of the tracker.
    ooo: Mutex<OooTracker>,
    data: Grove<T>,
}

impl<T: Clone> Clone for OnceVec<T> {
    fn clone(&self) -> Self {
        let result = Self::new();
        for v in self.iter() {
            result.push(v.clone());
        }
        result
    }
}

impl<T: fmt::Debug> fmt::Debug for OnceVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        let mut it = self.iter();
        match it.next() {
            Some(x) => write!(f, "{x:?}")?,
            None => {
                return write!(f, "]");
            }
        }
        for x in it {
            write!(f, ", {x:?}")?;
        }
        write!(f, "]")
    }
}

impl<T> PartialEq for OnceVec<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}

impl<T> Eq for OnceVec<T> where T: Eq {}

impl<T> OnceVec<T> {
    /// Creates a OnceVec from a Vec.
    ///
    /// # Example
    /// ```
    /// # use once::OnceVec;
    /// let v = vec![1, 3, 5, 2];
    /// let w = OnceVec::from_vec(v.clone());
    /// assert_eq!(w.len(), 4);
    /// for i in 0..4 {
    ///     assert_eq!(v[i], w[i]);
    /// }
    /// ```
    pub fn from_vec(vec: Vec<T>) -> Self {
        let data = Grove::from_vec(vec);
        let len = data.len();
        Self {
            len: AtomicUsize::new(len),
            ooo: Mutex::new(OooTracker::default()),
            data,
        }
    }

    pub fn new() -> Self {
        Self {
            len: AtomicUsize::new(0),
            ooo: Mutex::new(OooTracker::default()),
            data: Grove::new(),
        }
    }

    /// Returns a list of out-of-order elements remaining.
    pub fn ooo_elements(&self) -> Vec<usize> {
        self.ooo.lock().unwrap().0.iter().copied().collect()
    }

    /// All data up to length self.len() are guaranteed to be fully written *after* reading
    /// self.len().
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Acquire)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len() {
            // Safety: we know that the data is fully written up to self.len(). We can use that
            // information to skip all checks and just read the data.
            Some(unsafe { self.data.get_unchecked(index) })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.data.get_mut(index)
    }

    pub fn get_or_insert(&self, index: usize, to_insert: impl FnOnce() -> T) -> &T {
        if let Some(val) = self.get(index) {
            val
        } else {
            self.push_checked(to_insert(), index);
            self.get(index).unwrap()
        }
    }

    pub fn last(&self) -> Option<&T> {
        if !self.is_empty() {
            Some(&self[self.len() - 1])
        } else {
            None
        }
    }

    /// Takes a lock on the `OnceVec`. The `OnceVec` cannot be updated while the lock is held.
    /// This is useful when used in conjuction with [`OnceVec::extend`];
    pub fn lock(&self) -> MutexGuard<'_, OooTracker> {
        self.ooo.lock().unwrap()
    }

    /// Push an element into the vector and check that it was inserted into the `index` position.
    ///
    /// This is useful for situations where pushing into the wrong position can cause unexpected
    /// future behaviour.
    ///
    /// # Panics
    ///
    /// Panics if the position of the new element is not `index`.
    pub fn push_checked(&self, value: T, index: usize) {
        assert_eq!(self.push(value), index);
    }

    /// Append an element to the end of the vector.
    ///
    /// Returns the index of the new element.
    ///
    /// # Example
    /// ```
    /// # use once::OnceVec;
    /// let v = OnceVec::<u32>::new();
    /// v.push(1);
    /// v.push(2);
    /// let x = &v[1usize];
    /// v.push(3);
    /// assert_eq!(*x, 2);
    /// ```
    pub fn push(&self, value: T) -> usize {
        let ooo = self.lock();
        assert!(
            ooo.0.is_empty(),
            "Cannot push while there are out-of-order elements"
        );
        let old_len = self.len.load(Ordering::Acquire);

        self.data.insert(old_len, value);

        self.len.store(old_len + 1, Ordering::Release);
        old_len
    }

    /// Append an element to an arbitrary position in the OnceVec.
    ///
    /// Whenever an element is pushed out of order, the we revisit the whole `OnceVec` and update
    /// the `len` to be the largest contiguous initial block that has been written to. The return
    /// value indicates the newly valid range. Elements in this range will no longer be consider to
    /// have been pushed out of order.
    ///
    /// It is invalid to use the ordinary push function when there are still elements pushed out of
    /// order.
    ///
    /// # Example
    /// ```
    /// # use once::OnceVec;
    /// let v = OnceVec::<u32>::new();
    /// assert_eq!(v.push_ooo(1, 0), 0..1);
    /// assert_eq!(v.len(), 1);
    ///
    /// v.push_checked(2, 1);
    ///
    /// assert_eq!(v.push_ooo(3, 3), 2..2);
    /// assert_eq!(v.len(), 2);
    ///
    /// assert_eq!(v.push_ooo(5, 2), 2..4);
    /// assert_eq!(v.len(), 4);
    ///
    /// v.push(4);
    ///
    /// assert_eq!(v[2usize], 5);
    /// assert_eq!(v[3usize], 3);
    /// assert_eq!(v[4usize], 4);
    /// ```
    pub fn push_ooo(&self, value: T, index: usize) -> std::ops::Range<usize> {
        let mut ooo = self.lock();
        if ooo.0.contains(&index) {
            panic!("Cannot push element out of order at the same index {index} twice");
        }

        let old_len = self.len.load(Ordering::Acquire);

        self.data.insert(index, value);

        if index != old_len {
            ooo.0.insert(index);
            return old_len..old_len;
        }
        let mut end = old_len + 1;
        while ooo.0.remove(&end) {
            end += 1;
        }

        self.len.store(end, Ordering::Release);
        old_len..end
    }

    /// Extend the `OnceVec` to up to index `new_max`, filling in the entries with the values of
    /// `f`. This takes the lock before calling `f`, which is useful behaviour if used in
    /// conjunction with [`OnceVec::lock`].
    ///
    /// This is thread-safe and guaranteed to be idempotent. `f` will only be called once per
    /// index.
    ///
    /// In case multiple `OnceVec`'s have to be simultaneously updated, one can use `extend` on one
    /// of them and `push_checked` into the others within the function.
    ///
    /// # Parameters
    ///
    /// * `new_max`: After calling this function, `self[new_max]` will be defined.
    /// * `f`: We will fill in the vector with `f(i)` at the `i`th index.
    ///
    /// # Example
    /// ```
    /// # use once::OnceVec;
    /// let v: OnceVec<usize> = OnceVec::new();
    /// v.extend(5, |i| i + 5);
    /// assert_eq!(v.len(), 6);
    /// for (i, &n) in v.iter().enumerate() {
    ///     assert_eq!(n, i + 5);
    /// }
    /// ```
    pub fn extend(&self, new_max: usize, mut f: impl FnMut(usize) -> T) {
        let ooo = self.lock();
        assert!(ooo.0.is_empty());
        let old_len = self.len.load(Ordering::Acquire);
        if new_max < old_len {
            return;
        }

        for i in old_len..=new_max {
            self.data.insert(i, f(i));

            // Do it inside the loop because f may use self
            self.len.store(i + 1, Ordering::Release)
        }
    }

    /// Iterate through the `OnceVec`.
    ///
    /// # Example
    /// ```
    /// # use once::OnceVec;
    /// let v: OnceVec<usize> = OnceVec::new();
    /// v.push(1);
    /// v.push(5);
    /// v.push(2);
    /// assert_eq!(v.iter().count(), 3);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let len = self.len();
        // We use `take` because `data.iter()` also iterates through the out-of-order elements, but
        // we don't want that.
        self.data.iter().take(len)
    }
}

impl<T: Send + Sync> OnceVec<T> {
    /// A parallel version of `extend`. If the `concurrent` feature is enabled, the function `f`
    /// will be run for different indices simultaneously using `rayon`, through the [`maybe_rayon`]
    /// crate.
    ///
    /// # Example
    #[cfg_attr(miri, doc = "```ignore")]
    #[cfg_attr(not(miri), doc = "```")]
    /// # use once::OnceVec;
    /// let v: OnceVec<usize> = OnceVec::new();
    /// v.maybe_par_extend(5, |i| i + 5);
    /// assert_eq!(v.len(), 6);
    /// for (i, &n) in v.iter().enumerate() {
    ///     assert_eq!(n, i + 5);
    /// }
    /// ```
    pub fn maybe_par_extend(&self, new_max: usize, f: impl Fn(usize) -> T + Send + Sync) {
        let ooo = self.lock();
        assert!(ooo.0.is_empty());

        let old_len = self.len.load(Ordering::Acquire);
        if new_max < old_len {
            return;
        }

        (old_len..=new_max).into_maybe_par_iter().for_each(|i| {
            self.data.insert(i, f(i));
        });

        self.len.store(new_max + 1, Ordering::Release)
    }
}

impl<T> Index<usize> for OnceVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        self.get(index).unwrap_or_else(|| {
            panic!(
                "Index out of bounds: the len is {} but the index is {index}",
                self.len()
            )
        })
    }
}

impl<T> IndexMut<usize> for OnceVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        let len = self.len();
        self.get_mut(index).unwrap_or_else(|| {
            panic!("Index out of bounds: the len is {len} but the index is {index}")
        })
    }
}

impl<T> Index<u32> for OnceVec<T> {
    type Output = T;

    fn index(&self, index: u32) -> &T {
        self.index(index as usize)
    }
}

impl<T> IndexMut<u32> for OnceVec<T> {
    fn index_mut(&mut self, index: u32) -> &mut T {
        self.index_mut(index as usize)
    }
}

unsafe impl<T: Send> Send for OnceVec<T> {}
unsafe impl<T: Sync> Sync for OnceVec<T> {}

impl<T> FromIterator<T> for OnceVec<T> {
    /// ```
    /// # use once::OnceVec;
    /// let elements = vec![1, 2, 3];
    ///
    /// let v1 = OnceVec::from_vec(elements.clone());
    /// // The `assert_eq` below lets the compiler infer that `v2` is a `OnceVec<i32>`.
    /// let v2 = elements.into_iter().collect();
    ///
    /// assert_eq!(v1, v2);
    /// ```
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let result = Self::new();
        for v in iter {
            result.push(v);
        }
        result
    }
}

/// A vector that supports negative indices, built on top of `OnceVec`.
///
/// `OnceBiVec` extends the functionality of `OnceVec` by allowing elements to be indexed
/// using negative integers. This is useful for scenarios where you need to represent
/// data that naturally starts from a negative index. Note that we still only support appending
/// elements to the end of the vector, so it's not possible to insert elements at arbitrarily
/// negative indices.
///
/// # Examples
///
/// ```
/// use once::OnceBiVec;
///
/// // Create a bidirectional vector with minimum degree -3
/// let vec = OnceBiVec::<i32>::new(-3);
///
/// // Insert elements at various positions
/// vec.push_ooo(10, -3); // At minimum degree
/// vec.push_ooo(30, -1);
/// vec.push_ooo(20, -2);
/// vec.push_ooo(50, 1);
/// vec.push_ooo(40, 0);
///
/// // Access elements using their indices
/// assert_eq!(vec[-3], 10);
/// assert_eq!(vec[-2], 20);
/// assert_eq!(vec[-1], 30);
/// assert_eq!(vec[0], 40);
/// assert_eq!(vec[1], 50);
///
/// // Get the range of valid indices
/// assert_eq!(vec.range(), -3..2);
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct OnceBiVec<T> {
    data: OnceVec<T>,
    min_degree: i32,
}

impl<T: fmt::Debug> fmt::Debug for OnceBiVec<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "BiVec({}) ", self.min_degree)?;
        self.data.fmt(formatter)
    }
}

impl<T> OnceBiVec<T> {
    /// Creates a new empty `OnceBiVec` with the specified minimum degree.
    ///
    /// # Parameters
    ///
    /// * `min_degree`: The minimum degree (lowest index) of the vector
    ///
    /// # Examples
    ///
    /// ```
    /// use once::OnceBiVec;
    ///
    /// let vec = OnceBiVec::<i32>::new(-5);
    /// assert_eq!(vec.min_degree(), -5);
    /// assert_eq!(vec.len(), -5);
    /// assert!(vec.is_empty());
    /// ```
    pub fn new(min_degree: i32) -> Self {
        Self {
            data: OnceVec::new(),
            min_degree,
        }
    }

    /// Creates an `OnceBiVec` from a `Vec` with the specified minimum degree.
    ///
    /// # Parameters
    ///
    /// * `min_degree`: The minimum degree (lowest index) of the vector
    /// * `data`: The vector of values to initialize with
    ///
    /// # Examples
    ///
    /// ```
    /// use once::OnceBiVec;
    ///
    /// let vec = OnceBiVec::from_vec(-2, vec![10, 20, 30]);
    /// assert_eq!(vec.min_degree(), -2);
    /// assert_eq!(vec.len(), 1); // -2 + 3 = 1
    /// assert_eq!(vec[-2], 10);
    /// assert_eq!(vec[-1], 20);
    /// assert_eq!(vec[0], 30);
    /// ```
    pub fn from_vec(min_degree: i32, data: Vec<T>) -> Self {
        Self {
            data: OnceVec::from_vec(data),
            min_degree,
        }
    }

    /// Creates an `OnceBiVec` from a `bivec::BiVec`.
    ///
    /// This is a convenience method for converting from the `bivec` crate's bidirectional vector
    /// implementation.
    ///
    /// # Parameters
    ///
    /// * `data`: The `bivec::BiVec` to convert from
    pub fn from_bivec(data: bivec::BiVec<T>) -> Self {
        Self::from_vec(data.min_degree(), data.into_vec())
    }

    /// Returns the minimum degree (lowest index) of the vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::OnceBiVec;
    ///
    /// let vec = OnceBiVec::<i32>::new(-3);
    /// assert_eq!(vec.min_degree(), -3);
    /// ```
    pub const fn min_degree(&self) -> i32 {
        self.min_degree
    }

    /// This returns the largest degree in the bivector. This is equal to `self.len() - 1`.
    ///
    /// # Example
    /// ```
    /// # use bivec::BiVec;
    /// let v = BiVec::from_vec(-2, vec![3, 4, 6, 8, 2]);
    /// assert_eq!(v.max_degree(), 2);
    /// ```
    pub fn max_degree(&self) -> i32 {
        self.len() - 1
    }

    /// This returns the "length" of the bivector, defined to be the smallest i such that `v[i]` is
    /// not defined.
    ///
    /// # Example
    /// ```
    /// # use bivec::BiVec;
    /// let v = BiVec::from_vec(-2, vec![3, 4, 6, 8, 2]);
    /// assert_eq!(v.len(), 3);
    /// ```
    pub fn len(&self) -> i32 {
        self.data.len() as i32 + self.min_degree
    }

    pub fn is_empty(&self) -> bool {
        self.data.len() == 0
    }

    pub fn push_checked(&self, value: T, index: i32) {
        assert_eq!(self.push(value), index);
    }

    pub fn push(&self, value: T) -> i32 {
        self.data.push(value) as i32 + self.min_degree
    }

    /// See [`OnceVec::push_ooo`].
    pub fn push_ooo(&self, value: T, index: i32) -> std::ops::Range<i32> {
        let result = self
            .data
            .push_ooo(value, (index - self.min_degree) as usize);

        (result.start as i32 + self.min_degree)..(result.end as i32 + self.min_degree)
    }

    pub fn ooo_elements(&self) -> Vec<i32> {
        self.data
            .ooo_elements()
            .into_iter()
            .map(|x| x as i32 + self.min_degree)
            .collect()
    }

    /// Returns whether the `OnceBiVec` has remaining out-of-order elements
    pub fn get(&self, index: i32) -> Option<&T> {
        self.data.get((index - self.min_degree).try_into().ok()?)
    }

    pub fn get_or_insert(&self, index: i32, to_insert: impl FnOnce() -> T) -> &T {
        self.data
            .get_or_insert((index - self.min_degree).try_into().unwrap(), to_insert)
    }

    pub fn range(&self) -> std::ops::Range<i32> {
        self.min_degree()..self.len()
    }

    /// Extend the `OnceBiVec` to up to index `new_max`, filling in the entries with the values of
    /// `f`. This takes the lock before calling `f`, which is useful behaviour if used in
    /// conjunction with [`OnceBiVec::lock`].
    ///
    /// This is thread-safe and guaranteed to be idempotent. `f` will only be called once per index.
    ///
    /// In case multiple `OnceVec`'s have to be simultaneously updated, one can use `extend` on one
    /// of them and `push_checked` into the others within the function.
    ///
    /// # Parameters
    ///
    /// * `new_max`: After calling this function, `self[new_max]` will be defined.
    /// * `f`: We will fill in the vector with `f(i)` at the `i`th index.
    ///
    /// # Example
    /// ```
    /// # use once::OnceBiVec;
    /// let v: OnceBiVec<i32> = OnceBiVec::new(-4);
    /// v.extend(5, |i| i + 5);
    /// assert_eq!(v.len(), 6);
    /// for (i, &n) in v.iter_enum() {
    ///     assert_eq!(n, i + 5);
    /// }
    /// ```
    pub fn extend(&self, new_max: i32, mut f: impl FnMut(i32) -> T) {
        if new_max < self.min_degree {
            return;
        }
        self.data.extend((new_max - self.min_degree) as usize, |i| {
            f(i as i32 + self.min_degree)
        });
    }

    pub fn last(&self) -> Option<&T> {
        self.data.last()
    }

    /// Takes a lock on the `OnceBiVec`. The `OnceBiVec` cannot be updated while the lock is held.
    /// This is useful when used in conjuction with [`OnceBiVec::extend`];
    pub fn lock(&self) -> MutexGuard<'_, OooTracker> {
        self.data.lock()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }

    pub fn iter_enum(&self) -> impl Iterator<Item = (i32, &T)> {
        let min_degree = self.min_degree;
        self.data
            .iter()
            .enumerate()
            .map(move |(i, t)| (i as i32 + min_degree, t))
    }
}

impl<T: Send + Sync> OnceBiVec<T> {
    /// A parallel version of `extend`. If the `concurrent` feature is enabled, the function `f`
    /// will be run for different indices simultaneously using `rayon`, through the [`maybe_rayon`]
    /// crate.
    ///
    /// # Example
    #[cfg_attr(miri, doc = "```ignore")]
    #[cfg_attr(not(miri), doc = "```")]
    /// # use once::OnceBiVec;
    /// let v: OnceBiVec<i32> = OnceBiVec::new(-4);
    /// v.maybe_par_extend(5, |i| i + 5);
    /// assert_eq!(v.len(), 6);
    /// for (i, &n) in v.iter_enum() {
    ///     assert_eq!(n, i + 5);
    /// }
    /// ```
    pub fn maybe_par_extend(&self, new_max: i32, f: impl (Fn(i32) -> T) + Send + Sync) {
        if new_max < self.min_degree {
            return;
        }
        self.data
            .maybe_par_extend((new_max - self.min_degree) as usize, |i| {
                f(i as i32 + self.min_degree)
            });
    }

    pub fn maybe_par_iter_enum(
        &self,
    ) -> impl MaybeParallelIterator<Item = (i32, &T)> + MaybeIndexedParallelIterator {
        self.range().into_maybe_par_iter().map(|i| (i, &self[i]))
    }
}

impl<T> Index<i32> for OnceBiVec<T> {
    type Output = T;

    fn index(&self, i: i32) -> &T {
        assert!(
            i >= self.min_degree(),
            "Index out of bounds: the minimum degree is {} but the index is {i}",
            self.min_degree()
        );
        assert!(
            i < self.len(),
            "Index out of bounds: the length is {} but the index is {i}",
            self.len()
        );
        &(self.data[(i - self.min_degree) as usize])
    }
}

impl<T> IndexMut<i32> for OnceBiVec<T> {
    fn index_mut(&mut self, i: i32) -> &mut T {
        assert!(
            i >= self.min_degree(),
            "Index out of bounds: the minimum degree is {} but the index is {i}",
            self.min_degree()
        );
        assert!(
            i < self.len(),
            "Index out of bounds: the length is {} but the index is {i}",
            self.len()
        );
        &mut (self.data[(i - self.min_degree) as usize])
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use super::*;

    #[test]
    fn test_push() {
        let v = OnceVec::new();
        for i in 0u32..1000u32 {
            v.push(i);
            assert_eq!(v[i], i);
        }
    }

    #[test]
    fn test_drop_ooo() {
        let v: OnceVec<u32> = OnceVec::new();
        v.push(4);
        v.push(3);
        v.push_ooo(6, 7);
        drop(v);
    }

    #[test]
    fn test_concurrent_push() {
        let v = Arc::new(OnceVec::<usize>::new());

        let num_threads = crate::test_utils::num_threads();
        let values_per_thread = crate::test_utils::values_per_thread();

        let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
                let v = Arc::clone(&v);
                thread::spawn(move || {
                    for i in 0..values_per_thread {
                        v.push(thread_id * values_per_thread + i);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(v.len(), num_threads * values_per_thread);

        // All values should be present, though not necessarily in order
        let mut found = vec![false; num_threads * values_per_thread];
        for i in 0..v.len() {
            found[v[i]] = true;
        }
        assert!(found.iter().all(|&x| x));
    }

    #[test]
    fn test_extend() {
        let v = OnceVec::<usize>::new();
        v.extend(10, |i| i * 2);

        assert_eq!(v.len(), 11);
        for i in 0..=10usize {
            assert_eq!(v[i], i * 2);
        }
    }

    #[test]
    fn test_push_ooo() {
        let v = OnceVec::<usize>::new();

        // Push out of order
        v.push_ooo(100, 10);
        assert_eq!(v.len(), 0); // Length is still 0 because there's a gap
        assert_eq!(v.ooo_elements(), vec![10]);

        // Fill the gap partially
        v.push_ooo(0, 0);
        assert_eq!(v.len(), 1); // Length is now 1
        assert_eq!(v.ooo_elements(), vec![10]);

        // Fill more of the gap
        v.push_ooo(10, 1);
        v.push_ooo(20, 2);
        v.push_ooo(30, 3);
        assert_eq!(v.len(), 4); // Length is now 4
        assert_eq!(v.ooo_elements(), vec![10]);

        // Fill the rest of the gap
        v.push_ooo(40, 4);
        v.push_ooo(50, 5);
        v.push_ooo(60, 6);
        v.push_ooo(70, 7);
        v.push_ooo(80, 8);
        v.push_ooo(90, 9);
        assert_eq!(v.len(), 11); // Length is now 11 (0-10)
        assert_eq!(v.ooo_elements().len(), 0);

        // Verify all values
        for i in 0..=10usize {
            assert_eq!(v[i], i * 10);
        }
    }

    #[test]
    fn test_from_vec() {
        let original = vec![10, 20, 30, 40, 50];
        let v = OnceVec::from_vec(original.clone());

        assert_eq!(v.len(), original.len());
        for (i, &val) in original.iter().enumerate() {
            assert_eq!(v[i], val);
        }
    }

    #[test]
    fn test_clone() {
        let v1 = OnceVec::new();
        v1.push(10);
        v1.push(20);
        v1.push(30);

        let v2 = v1.clone();
        assert_eq!(v1.len(), v2.len());
        for i in 0..v1.len() {
            assert_eq!(v1[i], v2[i]);
        }

        // Modifying one doesn't affect the other
        v1.push(40);
        assert_eq!(v1.len(), 4);
        assert_eq!(v2.len(), 3);
    }

    #[test]
    fn test_iter() {
        let v = OnceVec::new();
        v.push(10);
        v.push(20);
        v.push(30);

        let mut iter = v.iter();
        assert_eq!(iter.next(), Some(&10));
        assert_eq!(iter.next(), Some(&20));
        assert_eq!(iter.next(), Some(&30));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_from_iterator() {
        let data = [1, 2, 3, 4, 5];
        let v: OnceVec<_> = data.iter().copied().collect();

        assert_eq!(v.len(), data.len());
        for (i, &val) in data.iter().enumerate() {
            assert_eq!(v[i], val);
        }
    }

    #[cfg(not(miri))]
    mod proptests {
        use std::collections::{HashMap, HashSet};

        use proptest::prelude::*;

        use super::*;

        #[derive(Debug, Clone)]
        enum OnceVecOperation {
            Push(i32),
            PushOoo(i32, usize),
            Extend(usize, i32), // extend to index, with multiplier
        }

        fn oncevec_operation_strategy() -> impl Strategy<Value = OnceVecOperation> {
            prop_oneof![
                prop::num::i32::ANY.prop_map(OnceVecOperation::Push),
                (prop::num::i32::ANY, 0..10usize)
                    .prop_map(|(v, i)| OnceVecOperation::PushOoo(v, i)),
                (0..10usize, prop::num::i32::ANY)
                    .prop_map(|(max, m)| OnceVecOperation::Extend(max, m)),
            ]
        }

        proptest! {
            #[test]
            fn proptest_oncevec_operations(
                ops in prop::collection::vec(
                    oncevec_operation_strategy(),
                    1..50
                )
            ) {
                let vec = OnceVec::new();
                let mut reference = Vec::new();
                let mut ooo_indices = HashMap::new();
                let mut all_indices = HashSet::new();

                for op in ops {
                    match op {
                        OnceVecOperation::Push(value) => {
                            // Only push if there are no out-of-order elements
                            if ooo_indices.is_empty() {
                                vec.push(value);
                                reference.push(value);
                                all_indices.insert(reference.len() - 1);
                            }
                        },
                        OnceVecOperation::PushOoo(value, idx) => {
                            // Only insert if the index doesn't already have a value
                            if all_indices.contains(&idx) {
                                // Skip invalid indices that would panic
                                continue;
                            } else if idx == reference.len() {
                                vec.push_ooo(value, idx);
                                reference.push(value);
                            } else {
                                vec.push_ooo(value, idx);
                                while reference.len() <= idx {
                                    reference.push(0); // Placeholder values
                                }
                                reference[idx] = value;
                                ooo_indices.insert(idx, value);
                            }
                            all_indices.insert(idx);
                        },
                        OnceVecOperation::Extend(max, multiplier) => {
                            if ooo_indices.is_empty() {
                                let to_insert = |i| (i as i32).saturating_mul(multiplier);
                                vec.extend(max, to_insert);
                                for i in reference.len()..=max {
                                    reference.push(to_insert(i));
                                    all_indices.insert(i);
                                }
                            }
                        }
                    }

                    // Check that the vectors match
                    for i in 0..vec.len().min(reference.len()) {
                        assert_eq!(vec[i], reference[i]);
                    }
                }
            }
        }
    }

    // OnceBiVec tests

    #[test]
    fn test_oncebivec_basic() {
        let v = OnceBiVec::<i32>::new(-3);

        // Check initial state
        assert_eq!(v.min_degree(), -3);
        assert_eq!(v.len(), -3);
        assert!(v.is_empty());

        // Push values
        v.push(10); // This will be at index -3
        v.push(20); // This will be at index -2
        v.push(30); // This will be at index -1

        // Check state after pushing
        assert_eq!(v.len(), 0); // -3 + 3 = 0
        assert_eq!(v.max_degree(), -1); // len() - 1
        assert!(!v.is_empty());

        // Check values
        assert_eq!(v[-3], 10);
        assert_eq!(v[-2], 20);
        assert_eq!(v[-1], 30);

        // Check range
        assert_eq!(v.range(), -3..0);
    }

    #[test]
    fn test_oncebivec_from_vec() {
        let data = vec![5, 10, 15, 20];
        let v = OnceBiVec::from_vec(-5, data.clone());

        // Check state
        assert_eq!(v.min_degree(), -5);
        assert_eq!(v.len(), -1); // -5 + 4 = -1

        // Check values
        for (i, &val) in data.iter().enumerate() {
            assert_eq!(v[i as i32 - 5], val);
        }
    }

    #[test]
    fn test_oncebivec_push_ooo() {
        let v = OnceBiVec::<i32>::new(-3);

        // Push out of order
        v.push_ooo(100, 0);
        assert_eq!(v.len(), -3); // Length is still -3 because there's a gap

        // Fill the gap
        v.push_ooo(10, -3);
        v.push_ooo(20, -2);
        v.push_ooo(30, -1);

        // Check state
        assert_eq!(v.len(), 1); // All gaps filled, so len is 1

        // Check values
        assert_eq!(v[-3], 10);
        assert_eq!(v[-2], 20);
        assert_eq!(v[-1], 30);
        assert_eq!(v[0], 100);
    }

    #[test]
    fn test_oncebivec_extend() {
        let v = OnceBiVec::<i32>::new(-5);

        // Extend from min_degree to 2
        v.extend(2, |i| i * 10);

        // Check state
        assert_eq!(v.len(), 3); // -5 + 8 = 3

        // Check values
        for i in -5..=2 {
            assert_eq!(v[i], i * 10);
        }
    }

    #[test]
    fn test_oncebivec_iter_enum() {
        let v = OnceBiVec::<i32>::new(-3);

        // Add some values
        v.push(10);
        v.push(20);
        v.push(30);

        // Check iterator
        let mut iter = v.iter();
        assert_eq!(iter.next(), Some(&10));
        assert_eq!(iter.next(), Some(&20));
        assert_eq!(iter.next(), Some(&30));
        assert_eq!(iter.next(), None);

        // Check enumerated iterator
        let expected_indices = [-3, -2, -1];
        let expected_values = [10, 20, 30];
        let actual_pairs: Vec<_> = v.iter_enum().collect();

        for (i, (idx, val)) in actual_pairs.iter().enumerate() {
            assert_eq!(*idx, expected_indices[i]);
            assert_eq!(**val, expected_values[i]);
        }
    }

    #[cfg(loom)]
    mod loom_tests {
        use super::*;
        use crate::std_or_loom::{sync::Arc, thread};

        #[test]
        fn loom_concurrent_push() {
            loom::model(|| {
                let vec = Arc::new(OnceVec::<usize>::new());

                // Thread 1: Push values
                let vec1 = Arc::clone(&vec);
                let t1 = thread::spawn(move || {
                    vec1.push(1);
                    vec1.push(3);
                });

                // Thread 2: Push values
                let vec2 = Arc::clone(&vec);
                let t2 = thread::spawn(move || {
                    vec2.push(2);
                    vec2.push(4);
                });

                t1.join().unwrap();
                t2.join().unwrap();

                assert_eq!(vec.len(), 4);
            });
        }

        #[test]
        fn loom_push_and_read() {
            loom::model(|| {
                let vec = Arc::new(OnceVec::<usize>::new());

                // Thread 1: Push values
                let vec1 = Arc::clone(&vec);
                let t1 = thread::spawn(move || {
                    vec1.push(1);
                    vec1.push(2);
                });

                // Thread 2: Read values
                let vec2 = Arc::clone(&vec);
                let t2 = thread::spawn(move || {
                    let len = vec2.len();
                    if len > 0 {
                        let _ = vec2.get(0);
                    }
                    if len > 1 {
                        let _ = vec2.get(1);
                    }
                });

                t1.join().unwrap();
                t2.join().unwrap();

                assert_eq!(vec.len(), 2);
            });
        }

        #[test]
        fn loom_extend_concurrent() {
            loom::model(|| {
                let vec = Arc::new(OnceVec::<usize>::new());

                // Thread 1: Extend
                let vec1 = Arc::clone(&vec);
                let t1 = thread::spawn(move || {
                    vec1.extend(2, |i| i + 1);
                });

                // Thread 2: Push
                let vec2 = Arc::clone(&vec);
                let t2 = thread::spawn(move || {
                    if vec2.len() == 0 {
                        vec2.push(100);
                    } else if vec2.len() == 3 {
                        vec2.push(200);
                    }
                });

                t1.join().unwrap();
                t2.join().unwrap();
            });
        }

        #[test]
        fn loom_oncebivec_iter_enum() {
            loom::model(|| {
                let vec = Arc::new(OnceBiVec::<i32>::new(-3));

                // Thread 1: Push values
                let vec1 = Arc::clone(&vec);
                let t1 = thread::spawn(move || {
                    vec1.push(10);
                    vec1.push(20);
                });

                // Thread 2: Read and enumerate values
                let vec2 = Arc::clone(&vec);
                let t2 = thread::spawn(move || {
                    let len = vec2.len();
                    if len > -2 {
                        // At least one element
                        let pairs: Vec<_> = vec2.iter_enum().collect();
                        for (idx, _) in pairs {
                            assert!(idx >= -3 && idx < len);
                        }
                    }
                });

                t1.join().unwrap();
                t2.join().unwrap();

                // Verify final state
                let pairs: Vec<_> = vec.iter_enum().collect();
                assert_eq!(pairs.len(), 2);
                assert_eq!(pairs[0].0, -3);
                assert_eq!(*pairs[0].1, 10);
                assert_eq!(pairs[1].0, -2);
                assert_eq!(*pairs[1].1, 20);
            });
        }
    }
}
