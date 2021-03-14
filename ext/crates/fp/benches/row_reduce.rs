use rand::prelude::*;

use fp::matrix::Matrix;
use fp::prime::ValidPrime;
use fp::vector::FpVectorT;

fn row_reduce_2(bencher: &mut bencher::Bencher) {
    let p = ValidPrime::new(2);
    let num_matrices = 3;
    let rows = 1000;
    let cols = 1000;
    let mut matrices = Vec::with_capacity(num_matrices);
    for _ in 0..num_matrices {
        matrices.push(Matrix::new(p, rows, cols));
    }
    let mut vec = vec![0; cols];
    let mut rng = rand::thread_rng();
    for m in matrices.iter_mut() {
        m.initialize_pivots();
        for row in 0..rows {
            for v in vec.iter_mut() {
                *v = rng.gen::<bool>() as u32;
            }
            m[row].pack(&vec);
        }
    }

    bencher.iter(|| {
        for m in matrices.iter_mut() {
            m.row_reduce();
        }
    });
}

bencher::benchmark_group!(main, row_reduce_2);
