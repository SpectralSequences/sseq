//! Zarr v3 save store for resolutions, chain maps, and chain homotopies.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, RwLock},
};

use anyhow::Context;
use dashmap::{DashMap, DashSet};
use fp::{
    matrix::{Matrix, QuasiInverse},
    prime::ValidPrime,
    vector::{FpSlice, FpSliceMut, FpVector},
};
use sseq::coordinates::{Bidegree, BidegreeGenerator};
use zarrs::{
    array::{ArrayBuilder, ArraySubset, CodecOptions, codec::Crc32cCodec, data_type},
    group::GroupBuilder,
    storage::{ReadableWritableListableStorage, ReadableWritableListableStorageTraits},
};

// zarr's `filesystem` and `zstd` features build only under `cfg(any(windows, unix))`, the condition
// that gates `positioned-io::RandomAccessFile`; other targets, such as the web frontend, get an
// in-memory store. Cargo.toml selects features by the same predicate, and the two `platform`
// modules hold every target `cfg`.

#[cfg(any(windows, unix))]
mod platform {
    use std::sync::{
        LazyLock,
        atomic::{AtomicU64, Ordering},
    };

    use zarrs::{
        array::codec::ZstdCodec,
        filesystem::FilesystemStore,
        storage::{
            AtomicRenameStorageTraits, Bytes, ListableStorageTraits, MaybeBytesIterator,
            OffsetBytesIterator, ReadableStorageTraits, StorageError, StoreKey, StoreKeys,
            StoreKeysPrefixes, StorePrefix, WritableStorageTraits, byte_range::ByteRangeIterator,
        },
    };

    use super::*;

    /// Open the on-disk zarr store rooted at `path`.
    pub fn open_store(path: &Path) -> anyhow::Result<ReadableWritableListableStorage> {
        Ok(Arc::new(AtomicWrites {
            inner: FilesystemStore::new(path)?,
            next_temporary: AtomicU64::new(0),
        }))
    }

    /// A [`FilesystemStore`] whose writes replace each file atomically.
    ///
    /// `FilesystemStore::set` truncates the file before writing it, and the sharding codec rewrites
    /// a whole shard on every store, so a crash mid-write would lose every element already in the
    /// shard. Here each value goes to a temporary key that is then renamed over the destination.
    /// Temporary keys are unique, so writers of one key, such as threads creating the same group,
    /// never collide; a crash leaves at most an orphaned temporary file, which nothing reads.
    struct AtomicWrites {
        /// The store every operation goes to.
        inner: FilesystemStore,
        /// Counter that makes each temporary key unique within the process.
        next_temporary: AtomicU64,
    }

    impl AtomicWrites {
        /// A fresh temporary key beside `key`, unique across processes and threads.
        fn temporary_key(&self, key: &StoreKey) -> Result<StoreKey, StorageError> {
            let n = self.next_temporary.fetch_add(1, Ordering::Relaxed);
            StoreKey::new(format!("{key}.{}.{n}.tmp", std::process::id()))
                .map_err(|e| StorageError::Other(e.to_string()))
        }
    }

    impl ReadableStorageTraits for AtomicWrites {
        fn get_partial_many<'a>(
            &'a self,
            key: &StoreKey,
            byte_ranges: ByteRangeIterator<'a>,
        ) -> Result<MaybeBytesIterator<'a>, StorageError> {
            self.inner.get_partial_many(key, byte_ranges)
        }

        fn size_key(&self, key: &StoreKey) -> Result<Option<u64>, StorageError> {
            self.inner.size_key(key)
        }

        fn supports_get_partial(&self) -> bool {
            self.inner.supports_get_partial()
        }
    }

    impl ListableStorageTraits for AtomicWrites {
        fn list(&self) -> Result<StoreKeys, StorageError> {
            self.inner.list()
        }

        fn list_prefix(&self, prefix: &StorePrefix) -> Result<StoreKeys, StorageError> {
            self.inner.list_prefix(prefix)
        }

        fn list_dir(&self, prefix: &StorePrefix) -> Result<StoreKeysPrefixes, StorageError> {
            self.inner.list_dir(prefix)
        }

        fn size_prefix(&self, prefix: &StorePrefix) -> Result<u64, StorageError> {
            self.inner.size_prefix(prefix)
        }
    }

    impl WritableStorageTraits for AtomicWrites {
        fn set(&self, key: &StoreKey, value: Bytes) -> Result<(), StorageError> {
            let temporary = self.temporary_key(key)?;
            // `set` syncs the file, so the rename never exposes unwritten data.
            self.inner.set(&temporary, value)?;
            self.inner.rename(&temporary, key).inspect_err(|_| {
                let _ = self.inner.erase(&temporary);
            })
        }

        /// Rewrite the whole value through [`Self::set`], so partial writes are atomic too.
        fn set_partial_many(
            &self,
            key: &StoreKey,
            offset_values: OffsetBytesIterator,
        ) -> Result<(), StorageError> {
            zarrs::storage::store_set_partial_many(self, key, offset_values)
        }

        fn erase(&self, key: &StoreKey) -> Result<(), StorageError> {
            self.inner.erase(key)
        }

        fn erase_prefix(&self, prefix: &StorePrefix) -> Result<(), StorageError> {
            self.inner.erase_prefix(prefix)
        }

        fn supports_set_partial(&self) -> bool {
            false
        }
    }

    /// Whether the root group's `zarr.json` is not on disk yet.
    pub fn root_group_missing(path: &Path) -> bool {
        !path.join("zarr.json").exists()
    }

    /// Stream-tier codec chain: zstd (see [`zstd_level`]) followed by CRC32C.
    pub fn stream_tier_codecs() -> Vec<Arc<dyn zarrs::array::BytesToBytesCodecTraits>> {
        vec![
            Arc::new(ZstdCodec::new(zstd_level(), false)),
            Arc::new(Crc32cCodec::new()),
        ]
    }

    /// Stream-tier zstd level when [`ZSTD_LEVEL_ENV`] is unset.
    ///
    /// Every quasi-inverse is compressed on the write path, and higher levels buy little ratio on
    /// this dense data for much more write time.
    const DEFAULT_ZSTD_LEVEL: i32 = 3;

    /// Overrides the stream-tier zstd level, for runs where disk use matters more than save time.
    ///
    /// Clamped to zstd's valid range; an unparseable value falls back to [`DEFAULT_ZSTD_LEVEL`]
    /// with a warning.
    const ZSTD_LEVEL_ENV: &str = "EXT_SAVE_ZSTD_LEVEL";

    /// Resolve the stream-tier zstd level, reading [`ZSTD_LEVEL_ENV`] at most once.
    fn zstd_level() -> i32 {
        static LEVEL: LazyLock<i32> = LazyLock::new(|| match std::env::var(ZSTD_LEVEL_ENV) {
            Ok(s) => match s.trim().parse::<i32>() {
                Ok(v) => {
                    let clamped = v.clamp(1, 22);
                    if clamped != v {
                        tracing::warn!(
                            "{ZSTD_LEVEL_ENV}={v} is outside zstd's [1, 22] range; using {clamped}"
                        );
                    }
                    clamped
                }
                Err(_) => {
                    tracing::warn!(
                        "{ZSTD_LEVEL_ENV}={s:?} is not a valid integer; using default level \
                         {DEFAULT_ZSTD_LEVEL}"
                    );
                    DEFAULT_ZSTD_LEVEL
                }
            },
            Err(_) => DEFAULT_ZSTD_LEVEL,
        });
        *LEVEL
    }
}

