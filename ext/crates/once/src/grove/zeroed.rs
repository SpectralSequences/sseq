//! The zeroed groves: maps from every index to an element that is zero until touched.

use std::num::NonZero;

use super::block::Block;
use crate::std_or_loom::{
    Zeroable,
    sync::atomic::{AtomicU32, AtomicUsize, Ordering},
};

const MAX_NUM_BLOCKS: usize = 32;

/// A map from every index to an `E`, zero until touched, with pinned elements and geometrically
/// growing capacity.
///
/// An element exists from the moment the block holding it is allocated, as the value whose bytes
/// are all zero. [`Self::entry`] touches an index, allocating its block if needed, so threads that
/// race to touch the same index all get the same element and nothing is created twice. Elements are
/// then filled through their own shared methods: a [`Grove`] is a `ZeroedGrove` of [`WriteOnce`]
/// cells, and a trie node holds its children in a [`ZeroedTwoEndedGrove`] of nodes.
///
/// [`Grove`]: super::Grove
/// [`WriteOnce`]: crate::write_once::WriteOnce
///
/// # Implementation Details
///
/// `ZeroedGrove` uses a series of fixed-size blocks to store elements. Each block's size is a power
/// of 2, and the block number determines its size. This approach allows for efficient memory usage
/// while still supporting large indices without allocating memory for all intermediate elements. We
/// use distinct lazily-allocated blocks to avoid any reallocation, which would invalidate pointers
/// held by other threads.
///
/// Allocating a block makes every element in it readable, so [`Self::get_entry`] and
/// [`Self::entries`] can also return elements that were never touched, depending on where the block
/// boundaries fall. Those elements are zero, which is why this type is not public: the wrappers
/// read a zero element as empty, so the block layout never shows through them.
// `repr(C)` so that an empty trie node reads as either variant under loom; see `Node`'s `Zeroable`
// impl.
#[cfg_attr(loom, repr(C))]
#[cfg_attr(not(loom), derive(Zeroable), zeroable(bound = ""))]
pub(crate) struct ZeroedGrove<E> {
    blocks: [Block<E>; MAX_NUM_BLOCKS],
    /// One past the highest index touched.
    max: AtomicUsize,
}

impl<E: Zeroable> ZeroedGrove<E> {
    /// Creates a `ZeroedGrove` that is zero everywhere.
    pub fn new() -> Self {
        Self {
            blocks: std::array::from_fn(|_| Block::new()),
            max: AtomicUsize::new(0),
        }
    }

    /// Finds the block and offset within the block for the given index.
    fn locate(&self, index: usize) -> (usize, usize) {
        let block_num = (usize::BITS - 1 - (index + 1).leading_zeros()) as usize;
        let block_offset = (index + 1) - (1 << block_num);
        (block_num, block_offset)
    }

    /// Ensures that the specified block is initialized.
    fn ensure_init(&self, block_num: usize) {
        // Safety: `Block::init` is only ever called through this method, and every block has a
        // well-defined `block_num`, and therefore a well-defined size.
        unsafe { self.blocks[block_num].init(NonZero::new(1 << block_num).unwrap()) };
    }

    /// The element at `index`, allocating its block if needed and counting it in [`Self::len`].
    pub fn entry(&self, index: usize) -> &E {
        let (block_num, block_offset) = self.locate(index);
        self.ensure_init(block_num);
        self.max.fetch_max(index + 1, Ordering::Release);
        // SAFETY: we just initialized the block, and `locate` only returns valid indices.
        unsafe { self.blocks[block_num].entry(block_offset) }
    }

    /// The element at `index`, or `None` if its block was never allocated.
    ///
    /// Every element of an allocated block is returned, including the zero ones nothing touched.
    pub fn get_entry(&self, index: usize) -> Option<&E> {
        let (block_num, block_offset) = self.locate(index);
        let block = &self.blocks[block_num];
        // SAFETY: we just observed that the block is initialized, and `locate` only returns valid
        // indices.
        block
            .is_init()
            .then(|| unsafe { block.entry(block_offset) })
    }

    /// The element at `index`, or `None` if its block was never allocated.
    pub fn get_entry_mut(&mut self, index: usize) -> Option<&mut E> {
        let (block_num, block_offset) = self.locate(index);
        self.blocks[block_num].get_mut(block_offset)
    }

    /// The element at `index`, without checking that its block is allocated.
    ///
    /// # Safety
    ///
    /// This thread must have observed the block holding `index` as allocated, for instance by
    /// touching `index` or by reading a value that was written there.
    pub unsafe fn get_entry_unchecked(&self, index: usize) -> &E {
        let (block_num, block_offset) = self.locate(index);
        // We know that we already observed the block to be initialized, so we can get away with a
        // relaxed load.
        let data_ptr = self.blocks[block_num].data().load(Ordering::Relaxed);
        // SAFETY: the block is initialized by assumption, and `locate` only returns valid indices.
        unsafe { &*data_ptr.add(block_offset) }
    }

