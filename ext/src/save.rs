//! Two-tier zarr save system for resolutions, chain maps, and chain homotopies.
//!
//! On native targets the store is backed by a real zarr v3 store on the local filesystem via
//! [`zarrs::filesystem::FilesystemStore`]. On `wasm32-unknown-unknown` we swap in
//! [`zarrs::storage::store::MemoryStore`] instead — `zarrs_filesystem` transitively pulls in
//! `positioned-io::RandomAccessFile`, which is gated to `cfg(any(windows, unix))` and doesn't
//! compile for WASM. The WASM frontend has no filesystem to persist to anyway, so the memory
//! store just acts as a no-op sink that's dropped at session end; the same code paths
//! exercise it on both targets.
//!
//! # Layout
//!
//! One zarr v3 store with per-namespace subgroups for named homomorphisms and chain homotopies:
//!
//! ```text
//! save_dir.zarr/
//!   zarr.json                              # root group
//!
//!   # Main resolution — shard + stream tier at root
//!   {kind}/zarr.json + c/...               # 2D or 3D vlen-bytes shard
//!   qi/n{n}_s{s}/{kind}/                   # per-bidegree group, kind-specific sub-arrays
//!
//!   # Named ResolutionHomomorphism — one subgroup per name
//!   products/{name}/
//!     chain_map/, secondary_composite/, secondary_intermediate/, secondary_homotopy/
//!
//!   # Named ChainHomotopy — one subgroup per (left_name, right_name)
//!   homotopies/{left}__{right}/
//!     chain_homotopy/, secondary_composite/, secondary_intermediate/, secondary_homotopy/
//! ```
//!
//! # Two tiers
//!
//! **Shard tier.** Small payloads (differentials, kernels, chain maps, secondary data) use a
//! `vlen-bytes` sharded array per kind, with shard shape `[SHARD_N, SHARD_S(, SHARD_IDX)]`,
//! inner chunk shape `[1, 1(, 1)]`, and CRC32C over each shard (no zstd — the payloads are too
//! small to benefit).
//!
//! **Stream tier.** Large payloads ([`SaveKind::ResQi`], [`SaveKind::NassauQi`]) use
//! per-bidegree zarr *groups* with kind-specific sub-arrays:
//!
//! - `res_qi/` — group attributes hold the scalar dimensions; `pivots/` is a 1D `i64` array,
//!   `rows/` is a 2D `u8` array shaped `[image_dim, num_limbs * 8]`, chunked over rows.
//! - `nassau_qi/` — group attributes hold `target_dim`, `zero_mask_dim`, `subalgebra_profile`,
//!   `num_commands`, `finished`. `commands/` is a 1D vlen-bytes array with one element per
//!   [`NassauCommand`].
//!
//! Group attributes include a `finished` flag, which is the source of truth — readers treat the
//! data as missing if the writer was dropped before calling `finish()`.
//!
//! Subgroups share the same underlying [`FilesystemStore`] via `Arc` clone; only the `group`
//! prefix differs. Shard arrays are created lazily on first write so that subgroups don't
//! populate kinds they never use.
//!
//! # Coordinate system
//!
//! Shard arrays are indexed by `(n, s)`, matching `MultiDegree<2>::coords()` and generalizing to
//! `MultiDegree<N>` for `N > 2`. Stems can be negative (e.g. `RP^\infty_{-k}`, A-module shifts),
//! and zarr v3 has no native support for negative chunk indices, so we apply a fixed internal
//! offset: every caller-supplied `n` is shifted to `n - N_MIN` before becoming a zarr index.
//! `N_MIN` is intentionally generous (-1024) and never exposed in the public API; sparse zarr
//! arrays cost essentially nothing for the empty negative regions, so the overhead is purely in
//! `zarr.json` metadata.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::Context;
use dashmap::{DashMap, DashSet};
use fp::{
    matrix::{Matrix, QuasiInverse},
    prime::ValidPrime,
    vector::{FpSlice, FpSliceMut, FpVector},
};
use sseq::coordinates::{Bidegree, BidegreeGenerator};
#[cfg(not(target_arch = "wasm32"))]
use zarrs::array::codec::ZstdCodec;
// The concrete backing store depends on the target: native uses a `FilesystemStore` for real
// on-disk persistence; WASM uses an in-memory `MemoryStore` so `zarrs_filesystem` (which pulls
// in `positioned-io::RandomAccessFile`, gated to windows/unix) doesn't need to compile. The
// WASM frontend has no filesystem to persist to anyway, so the memory store just serves as a
// no-op sink and is dropped at session end.
#[cfg(not(target_arch = "wasm32"))]
use zarrs::filesystem::FilesystemStore;
#[cfg(target_arch = "wasm32")]
use zarrs::storage::store::MemoryStore;
use zarrs::{
    array::{ArrayBuilder, ArraySubset, CodecOptions, codec::Crc32cCodec, data_type},
    group::GroupBuilder,
    storage::{ReadableWritableListableStorage, ReadableWritableListableStorageTraits},
};

