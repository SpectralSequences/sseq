use std::arch::x86_64;

use super::{MatrixBlock, MatrixBlockSlice, MatrixBlockSliceMut};

const UNIT_OFFSETS: [i64; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

/// Performs C = alpha * A * B + beta * C where A, B, C are 64x64 matrices
pub fn gemm_block_avx512_unrolled(
    alpha: bool,
    a: MatrixBlock,
    b: MatrixBlock,
    beta: bool,
    c: &mut MatrixBlockSliceMut,
) {
    if !beta {
        setzero_block_avx512(c);
    }

    if !alpha {
        return;
    }

    let mut c_zmms = gather_block_avx512(c.as_slice());

    unsafe {
        std::arch::asm!(
            // ===== SETUP: Load B matrix =====
            // Load all 8 rows of B matrix
            "vmovdqa64 zmm16, zmmword ptr [{b_data_ptr}]",        // B[0-7]
            "vmovdqa64 zmm17, zmmword ptr [{b_data_ptr} + 64]",   // B[8-15]
            "vmovdqa64 zmm18, zmmword ptr [{b_data_ptr} + 64*2]", // B[16-23]
            "vmovdqa64 zmm19, zmmword ptr [{b_data_ptr} + 64*3]", // B[24-31]
            "vmovdqa64 zmm20, zmmword ptr [{b_data_ptr} + 64*4]", // B[32-39]
            "vmovdqa64 zmm21, zmmword ptr [{b_data_ptr} + 64*5]", // B[40-47]
            "vmovdqa64 zmm22, zmmword ptr [{b_data_ptr} + 64*6]", // B[48-55]
            "vmovdqa64 zmm23, zmmword ptr [{b_data_ptr} + 64*7]", // B[56-63]

            // ===== 8-WAY PARALLEL PROCESSING =====

            // Iteration 0: Process limbs 0-7
            "mov {limb0}, [{a_data_ptr} + 0]",   // Load limb 0
            "mov {limb1}, [{a_data_ptr} + 8]",   // Load limb 1
            "mov {limb2}, [{a_data_ptr} + 16]",  // Load limb 2
            "mov {limb3}, [{a_data_ptr} + 24]",  // Load limb 3
            "mov {limb4}, [{a_data_ptr} + 32]",  // Load limb 4
            "mov {limb5}, [{a_data_ptr} + 40]",  // Load limb 5
            "mov {limb6}, [{a_data_ptr} + 48]",  // Load limb 6
            "mov {limb7}, [{a_data_ptr} + 56]",  // Load limb 7

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 0
            "kmovq k2, {limb1}",                 // k2 = limb 1
            "kmovq k3, {limb2}",                 // k3 = limb 2
            "kmovq k4, {limb3}",                 // k4 = limb 3
            "kmovq k5, {limb4}",                 // k5 = limb 4
            "kmovq k6, {limb5}",                 // k6 = limb 5
            "kmovq k7, {limb6}",                 // k7 = limb 6
            "kmovq k0, {limb7}",                 // k0 = limb 7

            // Initialize 8 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0
            "vpxorq zmm7, zmm7, zmm7",           // result[7] = 0

            // BYTE 0: Process byte 0 of all 8 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm7 {{k1}}, zmm7, zmm16",   // if (byte0 of limb7) result[7] ^= B[0]


            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k0, k0, 8",                // k0 >>= 8, byte 1 now in low position
            "kshiftrq k1, k1, 8",                // k1 >>= 8
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm7 {{k1}}, zmm7, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb7) result[7] ^= B[1]

            // BYTE 2: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm18",
            "vpxorq zmm1 {{k2}}, zmm1, zmm18",
            "vpxorq zmm2 {{k3}}, zmm2, zmm18",
            "vpxorq zmm3 {{k4}}, zmm3, zmm18",
            "vpxorq zmm4 {{k5}}, zmm4, zmm18",
            "vpxorq zmm5 {{k6}}, zmm5, zmm18",
            "vpxorq zmm6 {{k7}}, zmm6, zmm18",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm18",

            // BYTE 3: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",

            // BYTE 4: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm20",
            "vpxorq zmm1 {{k2}}, zmm1, zmm20",
            "vpxorq zmm2 {{k3}}, zmm2, zmm20",
            "vpxorq zmm3 {{k4}}, zmm3, zmm20",
            "vpxorq zmm4 {{k5}}, zmm4, zmm20",
            "vpxorq zmm5 {{k6}}, zmm5, zmm20",
            "vpxorq zmm6 {{k7}}, zmm6, zmm20",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm20",

            // BYTE 5: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",

            // BYTE 6: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm22",
            "vpxorq zmm1 {{k2}}, zmm1, zmm22",
            "vpxorq zmm2 {{k3}}, zmm2, zmm22",
            "vpxorq zmm3 {{k4}}, zmm3, zmm22",
            "vpxorq zmm4 {{k5}}, zmm4, zmm22",
            "vpxorq zmm5 {{k6}}, zmm5, zmm22",
            "vpxorq zmm6 {{k7}}, zmm6, zmm22",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm22",

            // BYTE 7: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 8 results =====
            // Step 1: Swap adjacent pairs for all 8 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpermq zmm15, zmm7, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpermq zmm15, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vshufi64x2 zmm15, zmm7, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Store all 8 results
            "kmovq k1, {one}",
            "vpxorq zmm24 {{k1}}, zmm24, zmm0", // C[0] ^= result[0]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm24 {{k1}}, zmm24, zmm1", // C[1] ^= result[1]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm24 {{k1}}, zmm24, zmm2", // C[2] ^= result[2]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm24 {{k1}}, zmm24, zmm3", // C[3] ^= result[3]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm24 {{k1}}, zmm24, zmm4", // C[4] ^= result[4]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm24 {{k1}}, zmm24, zmm5", // C[5] ^= result[5]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm24 {{k1}}, zmm24, zmm6", // C[6] ^= result[6]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm24 {{k1}}, zmm24, zmm7", // C[7] ^= result[7]

            // Iteration 1: Process limbs 8-15
            "mov {limb0}, [{a_data_ptr} + 64]",     // Load limb 8
            "mov {limb1}, [{a_data_ptr} + 72]",     // Load limb 9
            "mov {limb2}, [{a_data_ptr} + 80]",     // Load limb 10
            "mov {limb3}, [{a_data_ptr} + 88]",     // Load limb 11
            "mov {limb4}, [{a_data_ptr} + 96]",     // Load limb 12
            "mov {limb5}, [{a_data_ptr} + 104]",    // Load limb 13
            "mov {limb6}, [{a_data_ptr} + 112]",    // Load limb 14
            "mov {limb7}, [{a_data_ptr} + 120]",    // Load limb 15

            // Load all 8 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 8
            "kmovq k2, {limb1}",                 // k2 = limb 9
            "kmovq k3, {limb2}",                 // k3 = limb 10
            "kmovq k4, {limb3}",                 // k4 = limb 11
            "kmovq k5, {limb4}",                 // k5 = limb 12
            "kmovq k6, {limb5}",                 // k6 = limb 13
            "kmovq k7, {limb6}",                 // k7 = limb 14
            "kmovq k0, {limb7}",                 // k0 = limb 15

            // Initialize 8 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[8] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[9] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[10] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[11] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[12] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[13] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[14] = 0
            "vpxorq zmm7, zmm7, zmm7",           // result[15] = 0

            // BYTE 0: Process byte 0 of all 8 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm7 {{k1}}, zmm7, zmm16",   // if (byte0 of limb7) result[7] ^= B[0]


            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k0, k0, 8",                // k0 >>= 8, byte 1 now in low position
            "kshiftrq k1, k1, 8",                // k1 >>= 8
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm7 {{k1}}, zmm7, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb7) result[7] ^= B[1]

            // BYTE 2: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm18",
            "vpxorq zmm1 {{k2}}, zmm1, zmm18",
            "vpxorq zmm2 {{k3}}, zmm2, zmm18",
            "vpxorq zmm3 {{k4}}, zmm3, zmm18",
            "vpxorq zmm4 {{k5}}, zmm4, zmm18",
            "vpxorq zmm5 {{k6}}, zmm5, zmm18",
            "vpxorq zmm6 {{k7}}, zmm6, zmm18",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm18",

            // BYTE 3: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",

            // BYTE 4: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm20",
            "vpxorq zmm1 {{k2}}, zmm1, zmm20",
            "vpxorq zmm2 {{k3}}, zmm2, zmm20",
            "vpxorq zmm3 {{k4}}, zmm3, zmm20",
            "vpxorq zmm4 {{k5}}, zmm4, zmm20",
            "vpxorq zmm5 {{k6}}, zmm5, zmm20",
            "vpxorq zmm6 {{k7}}, zmm6, zmm20",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm20",

            // BYTE 5: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",

            // BYTE 6: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm22",
            "vpxorq zmm1 {{k2}}, zmm1, zmm22",
            "vpxorq zmm2 {{k3}}, zmm2, zmm22",
            "vpxorq zmm3 {{k4}}, zmm3, zmm22",
            "vpxorq zmm4 {{k5}}, zmm4, zmm22",
            "vpxorq zmm5 {{k6}}, zmm5, zmm22",
            "vpxorq zmm6 {{k7}}, zmm6, zmm22",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm22",

            // BYTE 7: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 8 results =====
            // Step 1: Swap adjacent pairs for all 8 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpermq zmm15, zmm7, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpermq zmm15, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vshufi64x2 zmm15, zmm7, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Store all 8 results
            "kmovq k1, {one}",
            "vpxorq zmm25 {{k1}}, zmm25, zmm0", // C[8] ^= result[8]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm25 {{k1}}, zmm25, zmm1", // C[9] ^= result[9]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm25 {{k1}}, zmm25, zmm2", // C[10] ^= result[10]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm25 {{k1}}, zmm25, zmm3", // C[11] ^= result[11]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm25 {{k1}}, zmm25, zmm4", // C[12] ^= result[12]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm25 {{k1}}, zmm25, zmm5", // C[13] ^= result[13]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm25 {{k1}}, zmm25, zmm6", // C[14] ^= result[14]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm25 {{k1}}, zmm25, zmm7", // C[15] ^= result[15]

            // Iteration 2: Process limbs 16-23
            "mov {limb0}, [{a_data_ptr} + 128]",  // Load limb 16
            "mov {limb1}, [{a_data_ptr} + 136]",  // Load limb 17
            "mov {limb2}, [{a_data_ptr} + 144]",  // Load limb 18
            "mov {limb3}, [{a_data_ptr} + 152]",  // Load limb 19
            "mov {limb4}, [{a_data_ptr} + 160]",  // Load limb 20
            "mov {limb5}, [{a_data_ptr} + 168]",  // Load limb 21
            "mov {limb6}, [{a_data_ptr} + 176]",  // Load limb 22
            "mov {limb7}, [{a_data_ptr} + 184]",  // Load limb 23

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 16
            "kmovq k2, {limb1}",                 // k2 = limb 17
            "kmovq k3, {limb2}",                 // k3 = limb 18
            "kmovq k4, {limb3}",                 // k4 = limb 19
            "kmovq k5, {limb4}",                 // k5 = limb 20
            "kmovq k6, {limb5}",                 // k6 = limb 21
            "kmovq k7, {limb6}",                 // k7 = limb 22
            "kmovq k0, {limb7}",                 // k0 = limb 23

            // Initialize 8 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[16] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[17] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[18] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[19] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[20] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[21] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[22] = 0
            "vpxorq zmm7, zmm7, zmm7",           // result[23] = 0

            // BYTE 0: Process byte 0 of all 8 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm7 {{k1}}, zmm7, zmm16",   // if (byte0 of limb7) result[7] ^= B[0]


            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k0, k0, 8",                // k0 >>= 8, byte 1 now in low position
            "kshiftrq k1, k1, 8",                // k1 >>= 8
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm7 {{k1}}, zmm7, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb7) result[7] ^= B[1]

            // BYTE 2: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm18",
            "vpxorq zmm1 {{k2}}, zmm1, zmm18",
            "vpxorq zmm2 {{k3}}, zmm2, zmm18",
            "vpxorq zmm3 {{k4}}, zmm3, zmm18",
            "vpxorq zmm4 {{k5}}, zmm4, zmm18",
            "vpxorq zmm5 {{k6}}, zmm5, zmm18",
            "vpxorq zmm6 {{k7}}, zmm6, zmm18",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm18",

            // BYTE 3: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",

            // BYTE 4: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm20",
            "vpxorq zmm1 {{k2}}, zmm1, zmm20",
            "vpxorq zmm2 {{k3}}, zmm2, zmm20",
            "vpxorq zmm3 {{k4}}, zmm3, zmm20",
            "vpxorq zmm4 {{k5}}, zmm4, zmm20",
            "vpxorq zmm5 {{k6}}, zmm5, zmm20",
            "vpxorq zmm6 {{k7}}, zmm6, zmm20",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm20",

            // BYTE 5: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",

            // BYTE 6: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm22",
            "vpxorq zmm1 {{k2}}, zmm1, zmm22",
            "vpxorq zmm2 {{k3}}, zmm2, zmm22",
            "vpxorq zmm3 {{k4}}, zmm3, zmm22",
            "vpxorq zmm4 {{k5}}, zmm4, zmm22",
            "vpxorq zmm5 {{k6}}, zmm5, zmm22",
            "vpxorq zmm6 {{k7}}, zmm6, zmm22",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm22",

            // BYTE 7: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 8 results =====
            // Step 1: Swap adjacent pairs for all 8 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpermq zmm15, zmm7, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpermq zmm15, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vshufi64x2 zmm15, zmm7, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Store all 8 results
            "kmovq k1, {one}",
            "vpxorq zmm26 {{k1}}, zmm26, zmm0", // C[16] ^= result[16]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm26 {{k1}}, zmm26, zmm1", // C[17] ^= result[17]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm26 {{k1}}, zmm26, zmm2", // C[18] ^= result[18]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm26 {{k1}}, zmm26, zmm3", // C[19] ^= result[19]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm26 {{k1}}, zmm26, zmm4", // C[20] ^= result[20]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm26 {{k1}}, zmm26, zmm5", // C[21] ^= result[21]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm26 {{k1}}, zmm26, zmm6", // C[22] ^= result[22]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm26 {{k1}}, zmm26, zmm7", // C[23] ^= result[23]

            // Iteration 3: Process limbs 24-31
            "mov {limb0}, [{a_data_ptr} + 192]",  // Load limb 24
            "mov {limb1}, [{a_data_ptr} + 200]",  // Load limb 25
            "mov {limb2}, [{a_data_ptr} + 208]",  // Load limb 26
            "mov {limb3}, [{a_data_ptr} + 216]",  // Load limb 27
            "mov {limb4}, [{a_data_ptr} + 224]",  // Load limb 28
            "mov {limb5}, [{a_data_ptr} + 232]",  // Load limb 29
            "mov {limb6}, [{a_data_ptr} + 240]",  // Load limb 30
            "mov {limb7}, [{a_data_ptr} + 248]",  // Load limb 31

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 24
            "kmovq k2, {limb1}",                 // k2 = limb 25
            "kmovq k3, {limb2}",                 // k3 = limb 26
            "kmovq k4, {limb3}",                 // k4 = limb 27
            "kmovq k5, {limb4}",                 // k5 = limb 28
            "kmovq k6, {limb5}",                 // k6 = limb 29
            "kmovq k7, {limb6}",                 // k7 = limb 30
            "kmovq k0, {limb7}",                 // k0 = limb 31

            // Initialize 8 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[24] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[25] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[26] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[27] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[28] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[29] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[30] = 0
            "vpxorq zmm7, zmm7, zmm7",           // result[31] = 0

            // BYTE 0: Process byte 0 of all 8 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm7 {{k1}}, zmm7, zmm16",   // if (byte0 of limb7) result[7] ^= B[0]


            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k0, k0, 8",                // k0 >>= 8, byte 1 now in low position
            "kshiftrq k1, k1, 8",                // k1 >>= 8
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm7 {{k1}}, zmm7, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb7) result[7] ^= B[1]

            // BYTE 2: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm18",
            "vpxorq zmm1 {{k2}}, zmm1, zmm18",
            "vpxorq zmm2 {{k3}}, zmm2, zmm18",
            "vpxorq zmm3 {{k4}}, zmm3, zmm18",
            "vpxorq zmm4 {{k5}}, zmm4, zmm18",
            "vpxorq zmm5 {{k6}}, zmm5, zmm18",
            "vpxorq zmm6 {{k7}}, zmm6, zmm18",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm18",

            // BYTE 3: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",

            // BYTE 4: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm20",
            "vpxorq zmm1 {{k2}}, zmm1, zmm20",
            "vpxorq zmm2 {{k3}}, zmm2, zmm20",
            "vpxorq zmm3 {{k4}}, zmm3, zmm20",
            "vpxorq zmm4 {{k5}}, zmm4, zmm20",
            "vpxorq zmm5 {{k6}}, zmm5, zmm20",
            "vpxorq zmm6 {{k7}}, zmm6, zmm20",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm20",

            // BYTE 5: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",

            // BYTE 6: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm22",
            "vpxorq zmm1 {{k2}}, zmm1, zmm22",
            "vpxorq zmm2 {{k3}}, zmm2, zmm22",
            "vpxorq zmm3 {{k4}}, zmm3, zmm22",
            "vpxorq zmm4 {{k5}}, zmm4, zmm22",
            "vpxorq zmm5 {{k6}}, zmm5, zmm22",
            "vpxorq zmm6 {{k7}}, zmm6, zmm22",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm22",

            // BYTE 7: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 8 results =====
            // Step 1: Swap adjacent pairs for all 8 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpermq zmm15, zmm7, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpermq zmm15, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vshufi64x2 zmm15, zmm7, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Store all 8 results
            "kmovq k1, {one}",
            "vpxorq zmm27 {{k1}}, zmm27, zmm0", // C[0] ^= result[0]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm27 {{k1}}, zmm27, zmm1", // C[1] ^= result[1]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm27 {{k1}}, zmm27, zmm2", // C[2] ^= result[2]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm27 {{k1}}, zmm27, zmm3", // C[3] ^= result[3]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm27 {{k1}}, zmm27, zmm4", // C[4] ^= result[4]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm27 {{k1}}, zmm27, zmm5", // C[5] ^= result[5]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm27 {{k1}}, zmm27, zmm6", // C[6] ^= result[6]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm27 {{k1}}, zmm27, zmm7", // C[7] ^= result[7]

            // Iteration 0: Process limbs 0-7
            "mov {limb0}, [{a_data_ptr} + 256]",   // Load limb 0
            "mov {limb1}, [{a_data_ptr} + 264]",   // Load limb 1
            "mov {limb2}, [{a_data_ptr} + 272]",  // Load limb 2
            "mov {limb3}, [{a_data_ptr} + 280]",  // Load limb 3
            "mov {limb4}, [{a_data_ptr} + 288]",  // Load limb 4
            "mov {limb5}, [{a_data_ptr} + 296]",  // Load limb 5
            "mov {limb6}, [{a_data_ptr} + 304]",  // Load limb 6
            "mov {limb7}, [{a_data_ptr} + 312]",  // Load limb 7

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 0
            "kmovq k2, {limb1}",                 // k2 = limb 1
            "kmovq k3, {limb2}",                 // k3 = limb 2
            "kmovq k4, {limb3}",                 // k4 = limb 3
            "kmovq k5, {limb4}",                 // k5 = limb 4
            "kmovq k6, {limb5}",                 // k6 = limb 5
            "kmovq k7, {limb6}",                 // k7 = limb 6
            "kmovq k0, {limb7}",                 // k0 = limb 7

            // Initialize 8 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0
            "vpxorq zmm7, zmm7, zmm7",           // result[7] = 0

            // BYTE 0: Process byte 0 of all 8 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm7 {{k1}}, zmm7, zmm16",   // if (byte0 of limb7) result[7] ^= B[0]


            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k0, k0, 8",                // k0 >>= 8, byte 1 now in low position
            "kshiftrq k1, k1, 8",                // k1 >>= 8
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm7 {{k1}}, zmm7, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb7) result[7] ^= B[1]

            // BYTE 2: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm18",
            "vpxorq zmm1 {{k2}}, zmm1, zmm18",
            "vpxorq zmm2 {{k3}}, zmm2, zmm18",
            "vpxorq zmm3 {{k4}}, zmm3, zmm18",
            "vpxorq zmm4 {{k5}}, zmm4, zmm18",
            "vpxorq zmm5 {{k6}}, zmm5, zmm18",
            "vpxorq zmm6 {{k7}}, zmm6, zmm18",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm18",

            // BYTE 3: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",

            // BYTE 4: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm20",
            "vpxorq zmm1 {{k2}}, zmm1, zmm20",
            "vpxorq zmm2 {{k3}}, zmm2, zmm20",
            "vpxorq zmm3 {{k4}}, zmm3, zmm20",
            "vpxorq zmm4 {{k5}}, zmm4, zmm20",
            "vpxorq zmm5 {{k6}}, zmm5, zmm20",
            "vpxorq zmm6 {{k7}}, zmm6, zmm20",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm20",

            // BYTE 5: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",

            // BYTE 6: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm22",
            "vpxorq zmm1 {{k2}}, zmm1, zmm22",
            "vpxorq zmm2 {{k3}}, zmm2, zmm22",
            "vpxorq zmm3 {{k4}}, zmm3, zmm22",
            "vpxorq zmm4 {{k5}}, zmm4, zmm22",
            "vpxorq zmm5 {{k6}}, zmm5, zmm22",
            "vpxorq zmm6 {{k7}}, zmm6, zmm22",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm22",

            // BYTE 7: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 8 results =====
            // Step 1: Swap adjacent pairs for all 8 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpermq zmm15, zmm7, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpermq zmm15, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vshufi64x2 zmm15, zmm7, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Store all 8 results
            "kmovq k1, {one}",
            "vpxorq zmm28 {{k1}}, zmm28, zmm0", // C[0] ^= result[0]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm28 {{k1}}, zmm28, zmm1", // C[1] ^= result[1]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm28 {{k1}}, zmm28, zmm2", // C[2] ^= result[2]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm28 {{k1}}, zmm28, zmm3", // C[3] ^= result[3]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm28 {{k1}}, zmm28, zmm4", // C[4] ^= result[4]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm28 {{k1}}, zmm28, zmm5", // C[5] ^= result[5]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm28 {{k1}}, zmm28, zmm6", // C[6] ^= result[6]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm28 {{k1}}, zmm28, zmm7", // C[7] ^= result[7]

            // Iteration 0: Process limbs 0-7
            "mov {limb0}, [{a_data_ptr} + 320]",   // Load limb 0
            "mov {limb1}, [{a_data_ptr} + 328]",   // Load limb 1
            "mov {limb2}, [{a_data_ptr} + 336]",  // Load limb 2
            "mov {limb3}, [{a_data_ptr} + 344]",  // Load limb 3
            "mov {limb4}, [{a_data_ptr} + 352]",  // Load limb 4
            "mov {limb5}, [{a_data_ptr} + 360]",  // Load limb 5
            "mov {limb6}, [{a_data_ptr} + 368]",  // Load limb 6
            "mov {limb7}, [{a_data_ptr} + 376]",  // Load limb 7

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 0
            "kmovq k2, {limb1}",                 // k2 = limb 1
            "kmovq k3, {limb2}",                 // k3 = limb 2
            "kmovq k4, {limb3}",                 // k4 = limb 3
            "kmovq k5, {limb4}",                 // k5 = limb 4
            "kmovq k6, {limb5}",                 // k6 = limb 5
            "kmovq k7, {limb6}",                 // k7 = limb 6
            "kmovq k0, {limb7}",                 // k0 = limb 7

            // Initialize 8 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0
            "vpxorq zmm7, zmm7, zmm7",           // result[7] = 0

            // BYTE 0: Process byte 0 of all 8 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm7 {{k1}}, zmm7, zmm16",   // if (byte0 of limb7) result[7] ^= B[0]


            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k0, k0, 8",                // k0 >>= 8, byte 1 now in low position
            "kshiftrq k1, k1, 8",                // k1 >>= 8
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm7 {{k1}}, zmm7, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb7) result[7] ^= B[1]

            // BYTE 2: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm18",
            "vpxorq zmm1 {{k2}}, zmm1, zmm18",
            "vpxorq zmm2 {{k3}}, zmm2, zmm18",
            "vpxorq zmm3 {{k4}}, zmm3, zmm18",
            "vpxorq zmm4 {{k5}}, zmm4, zmm18",
            "vpxorq zmm5 {{k6}}, zmm5, zmm18",
            "vpxorq zmm6 {{k7}}, zmm6, zmm18",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm18",

            // BYTE 3: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",

            // BYTE 4: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm20",
            "vpxorq zmm1 {{k2}}, zmm1, zmm20",
            "vpxorq zmm2 {{k3}}, zmm2, zmm20",
            "vpxorq zmm3 {{k4}}, zmm3, zmm20",
            "vpxorq zmm4 {{k5}}, zmm4, zmm20",
            "vpxorq zmm5 {{k6}}, zmm5, zmm20",
            "vpxorq zmm6 {{k7}}, zmm6, zmm20",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm20",

            // BYTE 5: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",

            // BYTE 6: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm22",
            "vpxorq zmm1 {{k2}}, zmm1, zmm22",
            "vpxorq zmm2 {{k3}}, zmm2, zmm22",
            "vpxorq zmm3 {{k4}}, zmm3, zmm22",
            "vpxorq zmm4 {{k5}}, zmm4, zmm22",
            "vpxorq zmm5 {{k6}}, zmm5, zmm22",
            "vpxorq zmm6 {{k7}}, zmm6, zmm22",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm22",

            // BYTE 7: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 8 results =====
            // Step 1: Swap adjacent pairs for all 8 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpermq zmm15, zmm7, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpermq zmm15, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vshufi64x2 zmm15, zmm7, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Store all 8 results
            "kmovq k1, {one}",
            "vpxorq zmm29 {{k1}}, zmm29, zmm0", // C[0] ^= result[0]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm29 {{k1}}, zmm29, zmm1", // C[1] ^= result[1]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm29 {{k1}}, zmm29, zmm2", // C[2] ^= result[2]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm29 {{k1}}, zmm29, zmm3", // C[3] ^= result[3]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm29 {{k1}}, zmm29, zmm4", // C[4] ^= result[4]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm29 {{k1}}, zmm29, zmm5", // C[5] ^= result[5]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm29 {{k1}}, zmm29, zmm6", // C[6] ^= result[6]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm29 {{k1}}, zmm29, zmm7", // C[7] ^= result[7]

            // Iteration 0: Process limbs 0-7
            "mov {limb0}, [{a_data_ptr} + 384]",   // Load limb 0
            "mov {limb1}, [{a_data_ptr} + 392]",   // Load limb 1
            "mov {limb2}, [{a_data_ptr} + 400]",  // Load limb 2
            "mov {limb3}, [{a_data_ptr} + 408]",  // Load limb 3
            "mov {limb4}, [{a_data_ptr} + 416]",  // Load limb 4
            "mov {limb5}, [{a_data_ptr} + 424]",  // Load limb 5
            "mov {limb6}, [{a_data_ptr} + 432]",  // Load limb 6
            "mov {limb7}, [{a_data_ptr} + 440]",  // Load limb 7

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 0
            "kmovq k2, {limb1}",                 // k2 = limb 1
            "kmovq k3, {limb2}",                 // k3 = limb 2
            "kmovq k4, {limb3}",                 // k4 = limb 3
            "kmovq k5, {limb4}",                 // k5 = limb 4
            "kmovq k6, {limb5}",                 // k6 = limb 5
            "kmovq k7, {limb6}",                 // k7 = limb 6
            "kmovq k0, {limb7}",                 // k0 = limb 7

            // Initialize 8 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0
            "vpxorq zmm7, zmm7, zmm7",           // result[7] = 0

            // BYTE 0: Process byte 0 of all 8 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm7 {{k1}}, zmm7, zmm16",   // if (byte0 of limb7) result[7] ^= B[0]


            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k0, k0, 8",                // k0 >>= 8, byte 1 now in low position
            "kshiftrq k1, k1, 8",                // k1 >>= 8
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm7 {{k1}}, zmm7, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb7) result[7] ^= B[1]

            // BYTE 2: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm18",
            "vpxorq zmm1 {{k2}}, zmm1, zmm18",
            "vpxorq zmm2 {{k3}}, zmm2, zmm18",
            "vpxorq zmm3 {{k4}}, zmm3, zmm18",
            "vpxorq zmm4 {{k5}}, zmm4, zmm18",
            "vpxorq zmm5 {{k6}}, zmm5, zmm18",
            "vpxorq zmm6 {{k7}}, zmm6, zmm18",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm18",

            // BYTE 3: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",

            // BYTE 4: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm20",
            "vpxorq zmm1 {{k2}}, zmm1, zmm20",
            "vpxorq zmm2 {{k3}}, zmm2, zmm20",
            "vpxorq zmm3 {{k4}}, zmm3, zmm20",
            "vpxorq zmm4 {{k5}}, zmm4, zmm20",
            "vpxorq zmm5 {{k6}}, zmm5, zmm20",
            "vpxorq zmm6 {{k7}}, zmm6, zmm20",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm20",

            // BYTE 5: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",

            // BYTE 6: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm22",
            "vpxorq zmm1 {{k2}}, zmm1, zmm22",
            "vpxorq zmm2 {{k3}}, zmm2, zmm22",
            "vpxorq zmm3 {{k4}}, zmm3, zmm22",
            "vpxorq zmm4 {{k5}}, zmm4, zmm22",
            "vpxorq zmm5 {{k6}}, zmm5, zmm22",
            "vpxorq zmm6 {{k7}}, zmm6, zmm22",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm22",

            // BYTE 7: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 8 results =====
            // Step 1: Swap adjacent pairs for all 8 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpermq zmm15, zmm7, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpermq zmm15, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vshufi64x2 zmm15, zmm7, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Store all 8 results
            "kmovq k1, {one}",
            "vpxorq zmm30 {{k1}}, zmm30, zmm0", // C[0] ^= result[0]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm30 {{k1}}, zmm30, zmm1", // C[1] ^= result[1]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm30 {{k1}}, zmm30, zmm2", // C[2] ^= result[2]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm30 {{k1}}, zmm30, zmm3", // C[3] ^= result[3]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm30 {{k1}}, zmm30, zmm4", // C[4] ^= result[4]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm30 {{k1}}, zmm30, zmm5", // C[5] ^= result[5]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm30 {{k1}}, zmm30, zmm6", // C[6] ^= result[6]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm30 {{k1}}, zmm30, zmm7", // C[7] ^= result[7]

            // Iteration 0: Process limbs 0-7
            "mov {limb0}, [{a_data_ptr} + 448]",   // Load limb 0
            "mov {limb1}, [{a_data_ptr} + 456]",   // Load limb 1
            "mov {limb2}, [{a_data_ptr} + 464]",  // Load limb 2
            "mov {limb3}, [{a_data_ptr} + 472]",  // Load limb 3
            "mov {limb4}, [{a_data_ptr} + 480]",  // Load limb 4
            "mov {limb5}, [{a_data_ptr} + 488]",  // Load limb 5
            "mov {limb6}, [{a_data_ptr} + 496]",  // Load limb 6
            "mov {limb7}, [{a_data_ptr} + 504]",  // Load limb 7

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 0
            "kmovq k2, {limb1}",                 // k2 = limb 1
            "kmovq k3, {limb2}",                 // k3 = limb 2
            "kmovq k4, {limb3}",                 // k4 = limb 3
            "kmovq k5, {limb4}",                 // k5 = limb 4
            "kmovq k6, {limb5}",                 // k6 = limb 5
            "kmovq k7, {limb6}",                 // k7 = limb 6
            "kmovq k0, {limb7}",                 // k0 = limb 7

            // Initialize 8 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0
            "vpxorq zmm7, zmm7, zmm7",           // result[7] = 0

            // BYTE 0: Process byte 0 of all 8 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm7 {{k1}}, zmm7, zmm16",   // if (byte0 of limb7) result[7] ^= B[0]


            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k0, k0, 8",                // k0 >>= 8, byte 1 now in low position
            "kshiftrq k1, k1, 8",                // k1 >>= 8
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm7 {{k1}}, zmm7, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]
            "kxorq k0, k0, k1",                  // |
            "kxorq k1, k1, k0",                  // | Swap k0 and k1
            "kxorq k0, k0, k1",                  // |
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb7) result[7] ^= B[1]

            // BYTE 2: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm18",
            "vpxorq zmm1 {{k2}}, zmm1, zmm18",
            "vpxorq zmm2 {{k3}}, zmm2, zmm18",
            "vpxorq zmm3 {{k4}}, zmm3, zmm18",
            "vpxorq zmm4 {{k5}}, zmm4, zmm18",
            "vpxorq zmm5 {{k6}}, zmm5, zmm18",
            "vpxorq zmm6 {{k7}}, zmm6, zmm18",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm18",

            // BYTE 3: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",

            // BYTE 4: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm20",
            "vpxorq zmm1 {{k2}}, zmm1, zmm20",
            "vpxorq zmm2 {{k3}}, zmm2, zmm20",
            "vpxorq zmm3 {{k4}}, zmm3, zmm20",
            "vpxorq zmm4 {{k5}}, zmm4, zmm20",
            "vpxorq zmm5 {{k6}}, zmm5, zmm20",
            "vpxorq zmm6 {{k7}}, zmm6, zmm20",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm20",

            // BYTE 5: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",

            // BYTE 6: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm22",
            "vpxorq zmm1 {{k2}}, zmm1, zmm22",
            "vpxorq zmm2 {{k3}}, zmm2, zmm22",
            "vpxorq zmm3 {{k4}}, zmm3, zmm22",
            "vpxorq zmm4 {{k5}}, zmm4, zmm22",
            "vpxorq zmm5 {{k6}}, zmm5, zmm22",
            "vpxorq zmm6 {{k7}}, zmm6, zmm22",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm7 {{k1}}, zmm7, zmm22",

            // BYTE 7: Rotate and process
            "kshiftrq k0, k0, 8",
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm7 {{k1}}, zmm7, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",
            "kxorq k0, k0, k1",
            "kxorq k1, k1, k0",
            "kxorq k0, k0, k1",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 8 results =====
            // Step 1: Swap adjacent pairs for all 8 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpermq zmm15, zmm7, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpermq zmm15, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vshufi64x2 zmm15, zmm7, zmm7, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",
            "vpxorq zmm7, zmm7, zmm15",

            // Store all 8 results
            "kmovq k1, {one}",
            "vpxorq zmm31 {{k1}}, zmm31, zmm0", // C[0] ^= result[0]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm31 {{k1}}, zmm31, zmm1", // C[1] ^= result[1]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm31 {{k1}}, zmm31, zmm2", // C[2] ^= result[2]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm31 {{k1}}, zmm31, zmm3", // C[3] ^= result[3]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm31 {{k1}}, zmm31, zmm4", // C[4] ^= result[4]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm31 {{k1}}, zmm31, zmm5", // C[5] ^= result[5]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm31 {{k1}}, zmm31, zmm6", // C[6] ^= result[6]
            "kshiftlq k1, k1, 1",
            "vpxorq zmm31 {{k1}}, zmm31, zmm7", // C[7] ^= result[7]

            permute1 = const 0b10110001, // Permutation for horizontal XOR
            permute2 = const 0b01001110, // Permutation for horizontal XOR

            // Constraints
            a_data_ptr = in(reg) a.limbs.as_ptr(),
            b_data_ptr = in(reg) b.limbs.as_ptr(),
            one = in(reg) 1u64, // Used for k-registers

            // Scratch registers
            limb0 = out(reg) _, limb1 = out(reg) _, limb2 = out(reg) _, limb3 = out(reg) _,
            limb4 = out(reg) _, limb5 = out(reg) _, limb6 = out(reg) _, limb7 = out(reg) _,

            // 7 k-registers for in-place rotation (avoiding k0)
            out("k1") _, out("k2") _, out("k3") _, out("k4") _,
            out("k5") _, out("k6") _, out("k7") _,

            // ZMM registers: 8 for B matrix + 16 for results and temps = 24 total
            out("zmm0") _, out("zmm1") _, out("zmm2") _, out("zmm3") _,   // Results 0-3
            out("zmm4") _, out("zmm5") _, out("zmm6") _, out("zmm7") _,   // Results 4-7
            out("zmm8") _, out("zmm9") _, out("zmm10") _, out("zmm11") _, // Temps for horizontal XOR
            out("zmm12") _, out("zmm13") _, out("zmm14") _, out("zmm15") _, // More temps
            out("zmm16") _, out("zmm17") _, out("zmm18") _, out("zmm19") _, // B[0]-B[3]
            out("zmm20") _, out("zmm21") _, out("zmm22") _, out("zmm23") _, // B[4]-B[7]

            inout("zmm24") c_zmms.0[0], inout("zmm25") c_zmms.0[1],
            inout("zmm26") c_zmms.0[2], inout("zmm27") c_zmms.0[3],
            inout("zmm28") c_zmms.0[4], inout("zmm29") c_zmms.0[5],
            inout("zmm30") c_zmms.0[6], inout("zmm31") c_zmms.0[7],

            options(nostack, preserves_flags)
        )
    }

    scatter_block_avx512(c, c_zmms);
}