    /// One past the highest index touched.
    ///
    /// Since this is a wait-free data structure, a concurrent reader may see a stale value, and it
    /// may count an index whose element another thread is still filling.
    pub fn len(&self) -> usize {
        self.max.load(Ordering::Acquire)
    }

    /// The elements below [`Self::len`] in allocated blocks, with their indices.
    pub fn entries(&self) -> impl Iterator<Item = (usize, &E)> {
        (0..self.len()).filter_map(move |i| self.get_entry(i).map(|e| (i, e)))
    }

    /// The elements below [`Self::len`] in allocated blocks, with their indices, mutably.
    pub fn entries_mut(&mut self) -> impl Iterator<Item = (usize, &mut E)> {
        let ptr = self as *mut Self;
        (0..self.len()).filter_map(move |i| {
            // SAFETY: Each index maps to a unique (block, offset) pair, so the returned `&mut E`
            // references are non-aliasing. We have `&mut self` so no other references exist.
            unsafe { (*ptr).get_entry_mut(i).map(|e| (i, e)) }
        })
    }
}

impl<E: Zeroable> Default for ZeroedGrove<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: std::fmt::Debug + Zeroable> std::fmt::Debug for ZeroedGrove<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.entries()).finish()
    }
}

/// A [`ZeroedGrove`] indexed by every `i32`, keeping the range of indices touched.
///
/// It holds one `ZeroedGrove` for non-negative indices and one for negative indices. Every field is
/// zero when nothing has been touched, so an all-zero `ZeroedTwoEndedGrove` is empty; this is what
/// lets a trie node hold its children in place.
// `repr(C)` so that an empty trie node reads as either variant under loom; see `Node`'s `Zeroable`
// impl.
#[cfg_attr(loom, repr(C))]
#[cfg_attr(not(loom), derive(Zeroable), zeroable(bound = ""))]
pub(crate) struct ZeroedTwoEndedGrove<E> {
    non_neg: ZeroedGrove<E>,
    neg: ZeroedGrove<E>,
    /// The least index touched, encoded by [`MIN_KEY`].
    min: AtomicU32,
    /// One past the greatest index touched, encoded by [`MAX_KEY`].
    max: AtomicU32,
}

/// `min` is stored as `min ^ MIN_KEY`. Its empty value `i32::MAX` becomes zero, and a smaller `min`
/// becomes a larger `u32`, so lowering `min` is a `fetch_max`.
const MIN_KEY: i32 = i32::MAX;

/// `max` is stored as `max ^ MAX_KEY`, which flips the sign bit. Its empty value `i32::MIN` becomes
/// zero, and the order of `i32` carries over to `u32`.
const MAX_KEY: i32 = i32::MIN;

/// Encode a bound with `key`, as [`MIN_KEY`] and [`MAX_KEY`] describe.
fn encode(bound: i32, key: i32) -> u32 {
    (bound ^ key) as u32
}

/// Decode a bound stored with `key`.
fn decode(stored: u32, key: i32) -> i32 {
    stored as i32 ^ key
}

impl<E: Zeroable> ZeroedTwoEndedGrove<E> {
    /// Creates a `ZeroedTwoEndedGrove` that is zero everywhere.
    pub fn new() -> Self {
        Self {
            non_neg: ZeroedGrove::new(),
            neg: ZeroedGrove::new(),
            min: AtomicU32::new(0),
            max: AtomicU32::new(0),
        }
    }

    /// The element at `idx`, allocating its block if needed and counting it in [`Self::range`].
    pub fn entry(&self, idx: i32) -> &E {
        let entry = if idx >= 0 {
            self.non_neg.entry(idx as usize)
        } else {
            self.neg.entry((-idx) as usize)
        };
        self.max
            .fetch_max(encode(idx + 1, MAX_KEY), Ordering::Relaxed);
        self.min.fetch_max(encode(idx, MIN_KEY), Ordering::Release);
        entry
    }

    /// The element at `idx`, or `None` if its block was never allocated.
    ///
    /// See [`ZeroedGrove::get_entry`].
    pub fn get_entry(&self, idx: i32) -> Option<&E> {
        if idx >= 0 {
            self.non_neg.get_entry(idx as usize)
        } else {
            self.neg.get_entry((-idx) as usize)
        }
    }

    /// The element at `idx`, or `None` if its block was never allocated.
    pub fn get_entry_mut(&mut self, idx: i32) -> Option<&mut E> {
        if idx >= 0 {
            self.non_neg.get_entry_mut(idx as usize)
        } else {
            self.neg.get_entry_mut((-idx) as usize)
        }
    }

