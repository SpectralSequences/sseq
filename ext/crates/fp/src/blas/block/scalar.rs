use super::{MatrixBlock, MatrixBlockSlice, MatrixBlockSliceMut};
use crate::limb::Limb;

/// Scalar (non-SIMD) implementation of 64 x 64 block GEMM over F_2.
///
/// Computes `C = alpha * A * B + beta * C` where all arithmetic is in F_2 (XOR for addition, AND
/// for multiplication).
///
/// # Algorithm
///
/// ```text
/// For each row i of A:
///   For each bit position k in A[i]:
///     If A[i][k] == 1:
///       C[i] ^= B[k]  (XOR row k of B into row i of C)
/// ```
///
/// Note that this is not quite the standard algorithm for matrix multiplication. The standard
/// algorithm would require us to iterate over a column of A for every output bit. The three loops
/// in the algorithm are independent, so we can instead iterate over outputs and *then* move down
/// the columns of A. This way, we only need to consider one limb of A at a time, and we don't need
/// to do bit extractions (except for iterating over the bits of a limb).
pub fn gemm_block_scalar(
    alpha: bool,
    a: MatrixBlock,
    b: MatrixBlock,
    beta: bool,
    mut c: MatrixBlock,
) -> MatrixBlock {
    if !beta {
        c = MatrixBlock::zero();
    }

    if !alpha {
        return c;
    }

    // For each row of A
    for (result_limb, a_limb) in c.iter_mut().zip(a.limbs.iter()) {
        let a_limb_iter = BitIterator::new(*a_limb);
        // For each bit in this row of A, XOR the corresponding row of B into C
        for (b_limb, a_bit) in b.limbs.iter().zip(a_limb_iter) {
            *result_limb ^= *b_limb * (a_bit as Limb);
        }
    }

    c
}

pub fn gather_block_scalar(a: MatrixBlockSlice) -> MatrixBlock {
    let mut limbs = [0; 64];
    for (i, limb) in a.iter().enumerate() {
        limbs[i] = *limb;
    }
    MatrixBlock { limbs }
}

pub fn setzero_block_scalar(c: &mut MatrixBlockSliceMut) {
    // Set all limbs to zero.
    for limb in c.iter_mut() {
        *limb = 0;
    }
}

struct BitIterator {
    limb: Limb,
    bit_index: usize,
}

impl BitIterator {
    fn new(limb: Limb) -> Self {
        Self { limb, bit_index: 0 }
    }
}

impl Iterator for BitIterator {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bit_index >= crate::constants::BITS_PER_LIMB {
            return None;
        }
        let result = self.limb & 1 == 1;
        self.limb >>= 1;
        self.bit_index += 1;
        Some(result)
    }
}