// pub fn gemm_block_avx512(
//     alpha: bool,
//     a: MatrixBlockSlice,
//     b: MatrixBlockSlice,
//     beta: bool,
//     c: &mut MatrixBlockSliceMut,
// ) {
//     if !beta {
//         setzero_block_avx512(c);
//     }

//     if !alpha {
//         return;
//     }

//     let a = a.to_owned();
//     let b = b.to_owned();
//     let mut c_zmms = gather_block_avx512(c.as_slice());

//     unsafe {
//         std::arch::asm!(
//             // ===== SETUP: Load B matrix =====
//             // Load all 8 rows of B matrix
//             "vmovdqu64 zmm16, zmmword ptr [{b_data_ptr}]",        // B[0-7]
//             "vmovdqu64 zmm17, zmmword ptr [{b_data_ptr} + 64]",   // B[8-15]
//             "vmovdqu64 zmm18, zmmword ptr [{b_data_ptr} + 64*2]", // B[16-23]
//             "vmovdqu64 zmm19, zmmword ptr [{b_data_ptr} + 64*3]", // B[24-31]
//             "vmovdqu64 zmm20, zmmword ptr [{b_data_ptr} + 64*4]", // B[32-39]
//             "vmovdqu64 zmm21, zmmword ptr [{b_data_ptr} + 64*5]", // B[40-47]
//             "vmovdqu64 zmm22, zmmword ptr [{b_data_ptr} + 64*6]", // B[48-55]
//             "vmovdqu64 zmm23, zmmword ptr [{b_data_ptr} + 64*7]", // B[56-63]

