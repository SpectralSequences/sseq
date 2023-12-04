use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use fp::{matrix::Matrix, prime::ValidPrime};
use rand::Rng;

fn random_matrix(p: ValidPrime, dimension: usize) -> Matrix {
    Matrix::from_vec(
        p,
        &(0..dimension)
            .map(|_| random_vector(p, dimension))
            .collect::<Vec<_>>(),
    )
}

fn row_reductions(c: &mut Criterion) {
    for p in [2, 3, 5, 7].iter() {
        let p = ValidPrime::new(*p);
        let mut group = c.benchmark_group(&format!("row_reduce_{p}"));
        for dimension in [10, 20, 69, 100, 420, 1000] {
            group.bench_function(&format!("row_reduce_{p}_{dimension}"), move |b| {
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
    result.resize_with(dimension, || rng.gen::<u32>() % p);
    result
}

criterion_group! {
    name = row_reduction;
    config = Criterion::default();
    targets = row_reductions
}

criterion_main!(row_reduction);
