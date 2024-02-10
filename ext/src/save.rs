use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Error, ErrorKind, Read, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use algebra::Algebra;
use anyhow::Context;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use sseq::coordinates::Bidegree;

/// A DashSet<PathBuf>> of files that are currently opened and being written to. When calling this
/// function for the first time, we set the ctrlc handler to delete currently opened files, then
/// exit.
fn open_files() -> &'static Mutex<HashSet<PathBuf>> {
    use std::{mem::MaybeUninit, sync::Once};

    static mut OPEN_FILES: MaybeUninit<Mutex<HashSet<PathBuf>>> = MaybeUninit::uninit();
    static ONCE: Once = Once::new();
    unsafe {
        ONCE.call_once(|| {
            OPEN_FILES.write(Default::default());
            #[cfg(unix)]
            ctrlc::set_handler(move || {
                let files = open_files().lock().unwrap();
                for file in &*files {
                    std::fs::remove_file(file)
                        .unwrap_or_else(|_| panic!("Error when deleting {file:?}"));
                }
                std::process::exit(130);
            })
            .expect("Error setting Ctrl-C handler");
        });
        OPEN_FILES.assume_init_ref()
    }
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

    pub fn resolution_data() -> impl Iterator<Item = SaveKind> {
        use SaveKind::*;
        static KINDS: [SaveKind; 4] = [Kernel, Differential, ResQi, AugmentationQi];
        KINDS.iter().copied()
    }

    pub fn nassau_data() -> impl Iterator<Item = SaveKind> {
        use SaveKind::*;
        static KINDS: [SaveKind; 2] = [NassauDifferential, NassauQi];
        KINDS.iter().copied()
    }

    pub fn secondary_data() -> impl Iterator<Item = SaveKind> {
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
pub struct ChecksumWriter<T: Write> {
    writer: T,
    path: PathBuf,
    adler: adler::Adler32,
}

impl<T: Write> ChecksumWriter<T> {
    pub fn new(path: PathBuf, writer: T) -> Self {
        Self {
            path,
            writer,
            adler: adler::Adler32::new(),
        }
    }
}

/// We only implement the functions required and the ones we actually use.
impl<T: Write> Write for ChecksumWriter<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let bytes_written = self.writer.write(buf)?;
        self.adler.write_slice(&buf[0..bytes_written]);
        Ok(bytes_written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(buf)?;
        self.adler.write_slice(buf);
        Ok(())
    }
}

impl<T: Write> std::ops::Drop for ChecksumWriter<T> {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            // We may not have finished writing, so the data is wrong. It should not be given a
            // valid checksum
            self.writer
                .write_u32::<LittleEndian>(self.adler.checksum())
                .unwrap();
            self.writer.flush().unwrap();
            assert!(
                open_files().lock().unwrap().remove(&self.path),
                "File {:?} already dropped",
                self.path
            );
        }
    }
}

pub struct ChecksumReader<T: Read> {
    reader: T,
    adler: adler::Adler32,
}

impl<T: Read> ChecksumReader<T> {
    pub fn new(reader: T) -> Self {
        Self {
            reader,
            adler: adler::Adler32::new(),
        }
    }
}

/// We only implement the functions required and the ones we actually use.
impl<T: Read> Read for ChecksumReader<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bytes_read = self.reader.read(buf)?;
        self.adler.write_slice(&buf[0..bytes_read]);
        Ok(bytes_read)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.reader.read_exact(buf)?;
        self.adler.write_slice(buf);
        Ok(())
    }
}

impl<T: Read> std::ops::Drop for ChecksumReader<T> {
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
// When zstd is disabled, we don't mutate path
#[allow(unused_mut)]
fn open_file(mut path: PathBuf) -> Option<Box<dyn Read>> {
    // We should try in decreasing order of access speed.
    match File::open(&path) {
        Ok(f) => {
            let mut reader = BufReader::new(f);
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
            if e.kind() != ErrorKind::NotFound {
                panic!("Error when opening {path:?}: {e}");
            }
        }
    }

    #[cfg(feature = "zstd")]
    {
        path.set_extension("zst");
        match File::open(&path) {
            Ok(f) => {
                return Some(Box::new(ChecksumReader::new(
                    zstd::stream::Decoder::new(f).unwrap(),
                )))
            }
            Err(e) => {
                if e.kind() != ErrorKind::NotFound {
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
    fn write_header(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        buffer.write_u32::<LittleEndian>(self.kind.magic())?;
        buffer.write_u32::<LittleEndian>(self.algebra.magic())?;
        buffer.write_u32::<LittleEndian>(self.b.s())?;
        buffer.write_i32::<LittleEndian>(if let Some(i) = self.idx {
            self.b.t() + ((i as i32) << 16)
        } else {
            self.b.t()
        })
    }

    fn validate_header(&self, buffer: &mut impl Read) -> std::io::Result<()> {
        macro_rules! check_header {
            ($name:literal, $value:expr, $format:literal) => {
                let data = buffer.read_u32::<LittleEndian>()?;
                if data != $value {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
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

    /// This panics if there is no save dir
    fn get_save_path(&self, mut dir: PathBuf) -> PathBuf {
        if let Some(idx) = self.idx {
            dir.push(format!(
                "{name}s/{s}_{t}_{idx}_{name}",
                name = self.kind.name(),
                s = self.b.s(),
                t = self.b.t()
            ));
        } else {
            dir.push(format!(
                "{name}s/{s}_{t}_{name}",
                name = self.kind.name(),
                s = self.b.s(),
                t = self.b.t()
            ));
        }
        dir
    }

    pub fn open_file(&self, dir: PathBuf) -> Option<Box<dyn Read>> {
        let mut f = open_file(self.get_save_path(dir))?;
        self.validate_header(&mut f).unwrap();
        Some(f)
    }

    pub fn exists(&self, dir: PathBuf) -> bool {
        #[allow(unused_mut)]
        let mut path = self.get_save_path(dir);
        if path.exists() {
            return true;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            path.set_extension("zst");
            if path.exists() {
                return true;
            }
        }
        false
    }

    pub fn delete_file(&self, dir: PathBuf) -> std::io::Result<()> {
        let p = self.get_save_path(dir);
        match std::fs::remove_file(p) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// # Arguments
    ///  - `overwrite`: Whether to overwrite a file if it already exists.
    pub fn create_file(&self, dir: PathBuf, overwrite: bool) -> impl Write {
        let p = self.get_save_path(dir);

        // We need to do this before creating any file. The ctrlc handler does not block other threads
        // from running, but it does lock [`open_files()`]. So this ensures we do not open new files
        // while handling ctrlc.
        assert!(
            open_files().lock().unwrap().insert(p.clone()),
            "File {p:?} is already opened"
        );

        let f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(!overwrite)
            .create(true)
            .truncate(true)
            .open(&p)
            .with_context(|| format!("Failed to create save file {p:?}"))
            .unwrap();
        let mut f = ChecksumWriter::new(p, BufWriter::new(f));
        self.write_header(&mut f).unwrap();
        f
    }
}