//             // Load all of matrix C for safekeeping
//             "vmovdqu64 zmm24, zmmword ptr [{c_data_ptr}]",        // C[0-7]
//             "vmovdqu64 zmm25, zmmword ptr [{c_data_ptr} + 64]",   // C[8-15]
//             "vmovdqu64 zmm26, zmmword ptr [{c_data_ptr} + 64*2]", // C[16-23]
//             "vmovdqu64 zmm27, zmmword ptr [{c_data_ptr} + 64*3]", // C[24-31]
//             "vmovdqu64 zmm28, zmmword ptr [{c_data_ptr} + 64*4]", // C[32-39]
//             "vmovdqu64 zmm29, zmmword ptr [{c_data_ptr} + 64*5]", // C[40-47]
//             "vmovdqu64 zmm30, zmmword ptr [{c_data_ptr} + 64*6]", // C[48-55]
//             "vmovdqu64 zmm31, zmmword ptr [{c_data_ptr} + 64*7]", // C[56-63]

//             // ===== 8-WAY PARALLEL PROCESSING =====

//             "mov {limb_idx}, 0",
//             "2:",

//             // Iteration 0: Process limbs 0-7
//             "mov {limb0}, [{a_data_ptr} + 8*{limb_idx} + 0]",   // Load limb 0
//             "mov {limb1}, [{a_data_ptr} + 8*{limb_idx} + 8]",   // Load limb 1
//             "mov {limb2}, [{a_data_ptr} + 8*{limb_idx} + 16]",  // Load limb 2
//             "mov {limb3}, [{a_data_ptr} + 8*{limb_idx} + 24]",  // Load limb 3
//             "mov {limb4}, [{a_data_ptr} + 8*{limb_idx} + 32]",  // Load limb 4
//             "mov {limb5}, [{a_data_ptr} + 8*{limb_idx} + 40]",  // Load limb 5
//             "mov {limb6}, [{a_data_ptr} + 8*{limb_idx} + 48]",  // Load limb 6
//             "mov {limb7}, [{a_data_ptr} + 8*{limb_idx} + 56]",  // Load limb 7

