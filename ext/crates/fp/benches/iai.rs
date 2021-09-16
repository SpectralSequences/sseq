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

fn random_vector(p: ValidPrime, dimension: usize) -> FpVector {
    let mut result = Vec::with_capacity(dimension);
    let mut rng = rand::thread_rng();
    for _ in 0..dimension {
        result.push(rng.gen::<u32>() % *p);
    }
    FpVector::from_slice(p, &result)
}

fn row_reduce_p_n(p: ValidPrime, dimension: usize) {
    random_matrix(p, dimension).row_reduce();
}

fn row_reduce_2_10() {
    row_reduce_p_n(ValidPrime::new(2), 10);
}

fn row_reduce_2_100() {
    row_reduce_p_n(ValidPrime::new(2), 100);
}

fn row_reduce_2_1000() {
    row_reduce_p_n(ValidPrime::new(2), 1000);
}

fn row_reduce_2_20() {
    row_reduce_p_n(ValidPrime::new(2), 20);
}

fn row_reduce_2_69() {
    row_reduce_p_n(ValidPrime::new(2), 69);
}

fn row_reduce_2_420() {
    row_reduce_p_n(ValidPrime::new(2), 420);
}

fn row_reduce_3_10() {
    row_reduce_p_n(ValidPrime::new(3), 10);
}

fn row_reduce_3_100() {
    row_reduce_p_n(ValidPrime::new(3), 100);
}

fn row_reduce_3_1000() {
    row_reduce_p_n(ValidPrime::new(3), 1000);
}

fn row_reduce_3_20() {
    row_reduce_p_n(ValidPrime::new(3), 20);
}

fn row_reduce_3_69() {
    row_reduce_p_n(ValidPrime::new(3), 69);
}

fn row_reduce_3_420() {
    row_reduce_p_n(ValidPrime::new(3), 420);
}

fn row_reduce_5_10() {
    row_reduce_p_n(ValidPrime::new(5), 10);
}

fn row_reduce_5_100() {
    row_reduce_p_n(ValidPrime::new(5), 100);
}

fn row_reduce_5_1000() {
    row_reduce_p_n(ValidPrime::new(5), 1000);
}

fn row_reduce_5_20() {
    row_reduce_p_n(ValidPrime::new(5), 20);
}

fn row_reduce_5_69() {
    row_reduce_p_n(ValidPrime::new(5), 69);
}

fn row_reduce_5_420() {
    row_reduce_p_n(ValidPrime::new(5), 420);
}

fn row_reduce_7_10() {
    row_reduce_p_n(ValidPrime::new(7), 10);
}

fn row_reduce_7_100() {
    row_reduce_p_n(ValidPrime::new(7), 100);
}

fn row_reduce_7_1000() {
    row_reduce_p_n(ValidPrime::new(7), 1000);
}

fn row_reduce_7_20() {
    row_reduce_p_n(ValidPrime::new(7), 20);
}

fn row_reduce_7_69() {
    row_reduce_p_n(ValidPrime::new(7), 69);
}

fn row_reduce_7_420() {
    row_reduce_p_n(ValidPrime::new(7), 420);
}

iai::main!(
    row_reduce_2_10,
    row_reduce_2_20,
    row_reduce_2_69,
    row_reduce_2_100,
    row_reduce_2_420,
    row_reduce_2_1000,
    row_reduce_3_10,
    row_reduce_3_20,
    row_reduce_3_69,
    row_reduce_3_100,
    row_reduce_3_420,
    row_reduce_3_1000,
    row_reduce_5_10,
    row_reduce_5_20,
    row_reduce_5_69,
    row_reduce_5_100,
    row_reduce_5_420,
    row_reduce_5_1000,
    row_reduce_7_10,
    row_reduce_7_20,
    row_reduce_7_69,
    row_reduce_7_100,
    row_reduce_7_420,
    row_reduce_7_1000,
);
