#![deny(clippy::use_self)]

extern crate alloc;

use alloc::alloc::Layout;
use core::ops::{Index, IndexMut};
use std::{
    cmp::{Eq, PartialEq},
    collections::BTreeSet,
    fmt,
    ptr::NonNull,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, MutexGuard,
    },
};

use maybe_rayon::prelude::*;

const USIZE_LEN: u32 = 0usize.count_zeros();

/// The maximum length of a OnceVec is 2^{MAX_OUTER_LENGTH} - 1. The performance cost of increasing
/// MAX_OUTER_LENGTH is relatively small, so we picked an arbitrary number.
const MAX_OUTER_LENGTH: usize = 32;

/// This is a wrapper around our out-of-order push tracker. See [`OnceVec`] documentation for
/// more details.
#[derive(Clone, Default)]
pub struct OooTracker(BTreeSet<usize>);

const fn inner_index(index: usize) -> (usize, usize) {
    let page = (USIZE_LEN - 1 - (index + 1).leading_zeros()) as usize;
    assert!(page < MAX_OUTER_LENGTH);
    let index = (index + 1) - (1 << page);
    (page, index)
}

struct Page<T>(Option<NonNull<T>>);

impl<T> Default for Page<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T> Page<T> {
    /// This always returns a layout of non-zero size.
    fn layout(page_index: usize) -> Layout {
        assert!(std::mem::size_of::<T>() > 0);

        Layout::from_size_align(
            (1 << page_index) * std::mem::size_of::<T>(),
            std::mem::align_of::<T>(),
        )
        .unwrap()
    }

    fn allocated(&self) -> bool {
        self.0.is_some()
    }

    fn allocate(&mut self, page_index: usize) {
        assert!(self.0.is_none());
        // This is safe because Self::layout always returns a layout of non-zero size.
        let ptr = NonNull::new(unsafe { alloc::alloc::alloc(Self::layout(page_index)) as *mut T });
        assert!(ptr.is_some());
        self.0 = ptr;
    }

    /// # Safety
    /// This has to be the `page`th page and `len` many items has to have been written into the
    /// `OnceVec` owning this page.
    unsafe fn deallocate(&mut self, len: usize, page_index: usize) {
        if let Some(ptr) = self.0 {
            if len > 0 {
                let (max_page, max_index) = inner_index(len - 1);
                let end = if page_index == max_page {
                    max_index + 1
                } else {
                    1 << page_index
                };
                for idx in 0..end {
                    std::ptr::drop_in_place(ptr.as_ptr().add(idx));
                }
            }
            alloc::alloc::dealloc(ptr.as_ptr() as *mut u8, Self::layout(page_index));
        }
    }

    fn ptr(&self) -> *mut T {
        self.0.unwrap().as_ptr()
    }

    /// # Safety
    /// This is safe only when this is the `page_index`th page of a `OnceVec` of length at least
    /// `len`.
    unsafe fn as_slice(&self, len: usize, page_index: usize) -> &[T] {
        if self.0.is_none() || len == 0 {
            &[]
        } else {
            let (max_page, max_index) = inner_index(len - 1);
            let len = if page_index == max_page {
                max_index + 1
            } else {
                1 << page_index
            };
            std::slice::from_raw_parts(self.0.unwrap().as_ptr() as *const T, len)
        }
    }
}

pub const DATA_LAYOUT: Layout = {
    let layout = Layout::from_size_align(
        MAX_OUTER_LENGTH * std::mem::size_of::<Page<()>>(),
        std::mem::align_of::<Page<()>>(),
    );
    match layout {
        Ok(x) => x,
        Err(_) => panic!(),
    }
};