//             // Load all 7 limbs into k-registers (these will be rotated in-place)
//             "kmovq k1, {limb0}",                 // k1 = limb 0
//             "kmovq k2, {limb1}",                 // k2 = limb 1
//             "kmovq k3, {limb2}",                 // k3 = limb 2
//             "kmovq k4, {limb3}",                 // k4 = limb 3
//             "kmovq k5, {limb4}",                 // k5 = limb 4
//             "kmovq k6, {limb5}",                 // k6 = limb 5
//             "kmovq k7, {limb6}",                 // k7 = limb 6
//             "kmovq k0, {limb7}",                 // k0 = limb 7

//             // Initialize 8 result registers
//             "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
//             "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
//             "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
//             "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
//             "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
//             "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
//             "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0
//             "vpxorq zmm7, zmm7, zmm7",           // result[7] = 0

//             // BYTE 0: Process byte 0 of all 8 limbs (already in low position)
//             "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
//             "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
//             "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
//             "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
//             "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
//             "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
//             "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]
//             "kxorq k0, k0, k1",                  // |
//             "kxorq k1, k1, k0",                  // | Swap k0 and k1
//             "kxorq k0, k0, k1",                  // |
//             "vpxorq zmm7 {{k1}}, zmm7, zmm16",   // if (byte0 of limb7) result[7] ^= B[0]

