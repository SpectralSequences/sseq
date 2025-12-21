use std::str::FromStr;

use fp::{
    matrix::Matrix,
    prime::{Prime, ValidPrime},
};
use rand::Rng;

fn main() {
    let p = query::with_default("Prime", "2", ValidPrime::from_str);
    let m = query::with_default("Rows", "64", usize::from_str);
    let n = query::with_default("Columns", "64", usize::from_str);

    if m == 0 || n == 0 {
        eprintln!("Error: matrix dimensions must be positive");
        std::process::exit(1);
    }

    // Create matrix and fill with random values
    let mut matrix = random_matrix(p, m, n);

    // Print original matrix if not too large
    let print_matrices = m <= 10 && n <= 10;
    if print_matrices {
        println!("{}", matrix);
    } else {
        println!("Matrix too large to display ({} x {})", m, n);
    }

    // Row reduce
    let start = std::time::Instant::now();
    println!("\nRow-reducing the matrix...");
    let rank = matrix.row_reduce();
    let duration = start.elapsed();
    println!("Row reduction completed in {:.2?}", duration);

    println!("\nRow-reduced matrix (rank {}):", rank);
    if print_matrices {
        println!("{}", matrix);
    } else {
        println!("Matrix too large to display ({} x {})", m, n);
    }
}

fn random_matrix(p: ValidPrime, rows: usize, cols: usize) -> Matrix {
    Matrix::from_vec(
        p,
        &(0..rows)
            .map(|_| random_vector(p, cols))
            .collect::<Vec<_>>(),
    )
}

fn random_vector(p: ValidPrime, dimension: usize) -> Vec<u32> {
    let mut result = Vec::with_capacity(dimension);
    let mut rng = rand::rng();
    result.resize_with(dimension, || rng.random_range(0..p.as_u32()));
    result
}
