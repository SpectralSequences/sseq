use crate::matrix::Matrix;

impl Matrix {
    pub fn fast_mul_7(&self, b: &Self) -> Matrix {
        assert_eq!(self.prime(), b.prime());
        assert_eq!(self.columns(), b.rows());

        let mut result = Matrix::new(self.prime(), self.rows(), b.columns());

        let a_data_ptr = self.data().as_ptr();
        let b_data_ptr = b.data().as_ptr();
        let result_data_ptr = result.data_mut().as_mut_ptr();

        unsafe {
            std::arch::asm!(
                // ===== SETUP: Load B matrix =====
                // Load all 8 rows of B matrix
                "vmovdqu64 zmm16, zmmword ptr [{b_data_ptr}]",        // B[0]
                "vmovdqu64 zmm17, zmmword ptr [{b_data_ptr} + 64]",   // B[1]
                "vmovdqu64 zmm18, zmmword ptr [{b_data_ptr} + 64*2]", // B[2]
                "vmovdqu64 zmm19, zmmword ptr [{b_data_ptr} + 64*3]", // B[3]
                "vmovdqu64 zmm20, zmmword ptr [{b_data_ptr} + 64*4]", // B[4]
                "vmovdqu64 zmm21, zmmword ptr [{b_data_ptr} + 64*5]", // B[5]
                "vmovdqu64 zmm22, zmmword ptr [{b_data_ptr} + 64*6]", // B[6]
                "vmovdqu64 zmm23, zmmword ptr [{b_data_ptr} + 64*7]", // B[7]

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
                "vmovq [{result_data_ptr} + 8*{limb_idx} + 0], xmm0",   // Store result for limb 0
                "vmovq [{result_data_ptr} + 8*{limb_idx} + 8], xmm1",   // Store result for limb 1
                "vmovq [{result_data_ptr} + 8*{limb_idx} + 16], xmm2",  // Store result for limb 2
                "vmovq [{result_data_ptr} + 8*{limb_idx} + 24], xmm3",  // Store result for limb 3
                "vmovq [{result_data_ptr} + 8*{limb_idx} + 32], xmm4",  // Store result for limb 4
                "vmovq [{result_data_ptr} + 8*{limb_idx} + 40], xmm5",  // Store result for limb 5
                "vmovq [{result_data_ptr} + 8*{limb_idx} + 48], xmm6",  // Store result for limb 6

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
                "vmovq [{result_data_ptr} + 504], xmm0",  // Store result for limb 63

                permute1 = const 0b10110001, // Permutation for horizontal XOR
                permute2 = const 0b01001110, // Permutation for horizontal XOR

                // Constraints
                a_data_ptr = in(reg) a_data_ptr,
                b_data_ptr = in(reg) b_data_ptr,
                result_data_ptr = in(reg) result_data_ptr,

                // Counter
                limb_idx = out(reg) _,

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

                options(nostack)
            )
        }

        result
    }
}
