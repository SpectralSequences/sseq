//! Lazily read values of a free module homomorphism, and the byte budget that evicts them.

use std::{
    collections::VecDeque,
    fmt,
    ops::Deref,
    sync::{
        Arc, Mutex, OnceLock, RwLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use fp::vector::FpVector;

/// Reads a homomorphism's values on the generators of one degree, for a degree added with
/// [`MuFreeModuleHomomorphism::add_generators_lazily`](super::MuFreeModuleHomomorphism::add_generators_lazily).
///
/// It must return one vector per generator of the degree. It runs on the first read of the degree,
/// and again after each eviction, so it can only fail by panicking.
pub type OutputLoader = Arc<dyn Fn(i32) -> Vec<FpVector> + Send + Sync>;

/// A byte budget shared by the lazily read degrees of any number of homomorphisms.
///
/// Reading a degree that takes the total past the budget evicts others by the clock rule: a degree
/// read since the hand last passed it is spared once, so a degree read over and over stays. An
/// evicted degree is read again through its loader on next use. A guard keeps its degree alive
/// after eviction, so the memory in use can exceed the budget by what readers are holding.
pub struct OutputCache {
    budget: u64,
    clock: Mutex<Clock>,
    loads: AtomicU64,
    evictions: AtomicU64,
}

/// The degrees in memory, in the order the clock hand visits them, and their total size.
struct Clock {
    resident: u64,
    ring: VecDeque<Arc<CachedDegree>>,
}

impl OutputCache {
    /// A cache that evicts once the degrees it holds exceed `budget` bytes.
    pub fn new(budget: u64) -> Arc<Self> {
        Arc::new(Self {
            budget,
            clock: Mutex::new(Clock {
                resident: 0,
                ring: VecDeque::new(),
            }),
            loads: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        })
    }

    /// The budget, in bytes.
    pub fn budget(&self) -> u64 {
        self.budget
    }

    /// Bytes held by the degrees in memory, not counting evicted ones that guards keep alive.
    pub fn resident_bytes(&self) -> u64 {
        self.clock.lock().unwrap().resident
    }

    /// How many times a degree was read through its loader.
    pub fn loads(&self) -> u64 {
        self.loads.load(Ordering::Relaxed)
    }

    /// How many times a degree was evicted.
    pub fn evictions(&self) -> u64 {
        self.evictions.load(Ordering::Relaxed)
    }

    /// Account for `entry`, just read, and evict until the total fits the budget again.
    ///
    /// The victims are cleared after the clock's lock is released, since a reader takes a degree's
    /// own lock first and this lock second.
    fn admit(&self, entry: Arc<CachedDegree>) {
        self.loads.fetch_add(1, Ordering::Relaxed);
        let mut victims = Vec::new();
        {
            let mut clock = self.clock.lock().unwrap();
            clock.resident += entry.bytes.load(Ordering::Relaxed);
            clock.ring.push_back(entry);
            while clock.resident > self.budget && clock.ring.len() > 1 {
                let candidate = clock.ring.pop_front().unwrap();
                if candidate.referenced.swap(false, Ordering::Relaxed) {
                    clock.ring.push_back(candidate);
                } else {
                    clock.resident -= candidate.bytes.load(Ordering::Relaxed);
                    victims.push(candidate);
                }
            }
        }
        for victim in victims {
            *victim.values.write().unwrap() = None;
            self.evictions.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl fmt::Debug for OutputCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutputCache")
            .field("budget", &self.budget)
            .field("resident", &self.resident_bytes())
            .field("loads", &self.loads())
            .field("evictions", &self.evictions())
            .finish()
    }
}

/// One degree's values under an [`OutputCache`]: absent until read, and again after eviction.
pub(super) struct CachedDegree {
    values: RwLock<Option<Arc<[FpVector]>>>,
    /// Held while reading through the loader, so that concurrent first reads load once.
    loading: Mutex<()>,
    /// Set on every read; the clock hand clears it as it passes.
    referenced: AtomicBool,
    /// The size of `values` when last read.
    bytes: AtomicU64,
}

impl CachedDegree {
    fn new() -> Self {
        Self {
            values: RwLock::new(None),
            loading: Mutex::new(()),
            referenced: AtomicBool::new(false),
            bytes: AtomicU64::new(0),
        }
    }

    fn is_loaded(&self) -> bool {
        self.values.read().unwrap().is_some()
    }

    /// The values, read through `load` and admitted to `cache` if they are not in memory.
    fn get(
        self: &Arc<Self>,
        cache: &OutputCache,
        load: impl FnOnce() -> Vec<FpVector>,
    ) -> Arc<[FpVector]> {
        if let Some(values) = self.cached() {
            return values;
        }
        let guard = self.loading.lock().unwrap();
        if let Some(values) = self.cached() {
            return values;
        }
        let values: Arc<[FpVector]> = load().into();
        let bytes = values
            .iter()
            .map(|v| 8 * FpVector::num_limbs(v.prime(), v.len()) as u64)
            .sum();
        self.bytes.store(bytes, Ordering::Relaxed);
        self.referenced.store(true, Ordering::Relaxed);
        *self.values.write().unwrap() = Some(Arc::clone(&values));
        drop(guard);
        cache.admit(Arc::clone(self));
        values
    }

    /// The values if they are in memory, marking them as read.
    fn cached(&self) -> Option<Arc<[FpVector]>> {
        let values = self.values.read().unwrap().clone()?;
        self.referenced.store(true, Ordering::Relaxed);
        Some(values)
    }
}

/// Where a homomorphism keeps its values in one degree.
pub(super) enum Slot {
    /// Held for good once present: computed, read eagerly, or read lazily without a budget.
    Resident(OnceLock<Vec<FpVector>>),
    /// Read lazily under an [`OutputCache`], which may evict it.
    Cached(Arc<CachedDegree>),
}

impl Slot {
    /// A slot holding `values` for good.
    pub(super) fn resident(values: Vec<FpVector>) -> Self {
        Self::Resident(OnceLock::from(values))
    }

    /// An empty slot, read lazily, under `cache` if there is one.
    pub(super) fn lazy(cache: Option<&Arc<OutputCache>>) -> Self {
        match cache {
            None => Self::Resident(OnceLock::new()),
            Some(_) => Self::Cached(Arc::new(CachedDegree::new())),
        }
    }

    pub(super) fn is_loaded(&self) -> bool {
        match self {
            Self::Resident(values) => values.get().is_some(),
            Self::Cached(degree) => degree.is_loaded(),
        }
    }

    /// The values, read through `load` if they are not in memory.
    pub(super) fn get(
        &self,
        cache: Option<&OutputCache>,
        load: impl FnOnce() -> Vec<FpVector>,
    ) -> Outputs<'_> {
        match self {
            Self::Resident(values) => Outputs::Borrowed(values.get_or_init(load)),
            Self::Cached(degree) => {
                Outputs::Shared(degree.get(cache.expect("a cached degree without its cache"), load))
            }
        }
    }
}

/// A homomorphism's values on the generators of one degree.
///
/// Borrowed from the homomorphism, or, for a degree under an [`OutputCache`], shared with it, so
/// that eviction cannot free them while this is alive.
pub enum Outputs<'a> {
    Borrowed(&'a [FpVector]),
    Shared(Arc<[FpVector]>),
}

impl Deref for Outputs<'_> {
    type Target = [FpVector];

    fn deref(&self) -> &[FpVector] {
        match self {
            Self::Borrowed(values) => values,
            Self::Shared(values) => values,
        }
    }
}

impl fmt::Debug for Outputs<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

/// A homomorphism's value on one generator; see [`Outputs`].
pub struct Output<'a> {
    outputs: Outputs<'a>,
    index: usize,
}

impl<'a> Output<'a> {
    pub(super) fn new(outputs: Outputs<'a>, index: usize) -> Self {
        Self { outputs, index }
    }
}

impl Deref for Output<'_> {
    type Target = FpVector;

    fn deref(&self) -> &FpVector {
        &self.outputs[self.index]
    }
}

impl fmt::Debug for Output<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}
