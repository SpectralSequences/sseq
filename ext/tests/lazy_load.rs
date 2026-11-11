//! Reading a save lazily: header-only reads, and Nassau differentials loaded on first use.

use ext::save::{SaveKind, ZarrSaveStore};
use sseq::coordinates::{Bidegree, BidegreeGenerator};

/// `read_prefix` returns a prefix of what `read` returns, for every payload size, in 2D and 3D
/// arrays, and `None` where nothing was written.
#[test]
fn read_prefix_matches_read() {
    let tempdir = tempfile::TempDir::new().unwrap();
    let store = ZarrSaveStore::create(tempdir.path()).unwrap();

    // Several elements of one shard, at mixed sizes, plus one in a negative stem.
    let sizes = [16usize, 17, 100, 4096, 70_000];
    for (k, &size) in sizes.iter().enumerate() {
        let b = Bidegree::n_s(k as i32 - 2, 3);
        let data: Vec<u8> = (0..size).map(|i| (i * 31 + k) as u8).collect();
        store.write(SaveKind::ChainMap, b, &data).unwrap();
        let g = BidegreeGenerator::new(b, k);
        store
            .write(SaveKind::SecondaryIntermediate, g, &data)
            .unwrap();
    }
    for (k, &size) in sizes.iter().enumerate() {
        let b = Bidegree::n_s(k as i32 - 2, 3);
        let full = store.read(SaveKind::ChainMap, b).unwrap().unwrap();
        assert_eq!(full.len(), size);
        for len in [0, 1, 16, size] {
            let prefix = store.read_prefix(SaveKind::ChainMap, b, len).unwrap();
            assert_eq!(prefix.as_deref(), Some(&full[..len]));
        }
        let g = BidegreeGenerator::new(b, k);
        let prefix = store
            .read_prefix(SaveKind::SecondaryIntermediate, g, 16)
            .unwrap();
        assert_eq!(prefix.as_deref(), Some(&full[..16]));
    }

    // Unwritten: in a shard that exists, in one that doesn't, and in an array that doesn't.
    let empty = Bidegree::n_s(7, 3);
    assert_eq!(
        store.read_prefix(SaveKind::ChainMap, empty, 8).unwrap(),
        None
    );
    let far = Bidegree::n_s(300, 100);
    assert_eq!(store.read_prefix(SaveKind::ChainMap, far, 8).unwrap(), None);
    assert_eq!(store.read_prefix(SaveKind::Kernel, far, 8).unwrap(), None);

    // Asking for more than was written is an error, not a short read.
    let b = Bidegree::n_s(-2, 3);
    assert!(store.read_prefix(SaveKind::ChainMap, b, 17).is_err());
}

#[cfg(feature = "nassau")]
mod nassau {
    use ext::{
        chain_complex::{ChainComplex, FreeChainComplex},
        utils::construct_nassau,
    };
    use sseq::coordinates::Bidegree;

    const SAVED: Bidegree = Bidegree::n_s(30, 12);

