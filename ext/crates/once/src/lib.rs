use core::cell::UnsafeCell;
use core::ops::{Index, IndexMut};
use std::cmp::{Eq, PartialEq};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

use rayon::iter::{IntoParallelIterator, ParallelIterator};

const USIZE_LEN: u32 = 0usize.count_zeros();

/// The maximum length of a OnceVec is 2^{MAX_OUTER_LENGTH} - 1. The performance cost of increasing
/// MAX_OUTER_LENGTH is relatively small, but [T; N] does not implement Default for N > 32, which
/// we need for initialization. So let us stick with 32.
const MAX_OUTER_LENGTH: usize = 32;

/// A OnceVec is a push-only vector which is (hopefully) thread-safe. To ensure thread-safety, we
/// need to ensure three things
///
///  1. Never reallocate, since this would invalidate pointers held by other threads
///  2. Prevent simultaneous pushes
///  3. Avoid reading partially written data
///
/// To ensure (1), we use an array of Vec's of exponentially increasing capacity. Each Vec is
/// allocated when we first push an item to it to avoid preallocating a huge amount of memory.
///
/// To ensure (2), we use a mutex to lock when *writing* only. Note that data races are instant UB,
/// even with UnsafeCell. An earlier attempt sought to panic if such a data race is detected with
/// compare_exchange, but panicking after the fact is too late.
///
/// To ensure (3), we store the length of the vector in an AtomicUsize. We update this value
/// *after* writing to the vec, and check the value *before* reading the vec. The invariant to be
/// maintained is that at any point in time, the values up to `self.len` are always fully written.
pub struct OnceVec<T> {
    len: AtomicUsize,
    lock: Mutex<()>,
    data: UnsafeCell<Box<[Vec<T>; MAX_OUTER_LENGTH]>>,
}

impl<T: Clone> Clone for OnceVec<T> {
    fn clone(&self) -> Self {
        // Must read the len before the data
        let len = self.len();
        let data = unsafe { self.get_inner().clone() };

        Self {
            len: AtomicUsize::new(len),
            lock: Mutex::new(()),
            data: UnsafeCell::new(Box::new(data)),
        }
    }
}

impl<T> Default for OnceVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: fmt::Debug> fmt::Debug for OnceVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        let mut it = self.iter();
        match it.next() {
            Some(x) => write!(f, "{:?}", x)?,
            None => {
                return write!(f, "]");
            }
        }
        for x in it {
            write!(f, ", {:?}", x)?;
        }
        write!(f, "]")
    }
}

impl<T> PartialEq for OnceVec<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &OnceVec<T>) -> bool {
        if self.len() != other.len() {
            return false;
        }
        for i in 0..self.len() {
            if self[i] != other[i] {
                return false;
            }
        }
        true
    }
}

impl<T> Eq for OnceVec<T> where T: Eq {}

impl<T> OnceVec<T> {
    pub fn into_vec(self) -> Vec<T> {
        self.into_iter().collect()
    }

    /// Creates a OnceVec from a Vec.
    ///
    /// # Example
    /// ```
    /// # use once::OnceVec;
    /// let v = vec![1, 3, 5, 2];
    /// let w = OnceVec::from_vec(v.clone());
    /// assert_eq!(w.len(), 4);
    /// for i in 0 .. 4 {
    ///     assert_eq!(v[i], w[i]);
    /// }
    /// ```
    pub fn from_vec(mut vec: Vec<T>) -> Self {
        let mut result = Self::new();
        *result.len.get_mut() = vec.len();

        let max_n = (USIZE_LEN - vec.len().leading_zeros()) as usize;

        let inner = result.data.get_mut();
        for k in (0..max_n).rev() {
            inner[k] = vec.split_off((1 << k) - 1);
            if k == max_n - 1 {
                inner[k].reserve(vec.len() - ((1 << k) - 1));
            }
        }

        result
    }

