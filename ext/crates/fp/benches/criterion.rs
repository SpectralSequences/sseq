use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use fp::{matrix::Matrix, prime::ValidPrime};
use rand::Rng;

#[cfg(feature = "odd-primes")]
static TEST_PRIMES: [u32; 4] = [2, 3, 5, 7];
#[cfg(not(feature = "odd-primes"))]
static TEST_PRIMES: [u32; 1] = [2];

fn random_matrix(p: ValidPrime, dimension: usize) -> Matrix {
    Matrix::from_vec(
        p,
        &(0..dimension)
            .map(|_| random_vector(p, dimension))
            .collect::<Vec<_>>(),
    )
}

fn row_reductions(c: &mut Criterion) {
    for p in TEST_PRIMES.iter() {
        let p = ValidPrime::new(*p);
        let mut group = c.benchmark_group(&format!("row_reduce_{}", p));
        let sizes = if *p == 2 {
            vec![10, 20, 69, 100, 420, 1000, 2000, 4000]
        } else {
            vec![10, 20, 69, 100, 420, 1000]
        };
        for dimension in sizes {
            group.bench_function(&format!("row_reduce_{}_{}", p, dimension), move |b| {
                b.iter_batched_ref(
                    || random_matrix(p, dimension),
                    |matrix| {
                        matrix.row_reduce();
                    },
                    BatchSize::SmallInput,
                )
            });
        }
        group.finish();
    }
}

fn random_vector(p: ValidPrime, dimension: usize) -> Vec<u32> {
    let mut result = Vec::with_capacity(dimension);
    let mut rng = rand::thread_rng();
    result.resize_with(dimension, || rng.gen::<u32>() % *p);
    result
}

criterion_group! {
    name = row_reduction;
    config = Criterion::default().sample_size(100).measurement_time(std::time::Duration::from_secs(100));
    targets = row_reductions
}

criterion_main!(row_reduction);
