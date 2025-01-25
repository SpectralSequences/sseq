use std::{
    collections::HashSet,
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex},
};

use algebra::Algebra;
use anyhow::Context;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use sseq::coordinates::Bidegree;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SaveDirectory {
    None,
    Combined(PathBuf),
    Split { read: PathBuf, write: PathBuf },
}

impl SaveDirectory {
    pub fn read(&self) -> Option<&PathBuf> {
        match self {
            Self::None => None,
            Self::Combined(x) => Some(x),
            Self::Split { read, .. } => Some(read),
        }
    }

    pub fn write(&self) -> Option<&PathBuf> {
        match self {
            Self::None => None,
            Self::Combined(x) => Some(x),
            Self::Split { write, .. } => Some(write),
        }
    }

    pub fn push<P: AsRef<Path>>(&mut self, p: P) {
        match self {
            Self::None => {}
            Self::Combined(d) => {
                d.push(p);
            }
            Self::Split { read, write } => {
                read.push(&p);
                write.push(p);
            }
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }
}

impl From<Option<PathBuf>> for SaveDirectory {
    fn from(x: Option<PathBuf>) -> Self {
        match x {
            None => Self::None,
            Some(x) => Self::Combined(x),
        }
    }
}

/// A `DashSet<PathBuf>>` of paths that are currently being used.
///
/// Suppose a path `p` is contained in this `DashSet`.
/// - If `p` points to a file, then a (unique) thread is currently writing to that file. The ctrlc
///   handler ensures that any such file will be deleted if the program is terminated.
/// - If `p` points to a directory, then a thread is in the process of creating a file in `p`.
fn paths_in_use() -> &'static Mutex<HashSet<PathBuf>> {
    static OPEN_FILES: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(|| {
        #[cfg(unix)]
        ctrlc::set_handler(move || {
            tracing::warn!("Ctrl-C detected. Deleting open files and exiting.");
            let paths = paths_in_use().lock().unwrap();
            for file in paths.iter().filter(|p| p.is_file()) {
                std::fs::remove_file(file)
                    .unwrap_or_else(|_| panic!("Error when deleting {file:?}"));
                tracing::warn!(?file, "deleted");
            }
            std::process::exit(130);
        })
        .expect("Error setting Ctrl-C handler");
        Default::default()
    });
    &OPEN_FILES
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum SaveKind {
    /// The kernel of a resolution differential
    Kernel,

    /// The differential and augmentation map in a resolution
    Differential,

    /// The quasi-inverse of the resolution differential
    ResQi,

    /// The quasi-inverse of the augmentation map
    AugmentationQi,

    /// Secondary composite
    SecondaryComposite,

    /// Intermediate data used by secondary code
    SecondaryIntermediate,

    /// A secondary homotopy
    SecondaryHomotopy,

    /// A chain map
    ChainMap,

    /// A chain homotopy
    ChainHomotopy,

    /// The differential with Nassau's algorithm. This does not store the chain map data because we
    /// always only resolve the sphere
    NassauDifferential,

    /// The quasi-inverse data in Nassau's algorithm
    NassauQi,
}

impl SaveKind {
    pub fn magic(self) -> u32 {
        match self {
            Self::Kernel => 0x0000D1FF,
            Self::Differential => 0xD1FF0000,
            Self::ResQi => 0x0100D1FF,
            Self::AugmentationQi => 0x0100A000,
            Self::SecondaryComposite => 0x00020000,
            Self::SecondaryIntermediate => 0x00020001,
            Self::SecondaryHomotopy => 0x00020002,
            Self::ChainMap => 0x10100000,
            Self::ChainHomotopy => 0x11110000,
            Self::NassauDifferential => 0xD1FF0001,
            Self::NassauQi => 0x0100D1FE,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::Differential => "differential",
            Self::ResQi => "res_qi",
            Self::AugmentationQi => "augmentation_qi",
            Self::SecondaryComposite => "secondary_composite",
            Self::SecondaryIntermediate => "secondary_intermediate",
            Self::SecondaryHomotopy => "secondary_homotopy",
            Self::ChainMap => "chain_map",
            Self::ChainHomotopy => "chain_homotopy",
            Self::NassauDifferential => "nassau_differential",
            Self::NassauQi => "nassau_qi",
        }
    }

    pub fn resolution_data() -> impl Iterator<Item = Self> {
        use SaveKind::*;
        static KINDS: [SaveKind; 4] = [Kernel, Differential, ResQi, AugmentationQi];
        KINDS.iter().copied()
    }

    pub fn nassau_data() -> impl Iterator<Item = Self> {
        use SaveKind::*;
        static KINDS: [SaveKind; 2] = [NassauDifferential, NassauQi];
        KINDS.iter().copied()
    }

    pub fn secondary_data() -> impl Iterator<Item = Self> {
        use SaveKind::*;
        static KINDS: [SaveKind; 3] =
            [SecondaryComposite, SecondaryIntermediate, SecondaryHomotopy];
        KINDS.iter().copied()
    }