/// A `OnceVec` is a push-only vector which is thread-safe. To ensure thread-safety, we need to
/// ensure three things
///
///  1. Never reallocate, since this would invalidate pointers held by other threads
///  2. Prevent simultaneous pushes
///  3. Avoid reading partially written data
///
/// To ensure (1), we divide the `OnceVec` into multiple pages, where the nth page has an
/// allocation that can store `1 << n` many elements. These are allocated on demand to avoid using
/// too much memory.
///
/// To ensure (2), we use a mutex to lock when *writing* only. Note that data races are instant UB,
/// even with UnsafeCell. An earlier attempt sought to panic if such a data race is detected with
/// compare_exchange, but panicking after the fact is too late.
///
/// To ensure (3), we store the length of the vector in an AtomicUsize. We update this value
/// *after* writing to the vec, and check the value *before* reading the vec. The invariant to be
/// maintained is that at any point in time, the values up to `self.len` are always fully written.
///
/// # Safety
/// We introduce the following terminology: a reference to a page is *safe* if it is an immutable
/// reference to an allocated page. A reference to a page is *unsafe* if it is a reference, mutable
/// or not, to an unallocated page. One should *never* create a mutable reference to an allocated
/// page (we write with `std::ptr::write`).
///
/// The safety invariant enforced and assumed throughout is that the lock must be taken when
/// creating an unsafe reference, and the unsafe reference cannot outlive the lock. Safe references
/// can be created and returned any time.
///
/// Note that in general, one has to obtain an immutable reference to a page before determining
/// whether it has been allocated. However, if we can infer from `self.len` that something has been
/// written to the `n`th page already, then the `n`th page must have been allocated.
///
/// The other safety invariant we maintain is that we only write to pages when the lock has been
/// taken.
pub struct OnceVec<T> {
    len: AtomicUsize,
    /// [`BTreeSet`] of elements that have been added out of order. We also use this mutex to
    /// prevent conflicting concurrent pushes. We use a newtype to wrap the [`BTreeSet`] because
    /// we want [`OnceVec::lock`] to be public, but we don't want to let people mess with the
    /// internals of the tracker.
    ooo: Mutex<OooTracker>,
    data: NonNull<Page<T>>,
}

impl<T> Drop for OnceVec<T> {
    fn drop(&mut self) {
        let len = self.len();

        unsafe {
            // The lock may be poisoned. Access is always safe because we have mutable reference,
            // but if we can acquire the lock we want to drop the elements inside. If the lock is
            // poisoned, then we are probably panicking so we don't care about memory leakage.
            if let Ok(ooo) = self.ooo.lock() {
                let ooo_iter = ooo.0.iter();
                for entry in ooo_iter {
                    std::ptr::drop_in_place(self.entry_ptr(*entry));
                }
            }
            for idx in 0..MAX_OUTER_LENGTH {
                // We have mutable reference so we can do whatever we want
                let page = &mut *self.data.as_ptr().add(idx);
                page.deallocate(len, idx);
            }
            alloc::alloc::dealloc(self.data.as_ptr() as *mut u8, DATA_LAYOUT);
        }
    }
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
    pub fn from_vec(mut vec: Vec<T>) -> Self {
        let mut result = Self::new();
        let len = vec.len();
        if len == 0 {
            return result;
        }

        *result.len.get_mut() = len;
        unsafe {
            result.allocate_for(len - 1);
            let (max_page, max_index) = inner_index(len - 1);
            for page in 0..max_page {
                std::ptr::copy_nonoverlapping(
                    vec.as_ptr().add((1 << page) - 1),
                    (*result.page_raw(page)).ptr(),
                    1 << page,
                );
            }
            std::ptr::copy_nonoverlapping(
                vec.as_ptr().add((1 << max_page) - 1),
                (*result.page_raw(max_page)).ptr(),
                max_index + 1,
            );
            // Don't drop the elements, but deallocate the vector
            vec.set_len(0);
        }

        result
    }

