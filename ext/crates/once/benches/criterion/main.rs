use std::hint::black_box;

use criterion::{
    BenchmarkGroup, Criterion, criterion_group, criterion_main, measurement::WallTime,
};
use once::{MultiIndexed, OnceBiVec, TwoEndedGrove, multiindexed::kdtrie::KdTrie};
use pprof::criterion::{Output, PProfProfiler};
use rand::{rng, seq::SliceRandom};

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(std::time::Duration::from_secs(5)).with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = run_benchmarks
}
criterion_main!(benches);

fn run_benchmarks(c: &mut Criterion) {
    run_insert_benchmarks(c, &|i| i as i32);
    run_insert_benchmarks(c, &|i| [i; 1000]);
    run_lookup_benchmarks(c, &|i| i as i32);
    run_lookup_benchmarks(c, &|i| [i; 1000]);
    run_iter_benchmarks(c, &|i| i as i32);
    run_iter_benchmarks(c, &|i| [i; 1000]);
}

/// A trait that matches OnceBiVec's semantics for benchmarking
trait Benchable<const K: usize, T> {
    fn name() -> &'static str;

    /// Create a new container with the given minimum bounds
    fn new(min: [i32; K]) -> Self;

    /// Push a value at the given coordinates, filling in any gaps with cloned values
    /// The coordinates must be pushed in order within each dimension
    fn push_checked(&self, coords: [i32; K], value: T);

    /// Get a value at the given coordinates if it exists and is within bounds
    fn get(&self, coords: [i32; K]) -> Option<&T>;

    /// Iterate over all items in the container
    fn iter<'a>(&'a self) -> impl Iterator<Item = ([i32; K], &'a T)>
    where
        T: 'a;
}

