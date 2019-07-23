//! A fast segmented stack allocator, supporting multiple objects of any type.
//!
//! A type of arena allocator, obstacks deallocate memory all at once, when the `Obstack` itself is
//! destroyed. The benefit is extremely fast allocation: just a pointer bump. Unlike a typed arena,
//! a single `Obstack` can contain values with any number of different types.
//!
//! For `Copy` types pushing a value to an `Obstack` returns a standard mutable reference:
//!
//!     # use obstack::Obstack;
//!     # let stack = Obstack::new();
//!     let r: &mut u8 = stack.push_copy(42);
//!     assert_eq!(*r, 42);
//!
//! As `Copy` types can't implement `Drop`, nothing needs to be done to deallocate the value beyond
//! deallocating the memory itself, which is done en-mass when the `Obstack` itself is dropped.
//! `push_copy()` is thus limited to types that implement `Copy`.
//!
//! Types that do not implement `Copy` *may* implement `Drop`. As Rust's type system doesn't have a
//! negative `!Drop` trait, `Obstack` has a second method - `push()` - that is not restricted to
//! `Copy` types. This method returns the wrapper type `Ref<T>`, that wraps the underlying mutable
//! reference. This wrapper owns the value on the stack, and ensures that `drop` is
//! called when the wrapper goes out of scope. Essentially `Ref` is the equivalent of `Box`, but
//! using an `Obstack` rather than the heap.
//!
//! In practice even when the `Ref` wrapper is used, if the underlying type doesn't actually
//! implement a meaningful `drop` method, the Rust compiler is able to optimize away all calls to
//! `drop`, resulting in the same performance as for `Copy` types; the actual `Ref::drop()` method
//! is `[inline(always)]` to aid this process. This is important as not all non-drop types can
//! implement `Copy` - notably mutable references can't.
//!
//! `Obstack` allocates memory as a segmented stack consisting of one or more segments of
//! contiguous memory. Each time the top segment becomes full, a new segment is allocated from the
//! heap. To ensure the total number of allocations remains small, segments are allocated in a
//! powers-of-two fashion, with each segment being twice the size of the previous one.
//!
//! Once a segment has been allocated it's stable for the life of the `Obstack`, allowing
//! the values contained in that segment to be referenced directly; Rust lifetimes ensure that the
//! references are valid for the lifetime of the `Obstack`.

#[cfg(test)]
extern crate rand;

use std::cell::UnsafeCell;
use std::cmp;
use std::fmt;
use std::mem;
use std::ptr;
use std::slice;

mod alignedvec;
use alignedvec::AlignedVec;

pub const DEFAULT_INITIAL_CAPACITY: usize = 256;

/// An obstack
#[derive(Debug)]
pub struct Obstack {
    state: UnsafeCell<State<usize>>,
}

impl Obstack {
    /// Constructs a new `Obstack` with the specified initial capacity.
    ///
    /// The obstack will be able to allocate at least `initial_capacity` bytes before having to
    /// allocate again.
    pub fn with_initial_capacity(initial_capacity: usize) -> Self {
        let n = initial_capacity;
        let n = if n.is_power_of_two() { n } else { n.next_power_of_two() };

        let state = State::new(n);
        Obstack {
            state: UnsafeCell::new(state)
        }
    }

    /// Constructs a new `Obstack`.
    ///
    /// The initial capacity will be set to `DEFAULT_INITIAL_CAPACITY`.
    pub fn new() -> Self {
        Self::with_initial_capacity(DEFAULT_INITIAL_CAPACITY-1)
    }

