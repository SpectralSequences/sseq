use super::{MatrixBlock, MatrixBlockMut};

/// Performs C = alpha * A * B + beta * C where A, B, C are 64x64 matrices
pub fn gemm_block_avx512_unrolled(
    alpha: bool,
    a: MatrixBlock,
    b: MatrixBlock,
    beta: bool,
    c: &mut MatrixBlockMut,
) {
    if !beta {
        setzero_block_avx512(c);
    }

    if !alpha {
        return;
    }

    unsafe {
        std::arch::asm!(
            // ===== SETUP: Load B matrix =====
            // Load all 8 rows of B matrix
            "vmovdqu64 zmm16, zmmword ptr [{b_data_ptr}]",        // B[0-7]
            "vmovdqu64 zmm17, zmmword ptr [{b_data_ptr} + 64]",   // B[8-15]
            "vmovdqu64 zmm18, zmmword ptr [{b_data_ptr} + 64*2]", // B[16-23]
            "vmovdqu64 zmm19, zmmword ptr [{b_data_ptr} + 64*3]", // B[24-31]
            "vmovdqu64 zmm20, zmmword ptr [{b_data_ptr} + 64*4]", // B[32-39]
            "vmovdqu64 zmm21, zmmword ptr [{b_data_ptr} + 64*5]", // B[40-47]
            "vmovdqu64 zmm22, zmmword ptr [{b_data_ptr} + 64*6]", // B[48-55]
            "vmovdqu64 zmm23, zmmword ptr [{b_data_ptr} + 64*7]", // B[56-63]

            // Load all of matrix C for safekeeping
            "vmovdqu64 zmm24, zmmword ptr [{c_data_ptr}]",        // C[0-7]
            "vmovdqu64 zmm25, zmmword ptr [{c_data_ptr} + 64]",   // C[8-15]
            "vmovdqu64 zmm26, zmmword ptr [{c_data_ptr} + 64*2]", // C[16-23]
            "vmovdqu64 zmm27, zmmword ptr [{c_data_ptr} + 64*3]", // C[24-31]
            "vmovdqu64 zmm28, zmmword ptr [{c_data_ptr} + 64*4]", // C[32-39]
            "vmovdqu64 zmm29, zmmword ptr [{c_data_ptr} + 64*5]", // C[40-47]
            "vmovdqu64 zmm30, zmmword ptr [{c_data_ptr} + 64*6]", // C[48-55]
            "vmovdqu64 zmm31, zmmword ptr [{c_data_ptr} + 64*7]", // C[56-63]

            // ===== 7-WAY PARALLEL PROCESSING =====
            // Only need ceil(64/7) = 10 iterations to cover all 64 limbs!

            // Iteration 0: Process limbs 0-6
            "mov {limb0}, [{a_data_ptr} + 0]",   // Load limb 0
            "mov {limb1}, [{a_data_ptr} + 8]",   // Load limb 1
            "mov {limb2}, [{a_data_ptr} + 16]",  // Load limb 2
            "mov {limb3}, [{a_data_ptr} + 24]",  // Load limb 3
            "mov {limb4}, [{a_data_ptr} + 32]",  // Load limb 4
            "mov {limb5}, [{a_data_ptr} + 40]",  // Load limb 5
            "mov {limb6}, [{a_data_ptr} + 48]",  // Load limb 6

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 0
            "kmovq k2, {limb1}",                 // k2 = limb 1
            "kmovq k3, {limb2}",                 // k3 = limb 2
            "kmovq k4, {limb3}",                 // k4 = limb 3
            "kmovq k5, {limb4}",                 // k5 = limb 4
            "kmovq k6, {limb5}",                 // k6 = limb 5
            "kmovq k7, {limb6}",                 // k7 = limb 6

            // Initialize 7 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0

            // BYTE 0: Process byte 0 of all 7 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]

            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k1, k1, 8",                // k1 >>= 8, byte 1 now in low position
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]

            // BYTE 2: Rotate and process
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

            // BYTE 3: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",

            // BYTE 4: Rotate and process
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

            // BYTE 5: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",

            // BYTE 6: Rotate and process
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

            // BYTE 7: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 7 results =====
            // Step 1: Swap adjacent pairs for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Store all 7 results
            "vmovq [{c_data_ptr} + 0], xmm0",   // Store result for limb 0
            "vmovq [{c_data_ptr} + 8], xmm1",   // Store result for limb 1
            "vmovq [{c_data_ptr} + 16], xmm2",  // Store result for limb 2
            "vmovq [{c_data_ptr} + 24], xmm3",  // Store result for limb 3
            "vmovq [{c_data_ptr} + 32], xmm4",  // Store result for limb 4
            "vmovq [{c_data_ptr} + 40], xmm5",  // Store result for limb 5
            "vmovq [{c_data_ptr} + 48], xmm6",  // Store result for limb 6

            // Iteration 1: Process limbs 7-13
            "mov {limb0}, [{a_data_ptr} + 56]",  // Load limb 0
            "mov {limb1}, [{a_data_ptr} + 64]",  // Load limb 1
            "mov {limb2}, [{a_data_ptr} + 72]",  // Load limb 2
            "mov {limb3}, [{a_data_ptr} + 80]",  // Load limb 3
            "mov {limb4}, [{a_data_ptr} + 88]",  // Load limb 4
            "mov {limb5}, [{a_data_ptr} + 96]",  // Load limb 5
            "mov {limb6}, [{a_data_ptr} + 104]", // Load limb 6

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 0
            "kmovq k2, {limb1}",                 // k2 = limb 1
            "kmovq k3, {limb2}",                 // k3 = limb 2
            "kmovq k4, {limb3}",                 // k4 = limb 3
            "kmovq k5, {limb4}",                 // k5 = limb 4
            "kmovq k6, {limb5}",                 // k6 = limb 5
            "kmovq k7, {limb6}",                 // k7 = limb 6

            // Initialize 7 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0

            // BYTE 0: Process byte 0 of all 7 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]

            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k1, k1, 8",                // k1 >>= 8, byte 1 now in low position
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]

            // BYTE 2: Rotate and process
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

            // BYTE 3: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",

            // BYTE 4: Rotate and process
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

            // BYTE 5: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",

            // BYTE 6: Rotate and process
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

            // BYTE 7: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 7 results =====
            // Step 1: Swap adjacent pairs for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Store all 7 results
            "vmovq [{c_data_ptr} + 56], xmm0",  // Store result for limb 0
            "vmovq [{c_data_ptr} + 64], xmm1",  // Store result for limb 1
            "vmovq [{c_data_ptr} + 72], xmm2",  // Store result for limb 2
            "vmovq [{c_data_ptr} + 80], xmm3",  // Store result for limb 3
            "vmovq [{c_data_ptr} + 88], xmm4",  // Store result for limb 4
            "vmovq [{c_data_ptr} + 96], xmm5",  // Store result for limb 5
            "vmovq [{c_data_ptr} + 104], xmm6", // Store result for limb 6

            // Iteration 2: Process limbs 14-20
            "mov {limb0}, [{a_data_ptr} + 112]", // Load limb 0
            "mov {limb1}, [{a_data_ptr} + 120]", // Load limb 1
            "mov {limb2}, [{a_data_ptr} + 128]", // Load limb 2
            "mov {limb3}, [{a_data_ptr} + 136]", // Load limb 3
            "mov {limb4}, [{a_data_ptr} + 144]", // Load limb 4
            "mov {limb5}, [{a_data_ptr} + 152]", // Load limb 5
            "mov {limb6}, [{a_data_ptr} + 160]", // Load limb 6

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 0
            "kmovq k2, {limb1}",                 // k2 = limb 1
            "kmovq k3, {limb2}",                 // k3 = limb 2
            "kmovq k4, {limb3}",                 // k4 = limb 3
            "kmovq k5, {limb4}",                 // k5 = limb 4
            "kmovq k6, {limb5}",                 // k6 = limb 5
            "kmovq k7, {limb6}",                 // k7 = limb 6

            // Initialize 7 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0

            // BYTE 0: Process byte 0 of all 7 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]

            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k1, k1, 8",                // k1 >>= 8, byte 1 now in low position
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]

            // BYTE 2: Rotate and process
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

            // BYTE 3: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",

            // BYTE 4: Rotate and process
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

            // BYTE 5: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",

            // BYTE 6: Rotate and process
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

            // BYTE 7: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 7 results =====
            // Step 1: Swap adjacent pairs for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Store all 7 results
            "vmovq [{c_data_ptr} + 112], xmm0", // Store result for limb 0
            "vmovq [{c_data_ptr} + 120], xmm1", // Store result for limb 1
            "vmovq [{c_data_ptr} + 128], xmm2", // Store result for limb 2
            "vmovq [{c_data_ptr} + 136], xmm3", // Store result for limb 3
            "vmovq [{c_data_ptr} + 144], xmm4", // Store result for limb 4
            "vmovq [{c_data_ptr} + 152], xmm5", // Store result for limb 5
            "vmovq [{c_data_ptr} + 160], xmm6", // Store result for limb 6

            // Iteration 3: Process limbs 21-27
            "mov {limb0}, [{a_data_ptr} + 168]", // Load limb 0
            "mov {limb1}, [{a_data_ptr} + 176]", // Load limb 1
            "mov {limb2}, [{a_data_ptr} + 184]", // Load limb 2
            "mov {limb3}, [{a_data_ptr} + 192]", // Load limb 3
            "mov {limb4}, [{a_data_ptr} + 200]", // Load limb 4
            "mov {limb5}, [{a_data_ptr} + 208]", // Load limb 5
            "mov {limb6}, [{a_data_ptr} + 216]", // Load limb 6

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 0
            "kmovq k2, {limb1}",                 // k2 = limb 1
            "kmovq k3, {limb2}",                 // k3 = limb 2
            "kmovq k4, {limb3}",                 // k4 = limb 3
            "kmovq k5, {limb4}",                 // k5 = limb 4
            "kmovq k6, {limb5}",                 // k6 = limb 5
            "kmovq k7, {limb6}",                 // k7 = limb 6

            // Initialize 7 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0

            // BYTE 0: Process byte 0 of all 7 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]

            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k1, k1, 8",                // k1 >>= 8, byte 1 now in low position
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]

            // BYTE 2: Rotate and process
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

            // BYTE 3: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",

            // BYTE 4: Rotate and process
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

            // BYTE 5: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",

            // BYTE 6: Rotate and process
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

            // BYTE 7: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 7 results =====
            // Step 1: Swap adjacent pairs for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Store all 7 results
            "vmovq [{c_data_ptr} + 168], xmm0", // Store result for limb 0
            "vmovq [{c_data_ptr} + 176], xmm1", // Store result for limb 1
            "vmovq [{c_data_ptr} + 184], xmm2", // Store result for limb 2
            "vmovq [{c_data_ptr} + 192], xmm3", // Store result for limb 3
            "vmovq [{c_data_ptr} + 200], xmm4", // Store result for limb 4
            "vmovq [{c_data_ptr} + 208], xmm5", // Store result for limb 5
            "vmovq [{c_data_ptr} + 216], xmm6", // Store result for limb 6

            // Iteration 4: Process limbs 28-34
            "mov {limb0}, [{a_data_ptr} + 224]", // Load limb 0
            "mov {limb1}, [{a_data_ptr} + 232]", // Load limb 1
            "mov {limb2}, [{a_data_ptr} + 240]", // Load limb 2
            "mov {limb3}, [{a_data_ptr} + 248]", // Load limb 3
            "mov {limb4}, [{a_data_ptr} + 256]", // Load limb 4
            "mov {limb5}, [{a_data_ptr} + 264]", // Load limb 5
            "mov {limb6}, [{a_data_ptr} + 272]", // Load limb 6

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 0
            "kmovq k2, {limb1}",                 // k2 = limb 1
            "kmovq k3, {limb2}",                 // k3 = limb 2
            "kmovq k4, {limb3}",                 // k4 = limb 3
            "kmovq k5, {limb4}",                 // k5 = limb 4
            "kmovq k6, {limb5}",                 // k6 = limb 5
            "kmovq k7, {limb6}",                 // k7 = limb 6

            // Initialize 7 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0

            // BYTE 0: Process byte 0 of all 7 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]

            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k1, k1, 8",                // k1 >>= 8, byte 1 now in low position
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]

            // BYTE 2: Rotate and process
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

            // BYTE 3: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",

            // BYTE 4: Rotate and process
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

            // BYTE 5: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",

            // BYTE 6: Rotate and process
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

            // BYTE 7: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 7 results =====
            // Step 1: Swap adjacent pairs for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Store all 7 results
            "vmovq [{c_data_ptr} + 224], xmm0", // Store result for limb 0
            "vmovq [{c_data_ptr} + 232], xmm1", // Store result for limb 1
            "vmovq [{c_data_ptr} + 240], xmm2", // Store result for limb 2
            "vmovq [{c_data_ptr} + 248], xmm3", // Store result for limb 3
            "vmovq [{c_data_ptr} + 256], xmm4", // Store result for limb 4
            "vmovq [{c_data_ptr} + 264], xmm5", // Store result for limb 5
            "vmovq [{c_data_ptr} + 272], xmm6", // Store result for limb 6

            // Iteration 5: Process limbs 35-41
            "mov {limb0}, [{a_data_ptr} + 280]", // Load limb 0
            "mov {limb1}, [{a_data_ptr} + 288]", // Load limb 1
            "mov {limb2}, [{a_data_ptr} + 296]", // Load limb 2
            "mov {limb3}, [{a_data_ptr} + 304]", // Load limb 3
            "mov {limb4}, [{a_data_ptr} + 312]", // Load limb 4
            "mov {limb5}, [{a_data_ptr} + 320]", // Load limb 5
            "mov {limb6}, [{a_data_ptr} + 328]", // Load limb 6

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 0
            "kmovq k2, {limb1}",                 // k2 = limb 1
            "kmovq k3, {limb2}",                 // k3 = limb 2
            "kmovq k4, {limb3}",                 // k4 = limb 3
            "kmovq k5, {limb4}",                 // k5 = limb 4
            "kmovq k6, {limb5}",                 // k6 = limb 5
            "kmovq k7, {limb6}",                 // k7 = limb 6

            // Initialize 7 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0

            // BYTE 0: Process byte 0 of all 7 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]

            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k1, k1, 8",                // k1 >>= 8, byte 1 now in low position
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]

            // BYTE 2: Rotate and process
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

            // BYTE 3: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",

            // BYTE 4: Rotate and process
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

            // BYTE 5: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",

            // BYTE 6: Rotate and process
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

            // BYTE 7: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 7 results =====
            // Step 1: Swap adjacent pairs for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Store all 7 results
            "vmovq [{c_data_ptr} + 280], xmm0", // Store result for limb 0
            "vmovq [{c_data_ptr} + 288], xmm1", // Store result for limb 1
            "vmovq [{c_data_ptr} + 296], xmm2", // Store result for limb 2
            "vmovq [{c_data_ptr} + 304], xmm3", // Store result for limb 3
            "vmovq [{c_data_ptr} + 312], xmm4", // Store result for limb 4
            "vmovq [{c_data_ptr} + 320], xmm5", // Store result for limb 5
            "vmovq [{c_data_ptr} + 328], xmm6", // Store result for limb 6

            // Iteration 6: Process limbs 42-48
            "mov {limb0}, [{a_data_ptr} + 336]", // Load limb 0
            "mov {limb1}, [{a_data_ptr} + 344]", // Load limb 1
            "mov {limb2}, [{a_data_ptr} + 352]", // Load limb 2
            "mov {limb3}, [{a_data_ptr} + 360]", // Load limb 3
            "mov {limb4}, [{a_data_ptr} + 368]", // Load limb 4
            "mov {limb5}, [{a_data_ptr} + 376]", // Load limb 5
            "mov {limb6}, [{a_data_ptr} + 384]", // Load limb 6

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 0
            "kmovq k2, {limb1}",                 // k2 = limb 1
            "kmovq k3, {limb2}",                 // k3 = limb 2
            "kmovq k4, {limb3}",                 // k4 = limb 3
            "kmovq k5, {limb4}",                 // k5 = limb 4
            "kmovq k6, {limb5}",                 // k6 = limb 5
            "kmovq k7, {limb6}",                 // k7 = limb 6

            // Initialize 7 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0

            // BYTE 0: Process byte 0 of all 7 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]

            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k1, k1, 8",                // k1 >>= 8, byte 1 now in low position
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]

            // BYTE 2: Rotate and process
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

            // BYTE 3: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",

            // BYTE 4: Rotate and process
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

            // BYTE 5: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",

            // BYTE 6: Rotate and process
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

            // BYTE 7: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 7 results =====
            // Step 1: Swap adjacent pairs for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Store all 7 results
            "vmovq [{c_data_ptr} + 336], xmm0", // Store result for limb 0
            "vmovq [{c_data_ptr} + 344], xmm1", // Store result for limb 1
            "vmovq [{c_data_ptr} + 352], xmm2", // Store result for limb 2
            "vmovq [{c_data_ptr} + 360], xmm3", // Store result for limb 3
            "vmovq [{c_data_ptr} + 368], xmm4", // Store result for limb 4
            "vmovq [{c_data_ptr} + 376], xmm5", // Store result for limb 5
            "vmovq [{c_data_ptr} + 384], xmm6", // Store result for limb 6

            // Iteration 7: Process limbs 49-55
            "mov {limb0}, [{a_data_ptr} + 392]", // Load limb 0
            "mov {limb1}, [{a_data_ptr} + 400]", // Load limb 1
            "mov {limb2}, [{a_data_ptr} + 408]", // Load limb 2
            "mov {limb3}, [{a_data_ptr} + 416]", // Load limb 3
            "mov {limb4}, [{a_data_ptr} + 424]", // Load limb 4
            "mov {limb5}, [{a_data_ptr} + 432]", // Load limb 5
            "mov {limb6}, [{a_data_ptr} + 440]", // Load limb 6

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 0
            "kmovq k2, {limb1}",                 // k2 = limb 1
            "kmovq k3, {limb2}",                 // k3 = limb 2
            "kmovq k4, {limb3}",                 // k4 = limb 3
            "kmovq k5, {limb4}",                 // k5 = limb 4
            "kmovq k6, {limb5}",                 // k6 = limb 5
            "kmovq k7, {limb6}",                 // k7 = limb 6

            // Initialize 7 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0

            // BYTE 0: Process byte 0 of all 7 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]

            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k1, k1, 8",                // k1 >>= 8, byte 1 now in low position
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]

            // BYTE 2: Rotate and process
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

            // BYTE 3: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",

            // BYTE 4: Rotate and process
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

            // BYTE 5: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",

            // BYTE 6: Rotate and process
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

            // BYTE 7: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 7 results =====
            // Step 1: Swap adjacent pairs for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Store all 7 results
            "vmovq [{c_data_ptr} + 392], xmm0", // Store result for limb 0
            "vmovq [{c_data_ptr} + 400], xmm1", // Store result for limb 1
            "vmovq [{c_data_ptr} + 408], xmm2", // Store result for limb 2
            "vmovq [{c_data_ptr} + 416], xmm3", // Store result for limb 3
            "vmovq [{c_data_ptr} + 424], xmm4", // Store result for limb 4
            "vmovq [{c_data_ptr} + 432], xmm5", // Store result for limb 5
            "vmovq [{c_data_ptr} + 440], xmm6", // Store result for limb 6

            // Iteration 8: Process limbs 56-62
            "mov {limb0}, [{a_data_ptr} + 448]", // Load limb 0
            "mov {limb1}, [{a_data_ptr} + 456]", // Load limb 1
            "mov {limb2}, [{a_data_ptr} + 464]", // Load limb 2
            "mov {limb3}, [{a_data_ptr} + 472]", // Load limb 3
            "mov {limb4}, [{a_data_ptr} + 480]", // Load limb 4
            "mov {limb5}, [{a_data_ptr} + 488]", // Load limb 5
            "mov {limb6}, [{a_data_ptr} + 496]", // Load limb 6

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 0
            "kmovq k2, {limb1}",                 // k2 = limb 1
            "kmovq k3, {limb2}",                 // k3 = limb 2
            "kmovq k4, {limb3}",                 // k4 = limb 3
            "kmovq k5, {limb4}",                 // k5 = limb 4
            "kmovq k6, {limb5}",                 // k6 = limb 5
            "kmovq k7, {limb6}",                 // k7 = limb 6

            // Initialize 7 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0

            // BYTE 0: Process byte 0 of all 7 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]

            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k1, k1, 8",                // k1 >>= 8, byte 1 now in low position
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]

            // BYTE 2: Rotate and process
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

            // BYTE 3: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",

            // BYTE 4: Rotate and process
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

            // BYTE 5: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",

            // BYTE 6: Rotate and process
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

            // BYTE 7: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 7 results =====
            // Step 1: Swap adjacent pairs for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Store all 7 results
            "vmovq [{c_data_ptr} + 448], xmm0", // Store result for limb 0
            "vmovq [{c_data_ptr} + 456], xmm1", // Store result for limb 1
            "vmovq [{c_data_ptr} + 464], xmm2", // Store result for limb 2
            "vmovq [{c_data_ptr} + 472], xmm3", // Store result for limb 3
            "vmovq [{c_data_ptr} + 480], xmm4", // Store result for limb 4
            "vmovq [{c_data_ptr} + 488], xmm5", // Store result for limb 5
            "vmovq [{c_data_ptr} + 496], xmm6", // Store result for limb 6

            // Iteration 9: Process limb 63 (only 1 limb, handle separately)
            "mov {limb0}, [{a_data_ptr} + 504]",
            "kmovq k1, {limb0}",                     // k1 = limb 0
            "vpxorq zmm0, zmm0, zmm0",               // result[0] = 0
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",       // if (byte0 of limb0) result[0] ^= B[0]
            "kshiftrq k1, k1, 8",                    // k1 >>= 8, byte 1 now in low position
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",       // if (byte1 of limb0) result[0] ^= B[1]
            "kshiftrq k1, k1, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm18",
            "kshiftrq k1, k1, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",
            "kshiftrq k1, k1, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm20",
            "kshiftrq k1, k1, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",
            "kshiftrq k1, k1, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm22",
            "kshiftrq k1, k1, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",
            "vpermq zmm8, zmm0, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpermq zmm8, zmm0, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vmovq [{c_data_ptr} + 504], xmm0",  // Store result for limb 63

            // Now reload from memory, xor with old C values, and store back
            "vmovdqu64 zmm16, zmmword ptr [{c_data_ptr}]",        // (A*B)[0-7]
            "vmovdqu64 zmm17, zmmword ptr [{c_data_ptr} + 64]",   // (A*B)[8-15]
            "vmovdqu64 zmm18, zmmword ptr [{c_data_ptr} + 64*2]", // (A*B)[16-23]
            "vmovdqu64 zmm19, zmmword ptr [{c_data_ptr} + 64*3]", // (A*B)[24-31]
            "vmovdqu64 zmm20, zmmword ptr [{c_data_ptr} + 64*4]", // (A*B)[32-39]
            "vmovdqu64 zmm21, zmmword ptr [{c_data_ptr} + 64*5]", // (A*B)[40-47]
            "vmovdqu64 zmm22, zmmword ptr [{c_data_ptr} + 64*6]", // (A*B)[48-55]
            "vmovdqu64 zmm23, zmmword ptr [{c_data_ptr} + 64*7]", // (A*B)[56-63]

            "vpxorq zmm24, zmm24, zmm16", // C[0-7] ^= (A*B)[0-7]
            "vpxorq zmm25, zmm25, zmm17", // C[8-15] ^= (A*B)[8-15]
            "vpxorq zmm26, zmm26, zmm18", // C[16-23] ^= (A*B)[16-23]
            "vpxorq zmm27, zmm27, zmm19", // C[24-31] ^= (A*B)[24-31]
            "vpxorq zmm28, zmm28, zmm20", // C[32-39] ^= (A*B)[32-39]
            "vpxorq zmm29, zmm29, zmm21", // C[40-47] ^= (A*B)[40-47]
            "vpxorq zmm30, zmm30, zmm22", // C[48-55] ^= (A*B)[48-55]
            "vpxorq zmm31, zmm31, zmm23", // C[56-63] ^= (A*B)[56-63]

            // Store the final results back to C
            "vmovdqu64 zmmword ptr [{c_data_ptr}], zmm24",        // (A*B+C)[0-7]
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 64], zmm25",   // (A*B+C)[8-15]
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*2], zmm26", // (A*B+C)[16-23]
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*3], zmm27", // (A*B+C)[24-31]
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*4], zmm28", // (A*B+C)[32-39]
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*5], zmm29", // (A*B+C)[40-47]
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*6], zmm30", // (A*B+C)[48-55]
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*7], zmm31", // (A*B+C)[56-63]

            permute1 = const 0b10110001, // Permutation for horizontal XOR
            permute2 = const 0b01001110, // Permutation for horizontal XOR

            // Constraints
            a_data_ptr = in(reg) a.limbs.as_ptr(),
            b_data_ptr = in(reg) b.limbs.as_ptr(),
            c_data_ptr = in(reg) c.limbs.as_mut_ptr(),

            // Scratch registers
            limb0 = out(reg) _, limb1 = out(reg) _, limb2 = out(reg) _, limb3 = out(reg) _,
            limb4 = out(reg) _, limb5 = out(reg) _, limb6 = out(reg) _,

            // 7 k-registers for in-place rotation (avoiding k0)
            out("k1") _, out("k2") _, out("k3") _, out("k4") _,
            out("k5") _, out("k6") _, out("k7") _,

            // ZMM registers: 8 for B matrix + 14 for results and temps = 22 total
            out("zmm0") _, out("zmm1") _, out("zmm2") _, out("zmm3") _,   // Results 0-3
            out("zmm4") _, out("zmm5") _, out("zmm6") _,                  // Results 4-6
            out("zmm8") _, out("zmm9") _, out("zmm10") _, out("zmm11") _, // Temps for horizontal XOR
            out("zmm12") _, out("zmm13") _, out("zmm14") _,               // More temps
            out("zmm16") _, out("zmm17") _, out("zmm18") _, out("zmm19") _, // B[0]-B[3]
            out("zmm20") _, out("zmm21") _, out("zmm22") _, out("zmm23") _, // B[4]-B[7]
            out("zmm24") _, out("zmm25") _, out("zmm26") _, out("zmm27") _, // C[0]-C[3]
            out("zmm28") _, out("zmm29") _, out("zmm30") _, out("zmm31") _, // C[4]-C[7]

            options(nostack, preserves_flags)
        )
    }
}