#[cfg(not(any(windows, unix)))]
mod platform {
    use zarrs::storage::store::MemoryStore;

    use super::*;

    /// Open an in-memory store, dropped at the end of the session.
    pub fn open_store(_path: &Path) -> anyhow::Result<ReadableWritableListableStorage> {
        Ok(Arc::new(MemoryStore::new()))
    }

    /// Always true: an in-memory store starts empty.
    pub fn root_group_missing(_path: &Path) -> bool {
        true
    }

    /// Stream-tier codec chain: CRC32C only, since `zstd-sys` does not build for this target.
    pub fn stream_tier_codecs() -> Vec<Arc<dyn zarrs::array::BytesToBytesCodecTraits>> {
        vec![Arc::new(Crc32cCodec::new())]
    }

    // SAFETY: this target is single-threaded, so `Send` and `Sync` hold vacuously. They are
    // missing only because the store's trait object is bounded by the no-op `MaybeSend +
    // MaybeSync`, and the rest of `ext` needs chain complexes to be `Send + Sync`.
    unsafe impl Send for super::ZarrSaveStore {}
    unsafe impl Sync for super::ZarrSaveStore {}
}

/// Most-negative stem the on-disk layout can store.
///
/// Stems can be negative (e.g. `RP^\infty_{-k}`, A-module shifts) and zarr v3 has no negative
/// chunk indices, so every caller-supplied `n` is stored at `n - N_MIN`. Callers never see the
/// offset. Sparse arrays cost nothing for the empty negative region beyond `zarr.json` metadata.
const N_MIN: i32 = -1024;

/// Initial number of slots in the n dimension, starting at [`N_MIN`].
///
/// This and the other spans are only the shape a shard-tier array is created with; a write past
/// the end grows it (see [`ZarrSaveStore::shard_array_covering`]).
const N_SPAN: u64 = 4096;

/// Initial number of slots in the s dimension. `s` is unsigned, so slots start at 0.
const S_SPAN: u64 = 1024;

/// Initial number of slots for the intra-bidegree index of indexed kinds.
const IDX_SPAN: u64 = 256;

/// Shard shape in the n dimension.
const SHARD_N: u64 = 8;

/// Shard shape in the s dimension.
const SHARD_S: u64 = 8;

/// Shard shape in the idx dimension.
const SHARD_IDX: u64 = 8;

/// Convert a zarrs error into `anyhow::Error` through its `Display`.
///
/// Some zarrs errors are not `Send + Sync` on wasm, so the blanket `From` does not apply there.
fn zarr_err<E: std::fmt::Display>(e: E) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}

/// Options that store a chunk even when it equals the fill value.
///
/// By default zarrs skips such a chunk, and a reader cannot tell it from one lost in a crash. The
/// stream tier treats a missing chunk of a finished quasi-inverse as corruption, so it must
/// store every chunk.
fn store_every_chunk() -> CodecOptions {
    CodecOptions::default().with_store_empty_chunks(true)
}

/// Number of matrix rows per chunk in the ResQi `rows` array.
///
/// Bounds the memory needed to read one chunk worth of rows.
const CHUNK_RES_QI_ROWS: u64 = 1024;

/// Number of commands per chunk in the NassauQi `commands` array.
///
/// Bounds the memory the writer buffers between chunk flushes.
const CHUNK_NASSAU_COMMANDS: u64 = 1024;

/// Upper bound on the number of commands in a NassauQi `commands` array.
///
/// Used as the array shape at creation time. The actual count is committed to the `num_commands`
/// group attribute on `finish()`.
const NASSAU_QI_MAX_COMMANDS: u64 = 1 << 24;

/// A zarr v3 save store, or one of its subgroups; `ext/SAVE-FORMAT.md` specifies the layout.
///
/// Shard-tier arrays are created on first write, so a subgroup holds only the kinds it uses. Their
/// payloads are too small for zstd to pay, unlike the stream tier's quasi-inverses.
pub struct ZarrSaveStore {
    /// Filesystem root of the underlying store, shared by all subgroups.
    path: PathBuf,
    /// Underlying zarr storage, shared by all subgroups.
    store: ReadableWritableListableStorage,
    /// Group prefix of every operation: empty for the root store, e.g. `/products/foo` otherwise.
    group: String,
    /// Shard-tier arrays known to exist, so later writes skip the metadata check.
    created: DashSet<SaveKind>,
    /// Opened shard-tier arrays, so each kind's `zarr.json` is parsed once.
    ///
    /// An `Array` holds no chunk data and its methods take `&self`, so one handle serves
    /// concurrent reads and shard-serialized writes.
    arrays: DashMap<SaveKind, Arc<ShardArray>>,
    /// The locks of this group, shared by every handle on it; see [`GroupLocks`].
    locks: Arc<GroupLocks>,
}

/// The locks of one group, shared by every [`ZarrSaveStore`] handle on it in the process.
#[derive(Default)]
struct GroupLocks {
    /// One lock per shard: shared by reads, exclusive for writes.
    ///
    /// The sharding codec rewrites the whole shard on every store, so concurrent writes to one
    /// shard lose data, and a read, which fetches the shard index and then the element separately,
    /// can see the index of one version and the bytes of another. Shards are independent.
    shards: DashMap<(SaveKind, [u64; 3]), Arc<RwLock<()>>>,
    /// Serializes rewriting an array's `zarr.json` to grow its shape, so no handle shrinks it.
    grow: std::sync::Mutex<()>,
}

impl GroupLocks {
    /// The locks of the group stored at `group_path`, shared with every other handle on it.
    ///
    /// Two handles on one group, from two `subgroup` calls or two `create` calls, write the same
    /// shard files, so they must share locks.
    fn for_group(group_path: &Path) -> Arc<Self> {
        static REGISTRY: LazyLock<DashMap<PathBuf, Arc<GroupLocks>>> = LazyLock::new(DashMap::new);
        Arc::clone(
            REGISTRY
                .entry(group_path.to_path_buf())
                .or_default()
                .value(),
        )
    }
}

/// Alias for the concrete opened-array type shared across the store and its streaming readers.
type ShardArray = zarrs::array::Array<dyn ReadableWritableListableStorageTraits>;

impl std::fmt::Debug for ZarrSaveStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZarrSaveStore")
            .field("path", &self.path)
            .field("group", &self.group)
            .finish_non_exhaustive()
    }
}