//             // BYTE 1: Rotate all masks right by 8 bits, then process
//             "kshiftrq k0, k0, 8",                // k0 >>= 8, byte 1 now in low position
//             "kshiftrq k1, k1, 8",                // k1 >>= 8
//             "kshiftrq k2, k2, 8",                // k2 >>= 8
//             "kshiftrq k3, k3, 8",                // k3 >>= 8
//             "kshiftrq k4, k4, 8",                // k4 >>= 8
//             "kshiftrq k5, k5, 8",                // k5 >>= 8
//             "kshiftrq k6, k6, 8",                // k6 >>= 8
//             "kshiftrq k7, k7, 8",                // k7 >>= 8
//             "vpxorq zmm7 {{k1}}, zmm7, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
//             "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
//             "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
//             "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
//             "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
//             "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
//             "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]
//             "kxorq k0, k0, k1",                  // |
//             "kxorq k1, k1, k0",                  // | Swap k0 and k1
//             "kxorq k0, k0, k1",                  // |
//             "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb7) result[7] ^= B[1]

//             // BYTE 2: Rotate and process
//             "kshiftrq k0, k0, 8",
//             "kshiftrq k1, k1, 8",
//             "kshiftrq k2, k2, 8",
//             "kshiftrq k3, k3, 8",
//             "kshiftrq k4, k4, 8",
//             "kshiftrq k5, k5, 8",
//             "kshiftrq k6, k6, 8",
//             "kshiftrq k7, k7, 8",
//             "vpxorq zmm0 {{k1}}, zmm0, zmm18",
//             "vpxorq zmm1 {{k2}}, zmm1, zmm18",
//             "vpxorq zmm2 {{k3}}, zmm2, zmm18",
//             "vpxorq zmm3 {{k4}}, zmm3, zmm18",
//             "vpxorq zmm4 {{k5}}, zmm4, zmm18",
//             "vpxorq zmm5 {{k6}}, zmm5, zmm18",
//             "vpxorq zmm6 {{k7}}, zmm6, zmm18",
//             "kxorq k0, k0, k1",
//             "kxorq k1, k1, k0",
//             "kxorq k0, k0, k1",
//             "vpxorq zmm7 {{k1}}, zmm7, zmm18",

