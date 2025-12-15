use std::arch::x86_64;

use crate::{
    blas::block::{MatrixBlock, MatrixBlockSlice},
    limb::Limb,
};

type SimdLimb = x86_64::__m512i;

#[target_feature(enable = "avx512f")]
fn load(limb: *const Limb) -> SimdLimb {
    unsafe { x86_64::_mm512_loadu_si512(limb as *const SimdLimb) }
}

#[target_feature(enable = "avx512f")]
fn store(limb: *mut Limb, val: SimdLimb) {
    unsafe { x86_64::_mm512_storeu_si512(limb as *mut SimdLimb, val) }
}

#[target_feature(enable = "avx512f")]
fn xor(left: SimdLimb, right: SimdLimb) -> SimdLimb {
    x86_64::_mm512_xor_si512(left, right)
}

super::add_simd_arch!("avx512f");

const UNIT_OFFSETS: [i64; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

/// Performs C = alpha * A * B + beta * C where A, B, C are 64x64 matrices
#[target_feature(enable = "avx512f")]
pub fn gemm_block_simd(
    alpha: bool,
    a: MatrixBlock,
    b: MatrixBlock,
    beta: bool,
    c: &mut MatrixBlock,
) {
    if !beta {
        *c = SimdBlock::zero().as_matrix_block();
    }

    if !alpha {
        return;
    }

    unsafe {
        std::arch::asm!(
            "mov {zmm_idx}, 0",
            "mov {limb_idx}, 0",
            "2:",

            "vmovdqa64 zmm10, zmmword ptr [{c_data_ptr} + {zmm_idx}]",

            "vmovdqa64 zmm12, zmmword ptr [{b_data_ptr}]",
            "vmovdqa64 zmm13, zmmword ptr [{b_data_ptr} + 64]",
            "vmovdqa64 zmm14, zmmword ptr [{b_data_ptr} + 64*2]",
            "vmovdqa64 zmm15, zmmword ptr [{b_data_ptr} + 64*3]",

            "mov {limb0}, [{a_data_ptr} + 8*{limb_idx} + 0]",
            "mov {limb1}, [{a_data_ptr} + 8*{limb_idx} + 8]",
            "mov {limb2}, [{a_data_ptr} + 8*{limb_idx} + 16]",
            "mov {limb3}, [{a_data_ptr} + 8*{limb_idx} + 24]",
            "mov {limb4}, [{a_data_ptr} + 8*{limb_idx} + 32]",
            "mov {limb5}, [{a_data_ptr} + 8*{limb_idx} + 40]",
            "mov {limb6}, [{a_data_ptr} + 8*{limb_idx} + 48]",
            "mov {limb7}, [{a_data_ptr} + 8*{limb_idx} + 56]",

            "kmovq k1, {limb0}",
            "kmovq k2, {limb1}",
            "kmovq k3, {limb2}",
            "kmovq k4, {limb3}",

            "vpxorq zmm0, zmm0, zmm0",
            "vpxorq zmm1, zmm1, zmm1",
            "vpxorq zmm2, zmm2, zmm2",
            "vpxorq zmm3, zmm3, zmm3",
            "vpxorq zmm4, zmm4, zmm4",
            "vpxorq zmm5, zmm5, zmm5",
            "vpxorq zmm6, zmm6, zmm6",
            "vpxorq zmm7, zmm7, zmm7",

            "vpxorq zmm0 {{k1}}, zmm0, zmm12",
            "vpxorq zmm1 {{k2}}, zmm1, zmm12",
            "vpxorq zmm2 {{k3}}, zmm2, zmm12",
            "vpxorq zmm3 {{k4}}, zmm3, zmm12",

            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm13",
            "vpxorq zmm1 {{k2}}, zmm1, zmm13",
            "vpxorq zmm2 {{k3}}, zmm2, zmm13",
            "vpxorq zmm3 {{k4}}, zmm3, zmm13",

            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm14",
            "vpxorq zmm1 {{k2}}, zmm1, zmm14",
            "vpxorq zmm2 {{k3}}, zmm2, zmm14",
            "vpxorq zmm3 {{k4}}, zmm3, zmm14",

            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm15",
            "vpxorq zmm1 {{k2}}, zmm1, zmm15",
            "vpxorq zmm2 {{k3}}, zmm2, zmm15",
            "vpxorq zmm3 {{k4}}, zmm3, zmm15",

            "kmovq k1, {limb4}",
            "kmovq k2, {limb5}",
            "kmovq k3, {limb6}",
            "kmovq k4, {limb7}",

            "vpxorq zmm4 {{k1}}, zmm4, zmm12",
            "vpxorq zmm5 {{k2}}, zmm5, zmm12",
            "vpxorq zmm6 {{k3}}, zmm6, zmm12",
            "vpxorq zmm7 {{k4}}, zmm7, zmm12",

            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "vpxorq zmm4 {{k1}}, zmm4, zmm13",
            "vpxorq zmm5 {{k2}}, zmm5, zmm13",
            "vpxorq zmm6 {{k3}}, zmm6, zmm13",
            "vpxorq zmm7 {{k4}}, zmm7, zmm13",

            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "vpxorq zmm4 {{k1}}, zmm4, zmm14",
            "vpxorq zmm5 {{k2}}, zmm5, zmm14",
            "vpxorq zmm6 {{k3}}, zmm6, zmm14",
            "vpxorq zmm7 {{k4}}, zmm7, zmm14",

            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "vpxorq zmm4 {{k1}}, zmm4, zmm15",
            "vpxorq zmm5 {{k2}}, zmm5, zmm15",
            "vpxorq zmm6 {{k3}}, zmm6, zmm15",
            "vpxorq zmm7 {{k4}}, zmm7, zmm15",

            "shr {limb0}, 32",
            "shr {limb1}, 32",
            "shr {limb2}, 32",
            "shr {limb3}, 32",
            "shr {limb4}, 32",
            "shr {limb5}, 32",
            "shr {limb6}, 32",
            "shr {limb7}, 32",

            "vmovdqa64 zmm12, zmmword ptr [{b_data_ptr} + 64*4]",
            "vmovdqa64 zmm13, zmmword ptr [{b_data_ptr} + 64*5]",
            "vmovdqa64 zmm14, zmmword ptr [{b_data_ptr} + 64*6]",
            "vmovdqa64 zmm15, zmmword ptr [{b_data_ptr} + 64*7]",

            "kmovq k1, {limb0}",
            "kmovq k2, {limb1}",
            "kmovq k3, {limb2}",
            "kmovq k4, {limb3}",

            "vpxorq zmm0 {{k1}}, zmm0, zmm12",
            "vpxorq zmm1 {{k2}}, zmm1, zmm12",
            "vpxorq zmm2 {{k3}}, zmm2, zmm12",
            "vpxorq zmm3 {{k4}}, zmm3, zmm12",

            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm13",
            "vpxorq zmm1 {{k2}}, zmm1, zmm13",
            "vpxorq zmm2 {{k3}}, zmm2, zmm13",
            "vpxorq zmm3 {{k4}}, zmm3, zmm13",

            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm14",
            "vpxorq zmm1 {{k2}}, zmm1, zmm14",
            "vpxorq zmm2 {{k3}}, zmm2, zmm14",
            "vpxorq zmm3 {{k4}}, zmm3, zmm14",

            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm15",
            "vpxorq zmm1 {{k2}}, zmm1, zmm15",
            "vpxorq zmm2 {{k3}}, zmm2, zmm15",
            "vpxorq zmm3 {{k4}}, zmm3, zmm15",

            "kmovq k1, {limb4}",
            "kmovq k2, {limb5}",
            "kmovq k3, {limb6}",
            "kmovq k4, {limb7}",

            "vpxorq zmm4 {{k1}}, zmm4, zmm12",
            "vpxorq zmm5 {{k2}}, zmm5, zmm12",
            "vpxorq zmm6 {{k3}}, zmm6, zmm12",
            "vpxorq zmm7 {{k4}}, zmm7, zmm12",

            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "vpxorq zmm4 {{k1}}, zmm4, zmm13",
            "vpxorq zmm5 {{k2}}, zmm5, zmm13",
            "vpxorq zmm6 {{k3}}, zmm6, zmm13",
            "vpxorq zmm7 {{k4}}, zmm7, zmm13",

            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "vpxorq zmm4 {{k1}}, zmm4, zmm14",
            "vpxorq zmm5 {{k2}}, zmm5, zmm14",
            "vpxorq zmm6 {{k3}}, zmm6, zmm14",
            "vpxorq zmm7 {{k4}}, zmm7, zmm14",

            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "vpxorq zmm4 {{k1}}, zmm4, zmm15",
            "vpxorq zmm5 {{k2}}, zmm5, zmm15",
            "vpxorq zmm6 {{k3}}, zmm6, zmm15",
            "vpxorq zmm7 {{k4}}, zmm7, zmm15",

            "kmovq k1, {one}",

            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",

            "vpxorq zmm10 {{k1}}, zmm10, zmm0",
            "kshiftlq k1, k1, 1",
            "vpxorq zmm10 {{k1}}, zmm10, zmm1",
            "kshiftlq k1, k1, 1",

            "vpermq zmm8, zmm2, {permute1}",
            "vpermq zmm9, zmm3, {permute1}",
            "vpxorq zmm2, zmm2, zmm8",
            "vpxorq zmm3, zmm3, zmm9",
            "vpermq zmm8, zmm2, {permute2}",
            "vpermq zmm9, zmm3, {permute2}",
            "vpxorq zmm2, zmm2, zmm8",
            "vpxorq zmm3, zmm3, zmm9",
            "vshufi64x2 zmm8, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm9, zmm3, zmm3, {permute2}",
            "vpxorq zmm2, zmm2, zmm8",
            "vpxorq zmm3, zmm3, zmm9",

            "vpxorq zmm10 {{k1}}, zmm10, zmm2",
            "kshiftlq k1, k1, 1",
            "vpxorq zmm10 {{k1}}, zmm10, zmm3",
            "kshiftlq k1, k1, 1",

            "vpermq zmm8, zmm4, {permute1}",
            "vpermq zmm9, zmm5, {permute1}",
            "vpxorq zmm4, zmm4, zmm8",
            "vpxorq zmm5, zmm5, zmm9",
            "vpermq zmm8, zmm4, {permute2}",
            "vpermq zmm9, zmm5, {permute2}",
            "vpxorq zmm4, zmm4, zmm8",
            "vpxorq zmm5, zmm5, zmm9",
            "vshufi64x2 zmm8, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm9, zmm5, zmm5, {permute2}",
            "vpxorq zmm4, zmm4, zmm8",
            "vpxorq zmm5, zmm5, zmm9",

            "vpxorq zmm10 {{k1}}, zmm10, zmm4",
            "kshiftlq k1, k1, 1",
            "vpxorq zmm10 {{k1}}, zmm10, zmm5",
            "kshiftlq k1, k1, 1",

            "vpermq zmm8, zmm6, {permute1}",
            "vpermq zmm9, zmm7, {permute1}",
            "vpxorq zmm6, zmm6, zmm8",
            "vpxorq zmm7, zmm7, zmm9",
            "vpermq zmm8, zmm6, {permute2}",
            "vpermq zmm9, zmm7, {permute2}",
            "vpxorq zmm6, zmm6, zmm8",
            "vpxorq zmm7, zmm7, zmm9",
            "vshufi64x2 zmm8, zmm6, zmm6, {permute2}",
            "vshufi64x2 zmm9, zmm7, zmm7, {permute2}",
            "vpxorq zmm6, zmm6, zmm8",
            "vpxorq zmm7, zmm7, zmm9",

            "vpxorq zmm10 {{k1}}, zmm10, zmm6",
            "kshiftlq k1, k1, 1",
            "vpxorq zmm10 {{k1}}, zmm10, zmm7",

            "vmovdqa64 zmmword ptr [{c_data_ptr} + {zmm_idx}], zmm10",

            "add {limb_idx}, 8",
            "add {zmm_idx}, 64",
            "cmp {limb_idx}, 64",
            "jl 2b",

            permute1 = const 0b10110001, // Permutation for horizontal XOR
            permute2 = const 0b01001110, // Permutation for horizontal XOR

            // Constraints
            a_data_ptr = in(reg) a.limbs_ptr(),
            b_data_ptr = in(reg) b.limbs_ptr(),
            c_data_ptr = in(reg) c.limbs_mut_ptr(),
            one = in(reg) 1u64,

            // Counters
            limb_idx = out(reg) _,
            zmm_idx = out(reg) _,

            // Scratch registers
            limb0 = out(reg) _, limb1 = out(reg) _, limb2 = out(reg) _, limb3 = out(reg) _,
            limb4 = out(reg) _, limb5 = out(reg) _, limb6 = out(reg) _, limb7 = out(reg) _,

            // 4 k-registers for in-place rotation
            out("k1") _, out("k2") _, out("k3") _, out("k4") _,

            // ZMM registers
            out("zmm0") _, out("zmm1") _, out("zmm2") _, out("zmm3") _,     // Results 0-3
            out("zmm4") _, out("zmm5") _, out("zmm6") _, out("zmm7") _,     // Results 4-7
            out("zmm8") _, out("zmm9") _,                                   // Temps for horizontal XOR
            out("zmm10") _,                                                 // C[0], C[1], etc.
            out("zmm12") _, out("zmm13") _, out("zmm14") _, out("zmm15") _, // B[0-3] and B[4-7]

            options(nostack)
        )
    }
}

#[derive(Clone, Copy)]
#[repr(align(128))]
struct SimdBlock([SimdLimb; 8]);

impl SimdBlock {
    #[target_feature(enable = "avx512f")]
    fn zero() -> Self {
        Self([x86_64::_mm512_setzero_si512(); 8])
    }

    #[target_feature(enable = "avx512f")]
    fn as_matrix_block(&self) -> MatrixBlock {
        unsafe { std::mem::transmute::<Self, MatrixBlock>(*self) }
    }
}

#[target_feature(enable = "avx512f")]
pub unsafe fn gather_simd(slice: MatrixBlockSlice) -> MatrixBlock {
    let mut result = SimdBlock::zero();
    let offsets = unsafe { x86_64::_mm512_loadu_epi64(&UNIT_OFFSETS as *const i64) };
    let stride = x86_64::_mm512_set1_epi64(slice.stride() as i64);
    let offsets = unsafe { x86_64::_mm512_mullo_epi64(offsets, stride) };

    for i in 0..8 {
        let ptr = unsafe { slice.limbs().add(8 * i * slice.stride()) as *const i64 };
        result.0[i] = unsafe { x86_64::_mm512_i64gather_epi64::<8>(offsets, ptr) };
    }
    result.as_matrix_block()
}