    /// Pushes a value to the `Obstack`.
    ///
    /// Returns a `Ref` that can be dereferenced to the value's location on the stack.
    ///
    ///     # use std::convert::From;
    ///     # use obstack::{Obstack, Ref};
    ///     # let stack = Obstack::new();
    ///     let r: Ref<String> = stack.push(String::from("Hello World!"));
    ///     assert_eq!(*r, "Hello World!");
    ///
    #[inline]
    pub fn push<'a, T>(&'a self, value: T) -> Ref<'a, T> {
        let ptr = self.alloc(&value) as *mut T;
        unsafe {
            ptr::write(ptr, value);

            Ref {
                ptr: &mut *ptr,
            }
        }
    }

    /// Pushes a `Copy` value to the `Obstack`.
    ///
    /// Returns a mutable reference to the value on the stack.
    ///
    ///     # use obstack::Obstack;
    ///     # let stack = Obstack::new();
    ///     let r: &mut [u8; 5] = stack.push_copy([1,2,3,4,5]);
    ///     assert_eq!(*r, [1,2,3,4,5]);
    ///
    #[inline]
    pub fn push_copy<'a, T>(&'a self, value: T) -> &'a mut T
        where T: Copy,
    {
        unsafe {
            let r = &mut *(self.alloc(&value) as *mut T);
            *r = value;
            r
        }
    }

    /// Copies values from a slice to the `Obstack`.
    ///
    /// Returns a mutable reference to a newly allocated slice:
    ///
    /// ```
    /// # use obstack::Obstack;
    /// # let stack = Obstack::new();
    /// let v: Box<[u8]> = Box::new([1,2,3,4,5]);
    /// let r: &mut [u8] = stack.copy_from_slice(&v[0..3]);
    ///
    /// assert_eq!(r.len(), 3);
    /// assert_eq!(r, &[1,2,3][..]);
    /// ```
    ///
    /// Zero-length slices work as expected without allocations:
    ///
    /// ```
    /// # use obstack::Obstack;
    /// # let stack = Obstack::new();
    /// let prev_used = stack.bytes_used();
    /// let r: &mut [u8] = stack.copy_from_slice(&[]);
    ///
    /// assert_eq!(prev_used, stack.bytes_used());
    /// assert_eq!(r.len(), 0);
    /// assert_eq!(r, &[]);
    /// ```
    ///
    /// As do slices of zero-sized types:
    ///
    /// ```
    /// # use std::usize;
    /// # use obstack::Obstack;
    /// # let stack = Obstack::new();
    /// let v: Box<[()]> = Box::new([(); 1_000]);
    /// let prev_used = stack.bytes_used();
    /// let r: &mut [()] = stack.copy_from_slice(&v);
    ///
    /// assert_eq!(prev_used, stack.bytes_used());
    /// assert_eq!(r.len(), 1_000);
    /// assert!(r == &[(); 1_000][..]);
    /// ```
    #[inline]
    pub fn copy_from_slice<'a, T>(&'a self, src: &[T]) -> &'a mut [T]
        where T: Copy,
    {
        unsafe {
            let ptr = self.alloc(src) as *mut T;
            let r = slice::from_raw_parts_mut(ptr, src.len());
            r.copy_from_slice(src);
            r
        }
    }

    /// Returns the total bytes currently used by values.
    ///
    /// Includes bytes used for alignment padding. However, this figure is *not* the total size
    /// *allocated* by the `Obstack`, which would include bytes allocated for parts of segments
    /// that haven't been used yet. Thus the return value of this method will change after every
    /// non-zero-sized value allocated.
    ///
    /// `bytes_used` always starts at zero:
    ///
    /// ```
    /// # use obstack::Obstack;
    /// let stack = Obstack::new();
    /// assert_eq!(stack.bytes_used(), 0);
    /// ```
    pub fn bytes_used(&self) -> usize {
        unsafe {
            let state = &*self.state.get();

            state.tip.len_bytes()
            + state.used_slabs.iter()
                              .map(|used_slab| used_slab.len_bytes())
                              .sum::<usize>()
        }
    }

    /// Alocates memory for a value, without initializing it.
    #[inline]
    fn alloc<'a, T: ?Sized>(&'a self, value_ref: &T) -> *mut u8 {
        let size = mem::size_of_val(value_ref);
        let alignment = mem::align_of_val(value_ref);

        if size > 0 {
            unsafe {
                let state = &mut *self.state.get();
                state.alloc(size, alignment) as *mut u8
            }
        } else {
            mem::align_of_val(value_ref) as *mut u8
        }
    }
}

/// A wrapper referencing a value in an `Obstack`.
///
/// A `Ref` value owns the value it references, and will invoke `drop` on the value when the `Ref`
/// goes out of scope. Effectively a `Ref` is a `Box` that uses an `Obstack` rather than the heap.
///
/// The inherent methods of `Ref` are all associated functions, which means you have to call them
/// as e.g. `Ref::unwrap(value)` instead of `value.unwrap()`. This avoids conflicts with methods of
/// the inner type `T`.
pub struct Ref<'a, T: 'a + ?Sized> {
    ptr: &'a mut T,
}