/// Most-negative stem the on-disk layout can store.
///
/// Hidden from callers; used internally to shift caller-supplied `n` values into the unsigned
/// zarr index space. See the module docs.
const N_MIN: i32 = -1024;

/// Number of slots in the n dimension.
///
/// Effective `n` range is `[N_MIN, N_MIN + N_SPAN)` = `[-1024, 3072)` — well beyond any
/// conceivable production stem.
const N_SPAN: u64 = 4096;

/// Number of slots in the s dimension. `s` is unsigned: `[0, S_SPAN)`.
const S_SPAN: u64 = 1024;

/// Number of slots reserved for the intra-bidegree index of indexed kinds.
const IDX_SPAN: u64 = 256;

/// Shard shape in the n dimension.
const SHARD_N: u64 = 8;

/// Shard shape in the s dimension.
const SHARD_S: u64 = 8;

/// Shard shape in the idx dimension.
const SHARD_IDX: u64 = 8;

/// zstd compression level for the stream tier.
#[cfg(not(target_arch = "wasm32"))]
const ZSTD_LEVEL: i32 = 19;

/// Convert a zarrs error into `anyhow::Error`.
///
/// On `wasm32-unknown-unknown` some zarrs error types contain `Arc<dyn TraitObj>` with only
/// `MaybeSend + MaybeSync` bounds (no-ops on wasm), so `anyhow::Error`'s `Send + Sync`
/// requirement rejects them via the blanket `From` impl. Formatting the error as a string
/// sidesteps the bound and works on both targets; the tradeoff is that the original error's
/// source chain is collapsed into its `Display` output.
fn zarr_err<E: std::fmt::Display>(e: E) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}

/// Build the bytes-to-bytes codec chain for stream-tier arrays.
///
/// On native targets this is `zstd(level=19) + crc32c`. On wasm32 the `zstd` feature is
/// disabled (its C build expects POSIX symbols the wasm libc shim doesn't expose) so we fall
/// back to `crc32c` alone — the memory-backed store on wasm is ephemeral, so skipping
/// compression is fine.
fn stream_tier_codecs() -> Vec<Arc<dyn zarrs::array::BytesToBytesCodecTraits>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        vec![
            Arc::new(ZstdCodec::new(ZSTD_LEVEL, false)),
            Arc::new(Crc32cCodec::new()),
        ]
    }
    #[cfg(target_arch = "wasm32")]
    {
        vec![Arc::new(Crc32cCodec::new())]
    }
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

pub struct ZarrSaveStore {
    /// Filesystem root of the underlying store. Same for all subgroups.
    path: PathBuf,
    /// Underlying zarr storage. Cheap to clone via `Arc`.
    ///
    /// On `wasm32-unknown-unknown` this wraps a trait object bounded only by
    /// `MaybeSend + MaybeSync` (no-ops on wasm), so the `Arc` isn't `Send`/`Sync` in Rust's
    /// eyes. The rest of `ext` requires chain complexes to be `Send + Sync`, and rippling
    /// that relaxation through the whole crate to accommodate the WASM frontend isn't worth
    /// it. Instead we force `Send + Sync` via the `unsafe impl`s below — sound because
    /// wasm32 is single-threaded, so the absent cross-thread guarantees are vacuously
    /// satisfied.
    store: ReadableWritableListableStorage,
    /// Group prefix applied to every operation.
    ///
    /// Empty for the root store; e.g. `"/products/foo"` or `"/homotopies/foo__bar"` for
    /// subgroups.
    group: String,
    /// Tracks shard-tier arrays already known to exist on disk for this `(store, group)`.
    ///
    /// Used to skip the `meta_path` check on subsequent writes.
    created: DashSet<SaveKind>,
    /// Per-kind write lock.
    ///
    /// Since zarrs 0.14, `Array::store_array_subset` is documented as requiring caller-side
    /// synchronization for "regions sharing any chunks" — the codec does a read-modify-write on
    /// the entire shard internally, so concurrent calls touching different inner chunks of the
    /// same shard race and lose writes. We serialize per kind, which is coarser than required
    /// (per shard would suffice) but simpler and entirely sufficient for our workload, since
    /// cross-kind parallelism dominates.
    write_locks: DashMap<SaveKind, Arc<Mutex<()>>>,
}

