//! One-shot large-matrix timing of M4RI (`row_reduce`) vs blocked GEMM
//! (`row_reduce_blas3`) over F₂ on half-rank inputs, printed per size. Run with
//! `--features concurrent` and set `RAYON_NUM_THREADS` to vary the core count.
//!
//! ```sh
//! RAYON_NUM_THREADS=4 cargo run --release -p fp --features concurrent \
//!     --example reduce_scaling -- 4096 8192 16384
//! ```

use std::time::Instant;

use fp::{matrix::Matrix, prime::TWO};
use rand::Rng;

fn random_matrix(rows: usize, cols: usize) -> Matrix {
    let mut rng = rand::rng();
    Matrix::from_vec(
        TWO,
        &(0..rows)
            .map(|_| (0..cols).map(|_| rng.random_range(0..2)).collect())
            .collect::<Vec<Vec<u32>>>(),
    )
}

/// `n × n` of rank ≤ `rank`, as a product (built with the fast GEMM).
fn rank_deficient(n: usize, rank: usize) -> Matrix {
    &random_matrix(n, rank) * &random_matrix(rank, n)
}

fn main() {
    let sizes: Vec<usize> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let sizes = if sizes.is_empty() {
        vec![4096, 8192, 16384]
    } else {
        sizes
    };
    let threads = std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "default".to_string());

    println!(
        "{:>7}  {:>10}  {:>10}  {:>7}   (threads={threads})",
        "n", "m4ri", "blas3", "ratio"
    );
    for n in sizes {
        let base = rank_deficient(n, n / 2);

        let mut a = base.clone();
        let t = Instant::now();
        let r1 = a.row_reduce();
        let m4ri = t.elapsed();
        drop(a);

        let mut b = base.clone();
        let t = Instant::now();
        let r2 = b.row_reduce_blas3();
        let blas3 = t.elapsed();

        assert_eq!(r1, r2, "rank mismatch at n={n}");
        println!(
            "{n:>7}  {:>10.3?}  {:>10.3?}  {:>6.2}x  (rank {r1})",
            m4ri,
            blas3,
            blas3.as_secs_f64() / m4ri.as_secs_f64(),
        );
    }
}