    /// The range of indices touched.
    ///
    /// Indices in the range need not have been touched. Outside the range, none have been, unless
    /// another thread is touching one concurrently.
    pub fn range(&self) -> std::ops::Range<i32> {
        // We use a Relaxed max load because every touch ends with a release-store of the min. We
        // have a chain of happens-before relationships:
        // load max <- acquire-load min <- release-store min <- store max
        decode(self.min.load(Ordering::Acquire), MIN_KEY)
            ..decode(self.max.load(Ordering::Relaxed), MAX_KEY)
    }

    /// The elements in allocated blocks within each half's length, with their indices.
    pub fn entries(&self) -> impl Iterator<Item = (i32, &E)> {
        let non_negs = self.non_neg.entries().map(|(i, e)| (i as i32, e));
        let negs = self.neg.entries().map(|(i, e)| (-(i as i32), e));
        non_negs.chain(negs)
    }

    /// The elements in allocated blocks within each half's length, with their indices, mutably.
    pub fn entries_mut(&mut self) -> impl Iterator<Item = (i32, &mut E)> {
        let non_negs = self.non_neg.entries_mut().map(|(i, e)| (i as i32, e));
        let negs = self.neg.entries_mut().map(|(i, e)| (-(i as i32), e));
        non_negs.chain(negs)
    }
}

impl<E: Zeroable> Default for ZeroedTwoEndedGrove<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: std::fmt::Debug + Zeroable> std::fmt::Debug for ZeroedTwoEndedGrove<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.entries()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::write_once::WriteOnce;

    #[test]
    fn test_locate() {
        let vec = ZeroedGrove::<WriteOnce<i32>>::new();
        assert_eq!(vec.locate(0), (0, 0));
        assert_eq!(vec.locate(1), (1, 0));
        assert_eq!(vec.locate(2), (1, 1));
        assert_eq!(vec.locate(3), (2, 0));
        assert_eq!(vec.locate(4), (2, 1));
        assert_eq!(vec.locate(5), (2, 2));
        assert_eq!(vec.locate(6), (2, 3));
        assert_eq!(vec.locate(7), (3, 0));
        assert_eq!(vec.locate(8), (3, 1));
        assert_eq!(vec.locate(9), (3, 2));
        assert_eq!(vec.locate(10), (3, 3));
        assert_eq!(vec.locate(11), (3, 4));
        assert_eq!(vec.locate(12), (3, 5));
        assert_eq!(vec.locate(13), (3, 6));
        assert_eq!(vec.locate(14), (3, 7));
        assert_eq!(vec.locate(15), (4, 0));
        assert_eq!(vec.locate(16), (4, 1));
        assert_eq!(vec.locate(17), (4, 2));
        // This should be good enough
    }

    /// The empty bounds encode to zero, and the encodings order bounds as `fetch_max` needs.
    #[test]
    fn test_bound_encoding() {
        assert_eq!(encode(i32::MAX, MIN_KEY), 0);
        assert_eq!(encode(i32::MIN, MAX_KEY), 0);
        let bounds = [
            i32::MIN,
            i32::MIN + 1,
            -2,
            -1,
            0,
            1,
            2,
            i32::MAX - 1,
            i32::MAX,
        ];
        for pair in bounds.windows(2) {
            let (lo, hi) = (pair[0], pair[1]);
            assert!(encode(lo, MIN_KEY) > encode(hi, MIN_KEY));
            assert!(encode(lo, MAX_KEY) < encode(hi, MAX_KEY));
        }
        for &b in &bounds {
            assert_eq!(decode(encode(b, MIN_KEY), MIN_KEY), b);
            assert_eq!(decode(encode(b, MAX_KEY), MAX_KEY), b);
        }
        assert!(crate::TwoEndedGrove::<i32>::new().range().is_empty());
    }

    /// Threads that race to touch the same indices all get the same elements.
    #[cfg(not(loom))]
    #[test]
    fn test_zeroed_grove_concurrent_entry() {
        use std::{sync::atomic::AtomicUsize, thread};

        let grove = ZeroedGrove::<AtomicUsize>::new();
        let num_threads = crate::test_utils::num_threads();
        let len = crate::test_utils::values_per_thread();
        thread::scope(|s| {
            for _ in 0..num_threads {
                s.spawn(|| {
                    for i in 0..len {
                        grove.entry(i).fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
        });
        assert_eq!(grove.len(), len);
        for i in 0..len {
            assert_eq!(
                grove.get_entry(i).unwrap().load(Ordering::Relaxed),
                num_threads
            );
        }
    }
}