// See the doc comment on `ZarrSaveStore::store` for why this is sound on wasm. On native the
// trait object is already `Send + Sync`, so the cfg gate avoids a redundant-impl warning.
#[cfg(target_arch = "wasm32")]
unsafe impl Send for ZarrSaveStore {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for ZarrSaveStore {}

impl std::fmt::Debug for ZarrSaveStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZarrSaveStore")
            .field("path", &self.path)
            .field("group", &self.group)
            .finish_non_exhaustive()
    }
}

impl ZarrSaveStore {
    pub fn create(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = std::path::absolute(path.as_ref())
            .with_context(|| format!("Failed to resolve path: {:?}", path.as_ref()))?;

        #[cfg(not(target_arch = "wasm32"))]
        let store: ReadableWritableListableStorage = Arc::new(FilesystemStore::new(&path)?);
        #[cfg(target_arch = "wasm32")]
        let store: ReadableWritableListableStorage = Arc::new(MemoryStore::new());

        // Root group.
        //
        // On WASM the store is in-memory, so `zarr.json` never "exists" on disk; we
        // unconditionally build the root group (the memory store silently overwrites any
        // existing entry).
        #[cfg(not(target_arch = "wasm32"))]
        let needs_root_group = !path.join("zarr.json").exists();
        #[cfg(target_arch = "wasm32")]
        let needs_root_group = true;
        if needs_root_group {
            GroupBuilder::new()
                .build(store.clone(), "/")?
                .store_metadata()?;
        }

        Ok(Self {
            path,
            store,
            group: String::new(),
            created: DashSet::new(),
            write_locks: DashMap::new(),
        })
    }

    /// Bind this store to a specific algebra, catching accidental algebra / prime mismatches.
    ///
    /// On a fresh store (the root group's `algebra_magic` attribute is unset) this writes
    /// `algebra_magic`, `prime`, and `algebra_prefix` to the root group so a later load can
    /// verify them. On an already-bound store the stored magic is compared against
    /// `algebra_magic` and a mismatch returns an error citing both values.
    ///
    /// Callers should invoke this once per resolution setup — typically from
    /// `Resolution::new_with_save` — before any data is read or written. Subgroups
    /// (`products/…`, `homotopies/…`) share the same underlying store and therefore the same
    /// root attributes, so they inherit the check without a second call.
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

        // Unbound store: write the magic and related fields now. We rebuild the root group's
        // attributes map with the new entries merged in.
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

    /// Open a subgroup at `{self.group}/{name}`.
    ///
    /// Shares the same underlying store as `self`. The subgroup's `zarr.json` is created if
    /// missing.
    pub fn subgroup(&self, name: &str) -> anyhow::Result<Self> {
        let group = format!("{}/{}", self.group, name);
        let group_path = self.path.join(group.trim_start_matches('/'));
        if !group_path.join("zarr.json").exists() {
            // Ensure each path component exists as a group so that intermediate
            // levels (e.g. `products/`) are valid zarr groups, not just dirs.
            self.ensure_intermediate_groups(&group)?;
            GroupBuilder::new()
                .build(self.store.clone(), &group)?
                .store_metadata()?;
        }
        Ok(Self {
            path: self.path.clone(),
            store: self.store.clone(),
            group,
            created: DashSet::new(),
            write_locks: DashMap::new(),
        })
    }

