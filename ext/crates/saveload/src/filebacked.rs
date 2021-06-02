use std::{
    io::{self, Read, Seek, SeekFrom, Write},
    ops::Deref,
    sync::{Arc, Weak},
};

use parking_lot::RwLock;
use tempfile::SpooledTempFile;

use crate::{Load, Save};

/// A smart pointer for very large structs. The idea is that a FileBacked<T> does not necessarily own
/// a `T`, but has enough data to know where to load it from when needed. FileBacked<T> will not
/// hold `T` in memory, but instead wait for the data to be accessed, at which point it will load the data
/// before handing over a pointer to it. As soon as the pointer is dropped, the memory can be deallocated.
pub struct FileBacked<T: Load> {
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
        // If `T` occupies less than 1MB, we can keep it in memory
        let tmp_file = RwLock::new(SpooledTempFile::new(1024 * 1024));
        // TODO: If `data` is large, the following line uses an unbuffered writer to write 1MB+
        // to disk. This is extremely slow, but shouldn't take more than several seconds, and is
        // only done once on initialization. This could be solved if one could check the memory
        // footprint of `data` at runtime, but afaik there is no Rust function that does that.
        T::save(&data, &mut *tmp_file.write()).unwrap();
        FileBacked {
            ptr: RwLock::new(Weak::new()),
            tmp_file,
            aux_data: aux_data.clone(),
        }
    }

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

impl<T> Save for FileBacked<T>
where
    T: Save + Load,
    T::AuxData: Clone,
{
    fn save(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        let mut tmp_file = self.tmp_file.write();
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
        if self.write_mode && Arc::strong_count(&self.data) == 1 {
            // The reference count could go up before the next line is executed,
            // but that would only lead to over-saving. This is potientially a small
            // performance hit, but it is safe.
            self.backing.save_changes();
        }
    }
}
