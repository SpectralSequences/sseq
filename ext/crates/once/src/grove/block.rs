use std::{mem::ManuallyDrop, num::NonZero, sync::atomic::Ordering};

use crate::{
    std_or_loom::{
        GetMut,
        sync::atomic::{AtomicPtr, AtomicUsize},
    },
    write_once::WriteOnce,
};

/// An allocation that can store a fixed number of elements.
#[derive(Debug)]
pub struct Block<T> {
    /// The number of elements in the block.
    len: AtomicUsize,

    /// A pointer to the data buffer.
    ///
    /// If `size` is nonzero, this points to a slice of `WriteOnce<T>` of that size. If `size` is
    /// zero, this is a null pointer.
    ///
    /// It would be more convenient to use `AtomicPtr<[WriteOnce<T>]` here. However, `AtomicPtr<T>`
    /// requires `T: Sized`. This is because pointers to DSTs are fat, and take two words in memory.
    /// Most architectures can operate atomically on at most one word.
    data: AtomicPtr<WriteOnce<T>>,
}

impl<T> Block<T> {
    /// Create a new block.
    pub(super) fn new() -> Self {
        Self {
            len: AtomicUsize::new(0),
            data: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    pub(super) fn is_init(&self) -> bool {
        !self.data.load(Ordering::Acquire).is_null()
    }

    pub(super) fn data(&self) -> &AtomicPtr<WriteOnce<T>> {
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

            // NB: Benchmarking shows that using `alloc_zeroed` is somehow significantly slower than
            // just pushing repeatedly to a Vec.
            let mut data_buffer = ManuallyDrop::new(Vec::with_capacity(size.get()));
            for _ in 0..size.get() {
                data_buffer.push(WriteOnce::none());
            }
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

    /// Insert a value at the given index.
    ///
    /// # Safety
    ///
    /// The block must be initialized.
    pub(super) unsafe fn insert(&self, index: usize, value: T) {
        let data_ptr = self.data.load(Ordering::Acquire);
        let len = self.len.load(Ordering::Relaxed);
        // Safety: the block has been initialized
        let data = unsafe { std::slice::from_raw_parts(data_ptr, len) };
        data[index].set(value);
    }

    /// Attempt to insert a value at the given index.
    ///
    /// # Safety
    ///
    /// The block must be initialized.
    pub(super) unsafe fn try_insert(&self, index: usize, value: T) -> Result<(), T> {
        // We can use Relaxed operations here because the initialization of the block has a
        // happens-before relationship with this operation, by assumption.
        let data_ptr = self.data.load(Ordering::Acquire);
        let len = self.len.load(Ordering::Relaxed);
        // Safety: the block has been initialized
        let data = unsafe { std::slice::from_raw_parts(data_ptr, len) };
        data[index].try_set(value)
    }

    /// Return the value at the given index.
    ///
    /// # Safety
    ///
    /// The block must be initialized.
    pub(super) unsafe fn get(&self, index: usize) -> Option<&T> {
        // We can use Relaxed operations here because the initialization of the block has a
        // happens-before relationship with this operation, by assumption.
        let data_ptr = self.data.load(Ordering::Acquire);
        let len = self.len.load(Ordering::Relaxed);
        // Safety: the block has been initialized
        let data = unsafe { std::slice::from_raw_parts(data_ptr, len) };
        data.get(index).and_then(|w| w.get())
    }

    /// Return a mutable reference to the value at the given index if it exists.
    ///
    /// This method is safe to call even if the block is uninitialized.
    pub(super) fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let len = self.len.get_by_mut();
        if len == 0 {
            return None;
        }
        let data_ptr = self.data.get_by_mut();
        // Safety: we just observed the length to be nonzero, so the pointer is not null
        let data = unsafe { std::slice::from_raw_parts_mut(data_ptr, len) };
        data.get_mut(index).and_then(|w| w.get_mut())
    }

    /// Return the value at the given index.
    ///
    /// # Safety
    ///
    /// The block must be initialized.
    pub(super) unsafe fn is_set(&self, index: usize) -> bool {
        // We can use Relaxed operations here because the initialization of the block has a
        // happens-before relationship with this operation, by assumption.
        let data_ptr = self.data.load(Ordering::Acquire);
        let len = self.len.load(Ordering::Relaxed);
        // Safety: the block has been initialized
        let data = unsafe { std::slice::from_raw_parts(data_ptr, len) };
        data.get(index).is_some_and(|w| w.is_set())
    }
}

impl<T> Drop for Block<T> {
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
