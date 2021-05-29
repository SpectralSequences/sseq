use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    sync::{Arc, Weak},
};

use parking_lot::RwLock;
use tempfile::tempfile_in;

use crate::{Load, Save};

/// A smart pointer for very large structs. The idea is that a FileBacked<T> does not necessarily own
/// a `T`, but has enough data to know where to load it from when needed. FileBacked<T> will not
/// hold `T` in memory, but instead wait for the data to be accessed, at which point it will load the data
/// before handing over a pointer to it. As soon as the pointer is dropped, the memory can be deallocated.
pub struct FileBacked<T>
where
    T: Load,
    T::AuxData: Clone,
{
    ptr: RwLock<Weak<T>>,
    tmp_file: RwLock<File>,
    aux_data: T::AuxData,
}

impl<T> FileBacked<T>
where
    T: Load,
    T::AuxData: Clone,
{
    pub fn upgrade(&self) -> Arc<T> {
        let read_ptr = self.ptr.read();
        let maybe_data = read_ptr.upgrade();
        if let Some(arc_data) = maybe_data {
            arc_data
        } else {
            drop(read_ptr);
            let mut write_ptr = self.ptr.write();
            let mut tmp_file = self.tmp_file.write();
            let data = T::load(&mut *tmp_file, &self.aux_data).unwrap();
            tmp_file.seek(SeekFrom::Start(0)).unwrap();
            let arc_data = Arc::new(data);
            *write_ptr = Arc::downgrade(&arc_data);
            arc_data
        }
    }
}

impl<T> FileBacked<T>
where
    T: Save + Load,
    T::AuxData: Clone,
{
    pub fn new(data: T, aux_data: &T::AuxData) -> FileBacked<T> {
        // If `T` occupies less than 1MB, we can keep it in memory
        let tmp_file = RwLock::new(tempfile_in("./").unwrap());
        eprintln!("Creating a FileBacked with tmp_file {:?}", tmp_file);
        let mut writer = std::io::BufWriter::new(tmp_file.write().try_clone().unwrap());
        Save::save(&data, &mut writer).unwrap();
        eprintln!("Created {:?}", tmp_file);
        FileBacked {
            ptr: RwLock::new(Weak::new()),
            tmp_file,
            aux_data: aux_data.clone(),
        }
    }
}

// impl<T> Deref for FileBacked<T>
// where
//     T: Load,
//     T::AuxData: Clone,
// {
//     type Target = T;

//     fn deref(&self) -> &<Self as Deref>::Target {
//         let file = &mut File::open(&self.file_path).unwrap();
//         file.seek(SeekFrom::Start(self.offset as u64)).unwrap();
//         let data = T::load(file, &self.aux_data).unwrap();
//         let pointer = Arc::new(data);
//         self.ptr = Arc::downgrade(&pointer);
//         pointer
//     }
// }

impl<T> Save for FileBacked<T>
where
    T: Save + Load,
    T::AuxData: Clone,
{
    fn save(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        Save::save(&self.upgrade(), buffer)
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
