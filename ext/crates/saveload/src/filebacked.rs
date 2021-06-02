use std::{
    fmt::{Debug, Result},
    io::{self, Read, Seek, SeekFrom, Write},
    ops::Deref,
    sync::{Arc, Weak},
};

use parking_lot::RwLock;
use tempfile::SpooledTempFile;

use crate::{Load, Save};

/// A wrapper for very large structs. The idea is that a `FileBacked<T>` represents a `T` without
/// keeping it in memory. At creation time, `FileBacked<T>` will take ownership of a `T` and save it
/// to a `SpooledTempFile`, and then drop `T`. By default the `SpooledTempFile` will keep structs
/// using less than 1MB in memory, as an attempt to reduce disk io. When the resource is requested,
/// the `upgrade` method will return a `FileBackedGuard<T>`, which can be used as a pointer to a
/// `T`. If the resource was already loaded (i.e., a `FileBackedGuard<T>` already exists), no disk
/// io is performed, so that all pointers always point to the same `T`.
///
/// Since `T` needs to be loaded/unloaded from disk, it needs to implement `Save + Load`. We also
/// want to own a copy of `T::AuxData`, so that `T` can be loaded as long as the `FileBacked<T>`
/// exists. Therefore `T::AuxData` needs to implement `Clone`. Another option would be to keep a
/// reference to a `T::AuxData` whose lifetime is guaranteed to be as long as `FileBacked<T>`, but
/// we found this solution tricky to implement, and requiring `Clone` is reasonable in practice.
///
/// # Example
/// ```
/// # use saveload::filebacked::{FileBacked, FileBackedGuard};
/// # use std::io::Error;
///
/// let v : Vec<u32> = vec![6, 3, 4, 2];
///
/// let filebacked_v : FileBacked<Vec<u32>> = FileBacked::new_with_capacity(v, &(), 0);
/// // `v` is removed from memory
///
/// let w : FileBackedGuard<Vec<u32>> = filebacked_v.upgrade(false);
///
/// assert_eq!(vec![6, 3, 4, 2], *w);
/// # Ok::<(), Error>(())
/// ```
pub struct FileBacked<T>
where
    T: Save + Load,
    T::AuxData: Clone,
{
    ptr: RwLock<Weak<T>>,
    tmp_file: RwLock<SpooledTempFile>,
    aux_data: T::AuxData,
}

impl<T> FileBacked<T>
where
    T: Save + Load,
    T::AuxData: Clone,
{
    pub fn new(data: T, aux_data: &T::AuxData) -> FileBacked<T> {
        // By default, keep everything less than 1MB large in memory.
        FileBacked::new_with_capacity(data, aux_data, 1024 * 1024)
    }

    pub fn new_with_capacity(data: T, aux_data: &T::AuxData, capacity: usize) -> FileBacked<T> {
        let tmp_file = RwLock::new(SpooledTempFile::new(capacity));
        T::save(&data, &mut *tmp_file.write()).unwrap();
        FileBacked {
            ptr: RwLock::new(Weak::new()),
            tmp_file,
            aux_data: aux_data.clone(),
        }
    }

    /// Save the current state of `T` to disk. Note that if `T` is not currently loaded this method
    /// will simply load `T` and immediately write it back. There is usually no need to call this
    /// function, since `FileBackedGuard` handles this when dropped.
    pub fn save_changes(&self) {
        let data = self.upgrade(false);
        let mut tmp_file = self.tmp_file.write();
        tmp_file.seek(SeekFrom::Start(0)).unwrap();
        if tmp_file.is_rolled() {
            T::save(
                &data,
                &mut std::io::BufWriter::new(tmp_file.try_clone().unwrap()),
            )
            .unwrap();
            eprintln!("Saved {:?}", tmp_file);
        } else {
            T::save(&data, &mut *tmp_file).unwrap();
        }
    }

    /// Request the resource `T`. This returns a `FileBackedGuard<T>` which can simply be used as a
    /// pointer to `T`. The returned guard will save changes back to disk when dropped if and only
    /// if `write_mode` is `true`.
    pub fn upgrade(&self, write_mode: bool) -> FileBackedGuard<T> {
        let read_ptr = self.ptr.read();
        let maybe_data = read_ptr.upgrade();
        if let Some(arc_data) = maybe_data {
            FileBackedGuard {
                backing: &self,
                data: arc_data,
                write_mode,
            }
        } else {
            drop(read_ptr);
            let mut write_ptr = self.ptr.write();
            let mut tmp_file = self.tmp_file.write();
            tmp_file.seek(SeekFrom::Start(0)).unwrap();
            let data = T::load(&mut *tmp_file, &self.aux_data).unwrap();
            let arc_data = Arc::new(data);
            *write_ptr = Arc::downgrade(&arc_data);
            FileBackedGuard {
                backing: &self,
                data: arc_data,
                write_mode,
            }
        }
    }
}

