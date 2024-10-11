use fp::{
    matrix::{
        arbitrary::{MatrixArbParams, MAX_COLUMNS, MAX_ROWS},
        Matrix,
    },
    prime::{Prime, ValidPrime},
};
use proptest::prelude::*;

/// An arbitrary pair of distinct indices in the range `0..rows`. These will be used to select
/// arbitrary row operations. We need them to be distinct for the row operation to be safe / not
/// panic.
///
/// Note that it would probably be faster to generate two numbers in the range and `prop_filter` out
/// those that aren't distinct. However, this causes some local failures, and having too many of
/// them could potentially cause the whole proptest to fail.
fn arb_row_pair(rows: usize) -> impl Strategy<Value = (usize, usize)> {
    let all_rows: Vec<usize> = (0usize..rows).collect();
    proptest::sample::subsequence(all_rows, 2)
        .prop_shuffle()
        .prop_map(|v| (v[0], v[1]))
}

/// An arbitrary sequence of row operation specifiers. The row operation `self[target] += c *
/// self[source]` is completely specified by the multiplicative coefficient `c` satisfying `0 < c <
/// p` and the pair of distinct valid row indices. The sequence will have a length in the range
/// `0..1000`. The 0 case is a sanity check that ensures that RREF matrices are indeed RREF, and it
/// seems likely that testing up to 1000 row operations will be sufficient to catch almost any bug.
fn arb_coeff_row_pair_seq(
    p: ValidPrime,
    rows: usize,
) -> impl Strategy<Value = Vec<(u32, (usize, usize))>> {
    proptest::collection::vec((1..p.as_u32(), arb_row_pair(rows)), 0..1000)
}

/// An arbitrary pair of matrices where the first is in RREF and the second is obtained from it by
/// applying a sequence of row operations.
fn arb_reduced_nonreduced_pair() -> impl Strategy<Value = (Matrix, Matrix)> {
    Matrix::arbitrary_rref_with(MatrixArbParams {
        rows: (2..=MAX_ROWS).boxed(),
        columns: (2..=MAX_COLUMNS).boxed(),
        ..Default::default()
    })
    .prop_flat_map(|m| {
        let row_ops = arb_coeff_row_pair_seq(m.prime(), m.rows());
        (Just(m), row_ops)
    })
    .prop_map(|(reduced_matrix, row_ops)| {
        let mut matrix = reduced_matrix.clone();
        for (c, (target, source)) in row_ops.into_iter() {
            matrix.safe_row_op(target, source, c);
        }
        (reduced_matrix, matrix)
    })
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1024,
        max_shrink_time: 30_000,
        max_shrink_iters: 1_000_000,
        .. ProptestConfig::default()
    })]

    /// Test if row reduction turns a matrix built from a sequence of row operations applied to a
    /// matrix in RREF back to that same RREF
    #[test]
    fn has_correct_reduction((reduced_matrix, mut matrix) in arb_reduced_nonreduced_pair()) {
        matrix.row_reduce();
        prop_assert_eq!(reduced_matrix, matrix);
    }
}