    /// Walk every prefix of `group` and create a zarr group for any prefix that doesn't already
    /// have one (e.g. `/products/` before `/products/foo`).
    fn ensure_intermediate_groups(&self, group: &str) -> anyhow::Result<()> {
        // Split on '/', skipping the leading empty segment.
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

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Filesystem path corresponding to this store's group prefix.
    ///
    /// Returns `self.path` for the root store.
    fn group_fs_path(&self) -> PathBuf {
        if self.group.is_empty() {
            self.path.clone()
        } else {
            self.path.join(self.group.trim_start_matches('/'))
        }
    }

    fn shard_array_path(&self, kind: SaveKind) -> String {
        format!("{}/{}", self.group, kind.name())
    }

    /// Translate signed `(n, s, [idx])` coordinates into the unsigned zarr indices used for
    /// shard arrays.
    ///
    /// The first coordinate (`n`) is offset by `-N_MIN`; later coordinates are passed through
    /// as-is.
    fn shard_zarr_coords<const N: usize>(coords: [i32; N]) -> Vec<u64> {
        let mut out = Vec::with_capacity(N);
        out.push((coords[0] - N_MIN) as u64);
        for &c in &coords[1..] {
            out.push(c as u64);
        }
        out
    }

    /// Get-or-create the per-kind write lock for this store.
    ///
    /// See the comment on the `write_locks` field for why this exists.
    fn write_lock(&self, kind: SaveKind) -> Arc<Mutex<()>> {
        Arc::clone(
            self.write_locks
                .entry(kind)
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .value(),
        )
    }

    /// Lazily create the shard-tier array for `kind` under this group.
    fn ensure_shard_array(&self, kind: SaveKind) -> anyhow::Result<()> {
        if self.created.contains(&kind) {
            return Ok(());
        }
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

    /// Write a sharded byte payload.
    ///
    /// Streamed kinds ([`SaveKind::ResQi`], [`SaveKind::NassauQi`]) are not supported here — use
    /// [`Self::nassau_qi_writer`] / [`Self::write_res_qi`] instead, since they may be multi-GB
    /// and would OOM the in-memory `Vec`.
    ///
    /// `location` is anything that implements [`SaveCoords`] — [`Bidegree`] for 2D kinds and
    /// [`BidegreeGenerator`] for 3D kinds. Negative `n` is fine; the offset is handled
    /// internally.
    pub fn write<const N: usize>(
        &self,
        kind: SaveKind,
        location: impl SaveCoords<N>,
        data: &[u8],
    ) -> anyhow::Result<()> {
        debug_assert!(
            !matches!(kind, SaveKind::ResQi | SaveKind::NassauQi),
            "write() is only for sharded kinds, got {:?}",
            kind
        );
        self.ensure_shard_array(kind)?;
        let lock = self.write_lock(kind);
        let _guard = lock.lock().unwrap();
        let arr = zarrs::array::Array::open(self.store.clone(), &self.shard_array_path(kind))?;
        let zarr_coords = Self::shard_zarr_coords(location.save_coords());
        let subset = ArraySubset::new_with_start_shape(zarr_coords, vec![1; N])?;
        // Force sequential codec execution. Holding our std::sync::Mutex across
        // `store_array_subset` is unsafe with rayon, because zarrs's sharding codec uses rayon
        // internally — the worker that holds the mutex would join on inner tasks and could be
        // assigned a new task that also needs the mutex, deadlocking. Sequential execution
        // avoids the join entirely.
        arr.store_array_subset_opt(
            &subset,
            vec![data.to_vec()],
            &CodecOptions::default().with_concurrent_target(1),
        )
        .map_err(zarr_err)?;
        Ok(())
    }

    /// Read a sharded byte payload.
    ///
    /// Returns `None` if no data has been written. [`SaveKind::ResQi`] / [`SaveKind::NassauQi`]
    /// are not supported here; use the dedicated per-kind APIs.
    pub fn read<const N: usize>(
        &self,
        kind: SaveKind,
        location: impl SaveCoords<N>,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        debug_assert!(
            !matches!(kind, SaveKind::ResQi | SaveKind::NassauQi),
            "read() is only for sharded kinds, got {:?}",
            kind
        );
        let arr = match zarrs::array::Array::open(self.store.clone(), &self.shard_array_path(kind))
        {
            Ok(arr) => arr,
            Err(_) => return Ok(None),
        };
        let zarr_coords = Self::shard_zarr_coords(location.save_coords());
        let subset = ArraySubset::new_with_start_shape(zarr_coords, vec![1; N])?;
        let data: Vec<Vec<u8>> = arr.retrieve_array_subset(&subset).map_err(zarr_err)?;
        match data.into_iter().next() {
            Some(element) if !element.is_empty() => Ok(Some(element)),
            _ => Ok(None),
        }
    }

    /// Check if a sharded payload exists.
    pub fn exists<const N: usize>(&self, kind: SaveKind, location: impl SaveCoords<N>) -> bool {
        debug_assert!(
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
        debug_assert!(
            !matches!(kind, SaveKind::ResQi | SaveKind::NassauQi),
            "delete() is only for sharded kinds, got {:?}",
            kind
        );
        // Overwrite with fill value (empty vec). The array must already exist for delete to be
        // meaningful. Same locking + sequential codec story as `write`.
        let lock = self.write_lock(kind);
        let _guard = lock.lock().unwrap();
        let arr = zarrs::array::Array::open(self.store.clone(), &self.shard_array_path(kind))?;
        let zarr_coords = Self::shard_zarr_coords(location.save_coords());
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

    /// Write a [`QuasiInverse`] as a structured zarr group at `qi/n{n}_s{s}/res_qi`.
    ///
    /// Layout:
    ///
    /// - group attributes: `source_dim`, `target_dim`, `image_dim`, `finished`
    /// - `pivots`: 1D `i64`, shape `[target_dim]`
    /// - `rows`: 2D `u8`, shape `[image_dim, num_limbs * 8]`, chunked over rows
    ///
    /// `finished` is set to `true` only on success, so a writer that crashes mid-write leaves
    /// the group in a state the matching reader treats as missing.
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

        // Group with attributes (finished = false until we're done)
        let mut group_attrs = serde_json::Map::new();
        group_attrs.insert("source_dim".into(), (source_dim as u64).into());
        group_attrs.insert("target_dim".into(), (target_dim as u64).into());
        group_attrs.insert("image_dim".into(), (image_dim as u64).into());
        group_attrs.insert("finished".into(), false.into());
        let group = GroupBuilder::new()
            .attributes(group_attrs.clone())
            .build(self.store.clone(), &group_path)?;
        group.store_metadata()?;

        // pivots array: 1D i64, single chunk.
        let pivots_shape = std::cmp::max(target_dim as u64, 1);
        let pivots_array = ArrayBuilder::new(
            vec![pivots_shape],
            vec![pivots_shape],
            data_type::int64(),
            zarrs::array::FillValue::from(0i64),
        )
        .bytes_to_bytes_codecs(stream_tier_codecs())
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
            .store_chunk(&[0], pivots_data)
            .map_err(zarr_err)?;

        // rows array: 2D u8 [image_dim, row_bytes], chunked over rows.
        let rows_shape_0 = std::cmp::max(image_dim as u64, 1);
        let rows_shape_1 = std::cmp::max(row_bytes as u64, 1);
        let chunk_rows = std::cmp::min(CHUNK_RES_QI_ROWS, rows_shape_0);
        let rows_array = ArrayBuilder::new(
            vec![rows_shape_0, rows_shape_1],
            vec![chunk_rows, rows_shape_1],
            data_type::uint8(),
            zarrs::array::FillValue::from(0u8),
        )
        .bytes_to_bytes_codecs(stream_tier_codecs())
        .build(self.store.clone(), &format!("{}/rows", group_path))?;
        rows_array.store_metadata()?;

        if image_dim > 0 && row_bytes > 0 {
            // Write rows in chunks of `chunk_rows` rows. Pad the last chunk with zeros so the
            // chunk-shape constraint is satisfied; the reader knows `image_dim` and ignores the
            // padded rows.
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
                        .store_chunk(&[chunk_idx, 0], std::mem::take(&mut chunk_buf))
                        .map_err(zarr_err)?;
                    chunk_idx += 1;
                    chunk_buf.reserve(chunk_byte_len);
                }
            }
            if !chunk_buf.is_empty() {
                chunk_buf.resize(chunk_byte_len, 0);
                rows_array
                    .store_chunk(&[chunk_idx, 0], chunk_buf)
                    .map_err(zarr_err)?;
            }
        }

        // Mark the group finished.
        group_attrs.insert("finished".into(), true.into());
        let finished_group = GroupBuilder::new()
            .attributes(group_attrs)
            .build(self.store.clone(), &group_path)?;
        finished_group.store_metadata()?;
        Ok(())
    }

    /// Open a streaming reader for the ResQi at bidegree `b`.
    ///
    /// Returns `None` if no finished group exists. The reader fetches one chunk of rows at a
    /// time so peak memory is bounded by `CHUNK_RES_QI_ROWS * row_bytes`.
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
        let pivots_chunk: Vec<i64> = pivots_array.retrieve_chunk(&[0]).map_err(zarr_err)?;
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

    /// Open a writer for a NassauQi at bidegree `b`.
    ///
    /// The header (target/zero mask dimensions and the subalgebra profile bytes) goes into group
    /// attributes; the body of the bytecode becomes a 1D vlen-bytes `commands` array, with each
    /// element holding one command.
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
        .bytes_to_bytes_codecs(stream_tier_codecs())
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

    /// Open a streaming reader for the NassauQi at bidegree `b`.
    ///
    /// Returns `None` if no finished group exists. The reader yields one [`NassauCommand`] at a
    /// time and fetches one chunk of commands at a time, so peak memory is bounded by
    /// `CHUNK_NASSAU_COMMANDS * avg_command_bytes`.
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

/// Streaming reader for a structured ResQi.
///
/// Reads matrix rows from the underlying chunked 2D `rows` array on demand.
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
    pub fn source_dimension(&self) -> usize {
        self.source_dim
    }

    pub fn target_dimension(&self) -> usize {
        self.target_dim
    }

    pub fn image_dimension(&self) -> usize {
        self.image_dim
    }

    pub fn pivots(&self) -> &[isize] {
        &self.pivots
    }

    /// Read the next matrix row into `dest`.
    ///
    /// Returns whether a row was read. Rows are returned in the order they were written (one per
    /// non-trivial pivot column).
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
            // Refill from the next chunk.
            let chunk: Vec<u8> = self
                .rows_array
                .retrieve_chunk(&[self.chunk_idx, 0])
                .map_err(zarr_err)?;
            // Cache rows-per-chunk on first fetch.
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

    /// Apply this quasi-inverse to all the vectors in `inputs`, accumulating the results into
    /// `results`.
    ///
    /// Mirrors the semantics of the legacy `QuasiInverse::stream_quasi_inverse` but reads from
    /// the structured zarr layout.
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
            assert!(got, "ResQi truncated: expected row for pivot {i}");
            for (input, result) in inputs.iter().zip_eq(results.iter_mut()) {
                result.into().add(row.as_slice(), input.into().entry(i));
            }
        }
        Ok(())
    }

    /// Materialize the full [`QuasiInverse`] in memory.
    ///
    /// Used by the load-on-resume path; for streaming application, prefer [`Self::apply`].
    pub fn into_quasi_inverse(mut self) -> anyhow::Result<QuasiInverse> {
        let mut rows: Vec<FpVector> = Vec::with_capacity(self.image_dim);
        for _ in 0..self.image_dim {
            let mut row = FpVector::new(self.p, self.source_dim);
            let got = self.next_row(&mut row)?;
            assert!(got, "ResQi truncated while materializing");
            rows.push(row);
        }
        let preimage = Matrix::from_rows(self.p, rows, self.source_dim);
        Ok(QuasiInverse::new(Some(self.pivots), preimage))
    }
}