impl ZarrSaveStore {
    /// Open the store at `path`, creating its root group if absent.
    ///
    /// An existing store is opened as is; [`Self::bind_to_algebra`] and [`Self::bind_module_spec`]
    /// check that it holds the expected complex. Fails if the path cannot be resolved or the store
    /// cannot be opened or initialized.
    pub fn create(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = std::path::absolute(path.as_ref())
            .with_context(|| format!("Failed to resolve path: {:?}", path.as_ref()))?;

        let store: ReadableWritableListableStorage = platform::open_store(&path)?;

        if platform::root_group_missing(&path) {
            GroupBuilder::new()
                .build(store.clone(), "/")?
                .store_metadata()?;
        }

        Ok(Self {
            store,
            group: String::new(),
            created: DashSet::new(),
            arrays: DashMap::new(),
            locks: GroupLocks::for_group(&path),
            path,
        })
    }

    /// Bind this store to the algebra `algebra_magic`, recording it with `prime` and the prefix.
    ///
    /// A store bound to another algebra is an error. [`Self::bind_module_spec`] pins the module,
    /// and [`construct_from_save`] reads `algebra_prefix` back. Subgroups share these attributes.
    ///
    /// [`construct_from_save`]: crate::utils::construct_from_save
    pub fn bind_to_algebra(
        &self,
        algebra_magic: u32,
        prime: u32,
        algebra_prefix: &str,
    ) -> anyhow::Result<()> {
        let root = zarrs::group::Group::open(self.store.clone(), "/").map_err(zarr_err)?;
        let attrs = root.attributes();
        if let Some(stored) = attrs.get("algebra_magic").and_then(|v| v.as_u64()) {
            let stored = stored as u32;
            if stored != algebra_magic {
                let stored_prefix = attrs
                    .get("algebra_prefix")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let stored_prime = attrs
                    .get("prime")
                    .and_then(|v| v.as_u64())
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "?".into());
                anyhow::bail!(
                    "Save store at {:?} was created with a different algebra: stored magic \
                     {stored:#010x} ({stored_prefix} at p={stored_prime}), expected magic \
                     {algebra_magic:#010x} ({algebra_prefix} at p={prime})",
                    self.path,
                );
            }
            return Ok(());
        }