//             // BYTE 3: Rotate and process
//             "kshiftrq k0, k0, 8",
//             "kshiftrq k1, k1, 8",
//             "kshiftrq k2, k2, 8",
//             "kshiftrq k3, k3, 8",
//             "kshiftrq k4, k4, 8",
//             "kshiftrq k5, k5, 8",
//             "kshiftrq k6, k6, 8",
//             "kshiftrq k7, k7, 8",
//             "vpxorq zmm7 {{k1}}, zmm7, zmm19",
//             "vpxorq zmm1 {{k2}}, zmm1, zmm19",
//             "vpxorq zmm2 {{k3}}, zmm2, zmm19",
//             "vpxorq zmm3 {{k4}}, zmm3, zmm19",
//             "vpxorq zmm4 {{k5}}, zmm4, zmm19",
//             "vpxorq zmm5 {{k6}}, zmm5, zmm19",
//             "vpxorq zmm6 {{k7}}, zmm6, zmm19",
//             "kxorq k0, k0, k1",
//             "kxorq k1, k1, k0",
//             "kxorq k0, k0, k1",
//             "vpxorq zmm0 {{k1}}, zmm0, zmm19",

//             // BYTE 4: Rotate and process
//             "kshiftrq k0, k0, 8",
//             "kshiftrq k1, k1, 8",
//             "kshiftrq k2, k2, 8",
//             "kshiftrq k3, k3, 8",
//             "kshiftrq k4, k4, 8",
//             "kshiftrq k5, k5, 8",
//             "kshiftrq k6, k6, 8",
//             "kshiftrq k7, k7, 8",
//             "vpxorq zmm0 {{k1}}, zmm0, zmm20",
//             "vpxorq zmm1 {{k2}}, zmm1, zmm20",
//             "vpxorq zmm2 {{k3}}, zmm2, zmm20",
//             "vpxorq zmm3 {{k4}}, zmm3, zmm20",
//             "vpxorq zmm4 {{k5}}, zmm4, zmm20",
//             "vpxorq zmm5 {{k6}}, zmm5, zmm20",
//             "vpxorq zmm6 {{k7}}, zmm6, zmm20",
//             "kxorq k0, k0, k1",
//             "kxorq k1, k1, k0",
//             "kxorq k0, k0, k1",
//             "vpxorq zmm7 {{k1}}, zmm7, zmm20",