    pub fn create_dir(self, p: &std::path::Path) -> anyhow::Result<()> {
        let mut p = p.to_owned();

        p.push(format!("{}s", self.name()));
        if !p.exists() {
            std::fs::create_dir_all(&p)
                .with_context(|| format!("Failed to create directory {p:?}"))?;
        } else if !p.is_dir() {
            return Err(anyhow::anyhow!("{p:?} is not a directory"));
        }
        Ok(())
    }
}

/// In addition to checking the checksum, we also keep track of which files are open, and we delete
/// the open files if the program is terminated halfway.
pub struct ChecksumWriter<T: io::Write> {
    writer: T,
    path: PathBuf,
    adler: adler::Adler32,
}

impl<T: io::Write> ChecksumWriter<T> {
    pub fn new(path: PathBuf, writer: T) -> Self {
        Self {
            path,
            writer,
            adler: adler::Adler32::new(),
        }
    }
}

/// We only implement the functions required and the ones we actually use.
impl<T: io::Write> io::Write for ChecksumWriter<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let bytes_written = self.writer.write(buf)?;
        self.adler.write_slice(&buf[0..bytes_written]);
        Ok(bytes_written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.writer.write_all(buf)?;
        self.adler.write_slice(buf);
        Ok(())
    }
}

impl<T: io::Write> std::ops::Drop for ChecksumWriter<T> {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            // We may not have finished writing, so the data is wrong. It should not be given a
            // valid checksum
            self.writer
                .write_u32::<LittleEndian>(self.adler.checksum())
                .unwrap();
            self.writer.flush().unwrap();
            assert!(
                paths_in_use().lock().unwrap().remove(&self.path),
                "File {:?} already dropped",
                self.path
            );
        }
        tracing::info!(file = ?self.path, "closing");
    }
}

pub struct ChecksumReader<T: io::Read> {
    reader: T,
    adler: adler::Adler32,
}

impl<T: io::Read> ChecksumReader<T> {
    pub fn new(reader: T) -> Self {
        Self {
            reader,
            adler: adler::Adler32::new(),
        }
    }
}

/// We only implement the functions required and the ones we actually use.
impl<T: io::Read> io::Read for ChecksumReader<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes_read = self.reader.read(buf)?;
        self.adler.write_slice(&buf[0..bytes_read]);
        Ok(bytes_read)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.reader.read_exact(buf)?;
        self.adler.write_slice(buf);
        Ok(())
    }
}

impl<T: io::Read> std::ops::Drop for ChecksumReader<T> {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            // If we are panicking, we may not have read everything, and panic in panic
            // is bad.
            assert_eq!(
                self.adler.checksum(),
                self.reader.read_u32::<LittleEndian>().unwrap(),
                "Invalid file checksum"
            );
            let mut buf = [0];
            // Check EOF
            assert_eq!(self.reader.read(&mut buf).unwrap(), 0, "EOF not reached");
        }
    }
}

/// Open the file pointed to by `path` as a `Box<dyn Read>`. If the file does not exist, look for
/// compressed versions.
fn open_file(path: PathBuf) -> Option<Box<dyn io::Read>> {
    use io::BufRead;

    // We should try in decreasing order of access speed.
    match File::open(&path) {
        Ok(f) => {
            let mut reader = io::BufReader::new(f);
            if reader
                .fill_buf()
                .unwrap_or_else(|e| panic!("Error when reading from {path:?}: {e}"))
                .is_empty()
            {
                // The file is empty. Delete the file and proceed as if it didn't exist
                std::fs::remove_file(&path)
                    .unwrap_or_else(|e| panic!("Error when deleting empty file {path:?}: {e}"));
                return None;
            }
            return Some(Box::new(ChecksumReader::new(reader)));
        }
        Err(e) => {
            if e.kind() != io::ErrorKind::NotFound {
                panic!("Error when opening {path:?}: {e}");
            }
        }
    }

    #[cfg(feature = "zstd")]
    {
        let mut path = path;
        path.set_extension("zst");
        match File::open(&path) {
            Ok(f) => {
                return Some(Box::new(ChecksumReader::new(
                    zstd::stream::Decoder::new(f).unwrap(),
                )))
            }
            Err(e) => {
                if e.kind() != io::ErrorKind::NotFound {
                    panic!("Error when opening {path:?}");
                }
            }
        }
    }

    None
}

pub struct SaveFile<A: Algebra> {
    pub kind: SaveKind,
    pub algebra: Arc<A>,
    pub b: Bidegree,
    pub idx: Option<usize>,
}

impl<A: Algebra> SaveFile<A> {
    fn write_header(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        buffer.write_u32::<LittleEndian>(self.kind.magic())?;
        buffer.write_u32::<LittleEndian>(self.algebra.magic())?;
        buffer.write_u32::<LittleEndian>(self.b.s())?;
        buffer.write_i32::<LittleEndian>(if let Some(i) = self.idx {
            self.b.t() + ((i as i32) << 16)
        } else {
            self.b.t()
        })
    }