// --- NassauQi writer/reader ---

/// One command in a NassauQi command stream.
///
/// Mirrors the original bytecode but as discrete typed values instead of an inline `i64` magic
/// number stream.
#[derive(Debug, Clone)]
pub enum NassauCommand {
    /// Switch to a new subalgebra signature.
    ///
    /// Subsequent pivot lifts are expressed in the masked basis under this signature.
    Signature(Vec<u16>),
    /// "Differential fix" — emitted (at most once) at the end of the zero-signature section
    /// when the bidegree was resolved through stem rather than through `t`.
    ///
    /// Carries no payload.
    Fix,
    /// A pivot column with its lift and image.
    ///
    /// `lift_bytes` and `image_bytes` are raw `FpVector` limb serialisations; the caller knows
    /// the dimensions from the current signature state and `target_dim`.
    Pivot {
        col: u64,
        lift_bytes: Vec<u8>,
        image_bytes: Vec<u8>,
    },
}

const NASSAU_CODE_SIGNATURE: i64 = -2;
const NASSAU_CODE_FIX: i64 = -3;

/// Writer for a structured NassauQi.
///
/// Each call to a `write_*` method appends one command to the in-memory buffer; the buffer is
/// flushed to the underlying zarr `commands` array when `CHUNK_NASSAU_COMMANDS` commands have
/// accumulated. `finish()` flushes any remaining commands and commits the `num_commands` and
/// `finished` group attributes.
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
    fn add_command(&mut self, bytes: Vec<u8>) -> anyhow::Result<()> {
        self.command_buf.push(bytes);
        self.commands_written += 1;
        if self.command_buf.len() == CHUNK_NASSAU_COMMANDS as usize {
            self.flush_chunk(false)?;
        }
        Ok(())
    }

    fn flush_chunk(&mut self, pad: bool) -> anyhow::Result<()> {
        if self.command_buf.is_empty() {
            return Ok(());
        }
        let mut chunk = std::mem::take(&mut self.command_buf);
        if pad {
            // Pad to chunk shape with empty elements; the reader knows num_commands and ignores
            // them.
            chunk.resize_with(CHUNK_NASSAU_COMMANDS as usize, Vec::new);
        }
        self.commands_array
            .store_chunk(&[self.chunk_idx], chunk)
            .map_err(zarr_err)?;
        self.chunk_idx += 1;
        self.command_buf.reserve(CHUNK_NASSAU_COMMANDS as usize);
        Ok(())
    }

    pub fn write_signature(&mut self, signature: &[u16]) -> anyhow::Result<()> {
        let mut bytes = Vec::with_capacity(8 + signature.len() * 2);
        bytes.extend_from_slice(&NASSAU_CODE_SIGNATURE.to_le_bytes());
        for &x in signature {
            bytes.extend_from_slice(&x.to_le_bytes());
        }
        self.add_command(bytes)
    }

    pub fn write_fix(&mut self) -> anyhow::Result<()> {
        let bytes = NASSAU_CODE_FIX.to_le_bytes().to_vec();
        self.add_command(bytes)
    }

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

    /// Finalize: flush the partial last chunk and commit the `num_commands` and
    /// `finished = true` attributes.
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

