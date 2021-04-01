use core::cell::UnsafeCell;
use core::ops::{Index, IndexMut};
use std::cmp::{Eq, PartialEq};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

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

    /// Since OnceVec never reallocates, with_capacity is the same as normal initialization.
    /// However, it is included for consistency.
    pub fn with_capacity(_capacity: usize) -> Self {
        Self::new()
    }

    /// Since OnceVec never reallocates, reserve is a noop. However, it is included for
    /// consistency.
    pub fn reserve(&self, _capacity: usize) {}

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
            let _lock = self.lock.lock();
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

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let len = self.len();
        unsafe { self.get_inner().iter().flatten().take(len) }
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
        if index >= len {
            panic!(
                "Index out of bounds: the len is {} but the index is {}",
                len, index
            );
        }
        let (page, index) = Self::inner_index(index);
        unsafe { self.get_inner().get_unchecked(page).get_unchecked(index) }
    }
}

impl<T> IndexMut<usize> for OnceVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        let len = self.len();
        if index >= len {
            panic!(
                "Index out of bounds: the len is {} but the index is {}",
                len, index
            );
        }
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

use saveload::{Load, Save};
use std::io;
use std::io::{Read, Write};

impl<T: Save> Save for OnceVec<T> {
    fn save(&self, buffer: &mut impl Write) -> io::Result<()> {
        self.len().save(buffer)?;
        for x in self.iter() {
            x.save(buffer)?;
        }
        Ok(())
    }
}

impl<T: Load> Load for OnceVec<T> {
    type AuxData = T::AuxData;

    fn load(buffer: &mut impl Read, data: &Self::AuxData) -> io::Result<Self> {
        let len = usize::load(buffer, &())?;
        let result: OnceVec<T> = OnceVec::new();
        for _ in 0..len {
            result.push(T::load(buffer, data)?);
        }
        Ok(result)
    }
}

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

    pub fn with_capacity(min_degree: i32, capacity: i32) -> Self {
        debug_assert!(capacity >= min_degree);
        Self {
            data: OnceVec::with_capacity((capacity - min_degree) as usize),
            min_degree,
        }
    }

    pub fn min_degree(&self) -> i32 {
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

    pub fn last(&self) -> Option<&T> {
        self.data.last()
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

impl<T> Index<i32> for OnceBiVec<T> {
    type Output = T;
    fn index(&self, i: i32) -> &T {
        &(self.data[(i - self.min_degree) as usize])
    }
}

impl<T> IndexMut<i32> for OnceBiVec<T> {
    fn index_mut(&mut self, i: i32) -> &mut T {
        &mut (self.data[(i - self.min_degree) as usize])
    }
}

impl<T: Save> Save for OnceBiVec<T> {
    fn save(&self, buffer: &mut impl Write) -> io::Result<()> {
        self.data.save(buffer)
    }
}

impl<T: Load> Load for OnceBiVec<T> {
    type AuxData = (i32, T::AuxData);

    fn load(buffer: &mut impl Read, data: &Self::AuxData) -> io::Result<Self> {
        let min_degree = data.0;
        let data = Load::load(buffer, &data.1)?;
        Ok(Self { data, min_degree })
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

    #[test]
    fn test_saveload() {
        use std::io::{Cursor, Seek, SeekFrom};

        let v: OnceVec<u32> = OnceVec::new();
        v.push(6);
        v.push(3);
        v.push(4);
        v.push(2);

        let mut cursor: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        v.save(&mut cursor).unwrap();

        cursor.seek(SeekFrom::Start(0)).unwrap();
        let v_saved_then_loaded: OnceVec<u32> = Load::load(&mut cursor, &()).unwrap();
        assert_eq!(v, v_saved_then_loaded);
        assert_eq!(0, cursor.bytes().count());

        // let mut w = BiVec::new(-3);
        // w.push(2);
        // w.push(6);
        // w.push(2);
        // w.push(7);

        // let mut cursor2 : Cursor<Vec<u8>> = Cursor::new(Vec::new());
        // w.save(&mut cursor2).unwrap();
        // cursor2.seek(SeekFrom::Start(0)).unwrap();
        // let w_saved_then_loaded : BiVec<u32> = Load::load(&mut cursor, &(-3, ())).unwrap();

        // assert_eq!(w, w_saved_then_loaded);
    }
}