impl<T> Clone for FileBacked<T>
where
    T: Save + Load,
    T::AuxData: Clone,
{
    fn clone(&self) -> Self {
        let mut tmp_file = self.tmp_file.write();
        tmp_file.seek(SeekFrom::Start(0)).unwrap();
        Self::new(
            T::load(&mut *tmp_file, &self.aux_data).unwrap(),
            &self.aux_data,
        )
    }
}

impl<T> std::fmt::Debug for FileBacked<T>
where
    T: Save + Load + Debug,
    T::AuxData: Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result {
        let data = self.upgrade(false);
        data.fmt(f)
    }
}

impl<T> PartialEq for FileBacked<T>
where
    T: Save + Load + PartialEq,
    T::AuxData: Clone,
{
    fn eq(&self, other: &Self) -> bool {
        let self_data = self.upgrade(false);
        let other_data = other.upgrade(false);
        self_data == other_data
    }
}

impl<T> Eq for FileBacked<T>
where
    T: Save + Load + Eq,
    T::AuxData: Clone,
{
}

impl<T> Save for FileBacked<T>
where
    T: Save + Load,
    T::AuxData: Clone,
{
    fn save(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        let mut tmp_file = self.tmp_file.write();
        tmp_file.seek(SeekFrom::Start(0))?;
        io::copy(&mut *tmp_file, buffer)?;
        Ok(())
    }
}

impl<T> Load for FileBacked<T>
where
    T: Save + Load,
    T::AuxData: Clone,
{
    type AuxData = T::AuxData;

    fn load(buffer: &mut impl Read, data: &Self::AuxData) -> std::io::Result<Self> {
        match <T as Load>::load(buffer, data) {
            Ok(loaded) => Ok(FileBacked::new(loaded, data)),
            Err(e) => Err(e),
        }
    }
}

/// An RAII-style guard wrapping a `T`. As long as the guard exists, the wrapped value is kept live
/// in memory. It implements `Drop` so that any changes are saved to disk before deallocation, if
/// the guard was created with `write_mode` set to `true`. Note that `FileBackedGuard<T>` implements
/// `Deref` but not `DerefMut`, so `T` needs interior mutability for it to be modified.
///
/// # Example
/// ```
/// # use saveload::filebacked::{FileBacked, FileBackedGuard};
/// # use std::io::Error;
/// use std::sync::Mutex; // We wrap around a `Mutex` for interior mutability
///
/// let v : Vec<u32> = vec![6, 3, 4, 2];
/// let filebacked_v = FileBacked::new(Mutex::new(v), &());
///
/// let guard_read = filebacked_v.upgrade(false);
/// guard_read.lock().unwrap()[0] = 5;
/// drop(guard_read);
///
/// assert_eq!(vec![6, 3, 4, 2], *filebacked_v.upgrade(false).lock().unwrap()); // `v` is unchanged
///
/// let guard_write = filebacked_v.upgrade(true);
/// guard_write.lock().unwrap()[0] = 5;
/// drop(guard_write);
///
/// assert_eq!(vec![5, 3, 4, 2], *filebacked_v.upgrade(false).lock().unwrap()); // `v` is modified
/// # Ok::<(), Error>(())
/// ```
pub struct FileBackedGuard<'a, T>
where
    T: Save + Load,
    T::AuxData: Clone,
{
    backing: &'a FileBacked<T>,
    data: Arc<T>,
    write_mode: bool,
}

impl<T> Deref for FileBackedGuard<'_, T>
where
    T: Save + Load,
    T::AuxData: Clone,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data.deref()
    }
}

impl<T> Drop for FileBackedGuard<'_, T>
where
    T: Save + Load,
    T::AuxData: Clone,
{
    fn drop(&mut self) {
        if self.write_mode {
            self.backing.save_changes();
        }
    }
}

impl<T> std::fmt::Debug for FileBackedGuard<'_, T>
where
    T: Save + Load + Debug,
    T::AuxData: Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result {
        self.data.fmt(f)
    }
}

impl<T> PartialEq for FileBackedGuard<'_, T>
where
    T: Save + Load + PartialEq,
    T::AuxData: Clone,
{
    fn eq(&self, other: &Self) -> bool {
        *self.data == *other.data
    }
}

impl<T> Eq for FileBackedGuard<'_, T>
where
    T: Save + Load + Eq,
    T::AuxData: Clone,
{
}

impl<T> Save for FileBackedGuard<'_, T>
where
    T: Save + Load,
    T::AuxData: Clone,
{
    fn save(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        self.backing.save(buffer)
    }
}