    /// Every saved bidegree `(s, t)` with `s > 0`, which is where differentials are read from.
    fn saved_degrees(res: &impl ChainComplex) -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        for s in 1..=SAVED.s() {
            for t in res.min_degree()..=SAVED.n() + s {
                if res.has_computed_bidegree(Bidegree::s_t(s, t)) {
                    out.push((s, t));
                }
            }
        }
        out
    }

    /// A lazily loaded resolution reads nothing until asked, then agrees with the one that saved
    /// it and with an eagerly loaded one, and resolves further like a fresh resolution.
    #[test]
    fn lazy_matches_eager_and_extends() {
        let tempdir = tempfile::TempDir::new().unwrap();
        let saved = construct_nassau("S_2", Some(tempdir.path().into())).unwrap();
        saved.compute_through_stem(SAVED);

        let lazy = construct_nassau("S_2", Some(tempdir.path().into())).unwrap();
        lazy.compute_through_stem(SAVED);
        let mut eager = construct_nassau("S_2", Some(tempdir.path().into())).unwrap();
        eager.set_load_eagerly(true);
        eager.compute_through_stem(SAVED);

        assert_eq!(
            saved.graded_dimension_string(),
            lazy.graded_dimension_string()
        );
        let degrees = saved_degrees(&saved);
        assert!(degrees.len() > 100);
        for &(s, t) in &degrees {
            assert!(!lazy.differential(s).is_loaded(t), "({s}, {t}) read early");
            assert!(eager.differential(s).is_loaded(t), "({s}, {t}) not read");
        }
        for &(s, t) in &degrees {
            let want = saved.differential(s).outputs_in_degree(t).to_vec();
            assert_eq!(&*lazy.differential(s).outputs_in_degree(t), &want[..]);
            assert_eq!(&*eager.differential(s).outputs_in_degree(t), &want[..]);
            assert_eq!(
                lazy.module(s).number_of_gens_in_degree(t),
                saved.module(s).number_of_gens_in_degree(t)
            );
        }

        // Resolving past the save reads the lazily loaded differentials below it.
        let further = Bidegree::n_s(40, 16);
        let fresh = construct_nassau("S_2", None).unwrap();
        fresh.compute_through_stem(further);
        let resumed = construct_nassau("S_2", Some(tempdir.path().into())).unwrap();
        resumed.compute_through_stem(further);
        assert_eq!(
            fresh.graded_dimension_string(),
            resumed.graded_dimension_string()
        );
    }

    /// Under a budget far below the save, concurrent reads evict and read degrees again, always
    /// see the saved values, and keep the cache within its budget; a guard outlives its eviction;
    /// and resolving further gives what a fresh resolution does.
    #[test]
    fn budget_evicts_and_rereads() {
        const BUDGET: u64 = 256;
        let tempdir = tempfile::TempDir::new().unwrap();
        let saved = construct_nassau("S_2", Some(tempdir.path().into())).unwrap();
        saved.compute_through_stem(SAVED);
        let degrees = saved_degrees(&saved);
        let want = |s: i32, t: i32| saved.differential(s).outputs_in_degree(t).to_vec();

        let mut lazy = construct_nassau("S_2", Some(tempdir.path().into())).unwrap();
        lazy.set_load_budget(Some(BUDGET));
        lazy.compute_through_stem(SAVED);
        let cache = std::sync::Arc::clone(lazy.load_cache().unwrap());

        // Held from the start, while every other degree is read past it.
        let &(held_s, held_t) = degrees
            .iter()
            .max_by_key(|&&(s, t)| want(s, t).len())
            .unwrap();
        let held_d = lazy.differential(held_s);
        let held = held_d.outputs_in_degree(held_t);

        let largest = degrees
            .iter()
            .map(|&(s, t)| {
                want(s, t)
                    .iter()
                    .map(|v| 8 * fp::vector::FpVector::num_limbs(v.prime(), v.len()) as u64)
                    .sum::<u64>()
            })
            .max()
            .unwrap();
        std::thread::scope(|scope| {
            for offset in 0..4 {
                let (lazy, degrees, want, cache) = (&lazy, &degrees, &want, &cache);
                scope.spawn(move || {
                    for round in 0..3 {
                        for k in 0..degrees.len() {
                            let (s, t) = degrees[(k * (offset + 1) + round) % degrees.len()];
                            let d = lazy.differential(s);
                            let got = d.outputs_in_degree(t);
                            assert_eq!(&*got, &want(s, t)[..], "({s}, {t})");
                            drop(got);
                            assert!(cache.resident_bytes() <= BUDGET.max(4 * largest));
                        }
                    }
                });
            }
        });
        assert!(cache.evictions() > 0, "{cache:?}");
        assert!(cache.loads() > degrees.len() as u64, "{cache:?}");
        assert_eq!(&*held, &want(held_s, held_t)[..]);
        drop(held);

        let further = Bidegree::n_s(40, 16);
        let fresh = construct_nassau("S_2", None).unwrap();
        fresh.compute_through_stem(further);
        lazy.compute_through_stem(further);
        assert_eq!(
            fresh.graded_dimension_string(),
            lazy.graded_dimension_string()
        );
    }
}