pub fn gemm_block_avx512(
    alpha: bool,
    a: MatrixBlock,
    b: MatrixBlock,
    beta: bool,
    c: &mut MatrixBlockMut,
) {
    if !beta {
        setzero_block_avx512(c);
    }

    if !alpha {
        return;
    }

    unsafe {
        std::arch::asm!(
            // ===== SETUP: Load B matrix =====
            // Load all 8 rows of B matrix
            "vmovdqu64 zmm16, zmmword ptr [{b_data_ptr}]",        // B[0-7]
            "vmovdqu64 zmm17, zmmword ptr [{b_data_ptr} + 64]",   // B[8-15]
            "vmovdqu64 zmm18, zmmword ptr [{b_data_ptr} + 64*2]", // B[16-23]
            "vmovdqu64 zmm19, zmmword ptr [{b_data_ptr} + 64*3]", // B[24-31]
            "vmovdqu64 zmm20, zmmword ptr [{b_data_ptr} + 64*4]", // B[32-39]
            "vmovdqu64 zmm21, zmmword ptr [{b_data_ptr} + 64*5]", // B[40-47]
            "vmovdqu64 zmm22, zmmword ptr [{b_data_ptr} + 64*6]", // B[48-55]
            "vmovdqu64 zmm23, zmmword ptr [{b_data_ptr} + 64*7]", // B[56-63]

            // Load all of matrix C for safekeeping
            "vmovdqu64 zmm24, zmmword ptr [{c_data_ptr}]",        // C[0-7]
            "vmovdqu64 zmm25, zmmword ptr [{c_data_ptr} + 64]",   // C[8-15]
            "vmovdqu64 zmm26, zmmword ptr [{c_data_ptr} + 64*2]", // C[16-23]
            "vmovdqu64 zmm27, zmmword ptr [{c_data_ptr} + 64*3]", // C[24-31]
            "vmovdqu64 zmm28, zmmword ptr [{c_data_ptr} + 64*4]", // C[32-39]
            "vmovdqu64 zmm29, zmmword ptr [{c_data_ptr} + 64*5]", // C[40-47]
            "vmovdqu64 zmm30, zmmword ptr [{c_data_ptr} + 64*6]", // C[48-55]
            "vmovdqu64 zmm31, zmmword ptr [{c_data_ptr} + 64*7]", // C[56-63]

            // ===== 7-WAY PARALLEL PROCESSING =====
            // Only need ceil(64/7) = 10 iterations to cover all 64 limbs!

            "mov {limb_idx}, 0",
            "2:",

            // Iteration 0: Process limbs 0-6
            "mov {limb0}, [{a_data_ptr} + 8*{limb_idx} + 0]",   // Load limb 0
            "mov {limb1}, [{a_data_ptr} + 8*{limb_idx} + 8]",   // Load limb 1
            "mov {limb2}, [{a_data_ptr} + 8*{limb_idx} + 16]",  // Load limb 2
            "mov {limb3}, [{a_data_ptr} + 8*{limb_idx} + 24]",  // Load limb 3
            "mov {limb4}, [{a_data_ptr} + 8*{limb_idx} + 32]",  // Load limb 4
            "mov {limb5}, [{a_data_ptr} + 8*{limb_idx} + 40]",  // Load limb 5
            "mov {limb6}, [{a_data_ptr} + 8*{limb_idx} + 48]",  // Load limb 6

            // Load all 7 limbs into k-registers (these will be rotated in-place)
            "kmovq k1, {limb0}",                 // k1 = limb 0
            "kmovq k2, {limb1}",                 // k2 = limb 1
            "kmovq k3, {limb2}",                 // k3 = limb 2
            "kmovq k4, {limb3}",                 // k4 = limb 3
            "kmovq k5, {limb4}",                 // k5 = limb 4
            "kmovq k6, {limb5}",                 // k6 = limb 5
            "kmovq k7, {limb6}",                 // k7 = limb 6

            // Initialize 7 result registers
            "vpxorq zmm0, zmm0, zmm0",           // result[0] = 0
            "vpxorq zmm1, zmm1, zmm1",           // result[1] = 0
            "vpxorq zmm2, zmm2, zmm2",           // result[2] = 0
            "vpxorq zmm3, zmm3, zmm3",           // result[3] = 0
            "vpxorq zmm4, zmm4, zmm4",           // result[4] = 0
            "vpxorq zmm5, zmm5, zmm5",           // result[5] = 0
            "vpxorq zmm6, zmm6, zmm6",           // result[6] = 0

            // BYTE 0: Process byte 0 of all 7 limbs (already in low position)
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",   // if (byte0 of limb0) result[0] ^= B[0]
            "vpxorq zmm1 {{k2}}, zmm1, zmm16",   // if (byte0 of limb1) result[1] ^= B[0]
            "vpxorq zmm2 {{k3}}, zmm2, zmm16",   // if (byte0 of limb2) result[2] ^= B[0]
            "vpxorq zmm3 {{k4}}, zmm3, zmm16",   // if (byte0 of limb3) result[3] ^= B[0]
            "vpxorq zmm4 {{k5}}, zmm4, zmm16",   // if (byte0 of limb4) result[4] ^= B[0]
            "vpxorq zmm5 {{k6}}, zmm5, zmm16",   // if (byte0 of limb5) result[5] ^= B[0]
            "vpxorq zmm6 {{k7}}, zmm6, zmm16",   // if (byte0 of limb6) result[6] ^= B[0]

            // BYTE 1: Rotate all masks right by 8 bits, then process
            "kshiftrq k1, k1, 8",                // k1 >>= 8, byte 1 now in low position
            "kshiftrq k2, k2, 8",                // k2 >>= 8
            "kshiftrq k3, k3, 8",                // k3 >>= 8
            "kshiftrq k4, k4, 8",                // k4 >>= 8
            "kshiftrq k5, k5, 8",                // k5 >>= 8
            "kshiftrq k6, k6, 8",                // k6 >>= 8
            "kshiftrq k7, k7, 8",                // k7 >>= 8
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",   // if (byte1 of limb0) result[0] ^= B[1]
            "vpxorq zmm1 {{k2}}, zmm1, zmm17",   // if (byte1 of limb1) result[1] ^= B[1]
            "vpxorq zmm2 {{k3}}, zmm2, zmm17",   // if (byte1 of limb2) result[2] ^= B[1]
            "vpxorq zmm3 {{k4}}, zmm3, zmm17",   // if (byte1 of limb3) result[3] ^= B[1]
            "vpxorq zmm4 {{k5}}, zmm4, zmm17",   // if (byte1 of limb4) result[4] ^= B[1]
            "vpxorq zmm5 {{k6}}, zmm5, zmm17",   // if (byte1 of limb5) result[5] ^= B[1]
            "vpxorq zmm6 {{k7}}, zmm6, zmm17",   // if (byte1 of limb6) result[6] ^= B[1]

            // BYTE 2: Rotate and process
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

            // BYTE 3: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",
            "vpxorq zmm1 {{k2}}, zmm1, zmm19",
            "vpxorq zmm2 {{k3}}, zmm2, zmm19",
            "vpxorq zmm3 {{k4}}, zmm3, zmm19",
            "vpxorq zmm4 {{k5}}, zmm4, zmm19",
            "vpxorq zmm5 {{k6}}, zmm5, zmm19",
            "vpxorq zmm6 {{k7}}, zmm6, zmm19",

            // BYTE 4: Rotate and process
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

            // BYTE 5: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",
            "vpxorq zmm1 {{k2}}, zmm1, zmm21",
            "vpxorq zmm2 {{k3}}, zmm2, zmm21",
            "vpxorq zmm3 {{k4}}, zmm3, zmm21",
            "vpxorq zmm4 {{k5}}, zmm4, zmm21",
            "vpxorq zmm5 {{k6}}, zmm5, zmm21",
            "vpxorq zmm6 {{k7}}, zmm6, zmm21",

            // BYTE 6: Rotate and process
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

            // BYTE 7: Rotate and process
            "kshiftrq k1, k1, 8",
            "kshiftrq k2, k2, 8",
            "kshiftrq k3, k3, 8",
            "kshiftrq k4, k4, 8",
            "kshiftrq k5, k5, 8",
            "kshiftrq k6, k6, 8",
            "kshiftrq k7, k7, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",
            "vpxorq zmm1 {{k2}}, zmm1, zmm23",
            "vpxorq zmm2 {{k3}}, zmm2, zmm23",
            "vpxorq zmm3 {{k4}}, zmm3, zmm23",
            "vpxorq zmm4 {{k5}}, zmm4, zmm23",
            "vpxorq zmm5 {{k6}}, zmm5, zmm23",
            "vpxorq zmm6 {{k7}}, zmm6, zmm23",

            // ===== PARALLEL HORIZONTAL XOR for all 7 results =====
            // Step 1: Swap adjacent pairs for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute1}",
            "vpermq zmm9, zmm1, {permute1}",
            "vpermq zmm10, zmm2, {permute1}",
            "vpermq zmm11, zmm3, {permute1}",
            "vpermq zmm12, zmm4, {permute1}",
            "vpermq zmm13, zmm5, {permute1}",
            "vpermq zmm14, zmm6, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 2: Swap quads for all 7 results simultaneously
            "vpermq zmm8, zmm0, {permute2}",
            "vpermq zmm9, zmm1, {permute2}",
            "vpermq zmm10, zmm2, {permute2}",
            "vpermq zmm11, zmm3, {permute2}",
            "vpermq zmm12, zmm4, {permute2}",
            "vpermq zmm13, zmm5, {permute2}",
            "vpermq zmm14, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Step 3: Swap halves for all 7 results simultaneously
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vshufi64x2 zmm9, zmm1, zmm1, {permute2}",
            "vshufi64x2 zmm10, zmm2, zmm2, {permute2}",
            "vshufi64x2 zmm11, zmm3, zmm3, {permute2}",
            "vshufi64x2 zmm12, zmm4, zmm4, {permute2}",
            "vshufi64x2 zmm13, zmm5, zmm5, {permute2}",
            "vshufi64x2 zmm14, zmm6, zmm6, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpxorq zmm1, zmm1, zmm9",
            "vpxorq zmm2, zmm2, zmm10",
            "vpxorq zmm3, zmm3, zmm11",
            "vpxorq zmm4, zmm4, zmm12",
            "vpxorq zmm5, zmm5, zmm13",
            "vpxorq zmm6, zmm6, zmm14",

            // Store all 7 results
            "vmovq [{c_data_ptr} + 8*{limb_idx} + 0], xmm0",   // Store result for limb 0
            "vmovq [{c_data_ptr} + 8*{limb_idx} + 8], xmm1",   // Store result for limb 1
            "vmovq [{c_data_ptr} + 8*{limb_idx} + 16], xmm2",  // Store result for limb 2
            "vmovq [{c_data_ptr} + 8*{limb_idx} + 24], xmm3",  // Store result for limb 3
            "vmovq [{c_data_ptr} + 8*{limb_idx} + 32], xmm4",  // Store result for limb 4
            "vmovq [{c_data_ptr} + 8*{limb_idx} + 40], xmm5",  // Store result for limb 5
            "vmovq [{c_data_ptr} + 8*{limb_idx} + 48], xmm6",  // Store result for limb 6

            // Increment limb index for next iteration
            "add {limb_idx}, 7",
            "cmp {limb_idx}, 63",                    // Check if we need another iteration
            "jl 2b",                                 // If not done, repeat

            // Iteration 9: Process limb 63 (only 1 limb, handle separately)
            "mov {limb0}, [{a_data_ptr} + 504]",
            "kmovq k1, {limb0}",                     // k1 = limb 0
            "vpxorq zmm0, zmm0, zmm0",               // result[0] = 0
            "vpxorq zmm0 {{k1}}, zmm0, zmm16",       // if (byte0 of limb0) result[0] ^= B[0]
            "kshiftrq k1, k1, 8",                    // k1 >>= 8, byte 1 now in low position
            "vpxorq zmm0 {{k1}}, zmm0, zmm17",       // if (byte1 of limb0) result[0] ^= B[1]
            "kshiftrq k1, k1, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm18",
            "kshiftrq k1, k1, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm19",
            "kshiftrq k1, k1, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm20",
            "kshiftrq k1, k1, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm21",
            "kshiftrq k1, k1, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm22",
            "kshiftrq k1, k1, 8",
            "vpxorq zmm0 {{k1}}, zmm0, zmm23",
            "vpermq zmm8, zmm0, {permute1}",
            "vpxorq zmm0, zmm0, zmm8",
            "vpermq zmm8, zmm0, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vshufi64x2 zmm8, zmm0, zmm0, {permute2}",
            "vpxorq zmm0, zmm0, zmm8",
            "vmovq [{c_data_ptr} + 504], xmm0",  // Store result for limb 63

            // Now reload from memory, xor with old C values, and store back
            "vmovdqu64 zmm16, zmmword ptr [{c_data_ptr}]",        // (A*B)[0-7]
            "vmovdqu64 zmm17, zmmword ptr [{c_data_ptr} + 64]",   // (A*B)[8-15]
            "vmovdqu64 zmm18, zmmword ptr [{c_data_ptr} + 64*2]", // (A*B)[16-23]
            "vmovdqu64 zmm19, zmmword ptr [{c_data_ptr} + 64*3]", // (A*B)[24-31]
            "vmovdqu64 zmm20, zmmword ptr [{c_data_ptr} + 64*4]", // (A*B)[32-39]
            "vmovdqu64 zmm21, zmmword ptr [{c_data_ptr} + 64*5]", // (A*B)[40-47]
            "vmovdqu64 zmm22, zmmword ptr [{c_data_ptr} + 64*6]", // (A*B)[48-55]
            "vmovdqu64 zmm23, zmmword ptr [{c_data_ptr} + 64*7]", // (A*B)[56-63]

            "vpxorq zmm24, zmm24, zmm16", // C[0-7] ^= (A*B)[0-7]
            "vpxorq zmm25, zmm25, zmm17", // C[8-15] ^= (A*B)[8-15]
            "vpxorq zmm26, zmm26, zmm18", // C[16-23] ^= (A*B)[16-23]
            "vpxorq zmm27, zmm27, zmm19", // C[24-31] ^= (A*B)[24-31]
            "vpxorq zmm28, zmm28, zmm20", // C[32-39] ^= (A*B)[32-39]
            "vpxorq zmm29, zmm29, zmm21", // C[40-47] ^= (A*B)[40-47]
            "vpxorq zmm30, zmm30, zmm22", // C[48-55] ^= (A*B)[48-55]
            "vpxorq zmm31, zmm31, zmm23", // C[56-63] ^= (A*B)[56-63]

            // Store the final results back to C
            "vmovdqu64 zmmword ptr [{c_data_ptr}], zmm24",        // (A*B+C)[0-7]
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 64], zmm25",   // (A*B+C)[8-15]
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*2], zmm26", // (A*B+C)[16-23]
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*3], zmm27", // (A*B+C)[24-31]
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*4], zmm28", // (A*B+C)[32-39]
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*5], zmm29", // (A*B+C)[40-47]
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*6], zmm30", // (A*B+C)[48-55]
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 64*7], zmm31", // (A*B+C)[56-63]

            permute1 = const 0b10110001, // Permutation for horizontal XOR
            permute2 = const 0b01001110, // Permutation for horizontal XOR

            // Constraints
            a_data_ptr = in(reg) a.limbs.as_ptr(),
            b_data_ptr = in(reg) b.limbs.as_ptr(),
            c_data_ptr = in(reg) c.limbs.as_mut_ptr(),

            // Counter
            limb_idx = out(reg) _,

            // Scratch registers
            limb0 = out(reg) _, limb1 = out(reg) _, limb2 = out(reg) _, limb3 = out(reg) _,
            limb4 = out(reg) _, limb5 = out(reg) _, limb6 = out(reg) _,

            // 7 k-registers for in-place rotation (avoiding k0)
            out("k1") _, out("k2") _, out("k3") _, out("k4") _,
            out("k5") _, out("k6") _, out("k7") _,

            // ZMM registers: 16 for B and C + 14 for results and temps = 30 total
            out("zmm0") _, out("zmm1") _, out("zmm2") _, out("zmm3") _,   // Results 0-3
            out("zmm4") _, out("zmm5") _, out("zmm6") _,                  // Results 4-6
            out("zmm8") _, out("zmm9") _, out("zmm10") _, out("zmm11") _, // Temps for horizontal XOR
            out("zmm12") _, out("zmm13") _, out("zmm14") _,               // More temps
            out("zmm16") _, out("zmm17") _, out("zmm18") _, out("zmm19") _, // B[0]-B[3]
            out("zmm20") _, out("zmm21") _, out("zmm22") _, out("zmm23") _, // B[4]-B[7]
            out("zmm24") _, out("zmm25") _, out("zmm26") _, out("zmm27") _, // C[0]-C[3]
            out("zmm28") _, out("zmm29") _, out("zmm30") _, out("zmm31") _, // C[4]-C[7]
            options(nostack)
        )
    }
}

pub fn setzero_block_avx512(c: &mut MatrixBlockMut) {
    unsafe {
        std::arch::asm!(
            "vpxorq zmm0, zmm0, zmm0",
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 8*0], zmm0", // u64 = 8 bytes, zmmword = 8 u64s
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 8*8], zmm0",
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 8*16], zmm0",
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 8*24], zmm0",
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 8*32], zmm0",
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 8*40], zmm0",
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 8*48], zmm0",
            "vmovdqu64 zmmword ptr [{c_data_ptr} + 8*56], zmm0",

            c_data_ptr = in(reg) c.limbs.as_mut_ptr(),

            out("zmm0") _,

            options(nostack, preserves_flags)
        )
    }
}
