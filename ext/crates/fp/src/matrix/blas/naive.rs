use super::{MatrixBlock, MatrixBlockMut};
use crate::{limb::Limb, matrix::Matrix, prime::TWO};

pub fn gemm_block_naive(
    alpha: bool,
    a: MatrixBlock,
    b: MatrixBlock,
    beta: bool,
    c: &mut MatrixBlockMut,
) {
    if !beta {
        setzero_block_naive(c);
    }

    if !alpha {
        return;
    }

    let bt = transpose_block(b);

    for (row_idx, a_row) in a.iter().enumerate() {
        for (col_idx, b_col) in bt.data().iter().enumerate() {
            let dot_product = (*a_row & *b_col).count_ones() as Limb % 2;
            *c.get_mut(row_idx) ^= dot_product << col_idx;
        }
    }
}

fn transpose_block(b: MatrixBlock) -> Matrix {
    let mut bt = Matrix::new(TWO, 64, 64);
    for (orig_col_idx, mut bt_row) in bt.iter_mut().enumerate() {
        for orig_row_idx in 0..64 {
            bt_row.set_entry(
                orig_row_idx,
                (b.get(orig_row_idx) >> orig_col_idx) as u32 & 1,
            );
        }
    }
    bt
}

pub fn setzero_block_naive(c: &mut MatrixBlockMut) {
    // Set all limbs to zero.
    for limb in c.limbs.iter_mut() {
        *limb = 0;
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::{matrix::arbitrary::MatrixArbParams, prime::TWO};

    proptest! {
        #[test]
        fn test_transpose_is_transpose(
            b in Matrix::arbitrary_with(MatrixArbParams {
                p: Some(TWO),
                rows: Just(64).boxed(),
                columns: Just(64).boxed(),
            })
        ) {
            let bt = transpose_block(b.block_at(0, 0));
            for i in 0..64 {
                for j in 0..64 {
                    prop_assert_eq!(b.row(i).entry(j), bt.row(j).entry(i));
                }
            }
        }
    }
}