impl<'a, T: ?Sized> Ref<'a, T> {
    /// Returns the owned value, consuming the `Ref`.
    ///
    /// This allows the value to taken out of the `Obstack` and used even after it goes out of
    /// scope:
    ///
    /// ```
    /// # use obstack::{Obstack, Ref};
    /// fn f() -> String {
    ///     let stack = Obstack::new();
    ///     let r = stack.push(String::from("foo"));
    ///
    ///     Ref::unwrap(r)
    /// }
    ///
    /// assert_eq!(f(), "foo");
    /// ```
    ///
    /// Since obstacks only free memory when they go out of scope, the `bytes_used` remains
    /// unchanged:
    ///
    /// ```
    /// # use obstack::{Obstack, Ref};
    /// # let stack = Obstack::new();
    /// let r = stack.push(String::new());
    ///
    /// let used = stack.bytes_used();
    /// let inner = Ref::unwrap(r);
    ///
    /// assert_eq!(used, stack.bytes_used());
    /// # assert_eq!(inner, "");
    /// ```
    #[inline]
    pub fn unwrap(this: Self) -> T
        where T: Sized
    {
        unsafe {
            let ptr: *const T = this.ptr;
            mem::forget(this);
            ptr::read(ptr)
        }
    }
}

impl<'a, T: ?Sized> Drop for Ref<'a, T> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(self.ptr);
        }
    }
}


#[derive(Debug)]
struct State<A> {
    tip: AlignedVec<A>,
    used_slabs: Vec<AlignedVec<A>>,
}

impl<A> State<A> {
    fn next_capacity(prev_capacity: usize, required: usize, alignment: usize) -> usize {
        cmp::max(prev_capacity + 1, required + alignment)
            .next_power_of_two()
    }

    fn new(min_initial_capacity: usize) -> State<A> {
        let capacity = Self::next_capacity(0, min_initial_capacity, 0);
        State {
           tip: AlignedVec::with_capacity_bytes(capacity),
           used_slabs: Vec::new(),
        }
    }

    #[inline(never)]
    unsafe fn alloc_from_new_slab(&mut self, size: usize, alignment: usize) -> *mut A {
        let new_capacity = Self::next_capacity(self.tip.capacity_bytes(),
                                               size, alignment);
        let new_tip = AlignedVec::with_capacity_bytes(new_capacity);
        let old_tip = mem::replace(&mut self.tip, new_tip);
        self.used_slabs.push(old_tip);

        self.alloc(size, alignment)
    }

    #[inline]
    unsafe fn alloc(&mut self, size: usize, alignment: usize) -> *mut A {
        let start_ptr = self.tip.as_mut_ptr()
                                .offset(self.tip.len_items() as isize);

        let padding = start_ptr as usize % alignment;

        debug_assert!(padding < alignment);
        debug_assert_eq!(padding, 0);

        let start_ptr = start_ptr.offset(AlignedVec::<A>::bytes_to_items(padding) as isize);

        let new_used = self.tip.len_items() + padding + AlignedVec::<A>::bytes_to_items(size);

        if new_used <= self.tip.capacity_items() {
            self.tip.set_len_items(new_used);
            start_ptr as *mut A
        } else {
            self.alloc_from_new_slab(size, alignment)
        }
    }
}


impl fmt::Display for Obstack {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        unsafe {
            let state = &*self.state.get();

            write!(f, "Obstack Slabs:\n")?;

            write!(f, "    {:p}: size = {}, used = {}\n",
                   state.tip.as_ptr(),
                   state.tip.capacity_bytes(),
                   state.tip.len_bytes())?;

            for slab in state.used_slabs.iter().rev() {
                write!(f, "    {:p}: size = {}, used = {}\n",
                       slab.as_ptr(), slab.capacity_bytes(), slab.len_bytes())?;
            }
            Ok(())
        }
    }
}

mod impls;
pub use impls::*;

#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::Cell;
    use std::ops::{Deref, DerefMut};
    use std::rc::Rc;
    use std::thread;

    use rand::{Rng, thread_rng};

}