        let mut new_attrs = attrs.clone();
        new_attrs.insert("algebra_magic".into(), (u64::from(algebra_magic)).into());
        new_attrs.insert("prime".into(), (u64::from(prime)).into());
        new_attrs.insert("algebra_prefix".into(), algebra_prefix.into());
        let group = GroupBuilder::new()
            .attributes(new_attrs)
            .build(self.store.clone(), "/")
            .map_err(zarr_err)?;
        group.store_metadata().map_err(zarr_err)?;
        Ok(())
    }

    /// Record `name` as the root group's `complex_name`, a label that never gates loading.
    pub fn set_complex_name(&self, name: &str) -> anyhow::Result<()> {
        let root = zarrs::group::Group::open(self.store.clone(), "/").map_err(zarr_err)?;
        let mut new_attrs = root.attributes().clone();
        new_attrs.insert("complex_name".into(), name.into());
        let group = GroupBuilder::new()
            .attributes(new_attrs)
            .build(self.store.clone(), "/")
            .map_err(zarr_err)?;
        group.store_metadata().map_err(zarr_err)?;
        Ok(())
    }

    /// Bind this store to the module `spec`, recording it in the root group.
    ///
    /// A store bound to a different module over the same algebra is an error. The recorded spec is
    /// what [`construct_from_save`] rebuilds the complex from.
    ///
    /// [`construct_from_save`]: crate::utils::construct_from_save
    pub fn bind_module_spec(&self, spec: &serde_json::Value) -> anyhow::Result<()> {
        let root = zarrs::group::Group::open(self.store.clone(), "/").map_err(zarr_err)?;
        let attrs = root.attributes();
        if let Some(stored) = attrs.get("module_spec") {
            if stored != spec {
                anyhow::bail!(
                    "Save store at {:?} was created for a different complex: the stored module \
                     spec does not match the one being resolved. Refusing to mix cached data \
                     between distinct complexes; use a separate save directory.\n  stored:   \
                     {stored}\n  expected: {spec}",
                    self.path,
                );
            }
            return Ok(());
        }

        let mut new_attrs = attrs.clone();
        new_attrs.insert("module_spec".into(), spec.clone());
        let group = GroupBuilder::new()
            .attributes(new_attrs)
            .build(self.store.clone(), "/")
            .map_err(zarr_err)?;
        group.store_metadata().map_err(zarr_err)?;
        Ok(())
    }

    /// Read the root-group attributes of an existing store at `path` without binding to a complex.
    pub fn read_root_attributes(
        path: impl AsRef<Path>,
    ) -> anyhow::Result<serde_json::Map<String, serde_json::Value>> {
        let path = std::path::absolute(path.as_ref())
            .with_context(|| format!("Failed to resolve path: {:?}", path.as_ref()))?;
        let store = platform::open_store(&path)?;
        let root = zarrs::group::Group::open(store, "/").map_err(zarr_err)?;
        Ok(root.attributes().clone())
    }

    /// Open the subgroup `name` of this group, creating it if missing.
    pub fn subgroup(&self, name: &str) -> anyhow::Result<Self> {
        let group = format!("{}/{}", self.group, name);
        let group_path = self.path.join(group.trim_start_matches('/'));
        if !group_path.join("zarr.json").exists() {
            // Intermediate levels such as `products/` must be groups, not bare directories.
            self.ensure_intermediate_groups(&group)?;
            GroupBuilder::new()
                .build(self.store.clone(), &group)?
                .store_metadata()?;
        }
        Ok(Self {
            path: self.path.clone(),
            store: self.store.clone(),
            locks: GroupLocks::for_group(&group_path),
            group,
            created: DashSet::new(),
            arrays: DashMap::new(),
        })
    }

    /// Walk every prefix of `group` and create a zarr group for any prefix that doesn't already
    /// have one (e.g. `/products/` before `/products/foo`).
    fn ensure_intermediate_groups(&self, group: &str) -> anyhow::Result<()> {
        let segments: Vec<&str> = group.split('/').filter(|s| !s.is_empty()).collect();
        for i in 1..segments.len() {
            let prefix = format!("/{}", segments[..i].join("/"));
            let meta = self
                .path
                .join(prefix.trim_start_matches('/'))
                .join("zarr.json");
            if !meta.exists() {
                GroupBuilder::new()
                    .build(self.store.clone(), &prefix)?
                    .store_metadata()?;
            }
        }
        Ok(())
    }

    /// Filesystem root of the store, shared by all its subgroups.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Filesystem path of this store's group.
    fn group_fs_path(&self) -> PathBuf {
        if self.group.is_empty() {
            self.path.clone()
        } else {
            self.path.join(self.group.trim_start_matches('/'))
        }
    }

    /// Zarr path of the shard-tier array for `kind` in this group.
    fn shard_array_path(&self, kind: SaveKind) -> String {
        format!("{}/{}", self.group, kind.name())
    }

    /// The zarr indices of `(n, s, [idx])`: `n` offset by [`N_MIN`], the rest unchanged.
    ///
    /// `None` for a stem below [`N_MIN`], which the layout cannot hold.
    fn shard_zarr_coords<const N: usize>(coords: [i32; N]) -> Option<Vec<u64>> {
        let mut out = Vec::with_capacity(N);
        out.push(u64::try_from(coords[0] - N_MIN).ok()?);
        for &c in &coords[1..] {
            out.push(u64::try_from(c).ok()?);
        }
        Some(out)
    }

    /// The shard holding `zarr_coords`, with `0` in the third slot for 2D kinds.
    ///
    /// A one-element store or retrieve touches only this shard, so it is what the lock covers.
    fn shard_coords(zarr_coords: &[u64]) -> [u64; 3] {
        let mut out = [0u64; 3];
        out[0] = zarr_coords[0] / SHARD_N;
        out[1] = zarr_coords[1] / SHARD_S;
        if zarr_coords.len() > 2 {
            out[2] = zarr_coords[2] / SHARD_IDX;
        }
        out
    }

    /// The lock of the shard holding `zarr_coords` for `kind`.
    fn shard_lock(&self, kind: SaveKind, zarr_coords: &[u64]) -> Arc<RwLock<()>> {
        let key = (kind, Self::shard_coords(zarr_coords));
        Arc::clone(self.locks.shards.entry(key).or_default().value())
    }

    /// Lazily create the shard-tier array for `kind` under this group.
    fn ensure_shard_array(&self, kind: SaveKind) -> anyhow::Result<()> {
        if self.created.contains(&kind) {
            return Ok(());
        }
        // Under the grow lock, so a creation racing a growth cannot write back the initial shape.
        let _guard = self.locks.grow.lock().unwrap();
        let meta_path = self.group_fs_path().join(kind.name()).join("zarr.json");
        if !meta_path.exists() {
            let array_path = self.shard_array_path(kind);
            let (shape, chunk_shape, subchunk) = if kind.is_indexed() {
                (
                    vec![N_SPAN, S_SPAN, IDX_SPAN],
                    vec![SHARD_N, SHARD_S, SHARD_IDX],
                    vec![1, 1, 1],
                )
            } else {
                (vec![N_SPAN, S_SPAN], vec![SHARD_N, SHARD_S], vec![1, 1])
            };
            let arr = ArrayBuilder::new(
                shape,
                chunk_shape,
                data_type::bytes(),
                zarrs::array::FillValue::from(Vec::<u8>::new()),
            )
            .subchunk_shape(subchunk)
            .bytes_to_bytes_codecs(vec![Arc::new(Crc32cCodec::new())])
            .build(self.store.clone(), &array_path)?;
            arr.store_metadata()?;
        }
        self.created.insert(kind);
        Ok(())
    }

    /// The shard-tier array for `kind`, created if absent with `create`, otherwise `None`.
    fn shard_array(&self, kind: SaveKind, create: bool) -> anyhow::Result<Option<Arc<ShardArray>>> {
        if let Some(arr) = self.arrays.get(&kind) {
            return Ok(Some(Arc::clone(arr.value())));
        }
        if create {
            self.ensure_shard_array(kind)?;
        }
        match zarrs::array::Array::open(self.store.clone(), &self.shard_array_path(kind)) {
            Ok(arr) => {
                let arr = Arc::new(arr);
                // If another thread opened it first, keep the existing handle; they're equivalent.
                let cached = Arc::clone(self.arrays.entry(kind).or_insert(arr).value());
                Ok(Some(cached))
            }
            Err(_) if !create => Ok(None),
            Err(e) => Err(zarr_err(e)),
        }
    }

    /// The shard-tier array for `kind`, created if absent and grown to cover `zarr_coords`.
    ///
    /// zarrs stores an element outside an array's shape without complaint, but a reader that
    /// follows the zarr spec cannot see it, so the shape must grow first. Each axis that is too
    /// short at least doubles, which keeps every axis a multiple of its shard length.
    fn shard_array_covering(
        &self,
        kind: SaveKind,
        zarr_coords: &[u64],
    ) -> anyhow::Result<Arc<ShardArray>> {
        let covers = |shape: &[u64]| zarr_coords.iter().zip(shape).all(|(&c, &len)| c < len);
        let arr = self
            .shard_array(kind, true)?
            .expect("shard_array(create=true) never returns None");
        if covers(arr.shape()) {
            return Ok(arr);
        }
        let _guard = self.locks.grow.lock().unwrap();
        // Another handle may have grown the array since this one cached it, so start from disk.
        let mut arr = zarrs::array::Array::open(self.store.clone(), &self.shard_array_path(kind))
            .map_err(zarr_err)?;
        if !covers(arr.shape()) {
            let shape = arr
                .shape()
                .iter()
                .zip(zarr_coords)
                .map(|(&len, &c)| {
                    if c < len {
                        len
                    } else {
                        (c + 1).next_power_of_two().max(2 * len)
                    }
                })
                .collect();
            arr.set_shape(shape).map_err(zarr_err)?;
            arr.store_metadata().map_err(zarr_err)?;
        }
        let arr = Arc::new(arr);
        self.arrays.insert(kind, Arc::clone(&arr));
        Ok(arr)
    }

    /// Lazily create `qi/` and `qi/n{n}_s{s}/` groups under this group prefix.
    fn ensure_qi_bidegree(&self, b: Bidegree) -> anyhow::Result<()> {
        let qi_root = self.group_fs_path().join("qi");
        if !qi_root.join("zarr.json").exists() {
            GroupBuilder::new()
                .build(self.store.clone(), &format!("{}/qi", self.group))?
                .store_metadata()?;
        }
        let bidegree_meta = qi_root
            .join(format!("n{}_s{}", b.n(), b.s()))
            .join("zarr.json");
        if !bidegree_meta.exists() {
            GroupBuilder::new()
                .build(
                    self.store.clone(),
                    &format!("{}/qi/n{}_s{}", self.group, b.n(), b.s()),
                )?
                .store_metadata()?;
        }
        Ok(())
    }

    /// Write the `kind` payload at `location`.
    ///
    /// Quasi-inverses can be too large to buffer; they go through [`Self::write_res_qi`] and
    /// [`Self::nassau_qi_writer`].
    pub fn write<const N: usize>(
        &self,
        kind: SaveKind,
        location: impl SaveCoords<N>,
        data: &[u8],
    ) -> anyhow::Result<()> {
        assert!(
            !matches!(kind, SaveKind::ResQi | SaveKind::NassauQi),
            "write() is only for sharded kinds, got {:?}",
            kind
        );
        let coords = location.save_coords();
        let zarr_coords = Self::shard_zarr_coords(coords).with_context(|| {
            format!("Cannot save {kind:?} at {coords:?}: stems below {N_MIN} are not supported")
        })?;
        let arr = self.shard_array_covering(kind, &zarr_coords)?;
        let lock = self.shard_lock(kind, &zarr_coords);
        let _guard = lock.write().unwrap();
        let subset = ArraySubset::new_with_start_shape(zarr_coords, vec![1; N])?;
        // Sequential codecs: the sharding codec runs on rayon, and a worker holding the lock could
        // pick up another task that needs it and deadlock.
        arr.store_array_subset_opt(
            &subset,
            vec![data.to_vec()],
            &CodecOptions::default().with_concurrent_target(1),
        )
        .map_err(zarr_err)?;
        Ok(())
    }

    /// Read the `kind` payload at `location`, or `None` if none was written.
    pub fn read<const N: usize>(
        &self,
        kind: SaveKind,
        location: impl SaveCoords<N>,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        assert!(
            !matches!(kind, SaveKind::ResQi | SaveKind::NassauQi),
            "read() is only for sharded kinds, got {:?}",
            kind
        );
        let arr = match self.shard_array(kind, false)? {
            Some(arr) => arr,
            None => return Ok(None),
        };
        let Some(zarr_coords) = Self::shard_zarr_coords(location.save_coords()) else {
            return Ok(None);
        };
        let lock = self.shard_lock(kind, &zarr_coords);
        let _guard = lock.read().unwrap();
        let subset = ArraySubset::new_with_start_shape(zarr_coords, vec![1; N])?;
        // Sequential codecs, as in `write`.
        let data: Vec<Vec<u8>> = arr
            .retrieve_array_subset_opt(&subset, &CodecOptions::default().with_concurrent_target(1))
            .map_err(zarr_err)?;
        match data.into_iter().next() {
            Some(element) if !element.is_empty() => Ok(Some(element)),
            _ => Ok(None),
        }
    }

    /// Check if a sharded payload exists.
    pub fn exists<const N: usize>(&self, kind: SaveKind, location: impl SaveCoords<N>) -> bool {
        assert!(
            !matches!(kind, SaveKind::ResQi | SaveKind::NassauQi),
            "exists() is only for sharded kinds, got {:?}",
            kind
        );
        matches!(self.read(kind, location), Ok(Some(_)))
    }

    /// Delete a sharded payload (overwrites with the empty fill value).
    pub fn delete<const N: usize>(
        &self,
        kind: SaveKind,
        location: impl SaveCoords<N>,
    ) -> anyhow::Result<()> {
        assert!(
            !matches!(kind, SaveKind::ResQi | SaveKind::NassauQi),
            "delete() is only for sharded kinds, got {:?}",
            kind
        );
        // A missing array has nothing to delete; otherwise lock and store as in `write`.
        let arr = match self.shard_array(kind, false)? {
            Some(arr) => arr,
            None => return Ok(()),
        };
        let Some(zarr_coords) = Self::shard_zarr_coords(location.save_coords()) else {
            return Ok(());
        };
        let lock = self.shard_lock(kind, &zarr_coords);
        let _guard = lock.write().unwrap();
        let subset = ArraySubset::new_with_start_shape(zarr_coords, vec![1; N])?;
        arr.store_array_subset_opt(
            &subset,
            vec![Vec::<u8>::new()],
            &CodecOptions::default().with_concurrent_target(1),
        )
        .map_err(zarr_err)?;
        Ok(())
    }

    // --- ResQi structured I/O ---

    /// Filesystem path of the ResQi group for bidegree `b`.
    fn res_qi_group_path(&self, b: Bidegree) -> String {
        format!("{}/qi/n{}_s{}/res_qi", self.group, b.n(), b.s())
    }

    /// Write `qi` as the group `qi/n{n}_s{s}/res_qi`, laid out as in `ext/SAVE-FORMAT.md`.
    ///
    /// `finished` is set last, so readers treat an interrupted write as missing.
    pub fn write_res_qi(&self, b: Bidegree, qi: &QuasiInverse) -> anyhow::Result<()> {
        self.ensure_qi_bidegree(b)?;
        let group_path = self.res_qi_group_path(b);

        let source_dim = qi.source_dimension();
        let target_dim = qi.target_dimension();
        let image_dim = qi.image_dimension();
        let preimage = qi.preimage();
        let p = preimage.prime();
        let num_limbs = FpVector::num_limbs(p, source_dim);
        let row_bytes = num_limbs * 8;

        let mut group_attrs = serde_json::Map::new();
        group_attrs.insert("source_dim".into(), (source_dim as u64).into());
        group_attrs.insert("target_dim".into(), (target_dim as u64).into());
        group_attrs.insert("image_dim".into(), (image_dim as u64).into());
        group_attrs.insert("finished".into(), false.into());
        let group = GroupBuilder::new()
            .attributes(group_attrs.clone())
            .build(self.store.clone(), &group_path)?;
        group.store_metadata()?;

        let pivots_shape = std::cmp::max(target_dim as u64, 1);
        let pivots_array = ArrayBuilder::new(
            vec![pivots_shape],
            vec![pivots_shape],
            data_type::int64(),
            zarrs::array::FillValue::from(0i64),
        )
        .bytes_to_bytes_codecs(platform::stream_tier_codecs())
        .build(self.store.clone(), &format!("{}/pivots", group_path))?;
        pivots_array.store_metadata()?;
        let mut pivots_data: Vec<i64> = match qi.pivots() {
            Some(p) => p.iter().map(|&x| x as i64).collect(),
            None => (0..target_dim as i64).collect(),
        };
        if pivots_data.is_empty() {
            pivots_data.push(0);
        }
        pivots_array
            .store_chunk_opt(&[0], pivots_data, &store_every_chunk())
            .map_err(zarr_err)?;

        let rows_shape_0 = std::cmp::max(image_dim as u64, 1);
        let rows_shape_1 = std::cmp::max(row_bytes as u64, 1);
        let chunk_rows = std::cmp::min(CHUNK_RES_QI_ROWS, rows_shape_0);
        let rows_array = ArrayBuilder::new(
            vec![rows_shape_0, rows_shape_1],
            vec![chunk_rows, rows_shape_1],
            data_type::uint8(),
            zarrs::array::FillValue::from(0u8),
        )
        .bytes_to_bytes_codecs(platform::stream_tier_codecs())
        .build(self.store.clone(), &format!("{}/rows", group_path))?;
        rows_array.store_metadata()?;

        if image_dim > 0 && row_bytes > 0 {
            // Pad the last chunk to the chunk shape; the reader stops at `image_dim`.
            let chunk_byte_len = (chunk_rows as usize) * row_bytes;
            let mut chunk_buf: Vec<u8> = Vec::with_capacity(chunk_byte_len);
            let mut chunk_idx: u64 = 0;
            for row_idx in 0..image_dim {
                let row_vec: FpVector = preimage.row(row_idx).to_owned();
                let buf_before = chunk_buf.len();
                row_vec.to_bytes(&mut chunk_buf)?;
                debug_assert_eq!(chunk_buf.len() - buf_before, row_bytes);
                if chunk_buf.len() == chunk_byte_len {
                    rows_array
                        .store_chunk_opt(
                            &[chunk_idx, 0],
                            std::mem::take(&mut chunk_buf),
                            &store_every_chunk(),
                        )
                        .map_err(zarr_err)?;
                    chunk_idx += 1;
                    chunk_buf.reserve(chunk_byte_len);
                }
            }
            if !chunk_buf.is_empty() {
                chunk_buf.resize(chunk_byte_len, 0);
                rows_array
                    .store_chunk_opt(&[chunk_idx, 0], chunk_buf, &store_every_chunk())
                    .map_err(zarr_err)?;
            }
        }

        group_attrs.insert("finished".into(), true.into());
        let finished_group = GroupBuilder::new()
            .attributes(group_attrs)
            .build(self.store.clone(), &group_path)?;
        finished_group.store_metadata()?;
        Ok(())
    }

    /// Stream the ResQi at `b`, one chunk of rows at a time, or `None` unless it was finished.
    pub fn stream_res_qi(&self, b: Bidegree, p: ValidPrime) -> anyhow::Result<Option<ResQiReader>> {
        let group_path = self.res_qi_group_path(b);
        let group = match zarrs::group::Group::open(self.store.clone(), &group_path) {
            Ok(g) => g,
            Err(_) => return Ok(None),
        };
        let attrs = group.attributes();
        if !attrs
            .get("finished")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return Ok(None);
        }
        let source_dim = attrs
            .get("source_dim")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let target_dim = attrs
            .get("target_dim")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let image_dim = attrs.get("image_dim").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let pivots_array =
            zarrs::array::Array::open(self.store.clone(), &format!("{}/pivots", group_path))?;
        // All-zero pivots, the fill value, are valid only when `target_dim <= 1`, and a store
        // written without `store_every_chunk` omits them.
        let pivots_chunk: Vec<i64> = match pivots_array
            .retrieve_chunk_if_exists(&[0])
            .map_err(zarr_err)?
        {
            Some(chunk) => chunk,
            None if target_dim <= 1 => vec![0; target_dim],
            None => anyhow::bail!("ResQi at {b} is finished but its pivots are missing"),
        };
        let pivots: Vec<isize> = pivots_chunk
            .into_iter()
            .take(target_dim)
            .map(|x| x as isize)
            .collect();

        let rows_array =
            zarrs::array::Array::open(self.store.clone(), &format!("{}/rows", group_path))?;

        Ok(Some(ResQiReader {
            p,
            source_dim,
            target_dim,
            image_dim,
            pivots,
            rows_array,
            chunk_buf: Vec::new(),
            chunk_idx: 0,
            pos_in_chunk: 0,
            row_bytes: FpVector::num_limbs(p, source_dim) * 8,
            chunk_rows: 0,
        }))
    }

    // --- NassauQi structured I/O ---

    /// Filesystem path of the NassauQi group for bidegree `b`.
    fn nassau_qi_group_path(&self, b: Bidegree) -> String {
        format!("{}/qi/n{}_s{}/nassau_qi", self.group, b.n(), b.s())
    }

    /// Open a writer for the NassauQi group at `b`, laid out as in `ext/SAVE-FORMAT.md`.
    pub fn nassau_qi_writer(
        &self,
        b: Bidegree,
        target_dim: u64,
        zero_mask_dim: u64,
        subalgebra_profile: &[u8],
    ) -> anyhow::Result<NassauQiWriter> {
        self.ensure_qi_bidegree(b)?;
        let group_path = self.nassau_qi_group_path(b);

        let mut group_attrs = serde_json::Map::new();
        group_attrs.insert("target_dim".into(), target_dim.into());
        group_attrs.insert("zero_mask_dim".into(), zero_mask_dim.into());
        group_attrs.insert(
            "subalgebra_profile".into(),
            serde_json::Value::Array(
                subalgebra_profile
                    .iter()
                    .map(|&b| serde_json::Value::from(b as u64))
                    .collect(),
            ),
        );
        group_attrs.insert("finished".into(), false.into());
        let group = GroupBuilder::new()
            .attributes(group_attrs.clone())
            .build(self.store.clone(), &group_path)?;
        group.store_metadata()?;

        let commands_array = ArrayBuilder::new(
            vec![NASSAU_QI_MAX_COMMANDS],
            vec![CHUNK_NASSAU_COMMANDS],
            data_type::bytes(),
            zarrs::array::FillValue::from(Vec::<u8>::new()),
        )
        .bytes_to_bytes_codecs(platform::stream_tier_codecs())
        .build(self.store.clone(), &format!("{}/commands", group_path))?;
        commands_array.store_metadata()?;

        Ok(NassauQiWriter {
            store: self.store.clone(),
            group_path,
            group_attrs,
            commands_array,
            command_buf: Vec::with_capacity(CHUNK_NASSAU_COMMANDS as usize),
            chunk_idx: 0,
            commands_written: 0,
        })
    }

    /// Stream the NassauQi at `b`, one chunk of commands at a time, or `None` unless it was
    /// finished.
    pub fn nassau_qi_reader(&self, b: Bidegree) -> anyhow::Result<Option<NassauQiReader>> {
        let group_path = self.nassau_qi_group_path(b);
        let group = match zarrs::group::Group::open(self.store.clone(), &group_path) {
            Ok(g) => g,
            Err(_) => return Ok(None),
        };
        let attrs = group.attributes();
        if !attrs
            .get("finished")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return Ok(None);
        }
        let target_dim = attrs
            .get("target_dim")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let zero_mask_dim = attrs
            .get("zero_mask_dim")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let num_commands = attrs
            .get("num_commands")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let subalgebra_profile: Vec<u8> = attrs
            .get("subalgebra_profile")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_u64().map(|n| n as u8))
                    .collect()
            })
            .unwrap_or_default();

        let commands_array =
            zarrs::array::Array::open(self.store.clone(), &format!("{}/commands", group_path))?;

        Ok(Some(NassauQiReader {
            target_dim,
            zero_mask_dim,
            subalgebra_profile,
            commands_array,
            num_commands,
            consumed: 0,
            chunk_buf: Vec::new(),
            pos_in_chunk: 0,
            chunk_idx: 0,
        }))
    }
}

