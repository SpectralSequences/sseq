use std::{marker::PhantomData, mem::ManuallyDrop, num::NonZero, sync::atomic::Ordering};

use crate::std_or_loom::{
    GetMut, Zeroable,
    sync::atomic::{AtomicPtr, AtomicUsize},
};

/// An allocation that can store a fixed number of elements.
#[derive(Debug)]
// `repr(C)` so that an empty trie node reads as either variant under loom; see `Node`'s `Zeroable`
// impl.
#[cfg_attr(loom, repr(C))]
#[cfg_attr(not(loom), derive(Zeroable), zeroable(bound = ""))]
pub struct Block<E> {
    /// The number of elements in the block.
    len: AtomicUsize,

    /// A pointer to the data buffer.
    ///
    /// If `size` is nonzero, this points to a slice of `E` of that size. If `size` is
    /// zero, this is a null pointer.
    ///
    /// It would be more convenient to use `AtomicPtr<[E]>` here. However, `AtomicPtr<T>`
    /// requires `T: Sized`. This is because pointers to DSTs are fat, and take two words in memory.
    /// Most architectures can operate atomically on at most one word.
    data: AtomicPtr<E>,

    /// The block owns the elements `data` points to.
    _owns: PhantomData<E>,
}

impl<E: Zeroable> Block<E> {
    /// Create a new block.
    pub(super) fn new() -> Self {
        Self {
            len: AtomicUsize::new(0),
            data: AtomicPtr::new(std::ptr::null_mut()),
            _owns: PhantomData,
        }
    }

    pub(super) fn is_init(&self) -> bool {
        !self.data.load(Ordering::Acquire).is_null()
    }

    pub(super) fn data(&self) -> &AtomicPtr<E> {
        &self.data
    }

    /// Initialize the block with a given size.
    ///
    /// # Safety
    ///
    /// For any given block, this method must always be called with the same size.
    pub(super) unsafe fn init(&self, size: NonZero<usize>) {
        if self.data.load(Ordering::Relaxed).is_null() {
            // We need to initialize the block

            let mut data_buffer = ManuallyDrop::new(zeroed_vec::<E>(size.get()));
            let data_ptr = data_buffer.as_mut_ptr();

            // We can use `Relaxed` here because we will release-store the data pointer, and so any
            // aquire-load of the data pointer will also see the instructions before it, in
            // particular this store.
            //
            // Note that potentially many threads could be trying to initialize the block at the
            // same time, and execute this store. This is not a problem, since by assumption all
            // those threads will attempt to store the same value.
            self.len.store(size.get(), Ordering::Relaxed);

            // `Release` means that any thread that sees the data pointer will also see the size. We
            // can use `Relaxed` for the failure case because we don't need to synchronize with any
            // other atomic operation.
            if self
                .data
                .compare_exchange(
                    std::ptr::null_mut(),
                    data_ptr,
                    Ordering::Release,
                    Ordering::Relaxed,
                )
                .is_err()
            {
                // Another thread initialized the block before us
                // Safety: this is the last use of `data_buffer`
                unsafe { ManuallyDrop::drop(&mut data_buffer) };
            }
        }
    }

    // **NOTE**: The following methods load the data pointer with `Acquire` ordering. If we
    // strengthen the safety contract to require that the initialized state was *observed* by the
    // caller, we could use `Relaxed` ordering here. In practice, this struct is only used by
    // `Grove`, which does always call `init` first, so this is entirely reasonable. However, it
    // seems to hit this bug in loom: https://github.com/tokio-rs/loom/issues/260
    //
    // The performance hit seems to be negligible, and the correctness checks that loom offer are
    // more important, so we stick with `Acquire` for now.

    /// Return the element at the given index.
    ///
    /// # Safety
    ///
    /// The block must be initialized, and `index` within it.
    pub(super) unsafe fn entry(&self, index: usize) -> &E {
        let data_ptr = self.data.load(Ordering::Acquire);
        // SAFETY: the block is initialized, so `data_ptr` points to its elements, and `index` is in
        // bounds by assumption.
        unsafe { &*data_ptr.add(index) }
    }

    /// Return a mutable reference to the element at the given index, if the block is initialized.
    pub(super) fn get_mut(&mut self, index: usize) -> Option<&mut E> {
        let len = self.len.get_by_mut();
        if index >= len {
            return None;
        }
        let data_ptr = self.data.get_by_mut();
        // Safety: index < len, so the pointer is in bounds. We reference a single element
        // rather than creating a slice over the entire block, so that the resulting `&mut`
        // does not alias other elements in the same allocation.
        Some(unsafe { &mut *data_ptr.add(index) })
    }
}

// SAFETY: a shared block hands out `&E` to every thread, and an element filled through it on one
// thread can be dropped on another. `Send` follows from `PhantomData<E>` without help.
unsafe impl<E: Send + Sync> Sync for Block<E> {}

impl<E> Drop for Block<E> {
    fn drop(&mut self) {
        let len = self.len.get_by_mut();
        let data_ptr = self.data.get_by_mut();
        if !data_ptr.is_null() {
            // Safety: initialization stores a pointer that came from exactly such a vector
            unsafe { Vec::from_raw_parts(data_ptr, len, len) };
            // vector is dropped here
        }
    }
}

// The allocator zeroes the allocation, so pages it fetches fresh from the OS are not written until
// an element in them is.
#[cfg(not(loom))]
use bytemuck::allocation::zeroed_vec;

/// `n` zeroed elements, built one by one since loom's atomics are not valid as zero bytes.
#[cfg(loom)]
fn zeroed_vec<E: Zeroable>(n: usize) -> Vec<E> {
    std::iter::repeat_with(E::zeroed).take(n).collect()
}
