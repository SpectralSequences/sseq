use std::arch::x86_64::{self, __m512i};

use crate::{constants::BITS_PER_LIMB, limb::Limb, matrix::Matrix};

mod fast_mul_6;
mod fast_mul_7;

impl std::ops::Mul for &Matrix {
    type Output = Matrix;

    fn mul(self, rhs: Self) -> Matrix {
        assert_eq!(self.prime(), rhs.prime());
        assert_eq!(self.columns(), rhs.rows());

        let mut result = Matrix::new(self.prime(), self.rows(), rhs.columns());
        for i in 0..self.rows() {
            for j in 0..rhs.columns() {
                for k in 0..self.columns() {
                    result
                        .row_mut(i)
                        .add_basis_element(j, self.row(i).entry(k) * rhs.row(k).entry(j));
                }
            }
        }
        result
    }
}

impl Matrix {
    pub fn transpose(&self) -> Matrix {
        let mut result = Matrix::new(self.prime(), self.columns(), self.rows());
        for i in 0..self.rows() {
            for j in 0..self.columns() {
                result.row_mut(j).set_entry(i, self.row(i).entry(j));
            }
        }
        result
    }

    pub fn fast_mul(&self, b: &Self) -> Matrix {
        assert_eq!(self.prime(), b.prime());
        assert_eq!(self.columns(), b.rows());

        let mut result = Matrix::new(self.prime(), self.rows(), b.columns());
        let bt = b.transpose();
        for (i, row) in self.iter().enumerate() {
            for (j, col) in bt.iter().enumerate() {
                result
                    .row_mut(i)
                    .set_entry(j, dot_product(row.limbs(), col.limbs()));
            }
        }
        result
    }

    pub fn fast_mul_2(&self, b: &Self) -> Matrix {
        assert_eq!(self.prime(), b.prime());
        assert_eq!(self.columns(), b.rows());

        let mut result = Matrix::new(self.prime(), self.rows(), b.columns());
        for (mut result_row, a_row) in result.iter_mut().zip(self.iter()) {
            let a_row_iter = BitIterator::new(a_row.limbs()[0]);
            let mut current_row = 0;
            for (b_row, a_bit) in b.iter().zip(a_row_iter) {
                current_row ^= b_row.limbs()[0] * a_bit as Limb;
            }
            result_row.limbs_mut()[0] = current_row;
        }

        result
    }

    pub fn fast_mul_3(&self, b: &Self) -> Matrix {
        assert_eq!(self.prime(), b.prime());
        assert_eq!(self.columns(), b.rows());

        let mut result = Matrix::new(self.prime(), self.rows(), b.columns());
        let b_block = b.simd_block_at([0, 0]);
        let mut scratch_block = SimdBlock::new();

        for (limb_index, limb) in self.data().iter().enumerate() {
            for (scratch_index, byte) in limb.to_le_bytes().iter().enumerate() {
                let mask = unsafe { x86_64::_cvtu32_mask8(*byte as u32) };
                scratch_block.set(scratch_index, unsafe {
                    x86_64::_mm512_maskz_mov_epi64(mask, b_block.get(scratch_index))
                });
            }
            let vxor = scratch_block.xor();
            let hxor_result = hxor(vxor);
            result.data_mut()[limb_index] = extract_u64(hxor_result);
        }

        result
    }

    pub fn fast_mul_4(&self, b: &Self) -> Matrix {
        assert_eq!(self.prime(), b.prime());
        assert_eq!(self.columns(), b.rows());

        let mut result = Matrix::new(self.prime(), self.rows(), b.columns());
        let b_block = b.simd_block_at([0, 0]);

        for (limb_index, limb) in self.data().iter().enumerate() {
            let mut scratch = unsafe { x86_64::_mm512_setzero_si512() };
            for (scratch_index, byte) in limb.to_le_bytes().iter().enumerate() {
                let mask = unsafe { x86_64::_cvtu32_mask8(*byte as u32) };
                scratch = unsafe {
                    x86_64::_mm512_mask_xor_epi64(
                        scratch,
                        mask,
                        b_block.get(scratch_index),
                        scratch,
                    )
                };
            }
            let hxor_result = hxor(scratch);
            result.data_mut()[limb_index] = extract_u64(hxor_result);
        }

        result
    }

    pub fn fast_mul_5(&self, b: &Self) -> Matrix {
        assert_eq!(self.prime(), b.prime());
        assert_eq!(self.columns(), b.rows());

        let mut result = Matrix::new(self.prime(), self.rows(), b.columns());
        let b_block = b.simd_block_at([0, 0]);

        for (limb_index, limb) in self.data().iter().enumerate() {
            let mut scratch = unsafe { x86_64::_mm512_setzero_si512() };
            for (scratch_index, byte) in limb.to_le_bytes().iter().enumerate() {
                let mask = unsafe { x86_64::_cvtu32_mask8(*byte as u32) };
                scratch = unsafe {
                    x86_64::_mm512_mask_xor_epi64(
                        scratch,
                        mask,
                        b_block.get(scratch_index),
                        scratch,
                    )
                };
            }
            let hxor_result = hxor2(scratch);
            result.data_mut()[limb_index] = hxor_result;
        }

        result
    }

    fn zmm_at(&self, coords: [usize; 2]) -> __m512i {
        let row = coords[0];
        let col = coords[1];

        let offset_array: [i64; 8] = std::array::from_fn(|i| (i * self.stride()) as i64);
        let offsets = unsafe { x86_64::_mm512_loadu_epi64(offset_array.as_ptr()) };

        unsafe {
            x86_64::_mm512_i64gather_epi64::<8>(
                offsets,
                self.data().as_ptr().add(row * self.stride() + col) as *const i64,
            )
        }
    }