// --- ResQi reader ---

/// Streaming reader for a ResQi's `rows`, one chunk at a time.
pub struct ResQiReader {
    p: ValidPrime,
    source_dim: usize,
    target_dim: usize,
    image_dim: usize,
    pivots: Vec<isize>,
    rows_array: zarrs::array::Array<dyn ReadableWritableListableStorageTraits>,
    /// Current chunk's flat byte buffer.
    chunk_buf: Vec<u8>,
    /// Index of the next chunk to fetch.
    chunk_idx: u64,
    /// Position within `chunk_buf`, in rows (not bytes).
    pos_in_chunk: usize,
    /// Bytes per row, computed at construction.
    row_bytes: usize,
    /// Number of rows per chunk; cached on first chunk fetch.
    chunk_rows: usize,
}

impl ResQiReader {
    /// Dimension of the source of the inverted map, the length of each row.
    pub fn source_dimension(&self) -> usize {
        self.source_dim
    }

    /// Dimension of the target of the inverted map, the number of pivots.
    pub fn target_dimension(&self) -> usize {
        self.target_dim
    }

    /// Dimension of the image, the number of rows: one per column with a pivot.
    pub fn image_dimension(&self) -> usize {
        self.image_dim
    }

    /// The row of each target column, or a negative value for a column without a pivot.
    pub fn pivots(&self) -> &[isize] {
        &self.pivots
    }