    pub fn new() -> Self {
        Self {
            len: AtomicUsize::new(0),
            lock: Mutex::new(()),
            data: UnsafeCell::new(Default::default()),
        }
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
            let (page, index) = Self::inner_index(index);
            unsafe { Some(self.get_inner().get_unchecked(page).get_unchecked(index)) }
        } else {
            None
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
    pub fn lock(&self) -> MutexGuard<()> {
        self.lock.lock().unwrap()
    }

    const fn inner_index(index: usize) -> (usize, usize) {
        let page = (USIZE_LEN - 1 - (index + 1).leading_zeros()) as usize;
        let index = (index + 1) - (1 << page);
        (page, index)
    }

    unsafe fn get_inner(&self) -> &[Vec<T>; MAX_OUTER_LENGTH] {
        &*self.data.get()
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
    pub fn push(&self, value: T) -> usize {
        unsafe {
            let _lock = self.lock();
            let old_len = self.len.load(Ordering::Acquire);
            let (page, index) = Self::inner_index(old_len);
            let inner = &mut *self.data.get();
            if index == 0 {
                inner[page].reserve_exact(old_len + 1);
            }
            inner[page].push(value);
            self.len.store(old_len + 1, Ordering::Release);
            old_len
        }
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
    ///
    /// # Arguments
    ///  - `new_max`: After calling this function, `self[new_max]` will be defined.
    ///  - `f`: We will fill in the vector with `f(i)` at the `i`th index.
    pub fn extend(&self, new_max: usize, mut f: impl FnMut(usize) -> T) {
        unsafe {
            let _lock = self.lock();
            let old_len = self.len.load(Ordering::Acquire);
            if new_max < old_len {
                return;
            }
            let inner = &mut *self.data.get();

            for i in old_len..=new_max {
                let (page, index) = Self::inner_index(i);
                if index == 0 {
                    inner[page].reserve_exact(i + 1);
                }
                inner[page].push(f(i));
                // Do it inside the loop because f may use self
                self.len.store(i + 1, Ordering::Release)
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let len = self.len();
        unsafe { self.get_inner().iter().flatten().take(len) }
    }
}

impl<T: Send + Sync> OnceVec<T> {
    /// A parallel version of `extend`, where the function `f` is run for different indices
    /// simultaneously using [`rayon`].
    ///
    /// # Example
    /// ```
    /// # use once::OnceVec;
    /// let v: OnceVec<usize> = OnceVec::new();
    /// v.par_extend(5, |i| i + 5);
    /// assert_eq!(v.len(), 6);
    /// for (i, &n) in v.iter().enumerate() {
    ///     assert_eq!(n, i + 5);
    /// }
    /// ```
    pub fn par_extend(&self, new_max: usize, f: impl Fn(usize) -> T + Send + Sync) {
        unsafe {
            let _lock = self.lock();
            let old_len = self.len.load(Ordering::Acquire);
            if new_max < old_len {
                return;
            }
            let inner = &mut *self.data.get();

            // Unfortunately there is no way to avoid collecting ATM. See
            // https://github.com/rayon-rs/rayon/issues/210
            let results: Vec<(usize, T)> = (old_len..=new_max)
                .into_par_iter()
                .map(|i| (i, f(i)))
                .collect();
            for (i, v) in results {
                let (page, index) = Self::inner_index(i);
                if index == 0 {
                    inner[page].reserve_exact(i + 1);
                }
                inner[page].push(v);
                // Do it inside the loop because f may use self
                self.len.store(i + 1, Ordering::Release)
            }
        }
    }
}

impl<T> IntoIterator for OnceVec<T> {
    type Item = T;
    type IntoIter = std::iter::Flatten<std::array::IntoIter<Vec<T>, MAX_OUTER_LENGTH>>;

    fn into_iter(self) -> Self::IntoIter {
        std::array::IntoIter::new(*self.data.into_inner()).flatten()
    }
}

impl<T> Index<usize> for OnceVec<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        let len = self.len();
        assert!(
            index < len,
            "Index out of bounds: the len is {} but the index is {}",
            len,
            index
        );
        let (page, index) = Self::inner_index(index);
        unsafe { self.get_inner().get_unchecked(page).get_unchecked(index) }
    }
}

impl<T> IndexMut<usize> for OnceVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        let len = self.len();
        assert!(
            index < len,
            "Index out of bounds: the len is {} but the index is {}",
            len,
            index
        );
        let (page, index) = Self::inner_index(index);
        unsafe {
            (*self.data.get())
                .get_unchecked_mut(page)
                .get_unchecked_mut(index)
        }
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

#[derive(Clone, PartialEq, Eq)]
pub struct OnceBiVec<T> {
    pub data: OnceVec<T>,
    min_degree: i32,
}

impl<T: fmt::Debug> fmt::Debug for OnceBiVec<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "BiVec({}) ", self.min_degree)?;
        self.data.fmt(formatter)
    }
}