    fn simd_block_at(&self, coords: [usize; 2]) -> SimdBlock {
        assert!(coords[1].is_multiple_of(BITS_PER_LIMB));

        let row = coords[0];
        let col = coords[1] / BITS_PER_LIMB;

        let zmms = std::array::from_fn(|i| self.zmm_at([row + i * 8, col]));

        SimdBlock { zmms }
    }
}

struct SimdBlock {
    zmms: [__m512i; 8],
}

impl SimdBlock {
    #[inline(always)]
    fn new() -> Self {
        Self {
            zmms: [unsafe { x86_64::_mm512_setzero_si512() }; 8],
        }
    }

    #[inline(always)]
    fn get(&self, index: usize) -> __m512i {
        assert!(index < 8);
        self.zmms[index]
    }

    #[inline(always)]
    fn set(&mut self, index: usize, value: __m512i) {
        assert!(index < 8);
        self.zmms[index] = value;
    }

    #[inline(always)]
    fn xor(&self) -> __m512i {
        let mut result = unsafe { x86_64::_mm512_setzero_si512() };
        for zmm in &self.zmms {
            result = unsafe { x86_64::_mm512_xor_si512(result, *zmm) };
        }
        result
    }
}

#[inline(always)]
fn hxor(mut a: __m512i) -> __m512i {
    let mut permuted = unsafe { x86_64::_mm512_permutex_epi64::<0b10110001>(a) };
    a = unsafe { x86_64::_mm512_xor_epi64(a, permuted) };

    permuted = unsafe { x86_64::_mm512_permutex_epi64::<0b00011011>(a) };
    a = unsafe { x86_64::_mm512_xor_epi64(a, permuted) };

    permuted = unsafe { x86_64::_mm512_shuffle_i64x2::<0b01001110>(a, a) };
    a = unsafe { x86_64::_mm512_xor_epi64(a, permuted) };

    a
}

fn hxor2(a: __m512i) -> u64 {
    let a: [u64; 8] = unsafe { std::mem::transmute(a) };
    a.iter().fold(0, |acc, &x| acc ^ x)
}

#[inline(always)]
fn extract_u64(a: __m512i) -> u64 {
    unsafe { x86_64::_mm_cvtsi128_si64(x86_64::_mm512_castsi512_si128(a)) as u64 }
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
        if self.bit_index >= BITS_PER_LIMB {
            return None;
        }
        let result = self.limb & 1 == 1;
        self.limb >>= 1;
        self.bit_index += 1;
        Some(result)
    }
}

fn dot_product(a: &[Limb], b: &[Limb]) -> u32 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x & y).count_ones())
        .sum()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::{matrix::arbitrary::MatrixArbParams, prime::TWO};

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 10000,
            max_shrink_time: 3600_000,
            max_shrink_iters: 1_000_000_000,
            .. ProptestConfig::default()
        })]

        #[test]
        fn test_fast_mul_is_mul(m in Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(64).boxed(),
            columns: Just(64).boxed(),
        }), n in Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(64).boxed(),
            columns: Just(64).boxed(),
        })) {
            let prod1 = (&m) * (&n);
            let prod2 = m.fast_mul(&n);
            prop_assert_eq!(prod1, prod2);
        }

        #[test]
        fn test_fast_mul_2_is_mul(m in Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(64).boxed(),
            columns: Just(64).boxed(),
        }), n in Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(64).boxed(),
            columns: Just(64).boxed(),
        })) {
            let prod1 = m.fast_mul(&n);
            let prod2 = m.fast_mul_2(&n);
            prop_assert_eq!(prod1, prod2);
        }

        #[test]
        fn test_fast_mul_3_is_mul(m in Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(64).boxed(),
            columns: Just(64).boxed(),
        }), n in Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(64).boxed(),
            columns: Just(64).boxed(),
        })) {
            let prod1 = m.fast_mul_2(&n);
            let prod2 = m.fast_mul_3(&n);
            prop_assert_eq!(prod1, prod2);
        }

        #[test]
        fn test_fast_mul_4_is_mul(m in Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(64).boxed(),
            columns: Just(64).boxed(),
        }), n in Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(64).boxed(),
            columns: Just(64).boxed(),
        })) {
            let prod1 = m.fast_mul_3(&n);
            let prod2 = m.fast_mul_4(&n);
            prop_assert_eq!(prod1, prod2);
        }

        #[test]
        fn test_fast_mul_5_is_mul(m in Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(64).boxed(),
            columns: Just(64).boxed(),
        }), n in Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(64).boxed(),
            columns: Just(64).boxed(),
        })) {
            let prod1 = m.fast_mul_4(&n);
            let prod2 = m.fast_mul_5(&n);
            prop_assert_eq!(prod1, prod2);
        }

        #[test]
        fn test_fast_mul_6_is_mul(m in Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(64).boxed(),
            columns: Just(64).boxed(),
        }), n in Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(64).boxed(),
            columns: Just(64).boxed(),
        })) {
            let prod1 = m.fast_mul_5(&n);
            let prod2 = m.fast_mul_6(&n);
            prop_assert_eq!(prod1, prod2);
        }

        #[test]
        fn test_fast_mul_7_is_mul(m in Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(64).boxed(),
            columns: Just(64).boxed(),
        }), n in Matrix::arbitrary_with(MatrixArbParams {
            p: Some(TWO),
            rows: Just(64).boxed(),
            columns: Just(64).boxed(),
        })) {
            let prod1 = m.fast_mul_6(&n);
            let prod2 = m.fast_mul_7(&n);
            prop_assert_eq!(prod1, prod2);
        }
    }
}
