use std::{cell::UnsafeCell, mem::MaybeUninit, sync::atomic::Ordering};

use crate::std_or_loom::{sync::atomic::AtomicU8, GetMut};

/// A thread-safe, wait-free, write-once cell that allows a value to be set exactly once.
///
/// `WriteOnce` provides a way to initialize a value exactly once in a thread-safe manner. Once a
/// value is set, it cannot be changed. This is useful in scenarios where you need to ensure that a
/// value is initialized exactly once, even in the presence of concurrent access.
///
/// `WriteOnce` is similar to [`std::sync::OnceLock`], but with a key difference: it takes a value
/// directly rather than a closure that initializes the value. In other words, we assume an
/// optimistic concurrency model, contrary to `OnceLock`'s pessimistic model.
///
/// This concurrency model allows `WriteOnce` to be fully wait-free. This also implies that, whereas
/// `OnceLock` guarantees that the closure is called at most once, `WriteOnce` guarantees that the
/// value is *set* at most once.
///
/// # Thread Safety
///
/// `WriteOnce` is designed to be thread-safe and wait-free, allowing concurrent attempts to set the
/// value from multiple threads. Only the first successful call to `set` or `try_set` will
/// initialize the value; subsequent attempts will either panic (with `set`) or return an error
/// (with `try_set`).
///
/// # Memory Safety
///
/// `WriteOnce` uses atomic operations to ensure memory safety and proper synchronization between
/// threads. It guarantees that:
///
/// - A value can be set exactly once
/// - Once set, the value can be safely read from any thread
/// - The value is properly dropped when the `WriteOnce` is dropped
///
/// # Examples
///
/// ```
/// use once::write_once::WriteOnce;
///
/// // Create a new WriteOnce with no value
/// let cell = WriteOnce::<String>::none();
///
/// // Set the value
/// cell.set("Hello, world!".to_string());
///
/// // Get the value
/// assert_eq!(cell.get(), Some(&"Hello, world!".to_string()));
///
/// // Attempting to set the value again will panic
/// // cell.set("Another value".to_string()); // This would panic
///
/// // Using try_set instead returns an error
/// let result = cell.try_set("Another value".to_string());
/// assert!(result.is_err());
/// if let Err(value) = result {
///     assert_eq!(value, "Another value".to_string());
/// }
/// ```
pub struct WriteOnce<T> {
    value: UnsafeCell<MaybeUninit<T>>,
    state: AtomicU8,
}

