use super::{MatrixBlock, MatrixBlockSlice, MatrixBlockSliceMut};
use crate::limb::Limb;

pub fn gemm_block_scalar(
    alpha: bool,
    a: MatrixBlock,
    b: MatrixBlock,
    beta: bool,
    c: &mut MatrixBlockSliceMut,
) {
    if !beta {
        setzero_block_scalar(c);
    }

    if !alpha {
        return;
    }

    for (result_limb, a_limb) in c.iter_mut().zip(a.limbs.iter()) {
        let a_limb_iter = BitIterator::new(*a_limb);
        for (b_limb, a_bit) in b.limbs.iter().zip(a_limb_iter) {
            *result_limb ^= *b_limb * (a_bit as Limb);
        }
    }
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
