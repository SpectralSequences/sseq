use proptest::prelude::*;

use fp::{matrix::Matrix, prime::ValidPrime};

fn arb_prime() -> impl Strategy<Value = ValidPrime> {
    prop_oneof![
        Just(ValidPrime::new(2)),
        Just(ValidPrime::new(3)),
        Just(ValidPrime::new(5)),
        Just(ValidPrime::new(7)),
    ]
}

fn arb_tableau(rows: usize, cols: usize) -> impl Strategy<Value = Vec<usize>> {
    let all_cols: Vec<usize> = (0usize..cols).collect();
    proptest::sample::subsequence(all_cols, 1..=usize::min(rows, cols))
}

fn arb_row_pair(rows: usize) -> impl Strategy<Value = (usize, usize)> {
    let all_rows: Vec<usize> = (0usize..rows).collect();
    proptest::sample::subsequence(all_rows, 2).prop_map(|v| (v[0], v[1]))
}

fn arb_coeff_row_pair_seq(
    p: ValidPrime,
    rows: usize,
) -> impl Strategy<Value = Vec<(u32, (usize, usize))>> {
    proptest::collection::vec((1..*p, arb_row_pair(rows)), 0..1000)
}

prop_compose! {
    fn arb_rref_matrix()
    (p in arb_prime(), rows in 2usize..100, columns in 2usize..100)
    (p in Just(p), rows in Just(rows), columns in Just(columns), tableau in arb_tableau(rows, columns)) -> Matrix
    {
        let row_vec: Vec<Vec<u32>> = tableau.iter().map(|col_idx| {
            let mut v = vec![0; columns];
            v[*col_idx] += 1;
            v
        })
        .chain(std::iter::repeat(vec![0; columns]).take(rows - tableau.len()))
        .collect();

        Matrix::from_vec(p, &row_vec)
    }
}

prop_compose! {
    fn arb_matrix_pair()
    (matrix in arb_rref_matrix())
    (row_ops in arb_coeff_row_pair_seq(matrix.prime(), matrix.rows()), mut matrix in Just(matrix)) -> (Matrix, Matrix) {
        let reduced_matrix = matrix.clone();
        for (c, (target, source)) in row_ops.into_iter() {
            matrix.safe_row_op(target, source, c) ;
        }
        (reduced_matrix, matrix)
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1024,
        max_shrink_time: 30_000,
        max_shrink_iters: 1_000_000,
        .. ProptestConfig::default()
    })]

    #[test]
    fn has_correct_reduction((reduced_matrix, mut matrix) in arb_matrix_pair()) {
        matrix.row_reduce();
        prop_assert_eq!(reduced_matrix, matrix);
    }
}