    fn validate_header(&self, buffer: &mut impl io::Read) -> io::Result<()> {
        macro_rules! check_header {
            ($name:literal, $value:expr, $format:literal) => {
                let data = buffer.read_u32::<LittleEndian>()?;
                if data != $value {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Invalid header: {} was {} but expected {}",
                            $name,
                            format_args!($format, data),
                            format_args!($format, $value)
                        ),
                    ));
                }
            };
        }

        check_header!("magic", self.kind.magic(), "{:#010x}");
        check_header!("algebra", self.algebra.magic(), "{:#06x}");
        check_header!("s", self.b.s(), "{}");
        check_header!(
            "t",
            if let Some(i) = self.idx {
                self.b.t() as u32 + ((i as u32) << 16)
            } else {
                self.b.t() as u32
            },
            "{}"
        );

        Ok(())
    }

    fn get_save_directory(&self, mut dir: PathBuf) -> PathBuf {
        dir.push(format!(
            "{name}s/{s}/",
            name = self.kind.name(),
            s = self.b.s()
        ));
        dir
    }

    /// This panics if there is no save dir
    fn add_save_path(&self, mut dir: PathBuf) -> PathBuf {
        let n = if self.b.n() < 0 {
            format!("m{}", -self.b.n())
        } else {
            self.b.n().to_string()
        };
        if let Some(idx) = self.idx {
            dir.push(format!(
                "{n}_{s}_{idx}_{name}",
                name = self.kind.name(),
                s = self.b.s(),
            ));
        } else {
            dir.push(format!(
                "{n}_{s}_{name}",
                name = self.kind.name(),
                s = self.b.s(),
            ));
        }
        dir
    }

    fn get_full_save_path(&self, mut dir: PathBuf) -> PathBuf {
        dir = self.get_save_directory(dir);
        dir = self.add_save_path(dir);
        dir
    }

    pub fn open_file(&self, dir: PathBuf) -> Option<Box<dyn io::Read>> {
        let file_path = self.get_full_save_path(dir);
        let path_string = file_path.to_string_lossy().into_owned();
        if let Some(mut f) = open_file(file_path) {
            self.validate_header(&mut f).unwrap();
            tracing::info!(file = path_string, "success open for reading");
            Some(f)
        } else {
            tracing::info!(file = path_string, "failed open for reading");
            None
        }
    }

    pub fn exists(&self, dir: PathBuf) -> bool {
        #[allow(unused_mut)]
        let mut path = self.get_full_save_path(dir);
        if path.exists() {
            return true;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut path = path;
            path.set_extension("zst");
            if path.exists() {
                return true;
            }
        }
        false
    }

    pub fn delete_file(&self, dir: PathBuf) -> io::Result<()> {
        let dir = self.get_save_directory(dir);
        let p = self.add_save_path(dir.clone());
        if let Err(e) = std::fs::remove_file(p) {
            if e.kind() != io::ErrorKind::NotFound {
                return Err(e);
            }
        };
        // We only delete the directory if no thread is attempting to write to it.
        if !paths_in_use().lock().unwrap().contains(&dir) {
            let harmless_errors = [io::ErrorKind::DirectoryNotEmpty, io::ErrorKind::NotFound];
            // `remove_dir` only deletes empty directories, so this is safe.
            match std::fs::remove_dir(dir) {
                Err(e) if harmless_errors.contains(&e.kind()) => Ok(()),
                x => x,
            }?;
        }
        Ok(())
    }

    /// # Arguments
    ///  - `overwrite`: Whether to overwrite a file if it already exists.
    pub fn create_file(&self, dir: PathBuf, overwrite: bool) -> impl io::Write {
        let dir = self.get_save_directory(dir);
        let p = self.add_save_path(dir.clone());
        tracing::info!(file = ?p, "open for writing");

        // We need to do this before creating any file. The ctrlc handler does not block other threads
        // from running, but it does lock [`open_files()`]. So this ensures we do not open new files
        // while handling ctrlc.
        assert!(
            paths_in_use().lock().unwrap().insert(p.clone()),
            "File {p:?} is already opened"
        );

        // We also add the directory to the set of paths in use. This is to ensure that we only
        // delete directories when no thread is attempting to write to a file in that directory. We
        // don't hold the mutex for the entirety of this function to guard it from getting poisoned.
        paths_in_use().lock().unwrap().insert(dir.clone());

        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create directories containing {p:?}"))
            .unwrap();

        let f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(!overwrite)
            .create(true)
            .truncate(true)
            .open(&p)
            .with_context(|| format!("Failed to create save file {p:?}"))
            .unwrap();

        // We have successfully created the file, so `dir` is nonempty, and calling
        // `std::fs::delete_dir(dir)` will have no effect. Therefore, we can remove `dir` from the
        // set of paths in use.
        paths_in_use().lock().unwrap().remove(&dir);

        let mut f = ChecksumWriter::new(p, io::BufWriter::new(f));
        self.write_header(&mut f).unwrap();
        f
    }
}