    pub fn new() -> Self {
        Self {
            len: AtomicUsize::new(0),
            ooo: Mutex::new(OooTracker::default()),
            // 0 represents the value None
            data: NonNull::new(unsafe { alloc::alloc::alloc_zeroed(DATA_LAYOUT) as *mut Page<T> })
                .unwrap(),
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
            // This is safe because the original index is < len.
            unsafe { Some(&*self.entry_ptr(index)) }
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.len() {
            // This is safe because the original index is < len.
            unsafe { Some(&mut *self.entry_ptr(index)) }
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

    /// # Returns
    /// A `*mut T` that is guaranteed to be valid for the lifetime of the borrow of `self`.
    ///
    /// # Safety
    /// If the index has not been written to, then the lock has have been taken, and no unsafe
    /// page references should exist.
    unsafe fn entry_ptr(&self, index: usize) -> *mut T {
        let (page, idx) = inner_index(index);
        (*self.page_raw(page)).ptr().add(idx)
    }

    /// # Returns
    /// A raw pointer to the `page`th page. This is guaranteed to be valid within the lifetime of
    /// the borrow of `self`.
    ///
    /// # Safety
    ///
    /// This function is safe in the sense that creating a raw pointer is always safe. Safety
    /// invariants as described in the top-level documentation must be upheld when turning this
    /// into a reference.
    fn page_raw(&self, page: usize) -> *mut Page<T> {
        assert!(page < MAX_OUTER_LENGTH);
        unsafe {
            // Only the `add` call is potentially unsafe, but since `page < MAX_OUTER_LENGTH`,
            // this is within our
            self.data.as_ptr().add(page)
        }
    }

    /// Obtain an immutable reference to the `page`th page. This panics if there has not been any
    /// elements written to the `page`th page, in which case the reference is (potentially) unsafe.
    /// Use `page
    fn page(&self, page: usize) -> &Page<T> {
        assert!(
            !self.is_empty(),
            "Cannot safely borrow a page when it is empty"
        );
        let (max_page, _) = inner_index(self.len() - 1);
        assert!(page <= max_page, "Cannot safely borrow an unallocated page");

        // This is
        unsafe { &*self.page_raw(page) }
    }

    /// Takes a lock on the `OnceVec`. The `OnceVec` cannot be updated while the lock is held.
    /// This is useful when used in conjuction with [`OnceVec::extend`];
    pub fn lock(&self) -> MutexGuard<OooTracker> {
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
        assert!(ooo.0.is_empty());
        let old_len = self.len.load(Ordering::Acquire);
        let (page, index) = inner_index(old_len);

        unsafe {
            if index == 0 {
                let page_ptr = self.page_raw(page);
                // Safety: since lock has been taken, we can create unsafe references.
                if !(*page_ptr).allocated() {
                    (*page_ptr).allocate(page);
                }
            }

            // The write is safe because `entry_ptr` returns valid pointers, and we have taken the
            // lock so there are no simultaneous read/writes of this particular pointer location.
            std::ptr::write(self.entry_ptr(old_len), value);
        }

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

        unsafe {
            // Safe since we have not made any unsafe references yet
            self.allocate_for(index);
            std::ptr::write(self.entry_ptr(index), value);
        }
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

    /// Allocate enough space to fit `new_max` many elements in total. No reference to `self.data`
    /// data may be alive when this function is called. In particular, the `OnceVec` ought to be
    /// locked.
    ///
    /// # Safety
    /// Lock must be taken and there should be no other unsafe references.
    unsafe fn allocate_for(&self, new_max: usize) {
        let max_page = inner_index(new_max).0;
        for i in 0..=max_page {
            let page_ptr = self.page_raw(i);
            // Safety assumption is propagated up call chain
            if !(*page_ptr).allocated() {
                // Only make mutable reference if the page is not allocated
                (*page_ptr).allocate(i);
            }
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
        let ooo = self.lock();
        assert!(ooo.0.is_empty());
        let old_len = self.len.load(Ordering::Acquire);
        if new_max < old_len {
            return;
        }

        unsafe {
            // We have taken locks and have created no unsafe references
            self.allocate_for(new_max);

            for i in old_len..=new_max {
                // This is safe because self.entry_ptr(i) is valid and lock has been taken to avoid
                // concurrent read/write
                std::ptr::write(self.entry_ptr(i), f(i));

                // Do it inside the loop because f may use self
                self.len.store(i + 1, Ordering::Release)
            }
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
        let page_end = if len == 0 {
            0
        } else {
            inner_index(len - 1).0 + 1
        };

        (0..page_end).flat_map(move |page| unsafe {
            // Safety: tautological
            self.page(page).as_slice(len, page)
        })
    }
}

impl<T: Send + Sync> OnceVec<T> {
    /// A parallel version of `extend`. If the `concurrent` feature is enabled, the function `f`
    /// will be run for different indices simultaneously using [`rayon`].
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

        unsafe {
            // This is safe since we have taken the lock and have made no unsafe references.
            self.allocate_for(new_max);

            (old_len..=new_max).into_maybe_par_iter().for_each(|i| {
                // These pointers are all non-aliasing so they can be written concurrently.
                std::ptr::write(self.entry_ptr(i), f(i));
            });
        }
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
        Self {
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
    pub fn lock(&self) -> MutexGuard<OooTracker> {
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
    /// will be run for different indices simultaneously using [`rayon`].
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
    use super::*;
    // use rstest::rstest_parametrize;

    #[test]
    fn test_inner_index() {
        assert_eq!(inner_index(0), (0, 0));
        assert_eq!(inner_index(1), (1, 0));
        assert_eq!(inner_index(2), (1, 1));
        assert_eq!(inner_index(3), (2, 0));
    }

    #[test]
    fn test_push() {
        let v = OnceVec::new();
        for i in 0u32..1000u32 {
            v.push(i);
            println!("i : {i}");
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
}