/// Streaming reader for a structured NassauQi.
///
/// Yields one [`NassauCommand`] at a time, fetching chunks from the underlying `commands` array
/// on demand.
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
    pub fn target_dim(&self) -> u64 {
        self.target_dim
    }

    pub fn zero_mask_dim(&self) -> u64 {
        self.zero_mask_dim
    }

    pub fn subalgebra_profile(&self) -> &[u8] {
        &self.subalgebra_profile
    }

    fn parse(bytes: Vec<u8>) -> anyhow::Result<NassauCommand> {
        if bytes.len() < 8 {
            anyhow::bail!("NassauQi command too short: {} bytes", bytes.len());
        }
        let code = i64::from_le_bytes(bytes[..8].try_into().unwrap());
        match code {
            NASSAU_CODE_SIGNATURE => {
                let payload = &bytes[8..];
                let sig: Vec<u16> = payload
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
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
            match self.commands_array.retrieve_chunk(&[self.chunk_idx]) {
                Ok(buf) => self.chunk_buf = buf,
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

/// Anything that names a save location, parameterised by its dimension `N`.
///
/// [`Bidegree`] is `SaveCoords<2>` and [`BidegreeGenerator`] is `SaveCoords<3>`, so methods that
/// only make sense in one of those dimensions (e.g. the stream-tier writer / reader, which are
/// per-bidegree) can take `impl SaveCoords<2>` and refuse the other type at compile time. The
/// store reads `(n, s, [idx])` from this, shifts `n` by the internal `N_MIN` offset, and uses
/// the result as a zarr index. Implementing this trait for `MultiDegree<N>` /
/// `MultiDegreeGenerator<N>` would extend the same API to higher-`N` gradings without further
/// changes here.
pub trait SaveCoords<const N: usize> {
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

    /// Whether this kind uses 3D indexing `(n, s, idx)`.
    ///
    /// The third coordinate is an intra-lift enumeration over basis elements; multi-hom
    /// disambiguation is handled by the store's group prefix, not by an extra coordinate.
    /// `SecondaryHomotopy` is per-bidegree (data for all generators is concatenated), so it
    /// stays 2D.
    fn is_indexed(self) -> bool {
        matches!(self, Self::SecondaryComposite | Self::SecondaryIntermediate)
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
}

#[derive(Debug)]
pub enum SaveDirectory {
    None,
    Store(ZarrSaveStore),
}

impl SaveDirectory {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    pub fn store(&self) -> Option<&ZarrSaveStore> {
        match self {
            Self::None => None,
            Self::Store(s) => Some(s),
        }
    }
}

impl From<Option<PathBuf>> for SaveDirectory {
    fn from(x: Option<PathBuf>) -> Self {
        match x {
            None => Self::None,
            Some(p) => Self::Store(ZarrSaveStore::create(p).expect("Failed to create zarr store")),
        }
    }
}