impl<T, const K: usize> Benchable<K, T> for MultiIndexed<K, T> {
    fn name() -> &'static str {
        "multi_graded"
    }

    fn new(_min: [i32; K]) -> Self {
        Self::new()
    }

    fn push_checked(&self, coords: [i32; K], value: T) {
        self.insert(coords, value);
    }

    fn get(&self, coords: [i32; K]) -> Option<&T> {
        self.get(coords)
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = ([i32; K], &'a T)>
    where
        T: 'a,
    {
        self.iter()
    }
}

impl<T> Benchable<1, T> for TwoEndedGrove<T> {
    fn name() -> &'static str {
        "two_ended_grove"
    }

    fn new(_min: [i32; 1]) -> Self {
        Self::new()
    }

    fn push_checked(&self, coords: [i32; 1], value: T) {
        self.insert(coords[0], value);
    }

    fn get(&self, coords: [i32; 1]) -> Option<&T> {
        self.get(coords[0])
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = ([i32; 1], &'a T)>
    where
        T: 'a,
    {
        self.enumerate().map(|(k, v)| ([k], v))
    }
}

impl<const K: usize, T> Benchable<K, T> for KdTrie<T> {
    fn name() -> &'static str {
        "kd_trie"
    }

    fn new(_min: [i32; K]) -> Self {
        Self::new(K)
    }

    fn push_checked(&self, coords: [i32; K], value: T) {
        self.insert(&coords, value);
    }

    fn get(&self, coords: [i32; K]) -> Option<&T> {
        self.get(&coords)
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = ([i32; K], &'a T)>
    where
        T: 'a,
    {
        self.iter().map(|(coords, value)| {
            (
                coords
                    .try_into()
                    .expect("coordinates are the right length by construction"),
                value,
            )
        })
    }
}

mod benchable_impl;

pub fn get_nth_diagonal<const K: usize>(n: usize) -> Vec<[i32; K]> {
    let mut result = Vec::new();
    let mut tuple = vec![0; K];

    // Generate all tuples where the sum of coordinates equals n
    generate_tuples::<K>(&mut tuple, 0, n, &mut result);

    result
}

/// Helper function to recursively generate the tuples
fn generate_tuples<const K: usize>(
    tuple: &mut Vec<i32>,
    index: usize,
    sum: usize,
    result: &mut Vec<[i32; K]>,
) {
    if index == K - 1 {
        // The last element gets whatever is left to reach the sum
        tuple[index] = sum as i32;
        result.push(tuple.clone().try_into().unwrap()); // Convert to [i32; K]
        return;
    }

    for i in 0..=sum {
        tuple[index] = i as i32;
        generate_tuples::<K>(tuple, index + 1, sum - i, result);
    }
}

fn get_n_coords<const K: usize>(n: usize, min: [i32; K]) -> Vec<[i32; K]> {
    (0..)
        .flat_map(get_nth_diagonal)
        .map(|mut v| {
            for (xi, mi) in v.iter_mut().zip(min.iter()) {
                *xi += mi;
            }
            v
        })
        .take(n)
        .collect()
}

const NUM_ELEMENTS: usize = 1 << 12;

// Insertion benchmarks

fn run_insert_benchmarks<T>(c: &mut Criterion, make_value: &dyn Fn(usize) -> T) {
    // Dim 1
    let mut g = c.benchmark_group(format!("insert_dim1_{}", std::any::type_name::<T>()));
    bench_insert_k::<1, _, OnceBiVec<_>>(&mut g, [0], make_value);
    bench_insert_k::<1, _, TwoEndedGrove<_>>(&mut g, [0], make_value);
    bench_insert_k::<1, _, MultiIndexed<1, _>>(&mut g, [0], make_value);
    bench_insert_k::<1, _, KdTrie<_>>(&mut g, [0], make_value);
    g.finish();

    run_insert_benchmark::<2, _, OnceBiVec<OnceBiVec<_>>, MultiIndexed<2, _>, KdTrie<_>>(
        c,
        [0, 0],
        make_value,
    );
    run_insert_benchmark::<3, _, OnceBiVec<OnceBiVec<OnceBiVec<_>>>, MultiIndexed<3, _>, KdTrie<_>>(
        c,
        [0, 0, 0],
        make_value,
    );
    run_insert_benchmark::<
        4,
        _,
        OnceBiVec<OnceBiVec<OnceBiVec<OnceBiVec<_>>>>,
        MultiIndexed<4, _>,
        KdTrie<_>,
    >(c, [0, 0, 0, 0], make_value);
    run_insert_benchmark::<
        5,
        _,
        OnceBiVec<OnceBiVec<OnceBiVec<OnceBiVec<OnceBiVec<_>>>>>,
        MultiIndexed<5, _>,
        KdTrie<_>,
    >(c, [0, 0, 0, 0, 0], make_value);

    let mut g = c.benchmark_group(format!("insert_dim6_{}", std::any::type_name::<T>()));
    bench_insert_k::<6, _, MultiIndexed<6, _>>(&mut g, [0, 0, 0, 0, 0, 0], make_value);
    bench_insert_k::<6, _, KdTrie<_>>(&mut g, [0, 0, 0, 0, 0, 0], make_value);
    g.finish();
}

fn run_insert_benchmark<
    const K: usize,
    T,
    B1: Benchable<K, T>,
    B2: Benchable<K, T>,
    B3: Benchable<K, T>,
>(
    c: &mut Criterion,
    min: [i32; K],
    make_value: &dyn Fn(usize) -> T,
) {
    let mut g = c.benchmark_group(format!("insert_dim{K}_{}", std::any::type_name::<T>()));
    bench_insert_k::<K, _, B1>(&mut g, min, make_value);
    bench_insert_k::<K, _, B2>(&mut g, min, make_value);
    bench_insert_k::<K, _, B3>(&mut g, min, make_value);
    g.finish();
}

// Benchmark insertion for different dimensions
fn bench_insert_k<const K: usize, T, B: Benchable<K, T>>(
    c: &mut BenchmarkGroup<'_, WallTime>,
    min: [i32; K],
    make_value: impl Fn(usize) -> T,
) {
    let coords: Vec<[i32; K]> = get_n_coords(NUM_ELEMENTS, min);

    c.bench_function(
        format!("{}_insert_k{}_{}", B::name(), K, std::any::type_name::<T>()),
        |b| {
            b.iter_batched(
                || B::new(min),
                |vec| {
                    for (i, coord) in coords.iter().enumerate() {
                        vec.push_checked(*coord, make_value(i));
                    }
                },
                criterion::BatchSize::SmallInput,
            )
        },
    );
}

// Lookup benchmarks

fn run_lookup_benchmarks<T>(c: &mut Criterion, make_value: &dyn Fn(usize) -> T) {
    // Dim 1
    let mut g = c.benchmark_group(format!("lookup_dim1_{}", std::any::type_name::<T>()));
    bench_lookup_k::<1, _, OnceBiVec<_>>(&mut g, [0], make_value);
    bench_lookup_k::<1, _, TwoEndedGrove<_>>(&mut g, [0], make_value);
    bench_lookup_k::<1, _, MultiIndexed<1, _>>(&mut g, [0], make_value);
    bench_lookup_k::<1, _, KdTrie<_>>(&mut g, [0], make_value);
    g.finish();

    run_lookup_benchmark::<2, _, OnceBiVec<OnceBiVec<_>>, MultiIndexed<2, _>, KdTrie<_>>(
        c,
        [0, 0],
        make_value,
    );
    run_lookup_benchmark::<3, _, OnceBiVec<OnceBiVec<OnceBiVec<_>>>, MultiIndexed<3, _>, KdTrie<_>>(
        c,
        [0, 0, 0],
        make_value,
    );
    run_lookup_benchmark::<
        4,
        _,
        OnceBiVec<OnceBiVec<OnceBiVec<OnceBiVec<_>>>>,
        MultiIndexed<4, _>,
        KdTrie<_>,
    >(c, [0, 0, 0, 0], make_value);
    run_lookup_benchmark::<
        5,
        _,
        OnceBiVec<OnceBiVec<OnceBiVec<OnceBiVec<OnceBiVec<_>>>>>,
        MultiIndexed<5, _>,
        KdTrie<_>,
    >(c, [0, 0, 0, 0, 0], make_value);

    let mut g = c.benchmark_group(format!("lookup_dim6_{}", std::any::type_name::<T>()));
    bench_lookup_k::<6, _, MultiIndexed<6, _>>(&mut g, [0, 0, 0, 0, 0, 0], make_value);
    bench_lookup_k::<6, _, KdTrie<_>>(&mut g, [0, 0, 0, 0, 0, 0], make_value);
    g.finish();
}

fn run_lookup_benchmark<
    const K: usize,
    T,
    B1: Benchable<K, T>,
    B2: Benchable<K, T>,
    B3: Benchable<K, T>,
>(
    c: &mut Criterion,
    min: [i32; K],
    make_value: &dyn Fn(usize) -> T,
) {
    let mut g = c.benchmark_group(format!("lookup_dim{K}_{}", std::any::type_name::<T>()));
    bench_lookup_k::<K, _, B1>(&mut g, min, make_value);
    bench_lookup_k::<K, _, B2>(&mut g, min, make_value);
    bench_lookup_k::<K, _, B3>(&mut g, min, make_value);
    g.finish();
}

// Benchmark lookups for different dimensions
fn bench_lookup_k<const K: usize, T, B: Benchable<K, T>>(
    c: &mut BenchmarkGroup<'_, WallTime>,
    min: [i32; K],
    make_value: &dyn Fn(usize) -> T,
) {
    let vec = B::new(min);
    let mut coords = get_n_coords(NUM_ELEMENTS, min);

    // Insert data
    for (i, coord) in coords.iter().enumerate() {
        vec.push_checked(*coord, make_value(i));
    }

    coords.shuffle(&mut rng());

    c.bench_function(
        format!("{}_lookup_k{}_{}", B::name(), K, std::any::type_name::<T>()),
        |b| {
            b.iter(|| {
                for coord in coords.iter() {
                    black_box(vec.get(*coord));
                }
            })
        },
    );
}

// Iteration benchmarks

fn run_iter_benchmarks<T>(c: &mut Criterion, make_value: &dyn Fn(usize) -> T) {
    // Dim 1
    let mut g = c.benchmark_group(format!("iter_dim1_{}", std::any::type_name::<T>()));
    bench_iter_k::<1, _, OnceBiVec<_>>(&mut g, [0], make_value);
    bench_iter_k::<1, _, TwoEndedGrove<_>>(&mut g, [0], make_value);
    bench_iter_k::<1, _, MultiIndexed<1, _>>(&mut g, [0], make_value);
    bench_iter_k::<1, _, KdTrie<_>>(&mut g, [0], make_value);
    g.finish();

    run_iter_benchmark::<2, _, OnceBiVec<OnceBiVec<_>>, MultiIndexed<2, _>, KdTrie<_>>(
        c,
        [0, 0],
        make_value,
    );
    run_iter_benchmark::<3, _, OnceBiVec<OnceBiVec<OnceBiVec<_>>>, MultiIndexed<3, _>, KdTrie<_>>(
        c,
        [0, 0, 0],
        make_value,
    );
    run_iter_benchmark::<
        4,
        _,
        OnceBiVec<OnceBiVec<OnceBiVec<OnceBiVec<_>>>>,
        MultiIndexed<4, _>,
        KdTrie<_>,
    >(c, [0, 0, 0, 0], make_value);
    run_iter_benchmark::<
        5,
        _,
        OnceBiVec<OnceBiVec<OnceBiVec<OnceBiVec<OnceBiVec<_>>>>>,
        MultiIndexed<5, _>,
        KdTrie<_>,
    >(c, [0, 0, 0, 0, 0], make_value);

    let mut g = c.benchmark_group(format!("iter_dim6_{}", std::any::type_name::<T>()));
    bench_iter_k::<6, _, MultiIndexed<6, _>>(&mut g, [0, 0, 0, 0, 0, 0], make_value);
    bench_iter_k::<6, _, KdTrie<_>>(&mut g, [0, 0, 0, 0, 0, 0], make_value);
    g.finish();
}

fn run_iter_benchmark<
    const K: usize,
    T,
    B1: Benchable<K, T>,
    B2: Benchable<K, T>,
    B3: Benchable<K, T>,
>(
    c: &mut Criterion,
    min: [i32; K],
    make_value: &dyn Fn(usize) -> T,
) {
    let mut g = c.benchmark_group(format!("iter_dim{K}_{}", std::any::type_name::<T>()));
    bench_iter_k::<K, _, B1>(&mut g, min, make_value);
    bench_iter_k::<K, _, B2>(&mut g, min, make_value);
    bench_iter_k::<K, _, B3>(&mut g, min, make_value);
    g.finish();
}

// Benchmark iters for different dimensions
fn bench_iter_k<const K: usize, T, B: Benchable<K, T>>(
    c: &mut BenchmarkGroup<'_, WallTime>,
    min: [i32; K],
    make_value: &dyn Fn(usize) -> T,
) {
    let vec = B::new(min);
    let coords = get_n_coords(NUM_ELEMENTS, min);

    // Insert data
    for (i, coord) in coords.iter().enumerate() {
        vec.push_checked(*coord, make_value(i));
    }

    c.bench_function(
        format!("{}_iter_k{}_{}", B::name(), K, std::any::type_name::<T>()),
        |b| {
            b.iter(|| {
                for x in vec.iter() {
                    black_box(x);
                }
            })
        },
    );
}