impl<T> WriteOnce<T> {
    /// Creates a new `WriteOnce` with no value.
    ///
    /// This initializes the cell in an empty state. The value can be set later using
    /// [`set`](Self::set) or [`try_set`](Self::try_set).
    ///
    /// # Examples
    ///
    /// ```
    /// use once::write_once::WriteOnce;
    ///
    /// let cell = WriteOnce::<i32>::none();
    /// assert_eq!(cell.get(), None);
    /// assert!(!cell.is_set());
    /// ```
    pub fn none() -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::uninit()),
            state: AtomicU8::new(WriteOnceState::Uninit as u8),
        }
    }

    /// Creates a new `WriteOnce` with a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::write_once::WriteOnce;
    ///
    /// let cell = WriteOnce::new(2);
    /// assert_eq!(cell.get(), Some(&2));
    /// assert!(cell.is_set());
    /// ```
    pub fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::new(value)),
            state: AtomicU8::new(WriteOnceState::Init as u8),
        }
    }

    /// Sets the value of the `WriteOnce`.
    ///
    /// This method sets the value of the cell if it hasn't been set yet.
    /// If the cell already has a value, this method will panic.
    ///
    /// # Panics
    ///
    /// Panics if the cell already has a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::write_once::WriteOnce;
    ///
    /// let cell = WriteOnce::<i32>::none();
    /// cell.set(42);
    /// assert_eq!(cell.get(), Some(&42));
    ///
    /// // Uncommenting the following line would cause a panic:
    /// // cell.set(100);
    /// ```
    pub fn set(&self, value: T) {
        assert!(self.try_set(value).is_ok(), "WriteOnce already set");
    }

    /// Attempts to set the value of the `WriteOnce`.
    ///
    /// This method tries to set the value of the cell if it hasn't been set yet.
    /// If the cell already has a value, this method will return an error containing
    /// the value that was attempted to be set.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the value was successfully set
    /// - `Err(value)` if the cell already had a value, returning the value that was
    ///   attempted to be set
    ///
    /// # Examples
    ///
    /// ```
    /// use once::write_once::WriteOnce;
    ///
    /// let cell = WriteOnce::<String>::none();
    ///
    /// // First attempt succeeds
    /// let result = cell.try_set("Hello".to_string());
    /// assert!(result.is_ok());
    /// assert_eq!(cell.get(), Some(&"Hello".to_string()));
    ///
    /// // Second attempt fails
    /// let result = cell.try_set("World".to_string());
    /// assert!(result.is_err());
    /// if let Err(value) = result {
    ///     assert_eq!(value, "World".to_string());
    /// }
    ///
    /// // The value remains unchanged
    /// assert_eq!(cell.get(), Some(&"Hello".to_string()));
    /// ```
    pub fn try_set(&self, value: T) -> Result<(), T> {
        // Initially, `is_some` is `Uninit`, so it's impossible to observe anything else without a
        // prior `set`. Therefore, we will never panic if `set` was never called.
        match self.state.compare_exchange(
            WriteOnceState::Uninit as u8,
            WriteOnceState::Writing as u8,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => {
                unsafe { self.value.get().write(MaybeUninit::new(value)) }
                // This store creates a happens-before relationship with the load in `get`
                self.state
                    .store(WriteOnceState::Init as u8, Ordering::Release);
                Ok(())
            }
            Err(_) => Err(value),
        }
    }

    /// Gets the value of the `WriteOnce`.
    ///
    /// Returns `Some(&T)` if the cell has a value, or `None` if it doesn't.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::write_once::WriteOnce;
    ///
    /// let cell = WriteOnce::<i32>::none();
    /// assert_eq!(cell.get(), None);
    ///
    /// cell.set(42);
    /// assert_eq!(cell.get(), Some(&42));
    /// ```
    pub fn get(&self) -> Option<&T> {
        if self.is_set() {
            // Safety: the value is initialized
            let value = unsafe { (*self.value.get()).assume_init_ref() };
            Some(value)
        } else {
            None
        }
    }

    /// # Safety
    ///
    /// This method is unsafe because it returns a reference to the value without checking if the
    /// value is initialized. The caller must ensure that the value is initialized before calling
    /// this method.
    pub unsafe fn get_unchecked(&self) -> &T {
        // Safety: by assumption
        unsafe { (*self.value.get()).assume_init_ref() }
    }

    /// Checks if the `WriteOnce` has a value.
    ///
    /// Returns `true` if the cell has a value, or `false` if it doesn't.
    ///
    /// This method only returns `true` if the value has been set and is ready to be read. In
    /// particular, it will return `false` if the value is currently being set by another thread.
    /// This may be relevant if the value is a large object and moving it in memory is expensive.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::write_once::WriteOnce;
    ///
    /// let cell = WriteOnce::<i32>::none();
    /// assert!(!cell.is_set());
    ///
    /// cell.set(42);
    /// assert!(cell.is_set());
    /// ```
    pub fn is_set(&self) -> bool {
        self.state.load(Ordering::Acquire) == WriteOnceState::Init as u8
    }

    /// Gets a mutable reference to the value of the `WriteOnce`.
    ///
    /// Returns `Some(&mut T)` if the cell has a value, or `None` if it doesn't.
    /// This method requires a mutable reference to the `WriteOnce`, which ensures
    /// that no other thread is accessing the value.
    ///
    /// # Examples
    ///
    /// ```
    /// use once::write_once::WriteOnce;
    ///
    /// let mut cell = WriteOnce::<String>::none();
    /// assert_eq!(cell.get_mut(), None);
    ///
    /// cell.set("Hello".to_string());
    ///
    /// // Modify the value through a mutable reference
    /// if let Some(value) = cell.get_mut() {
    ///     value.push_str(", world!");
    /// }
    ///
    /// assert_eq!(cell.get(), Some(&"Hello, world!".to_string()));
    /// ```
    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.state.get_by_mut() == WriteOnceState::Init as u8 {
            // Safety: the value is initialized
            let value = unsafe { (*self.value.get()).assume_init_mut() };
            Some(value)
        } else {
            None
        }
    }
}

impl<T> Drop for WriteOnce<T> {
    fn drop(&mut self) {
        // We have an exclusive reference to `self`, so we know that no other thread is accessing
        // it. Moreover, we also have a happens-before relationship with all other operations on
        // this `WriteOnce`, including a possible `set` that initialized the value. Therefore, the
        // following code will never lead to a memory leak.
        if self.state.get_by_mut() == WriteOnceState::Init as u8 {
            // Safety: the value is initialized
            unsafe { self.value.get_mut().assume_init_drop() };
        }
    }
}

impl<T: Clone> Clone for WriteOnce<T> {
    fn clone(&self) -> Self {
        if let Some(value) = self.get() {
            Self::new(value.clone())
        } else {
            Self::none()
        }
    }
}

// We implement the other standard traits by pretending that we are `Option<T>`.

impl<T: std::fmt::Debug> std::fmt::Debug for WriteOnce<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.get())
    }
}

impl<T: PartialEq> PartialEq for WriteOnce<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl<T: Eq> Eq for WriteOnce<T> {}

impl<T: PartialOrd> PartialOrd for WriteOnce<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

impl<T: Ord> Ord for WriteOnce<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T: std::hash::Hash> std::hash::Hash for WriteOnce<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state)
    }
}

unsafe impl<T: Send> Send for WriteOnce<T> {}
unsafe impl<T: Sync> Sync for WriteOnce<T> {}