impl<T> OnceBiVec<T> {
    pub fn new(min_degree: i32) -> Self {
        OnceBiVec {
            data: OnceVec::new(),
            min_degree,
        }
    }

    pub fn from_vec(min_degree: i32, data: Vec<T>) -> Self {
        Self {
            data: OnceVec::from_vec(data),
            min_degree,
        }
    }

    pub fn from_bivec(data: bivec::BiVec<T>) -> Self {
        Self::from_vec(data.min_degree(), data.into_vec())
    }

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

    /// This returns the "length" of the bivector, defined to be the smallest i such that
    /// `v[i]`
    /// is not defined.
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

    pub fn get(&self, index: i32) -> Option<&T> {
        self.data.get((index - self.min_degree) as usize)
    }

    pub fn range(&self) -> std::ops::Range<i32> {
        self.min_degree()..self.len()
    }

    /// Extend the `OnceBiVec` to up to index `new_max`, filling in the entries with the values of
    /// `f`. This takes the lock before calling `f`, which is useful behaviour if used in
    /// conjunction with [`OnceBiVec::lock`].
    ///
    /// This is thread-safe and guaranteed to be idempotent. `f` will only be called once per
    /// index.
    ///
    /// In case multiple `OnceVec`'s have to be simultaneously updated, one can use `extend` on one
    /// of them and `push_checked` into the others within the function.
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
    ///
    /// # Arguments
    ///  - `new_max`: After calling this function, `self[new_max]` will be defined.
    ///  - `f`: We will fill in the vector with `f(i)` at the `i`th index.
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
    pub fn lock(&self) -> MutexGuard<()> {
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
    /// A parallel version of `extend`, where the function `f` is run for different indices
    /// simultaneously using [`rayon`].
    ///
    /// # Example
    /// ```
    /// # use once::OnceBiVec;
    /// let v: OnceBiVec<i32> = OnceBiVec::new(-4);
    /// v.par_extend(5, |i| i + 5);
    /// assert_eq!(v.len(), 6);
    /// for (i, &n) in v.iter_enum() {
    ///     assert_eq!(n, i + 5);
    /// }
    /// ```
    pub fn par_extend(&self, new_max: i32, f: impl (Fn(i32) -> T) + Send + Sync) {
        if new_max < self.min_degree {
            return;
        }
        self.data
            .par_extend((new_max - self.min_degree) as usize, |i| {
                f(i as i32 + self.min_degree)
            });
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
    use super::*;
    // use rstest::rstest_parametrize;

    #[test]
    fn test_inner_index() {
        assert_eq!(OnceVec::<()>::inner_index(0), (0, 0));
        assert_eq!(OnceVec::<()>::inner_index(1), (1, 0));
        assert_eq!(OnceVec::<()>::inner_index(2), (1, 1));
        assert_eq!(OnceVec::<()>::inner_index(3), (2, 0));
    }

    #[test]
    fn test_push() {
        let v = OnceVec::new();
        for i in 0u32..100_000u32 {
            v.push(i);
            println!("i : {}", i);
            assert_eq!(v[i], i);
        }
    }
}