    /// Read the next row into `dest`; `false` once every row has been read.
    pub fn next_row(&mut self, dest: &mut FpVector) -> anyhow::Result<bool> {
        if self.image_dim == 0 || self.row_bytes == 0 {
            return Ok(false);
        }
        let total_rows_consumed =
            (self.chunk_idx.saturating_sub(1) as usize) * self.chunk_rows + self.pos_in_chunk;
        if total_rows_consumed >= self.image_dim {
            return Ok(false);
        }
        if self.chunk_buf.is_empty() || self.pos_in_chunk * self.row_bytes >= self.chunk_buf.len() {
            // A missing chunk would otherwise read as the fill value: rows of zeros.
            let chunk: Vec<u8> = self
                .rows_array
                .retrieve_chunk_if_exists(&[self.chunk_idx, 0])
                .map_err(zarr_err)?
                .with_context(|| format!("ResQi rows chunk {} is missing", self.chunk_idx))?;
            if self.chunk_rows == 0 && self.row_bytes > 0 {
                self.chunk_rows = chunk.len() / self.row_bytes;
            }
            self.chunk_buf = chunk;
            self.pos_in_chunk = 0;
            self.chunk_idx += 1;
        }
        let start = self.pos_in_chunk * self.row_bytes;
        let end = start + self.row_bytes;
        dest.update_from_bytes(&mut &self.chunk_buf[start..end])?;
        self.pos_in_chunk += 1;
        Ok(true)
    }