/// The possible states of a `WriteOnce`.
///
/// We distinguish between `Uninit` and `Writing` so that we reach the `Err` branch of `set` if
/// `set` has been called by any thread before.
///
/// We distinguish between `Writing` and `Init` so that loading `Init` has a happens-before
/// relationship with the write in `set`.
#[repr(u8)]
enum WriteOnceState {
    Uninit = 0,
    Writing = 1,
    Init = 2,
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Test creating a new WriteOnce
        let cell = WriteOnce::<i32>::none();
        assert!(!cell.is_set());
        assert_eq!(cell.get(), None);

        // Test setting a value
        cell.set(42);
        assert!(cell.is_set());
        assert_eq!(cell.get(), Some(&42));

        // Test that try_set returns an error when the cell already has a value
        let result = cell.try_set(100);
        assert!(result.is_err());
        if let Err(value) = result {
            assert_eq!(value, 100);
        }

        // Test that the value remains unchanged
        assert_eq!(cell.get(), Some(&42));
    }

    #[test]
    fn test_get_mut() {
        // Test get_mut with no value
        let mut cell = WriteOnce::<String>::none();
        assert_eq!(cell.get_mut(), None);

        // Test get_mut with a value
        cell.set("Hello".to_string());
        {
            let value = cell.get_mut().unwrap();
            value.push_str(", world!");
        }
        assert_eq!(cell.get(), Some(&"Hello, world!".to_string()));
    }

    #[test]
    #[should_panic(expected = "WriteOnce already set")]
    fn test_set_panics_when_already_set() {
        let cell = WriteOnce::<i32>::none();
        cell.set(42);
        cell.set(100); // This should panic
    }

    #[test]
    fn test_drop_behavior() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

        struct DropCounter;
        impl Drop for DropCounter {
            fn drop(&mut self) {
                DROP_COUNT.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Test that the value is dropped when the WriteOnce is dropped
        {
            let cell = WriteOnce::<DropCounter>::none();
            cell.set(DropCounter);
            assert_eq!(DROP_COUNT.load(Ordering::Relaxed), 0);
        }
        assert_eq!(DROP_COUNT.load(Ordering::Relaxed), 1);

        // Test that the value is not dropped if it was never set
        {
            let _cell = WriteOnce::<DropCounter>::none();
        }
        assert_eq!(DROP_COUNT.load(Ordering::Relaxed), 1); // Still 1
    }

    #[test]
    fn test_thread_safety() {
        let cell = Arc::new(WriteOnce::<i32>::none());
        let cell_clone = Arc::clone(&cell);

        // Spawn a thread that sets the value
        let thread = thread::spawn(move || {
            cell_clone.set(42);
        });

        // Wait for the thread to complete
        thread.join().unwrap();

        // Check that the value was set
        assert!(cell.is_set());
        assert_eq!(cell.get(), Some(&42));
    }

    #[test]
    fn test_concurrent_set() {
        let cell = Arc::new(WriteOnce::<i32>::none());
        let mut handles = Vec::new();

        // Spawn 10 threads that all try to set the value
        for i in 0..10 {
            let cell_clone = Arc::clone(&cell);
            let handle = thread::spawn(move || {
                let _ = cell_clone.try_set(i);
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Check that the value was set exactly once
        assert!(cell.is_set());
        let value = cell.get().unwrap();
        assert!(*value >= 0 && *value < 10);
    }

    #[cfg(loom)]
    mod loom_tests {
        use super::*;
        use crate::std_or_loom::{sync::Arc, thread};

        #[test]
        fn loom_concurrent_set_and_get() {
            loom::model(|| {
                let cell = Arc::new(WriteOnce::<i32>::none());

                // Thread 1: Try to set the value
                let cell1 = Arc::clone(&cell);
                let t1 = thread::spawn(move || {
                    let _ = cell1.try_set(42);
                });

                // Thread 2: Try to set the value
                let cell2 = Arc::clone(&cell);
                let t2 = thread::spawn(move || {
                    let _ = cell2.try_set(100);
                });

                // Thread 3: Get the value
                let cell3 = Arc::clone(&cell);
                let t3 = thread::spawn(move || {
                    let _ = cell3.get();
                });

                t1.join().unwrap();
                t2.join().unwrap();
                t3.join().unwrap();

                // The value should be either 42 or 100, depending on which thread won the race
                assert!(cell.is_set());
                let value = cell.get().unwrap();
                assert!(*value == 42 || *value == 100);
            });
        }

        #[test]
        fn loom_is_set_during_write() {
            loom::model(|| {
                let cell = Arc::new(WriteOnce::<i32>::none());

                // Thread 1: Set the value
                let cell1 = Arc::clone(&cell);
                let t1 = thread::spawn(move || {
                    // This will set the state to Writing and then to Init
                    cell1.set(42);
                });

                // Thread 2: Check if the value is set
                let cell2 = Arc::clone(&cell);
                let t2 = thread::spawn(move || {
                    // This may observe any of the three states
                    let is_set = cell2.is_set();
                    if is_set {
                        // If is_set returns true, get() should return Some
                        assert!(cell2.get().is_some());
                    }
                });

                t1.join().unwrap();
                t2.join().unwrap();

                // After both threads complete, the value should be set
                assert!(cell.is_set());
                assert_eq!(cell.get(), Some(&42));
            });
        }
    }
}
