use fp::{
    matrix::Matrix,
    prime::{Prime, ValidPrime},
};
use proptest::prelude::*;

/// An increasing sequence of numbers between 0 and `cols`, where the sequence has a length between
/// 1 and the smaller of `rows` and `cols`. Similar in spirit to a Young tableau / diagram.
///
/// The intent is for those to be specifiers for consecutive row vectors containing a pivot. The
/// fact that they are increasing means that the resulting matrix will be in RREF, and the bounds on
/// the values and the length mean that they are valid to specify a matrix of size rows x cols.
fn arb_tableau(rows: usize, cols: usize) -> impl Strategy<Value = Vec<usize>> {
    let all_cols: Vec<usize> = (0usize..cols).collect();
    proptest::sample::subsequence(all_cols, 1..=usize::min(rows, cols))
}

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

// This is a macro used to define functions that take in values from strategies as arguments.
// (Notice that the first set of parentheses contains ordinary arguments and the second uses the
// proptest `in` syntax.) This is different from the previous functions that produce strategies from
// concrete values.
prop_compose! {
    /// An arbitrary matrix in RREF, over the specified prime with the specified dimensions.
    fn arb_rref_matrix(p: ValidPrime, rows: usize, columns: usize)
        (tableau in arb_tableau(rows, columns)) -> Matrix
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
    /// An arbitrary pair of matrices where the first is in RREF and the second is obtained from it
    /// by applying a sequence of row operations. They are defined over an arbitrary prime and the
    /// dimensions are in the range `2..100`. We use the triple parenthesis syntax because we need
    /// to generate the prime and the dimensions first before getting the RREF matrix and the
    /// sequence of row operations, which both depend on those values. The documentation for
    /// [`prop_compose`] has more information.
    fn arb_reduced_nonreduced_pair()
        (p in any::<ValidPrime>(), rows in 2usize..100, cols in 2usize..100)
        (reduced_matrix in arb_rref_matrix(p, rows, cols),
         row_ops in arb_coeff_row_pair_seq(p, rows)) -> (Matrix, Matrix)
    {
        let mut matrix = reduced_matrix.clone();
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

    /// Test if row reduction turns a matrix built from a sequence of row operations applied to a
    /// matrix in RREF back to that same RREF
    #[test]
    fn has_correct_reduction((reduced_matrix, mut matrix) in arb_reduced_nonreduced_pair()) {
        matrix.row_reduce();
        prop_assert_eq!(reduced_matrix, matrix);
    }
}