//             // BYTE 5: Rotate and process
//             "kshiftrq k0, k0, 8",
//             "kshiftrq k1, k1, 8",
//             "kshiftrq k2, k2, 8",
//             "kshiftrq k3, k3, 8",
//             "kshiftrq k4, k4, 8",
//             "kshiftrq k5, k5, 8",
//             "kshiftrq k6, k6, 8",
//             "kshiftrq k7, k7, 8",
//             "vpxorq zmm7 {{k1}}, zmm7, zmm21",
//             "vpxorq zmm1 {{k2}}, zmm1, zmm21",
//             "vpxorq zmm2 {{k3}}, zmm2, zmm21",
//             "vpxorq zmm3 {{k4}}, zmm3, zmm21",
//             "vpxorq zmm4 {{k5}}, zmm4, zmm21",
//             "vpxorq zmm5 {{k6}}, zmm5, zmm21",
//             "vpxorq zmm6 {{k7}}, zmm6, zmm21",
//             "kxorq k0, k0, k1",
//             "kxorq k1, k1, k0",
//             "kxorq k0, k0, k1",
//             "vpxorq zmm0 {{k1}}, zmm0, zmm21",

//             // BYTE 6: Rotate and process
//             "kshiftrq k0, k0, 8",
//             "kshiftrq k1, k1, 8",
//             "kshiftrq k2, k2, 8",
//             "kshiftrq k3, k3, 8",
//             "kshiftrq k4, k4, 8",
//             "kshiftrq k5, k5, 8",
//             "kshiftrq k6, k6, 8",
//             "kshiftrq k7, k7, 8",
//             "vpxorq zmm0 {{k1}}, zmm0, zmm22",
//             "vpxorq zmm1 {{k2}}, zmm1, zmm22",
//             "vpxorq zmm2 {{k3}}, zmm2, zmm22",
//             "vpxorq zmm3 {{k4}}, zmm3, zmm22",
//             "vpxorq zmm4 {{k5}}, zmm4, zmm22",
//             "vpxorq zmm5 {{k6}}, zmm5, zmm22",
//             "vpxorq zmm6 {{k7}}, zmm6, zmm22",
//             "kxorq k0, k0, k1",
//             "kxorq k1, k1, k0",
//             "kxorq k0, k0, k1",
//             "vpxorq zmm7 {{k1}}, zmm7, zmm22",

//             // BYTE 7: Rotate and process
//             "kshiftrq k0, k0, 8",
//             "kshiftrq k1, k1, 8",
//             "kshiftrq k2, k2, 8",
//             "kshiftrq k3, k3, 8",
//             "kshiftrq k4, k4, 8",
//             "kshiftrq k5, k5, 8",
//             "kshiftrq k6, k6, 8",
//             "kshiftrq k7, k7, 8",
//             "vpxorq zmm7 {{k1}}, zmm7, zmm23",
//             "vpxorq zmm1 {{k2}}, zmm1, zmm23",
//             "vpxorq zmm2 {{k3}}, zmm2, zmm23",
//             "vpxorq zmm3 {{k4}}, zmm3, zmm23",
//             "vpxorq zmm4 {{k5}}, zmm4, zmm23",
//             "vpxorq zmm5 {{k6}}, zmm5, zmm23",
//             "vpxorq zmm6 {{k7}}, zmm6, zmm23",
//             "kxorq k0, k0, k1",
//             "kxorq k1, k1, k0",
//             "kxorq k0, k0, k1",
//             "vpxorq zmm0 {{k1}}, zmm0, zmm23",

//             // ===== PARALLEL HORIZONTAL XOR for all 8 results =====
//             // Step 1: Swap adjacent pairs for all 8 results simultaneously
//             "vpermq zmm8, zmm0, {permute1}",
//             "vpermq zmm9, zmm1, {permute1}",
//             "vpermq zmm10, zmm2, {permute1}",
//             "vpermq zmm11, zmm3, {permute1}",
//             "vpermq zmm12, zmm4, {permute1}",
//             "vpermq zmm13, zmm5, {permute1}",
//             "vpermq zmm14, zmm6, {permute1}",
//             "vpermq zmm15, zmm7, {permute1}",
//             "vpxorq zmm0, zmm0, zmm8",
//             "vpxorq zmm1, zmm1, zmm9",
//             "vpxorq zmm2, zmm2, zmm10",
//             "vpxorq zmm3, zmm3, zmm11",
//             "vpxorq zmm4, zmm4, zmm12",
//             "vpxorq zmm5, zmm5, zmm13",
//             "vpxorq zmm6, zmm6, zmm14",
//             "vpxorq zmm7, zmm7, zmm15",

//             // Step 2: Swap quads for all 7 results simultaneously
//             "vpermq zmm8, zmm0, {permute2}",
//             "vpermq zmm9, zmm1, {permute2}",
//             "vpermq zmm10, zmm2, {permute2}",
//             "vpermq zmm11, zmm3, {permute2}",
//             "vpermq zmm12, zmm4, {permute2}",
//             "vpermq zmm13, zmm5, {permute2}",
//             "vpermq zmm14, zmm6, {permute2}",
//             "vpermq zmm15, zmm7, {permute2}",
//             "vpxorq zmm0, zmm0, zmm8",
//             "vpxorq zmm1, zmm1, zmm9",
//             "vpxorq zmm2, zmm2, zmm10",
//             "vpxorq zmm3, zmm3, zmm11",
//             "vpxorq zmm4, zmm4, zmm12",
//             "vpxorq zmm5, zmm5, zmm13",
//             "vpxorq zmm6, zmm6, zmm14",
//             "vpxorq zmm7, zmm7, zmm15",

//             // Step 3: Swap halves for all 7 results simultaneously
//             "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
//             "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
//             "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
//             "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
//             "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
//             "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
//             "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
//             "vshufi64x2 zmm15, zmm7, zmm7, {permute2}",
//             "vpxorq zmm0, zmm0, zmm8",
//             "vpxorq zmm1, zmm1, zmm9",
//             "vpxorq zmm2, zmm2, zmm10",
//             "vpxorq zmm3, zmm3, zmm11",
//             "vpxorq zmm4, zmm4, zmm12",
//             "vpxorq zmm5, zmm5, zmm13",
//             "vpxorq zmm6, zmm6, zmm14",
//             "vpxorq zmm7, zmm7, zmm15",

