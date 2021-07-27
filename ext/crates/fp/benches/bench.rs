use std::time::Duration;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use fp::{
    matrix::Matrix,
    prime::ValidPrime,
    vector::{initialize_limb_bit_index_table, FpVector},
};
use rand::Rng;
// mod multinomial;
// mod row_reduce;
// use crate::multinomial::main as multinomial;
// use crate::row_reduce::main as row_reduce;

fn random_matrix(p: ValidPrime) -> Matrix {
    let rows = 1000;
    let cols = 1000;
    let mut vec = vec![0; cols];
    let mut rng = rand::thread_rng();
    let mut vectors = Vec::with_capacity(rows);
    for _ in 0..rows {
        for v in vec.iter_mut() {
            *v = rng.gen::<u32>() % *p;
        }
        vectors.push(FpVector::from_slice(p, &vec));
    }
    Matrix::from_rows(p, vectors, cols)
}

fn row_reductions(c: &mut Criterion) {
    for prime in [2, 3, 5, 7] {
        let p = ValidPrime::new(prime);
        initialize_limb_bit_index_table(p);
        c.bench_function(&format!("row_reduce_{}", p), move |b| {
            b.iter_batched_ref(
                || random_matrix(p),
                |matrix| {
                    matrix.row_reduce();
                },
                BatchSize::SmallInput,
            )
        });
    }
}

fn random_vector(dimension: usize) -> FpVector {
    let mut result = Vec::with_capacity(dimension);
    let mut rng = rand::thread_rng();
    for _ in 0..dimension {
        result.push(rng.gen::<u32>() % 2);
    }
    FpVector::from_slice(ValidPrime::new(2), &result)
}

fn vector_add(c: &mut Criterion) {
    c.bench_function("add_no_simd", |b| {
        b.iter_batched_ref(
            || (random_vector(10000), random_vector(10000)),
            |(vec, other)| {
                vec.add_nosimd(other, 1);
            },
            BatchSize::SmallInput,
        );
    });
    c.bench_function("add_simd", |b| {
        b.iter_batched_ref(
            || (random_vector(10000), random_vector(10000)),
            |(vec, other)| {
                vec.add(other, 1);
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group! {
    name = row_reduction;
    config = Criterion::default().measurement_time(Duration::from_millis(100_000));
    targets = row_reductions
}
criterion_group!(add, vector_add);

criterion_main!(add);
