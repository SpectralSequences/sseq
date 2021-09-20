use std::time::Duration;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use fp::{matrix::Matrix, prime::ValidPrime, vector::FpVector};
use rand::Rng;

fn random_matrix(p: ValidPrime, dimension: usize) -> Matrix {
    Matrix::from_rows(
        p,
        (0..dimension)
            .map(|_| random_vector(p, dimension))
            .collect(),
        dimension,
    )
}

fn row_reductions(c: &mut Criterion) {
    for p in [2, 3, 5, 7].iter() {
        let p = ValidPrime::new(*p);
        let mut group = c.benchmark_group(&format!("row_reduce_{}", p));
        for dimension in [10, 20, 69, 100, 420, 1000] {
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

fn random_vector(p: ValidPrime, dimension: usize) -> FpVector {
    let mut result = Vec::with_capacity(dimension);
    let mut rng = rand::thread_rng();
    for _ in 0..dimension {
        result.push(rng.gen::<u32>() % *p);
    }
    FpVector::from_slice(p, &result)
}

criterion_group! {
    name = row_reduction;
    config = Criterion::default().sample_size(100).measurement_time(Duration::from_secs(100));
    targets = row_reductions
}

criterion_main!(row_reduction);
