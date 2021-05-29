//! This crate provides a simple interface for saving and loading. The main non-trivial feature
//! here is that auxiliary data can be supplied to the `load` function. This is essential when the
//! data structures we want to save or load contains references to other structures. For example, a
//! `FreeModuleHomomorphism` contains `source : Arc<FreeModule>`, which should be supplied to the
//! `load` function along with the binary data. In practice, this is also used for fields that are
//! often duplicated along a lot of objects, e.g. the prime `p`.
//!
//! The interface is intended for a binary data structure. The [`Save::save`] function is called
//! with a `buffer` implementing [`Write`](std::io::Write). In practice, this is either
//! [`BufWriter`](std::io::BufWriter) for actually writing to files, or a [`Cursor`](std::io::Cursor)
//! for testing. The `save` function is allowed to write anything to the `buffer`, and in
//! particular, data of any length. The [`Load::load`] function takes in a `buffer` implementing
//! [`Read`](std::io::Read) (again this is usually either [`BufReader`](std::io::BufReader) or
//! [`Cursor`](std::io::Cursor)). It MUST read the exact same number of bytes when called (with the
//! correct auxiliary data).
//!
//! [`Save`] and [`Load`] are implemented for a number of primitive types such as [`u32`] and
//! [`i32`], as well as `Vec<T>` where `T` implements `Save` or `Load`. These are all defined in
//! `default_impl.rs`. In most cases, saving and loading can be performed by calling the
//! `save`/`load` functions of these types, and so one does not have to directly work with `Read`
//! or `Write`. However, if one were to read bytes directly, they are reminded that they should use
//! [`Read::read_exact`](std::io::Read::read_exact) instead of
//! [`Read::read`](std::io::Read::read_exact), as `read` does not make any guarantees about the
//! number of bytes actually read.

use std::io;
use std::io::{Read, Write};

mod default_impl;
pub mod filebacked;

pub trait Save {
    /// # Example
    /// ```
    /// # use saveload::{Save, Load};
    /// # use std::io::{Read, Cursor, SeekFrom, Seek, Error};
    ///
    /// let v : Vec<u32> = vec![6, 3, 4, 2];
    ///
    /// let mut cursor : Cursor<Vec<u8>> = Cursor::new(Vec::new());
    /// v.save(&mut cursor)?;
    ///
    /// cursor.seek(SeekFrom::Start(0))?;
    ///
    /// let mut w : Vec<u32> = Load::load(&mut cursor, &())?;
    ///
    /// assert_eq!(v, w);
    /// assert_eq!(0, cursor.bytes().count());
    /// # Ok::<(), Error>(())
    /// ```
    fn save(&self, buffer: &mut impl Write) -> io::Result<()>;
}

pub trait Load: Sized {
    /// The type of the auxiliary data needed for loading.
    ///
    /// # Example
    /// ```ignore
    /// impl Load for FpVector {
    ///     type AuxData = u32;
    ///
    ///     fn load(buffer : &mut impl Read, p : &u32) -> io::Result<Self> {
    ///         let dimension = usize::load(buffer, &())?;
    ///
    ///         if dimension == 0 {
    ///             return Ok(FpVector::new(*p, 0));
    ///         }
    ///         ...
    ///     }
    /// }
    /// ```

    type AuxData;

    /// The auxiliary data is a borrow. This is useful for `Vec<T>` where we pass the *same*
    /// auxiliary data to all the entries in the `Vec`.
    ///
    /// If `AuxData = ()`, then the load function will look like `Load::load(&mut f, &())`, which
    /// is a pretty common occurrence.
    fn load(buffer: &mut impl Read, data: &Self::AuxData) -> io::Result<Self>;
}