//             // Store all 8 results
//             "vmovq [{c_data_ptr} + 8*{limb_idx} + 0], xmm0",   // Store result for limb 0
//             "vmovq [{c_data_ptr} + 8*{limb_idx} + 8], xmm1",   // Store result for limb 1
//             "vmovq [{c_data_ptr} + 8*{limb_idx} + 16], xmm2",  // Store result for limb 2
//             "vmovq [{c_data_ptr} + 8*{limb_idx} + 24], xmm3",  // Store result for limb 3
//             "vmovq [{c_data_ptr} + 8*{limb_idx} + 32], xmm4",  // Store result for limb 4
//             "vmovq [{c_data_ptr} + 8*{limb_idx} + 40], xmm5",  // Store result for limb 5
//             "vmovq [{c_data_ptr} + 8*{limb_idx} + 48], xmm6",  // Store result for limb 6
//             "vmovq [{c_data_ptr} + 8*{limb_idx} + 56], xmm7",  // Store result for limb 7

//             // Increment limb index for next iteration
//             "add {limb_idx}, 8",
//             "cmp {limb_idx}, 64",                    // Check if we need another iteration
//             "jl 2b",                                 // If not done, repeat

//             // Now reload from memory, xor with old C values, and store back
//             "vmovdqu64 zmm16, zmmword ptr [{c_data_ptr}]",        // (A*B)[0-7]
//             "vmovdqu64 zmm17, zmmword ptr [{c_data_ptr} + 64]",   // (A*B)[8-15]
//             "vmovdqu64 zmm18, zmmword ptr [{c_data_ptr} + 64*2]", // (A*B)[16-23]
//             "vmovdqu64 zmm19, zmmword ptr [{c_data_ptr} + 64*3]", // (A*B)[24-31]
//             "vmovdqu64 zmm20, zmmword ptr [{c_data_ptr} + 64*4]", // (A*B)[32-39]
//             "vmovdqu64 zmm21, zmmword ptr [{c_data_ptr} + 64*5]", // (A*B)[40-47]
//             "vmovdqu64 zmm22, zmmword ptr [{c_data_ptr} + 64*6]", // (A*B)[48-55]
//             "vmovdqu64 zmm23, zmmword ptr [{c_data_ptr} + 64*7]", // (A*B)[56-63]

//             "vpxorq zmm24, zmm24, zmm16", // C[0-7] ^= (A*B)[0-7]
//             "vpxorq zmm25, zmm25, zmm17", // C[8-15] ^= (A*B)[8-15]
//             "vpxorq zmm26, zmm26, zmm18", // C[16-23] ^= (A*B)[16-23]
//             "vpxorq zmm27, zmm27, zmm19", // C[24-31] ^= (A*B)[24-31]
//             "vpxorq zmm28, zmm28, zmm20", // C[32-39] ^= (A*B)[32-39]
//             "vpxorq zmm29, zmm29, zmm21", // C[40-47] ^= (A*B)[40-47]
//             "vpxorq zmm30, zmm30, zmm22", // C[48-55] ^= (A*B)[48-55]
//             "vpxorq zmm31, zmm31, zmm23", // C[56-63] ^= (A*B)[56-63]

//             // Store the final results back to C
//             "vmovdqu64 zmmword ptr [{c_data_ptr}], zmm24",        // (A*B+C)[0-7]
//             "vmovdqu64 zmmword ptr [{c_data_ptr} + 64], zmm25",   // (A*B+C)[8-15]
//             "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*2], zmm26", // (A*B+C)[16-23]
//             "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*3], zmm27", // (A*B+C)[24-31]
//             "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*4], zmm28", // (A*B+C)[32-39]
//             "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*5], zmm29", // (A*B+C)[40-47]
//             "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*6], zmm30", // (A*B+C)[48-55]
//             "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*7], zmm31", // (A*B+C)[56-63]

//             permute1 = const 0b10110001, // Permutation for horizontal XOR
//             permute2 = const 0b01001110, // Permutation for horizontal XOR

//             // Constraints
//             a_data_ptr = in(reg) a.limbs.as_ptr(),
//             b_data_ptr = in(reg) b.limbs.as_ptr(),
//             c_data_ptr = in(reg) c.limbs,

//             // c_stride = in(reg) c.stride,

//             // Counter
//             limb_idx = out(reg) _,

//             // Scratch registers
//             limb0 = out(reg) _, limb1 = out(reg) _, limb2 = out(reg) _, limb3 = out(reg) _,
//             limb4 = out(reg) _, limb5 = out(reg) _, limb6 = out(reg) _, limb7 = out(reg) _,

//             // 8 k-registers for in-place rotation
//             out("k0") _, out("k1") _, out("k2") _, out("k3") _,
//             out("k4") _, out("k5") _, out("k6") _, out("k7") _,

//             // ZMM registers: 16 for B and C + 14 for results and temps = 30 total
//             out("zmm0") _, out("zmm1") _, out("zmm2") _, out("zmm3") _,     // Results 0-3
//             out("zmm4") _, out("zmm5") _, out("zmm6") _, out("zmm7") _,     // Results 4-7
//             out("zmm8") _, out("zmm9") _, out("zmm10") _, out("zmm11") _,   // Temps for horizontal XOR
//             out("zmm12") _, out("zmm13") _, out("zmm14") _, out("zmm15") _, // More temps
//             out("zmm16") _, out("zmm17") _, out("zmm18") _, out("zmm19") _, // B[0]-B[3]
//             out("zmm20") _, out("zmm21") _, out("zmm22") _, out("zmm23") _, // B[4]-B[7]

//             inout("zmm24") c_zmms[0], inout("zmm25") c_zmms[1], // C[0]-C[1]
//             inout("zmm26") c_zmms[2], inout("zmm27") c_zmms[3], // C[2]-C[3]
//             inout("zmm28") c_zmms[4], inout("zmm29") c_zmms[5], // C[4]-C[5]
//             inout("zmm30") c_zmms[6], inout("zmm31") c_zmms[7], // C[6]-C[7]

//             options(nostack)
//         )
//     }
// }

pub struct SimdBlock([x86_64::__m512i; 8]);

impl SimdBlock {
    pub fn as_matrix_block(&self) -> MatrixBlock {
        MatrixBlock {
            limbs: unsafe { std::mem::transmute(self.0) },
        }
    }
}

pub fn gather_block_avx512(a: MatrixBlockSlice) -> SimdBlock {
    let mut result = SimdBlock([unsafe { x86_64::_mm512_setzero_si512() }; 8]);
    let offsets = unsafe { x86_64::_mm512_loadu_epi64(&UNIT_OFFSETS as *const i64) };
    let stride = unsafe { x86_64::_mm512_set1_epi64(a.stride as i64) };
    let offsets = unsafe { x86_64::_mm512_mullo_epi64(offsets, stride) };

    for i in 0..8 {
        let ptr = unsafe { a.limbs.add(8 * i * a.stride) as *const i64 };
        result.0[i] = unsafe { x86_64::_mm512_i64gather_epi64::<8>(offsets, ptr) };
    }
    result
}

pub fn scatter_block_avx512(c: &mut MatrixBlockSliceMut, values: SimdBlock) {
    let offsets = unsafe { x86_64::_mm512_loadu_epi64(&UNIT_OFFSETS as *const i64) };
    let stride = unsafe { x86_64::_mm512_set1_epi64(c.stride as i64) };
    let offsets = unsafe { x86_64::_mm512_mullo_epi64(offsets, stride) };

    for i in 0..8 {
        let ptr = unsafe { c.limbs.add(8 * i * c.stride) as *mut i64 };
        unsafe { x86_64::_mm512_i64scatter_epi64::<8>(ptr, offsets, values.0[i]) };
    }
}

pub fn setzero_block_avx512(c: &mut MatrixBlockSliceMut) {
    scatter_block_avx512(c, SimdBlock([unsafe { x86_64::_mm512_setzero_si512() }; 8]));
}