    /// Apply this quasi-inverse to `inputs`, adding the results into `results`.
    pub fn apply<T, S>(mut self, results: &mut [T], inputs: &[S]) -> anyhow::Result<()>
    where
        for<'a> &'a mut T: Into<FpSliceMut<'a>>,
        for<'a> &'a S: Into<FpSlice<'a>>,
    {
        use itertools::Itertools;
        assert_eq!(results.len(), inputs.len());
        let mut row = FpVector::new(self.p, self.source_dim);
        let pivots = self.pivots.clone();
        for (i, &r) in pivots.iter().enumerate() {
            if r < 0 {
                continue;
            }
            let got = self.next_row(&mut row)?;
            anyhow::ensure!(got, "ResQi truncated: expected row for pivot {i}");
            for (input, result) in inputs.iter().zip_eq(results.iter_mut()) {
                result.into().add(row.as_slice(), input.into().entry(i));
            }
        }
        Ok(())
    }

    /// Read the whole [`QuasiInverse`] into memory; [`Self::apply`] streams it instead.
    pub fn into_quasi_inverse(mut self) -> anyhow::Result<QuasiInverse> {
        let mut rows: Vec<FpVector> = Vec::with_capacity(self.image_dim);
        for _ in 0..self.image_dim {
            let mut row = FpVector::new(self.p, self.source_dim);
            let got = self.next_row(&mut row)?;
            anyhow::ensure!(got, "ResQi truncated while materializing");
            rows.push(row);
        }
        let preimage = Matrix::from_rows(self.p, rows, self.source_dim);
        Ok(QuasiInverse::new(Some(self.pivots), preimage))
    }
}

// --- NassauQi writer/reader ---

/// One command of a NassauQi command stream.
#[derive(Debug, Clone)]
pub enum NassauCommand {
    /// Switch to a subalgebra signature; later lifts are in its masked basis.
    Signature(Vec<u16>),
    /// Ends the zero-signature section of a bidegree resolved through a stem rather than `t`.
    Fix,
    /// A pivot column with its lift and image, as raw `FpVector` limbs; their dimensions come from
    /// the current signature and `target_dim`.
    Pivot {
        col: u64,
        lift_bytes: Vec<u8>,
        image_bytes: Vec<u8>,
    },
}

const NASSAU_CODE_SIGNATURE: i64 = -2;
const NASSAU_CODE_FIX: i64 = -3;

/// Writer for a NassauQi, flushing every `CHUNK_NASSAU_COMMANDS` commands.
pub struct NassauQiWriter {
    store: ReadableWritableListableStorage,
    group_path: String,
    group_attrs: serde_json::Map<String, serde_json::Value>,
    commands_array: zarrs::array::Array<dyn ReadableWritableListableStorageTraits>,
    command_buf: Vec<Vec<u8>>,
    chunk_idx: u64,
    commands_written: u64,
}

impl NassauQiWriter {
    /// Buffer one encoded command, flushing once a chunk is full.
    fn add_command(&mut self, bytes: Vec<u8>) -> anyhow::Result<()> {
        self.command_buf.push(bytes);
        self.commands_written += 1;
        if self.command_buf.len() == CHUNK_NASSAU_COMMANDS as usize {
            self.flush_chunk(false)?;
        }
        Ok(())
    }

    /// Store the buffered commands as the next chunk, padded to the chunk shape if `pad`.
    fn flush_chunk(&mut self, pad: bool) -> anyhow::Result<()> {
        if self.command_buf.is_empty() {
            return Ok(());
        }
        let mut chunk = std::mem::take(&mut self.command_buf);
        if pad {
            // Pad to the chunk shape; the reader stops at `num_commands`.
            chunk.resize_with(CHUNK_NASSAU_COMMANDS as usize, Vec::new);
        }
        self.commands_array
            .store_chunk_opt(&[self.chunk_idx], chunk, &store_every_chunk())
            .map_err(zarr_err)?;
        self.chunk_idx += 1;
        self.command_buf.reserve(CHUNK_NASSAU_COMMANDS as usize);
        Ok(())
    }

    /// Append a [`NassauCommand::Signature`].
    pub fn write_signature(&mut self, signature: &[u16]) -> anyhow::Result<()> {
        let mut bytes = Vec::with_capacity(8 + signature.len() * 2);
        bytes.extend_from_slice(&NASSAU_CODE_SIGNATURE.to_le_bytes());
        for &x in signature {
            bytes.extend_from_slice(&x.to_le_bytes());
        }
        self.add_command(bytes)
    }

    /// Append a [`NassauCommand::Fix`].
    pub fn write_fix(&mut self) -> anyhow::Result<()> {
        let bytes = NASSAU_CODE_FIX.to_le_bytes().to_vec();
        self.add_command(bytes)
    }

    /// Append a [`NassauCommand::Pivot`] for column `col`.
    pub fn write_pivot(&mut self, col: u64, lift: FpSlice, image: FpSlice) -> anyhow::Result<()> {
        let lift_vec: FpVector = lift.to_owned();
        let mut lift_bytes = Vec::new();
        lift_vec.to_bytes(&mut lift_bytes)?;
        let image_vec: FpVector = image.to_owned();
        let mut image_bytes = Vec::new();
        image_vec.to_bytes(&mut image_bytes)?;

        let mut bytes = Vec::with_capacity(8 + 4 + lift_bytes.len() + image_bytes.len());
        bytes.extend_from_slice(&(col as i64).to_le_bytes());
        bytes.extend_from_slice(&(lift_bytes.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&lift_bytes);
        bytes.extend_from_slice(&image_bytes);
        self.add_command(bytes)
    }

    /// Flush the last chunk and commit `num_commands` and `finished`.
    pub fn finish(mut self) -> anyhow::Result<()> {
        let total = self.commands_written;
        self.flush_chunk(true)?;
        self.group_attrs.insert("num_commands".into(), total.into());
        self.group_attrs.insert("finished".into(), true.into());
        let group = GroupBuilder::new()
            .attributes(self.group_attrs)
            .build(self.store.clone(), &self.group_path)?;
        group.store_metadata()?;
        Ok(())
    }
}

/// Streaming reader for a NassauQi's commands, one chunk at a time.
pub struct NassauQiReader {
    target_dim: u64,
    zero_mask_dim: u64,
    subalgebra_profile: Vec<u8>,
    commands_array: zarrs::array::Array<dyn ReadableWritableListableStorageTraits>,
    num_commands: u64,
    consumed: u64,
    chunk_buf: Vec<Vec<u8>>,
    pos_in_chunk: usize,
    chunk_idx: u64,
}

impl NassauQiReader {
    /// Dimension of the target, in the original basis.
    pub fn target_dim(&self) -> u64 {
        self.target_dim
    }

    /// Dimension of the target masked under the zero signature.
    pub fn zero_mask_dim(&self) -> u64 {
        self.zero_mask_dim
    }

    /// Profile of the subalgebra the signatures are taken over.
    pub fn subalgebra_profile(&self) -> &[u8] {
        &self.subalgebra_profile
    }

    /// Decode one command, encoded as in `ext/SAVE-FORMAT.md`.
    fn parse(bytes: Vec<u8>) -> anyhow::Result<NassauCommand> {
        if bytes.len() < 8 {
            anyhow::bail!("NassauQi command too short: {} bytes", bytes.len());
        }
        let code = i64::from_le_bytes(bytes[..8].try_into().unwrap());
        match code {
            NASSAU_CODE_SIGNATURE => {
                let payload = &bytes[8..];
                anyhow::ensure!(
                    payload.len().is_multiple_of(2),
                    "NassauQi signature payload has odd length {}",
                    payload.len()
                );
                // The length was checked to be even, so `as_chunks` leaves no remainder.
                let sig: Vec<u16> = payload
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&[a, b]| u16::from_le_bytes([a, b]))
                    .collect();
                Ok(NassauCommand::Signature(sig))
            }
            NASSAU_CODE_FIX => Ok(NassauCommand::Fix),
            col if col >= 0 => {
                if bytes.len() < 12 {
                    anyhow::bail!("NassauQi pivot command too short: {} bytes", bytes.len());
                }
                let lift_byte_len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
                if bytes.len() < 12 + lift_byte_len {
                    anyhow::bail!(
                        "NassauQi pivot command lift truncated: have {}, need {}",
                        bytes.len(),
                        12 + lift_byte_len
                    );
                }
                let lift_bytes = bytes[12..12 + lift_byte_len].to_vec();
                let image_bytes = bytes[12 + lift_byte_len..].to_vec();
                Ok(NassauCommand::Pivot {
                    col: col as u64,
                    lift_bytes,
                    image_bytes,
                })
            }
            _ => anyhow::bail!("Unknown NassauQi command code: {code}"),
        }
    }
}

impl Iterator for NassauQiReader {
    type Item = anyhow::Result<NassauCommand>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.consumed >= self.num_commands {
            return None;
        }
        if self.pos_in_chunk >= self.chunk_buf.len() {
            match self
                .commands_array
                .retrieve_chunk_if_exists(&[self.chunk_idx])
            {
                Ok(Some(buf)) => self.chunk_buf = buf,
                Ok(None) => {
                    return Some(Err(anyhow::anyhow!(
                        "NassauQi commands chunk {} is missing",
                        self.chunk_idx
                    )));
                }
                Err(e) => return Some(Err(zarr_err(e))),
            }
            self.chunk_idx += 1;
            self.pos_in_chunk = 0;
        }
        let bytes = std::mem::take(&mut self.chunk_buf[self.pos_in_chunk]);
        self.pos_in_chunk += 1;
        self.consumed += 1;
        Some(Self::parse(bytes))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.num_commands - self.consumed) as usize;
        (remaining, Some(remaining))
    }
}

// --- SaveCoords, SaveKind, SaveDirectory ---

/// A save location with `N` coordinates `(n, s, [idx])`.
///
/// [`Bidegree`] is `SaveCoords<2>` and [`BidegreeGenerator`] is `SaveCoords<3>`, so a
/// per-bidegree method takes `impl SaveCoords<2>` and rejects a generator at compile time.
pub trait SaveCoords<const N: usize> {
    /// The coordinates `(n, s, [idx])` of this location.
    fn save_coords(&self) -> [i32; N];
}

impl SaveCoords<2> for Bidegree {
    fn save_coords(&self) -> [i32; 2] {
        [self.n(), self.s()]
    }
}

impl SaveCoords<3> for BidegreeGenerator {
    fn save_coords(&self) -> [i32; 3] {
        [self.n(), self.s(), self.idx() as i32]
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum SaveKind {
    Kernel,
    Differential,
    ResQi,
    AugmentationQi,
    SecondaryComposite,
    SecondaryIntermediate,
    SecondaryHomotopy,
    ChainMap,
    ChainHomotopy,
    NassauDifferential,
    NassauQi,
}

impl SaveKind {
    /// Name of this kind's array or group in the store.
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

    /// Whether this kind is indexed by `(n, s, idx)`, with `idx` enumerating basis elements.
    ///
    /// `SecondaryHomotopy` concatenates every generator of a bidegree, so it is 2D.
    fn is_indexed(self) -> bool {
        matches!(self, Self::SecondaryComposite | Self::SecondaryIntermediate)
    }
}

#[derive(Debug)]
pub enum SaveDirectory {
    None,
    Store(ZarrSaveStore),
}

impl SaveDirectory {
    /// Whether nothing is saved.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Whether there is a store to save to.
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// The store, if there is one.
    pub fn store(&self) -> Option<&ZarrSaveStore> {
        match self {
            Self::None => None,
            Self::Store(s) => Some(s),
        }
    }
}

impl TryFrom<Option<PathBuf>> for SaveDirectory {
    type Error = anyhow::Error;

    /// `None` is [`SaveDirectory::None`]; `Some(path)` opens or creates the store there.
    fn try_from(x: Option<PathBuf>) -> anyhow::Result<Self> {
        match x {
            None => Ok(Self::None),
            Some(p) => Ok(Self::Store(ZarrSaveStore::create(p)?)),
        }
    }
}
