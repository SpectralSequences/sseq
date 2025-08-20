// use std::arch::x86_64;

use super::{MatrixBlock, MatrixBlockSliceMut};

pub fn gemm_block_avx(
    _alpha: bool,
    _a: MatrixBlock,
    _b: MatrixBlock,
    _beta: bool,
    _c: MatrixBlockSliceMut,
) {
    todo!("Implement AVX GEMM block multiplication");

    // if !beta {
    //     setzero_block_avx(c);
    // }

    // if !alpha {
    //     return;
    // }

    // let all_ones = unsafe { x86_64::_mm256_set1_epi64x(1) };
    // let b_limbs_0_15 = {
    //     let mut x = [unsafe { x86_64::_mm256_setzero_pd() }; 4];
    //     for i in 0..4 {
    //         x[i] = unsafe { x86_64::_mm256_loadu_pd(b.ptr_at(4 * i) as *const f64) };
    //     }
    //     x
    // };
    // let mut result_limbs_0_15 = {
    //     let mut x = [unsafe { x86_64::_mm256_setzero_pd() }; 4];
    //     for i in 0..4 {
    //         x[i] = unsafe { x86_64::_mm256_loadu_pd(c.ptr_at(4 * i) as *const f64) };
    //     }
    //     x
    // };

    // for (a_limb_idx, a_limb) in a.iter().enumerate() {
    //     for a_byte in a_limb.to_le_bytes().iter() {
    //         let neg_a_byte = !a_byte;
    //         let neg_a_lower_nibble = (neg_a_byte & 0x0F) as i64;
    //         let mask = [
    //             (neg_a_lower_nibble & 0x01),
    //             (neg_a_lower_nibble & 0x02) >> 1,
    //             (neg_a_lower_nibble & 0x04) >> 2,
    //             (neg_a_lower_nibble & 0x08) >> 3,
    //         ];
    //         let mask = unsafe { x86_64::_mm256_set_epi64x(mask[3], mask[2], mask[1], mask[0]) };
    //         let mask = unsafe { x86_64::_mm256_sub_epi64(mask, all_ones) };
    //         let mask = unsafe { x86_64::_mm256_castsi256_pd(mask) };

    //         let mut tmp = unsafe { x86_64::_mm256_setzero_pd() };
    //         for i in 0..4 {
    //             let b_ymm = b_limbs_0_15[i];
    //             let b_masked = unsafe { x86_64::_mm256_and_pd(b_ymm, mask) };
    //             tmp = unsafe { x86_64::_mm256_xor_pd(b_masked, tmp) };
    //         }

    //         let neg_a_upper_nibble = ((neg_a_byte >> 4) & 0x0F) as i64;
    //         let mask = [
    //             (neg_a_upper_nibble & 0x01),
    //             (neg_a_upper_nibble & 0x02) >> 1,
    //             (neg_a_upper_nibble & 0x04) >> 2,
    //             (neg_a_upper_nibble & 0x08) >> 3,
    //         ];
    //         let mask = unsafe { x86_64::_mm256_set_epi64x(mask[3], mask[2], mask[1], mask[0]) };
    //         let mask = unsafe { x86_64::_mm256_sub_epi64(mask, all_ones) };
    //         let mask = unsafe { x86_64::_mm256_castsi256_pd(mask) };

    //         for i in 0..4 {
    //             let b_masked = unsafe { x86_64::_mm256_and_pd(b_limbs_0_15[i], mask) };
    //             result_limbs_0_15[a_limb_idx] =
    //                 unsafe { x86_64::_mm256_xor_pd(b_masked, result_limbs_0_15[a_limb_idx]) };
    //         }
    //     }
    // }

    // unsafe {
    //     std::arch::asm!(
    //         // ===== FIRST HALF: Process with B[0-31] =====
    //         // Load first half of B matrix (B[0-31])
    //         "vmovupd ymm8, ymmword ptr [{b_data_ptr}]",         // B[0] (first 4 elements)
    //         "vmovupd ymm9, ymmword ptr [{b_data_ptr} + 32]",    // B[0] (last 4 elements)
    //         "vmovupd ymm10, ymmword ptr [{b_data_ptr} + 64]",   // B[1] (first 4 elements)
    //         "vmovupd ymm11, ymmword ptr [{b_data_ptr} + 96]",   // B[1] (last 4 elements)
    //         "vmovupd ymm12, ymmword ptr [{b_data_ptr} + 128]",  // B[2] (first 4 elements)
    //         "vmovupd ymm13, ymmword ptr [{b_data_ptr} + 160]",  // B[2] (last 4 elements)
    //         "vmovupd ymm14, ymmword ptr [{b_data_ptr} + 192]",  // B[3] (first 4 elements)
    //         "vmovupd ymm15, ymmword ptr [{b_data_ptr} + 224]",  // B[3] (last 4 elements)

    //         // Process all 64 A limbs with first half of B
    //         "mov {limb_idx}, 0",                     // limb_idx = 0

    //         "2:",                                    // Loop through A limbs
    //         // Load current A limb
    //         "mov {a_limb}, [{a_data_ptr} + {limb_idx} * 8]",

    //         // Initialize result for this limb
    //         "vxorpd ymm0, ymm0, ymm0",               // result (first 4 elements)
    //         "vxorpd ymm1, ymm1, ymm1",               // result (last 4 elements)

    //         // Process nibbles 0-7 (first 32 bits) with B[0-31]
    //         "mov {nibble_idx}, 0",                   // nibble_idx = 0

    //         "3:",                                    // Loop through nibbles
    //         // Extract current nibble (4 bits)
    //         "mov {nibble}, {a_limb}",
    //         "mov {shift_amount}, {nibble_idx}",
    //         "shl {shift_amount}, 2",                 // shift_amount = nibble_idx * 4
    //         "mov cl, {shift_amount:l}",              // Move to CL for shift
    //         "shr {nibble}, cl",                      // Shift to get nibble
    //         "and {nibble}, 0xF",                     // Extract 4 bits

    //         // Process each bit of the nibble with corresponding B row
    //         "mov {bit_mask}, {nibble}",
    //         "and {bit_mask}, 1",                     // bit 0
    //         "neg {bit_mask}",
    //         "vmovq xmm2, {bit_mask}",
    //         "vbroadcastsd ymm2, xmm2",
    //         "vpand ymm3, ymm8, ymm2",
    //         "vxorpd ymm0, ymm0, ymm3",
    //         "vpand ymm3, ymm9, ymm2",
    //         "vxorpd ymm1, ymm1, ymm3",

    //         "mov {bit_mask}, {nibble}",
    //         "shr {bit_mask}, 1",
    //         "and {bit_mask}, 1",                     // bit 1
    //         "neg {bit_mask}",
    //         "vmovq xmm2, {bit_mask}",
    //         "vbroadcastsd ymm2, xmm2",
    //         "vpand ymm3, ymm10, ymm2",
    //         "vxorpd ymm0, ymm0, ymm3",
    //         "vpand ymm3, ymm11, ymm2",
    //         "vxorpd ymm1, ymm1, ymm3",

    //         "mov {bit_mask}, {nibble}",
    //         "shr {bit_mask}, 2",
    //         "and {bit_mask}, 1",                     // bit 2
    //         "neg {bit_mask}",
    //         "vmovq xmm2, {bit_mask}",
    //         "vbroadcastsd ymm2, xmm2",
    //         "vpand ymm3, ymm12, ymm2",
    //         "vxorpd ymm0, ymm0, ymm3",
    //         "vpand ymm3, ymm13, ymm2",
    //         "vxorpd ymm1, ymm1, ymm3",

    //         "mov {bit_mask}, {nibble}",
    //         "shr {bit_mask}, 3",
    //         "and {bit_mask}, 1",                     // bit 3
    //         "neg {bit_mask}",
    //         "vmovq xmm2, {bit_mask}",
    //         "vbroadcastsd ymm2, xmm2",
    //         "vpand ymm3, ymm14, ymm2",
    //         "vxorpd ymm0, ymm0, ymm3",
    //         "vpand ymm3, ymm15, ymm2",
    //         "vxorpd ymm1, ymm1, ymm3",

    //         // Load next 4 B rows for next nibble
    //         "inc {nibble_idx}",
    //         "cmp {nibble_idx}, 8",                   // Check if done with all 8 nibbles
    //         "jge 4f",                                // Jump out if done

    //         // Calculate B offset for next nibble: nibble_idx * 4 * 64 bytes
    //         "mov {b_offset}, {nibble_idx}",
    //         "shl {b_offset}, 8",                     // b_offset = nibble_idx * 256
    //         "vmovupd ymm8, ymmword ptr [{b_data_ptr} + {b_offset}]",
    //         "vmovupd ymm9, ymmword ptr [{b_data_ptr} + {b_offset} + 32]",
    //         "vmovupd ymm10, ymmword ptr [{b_data_ptr} + {b_offset} + 64]",
    //         "vmovupd ymm11, ymmword ptr [{b_data_ptr} + {b_offset} + 96]",
    //         "vmovupd ymm12, ymmword ptr [{b_data_ptr} + {b_offset} + 128]",
    //         "vmovupd ymm13, ymmword ptr [{b_data_ptr} + {b_offset} + 160]",
    //         "vmovupd ymm14, ymmword ptr [{b_data_ptr} + {b_offset} + 192]",
    //         "vmovupd ymm15, ymmword ptr [{b_data_ptr} + {b_offset} + 224]",

    //         "jmp 3b",                                // Continue nibble loop

    //         "4:",                                    // End of nibbles for this limb
    //         // Horizontal XOR to reduce results
    //         "vperm2f128 ymm4, ymm0, ymm0, 0x01",
    //         "vxorpd ymm0, ymm0, ymm4",
    //         "vhaddpd ymm0, ymm0, ymm0",

    //         "vperm2f128 ymm4, ymm1, ymm1, 0x01",
    //         "vxorpd ymm1, ymm1, ymm4",
    //         "vhaddpd ymm1, ymm1, ymm1",
    //         "vxorpd ymm0, ymm0, ymm1",

    //         "vmovhlps xmm4, xmm4, xmm0",
    //         "vxorpd xmm0, xmm0, xmm4",

    //         // Store half-computed result (C += half_result)
    //         "vmovq xmm5, qword ptr [{c_data_ptr} + {limb_idx} * 8]",
    //         "vxorpd xmm0, xmm0, xmm5",
    //         "vmovq [{c_data_ptr} + {limb_idx} * 8], xmm0",

    //         // Next A limb
    //         "inc {limb_idx}",
    //         "cmp {limb_idx}, 64",
    //         "jl 2b",                                 // Continue with next limb

    //         // ===== SECOND HALF: Process with B[32-63] =====
    //         // Load second half of B matrix (B[32-63])
    //         "vmovupd ymm8, ymmword ptr [{b_data_ptr} + 2048]",      // B[32] (first 4 elements)
    //         "vmovupd ymm9, ymmword ptr [{b_data_ptr} + 2080]",      // B[32] (last 4 elements)
    //         "vmovupd ymm10, ymmword ptr [{b_data_ptr} + 2112]",     // B[33] (first 4 elements)
    //         "vmovupd ymm11, ymmword ptr [{b_data_ptr} + 2144]",     // B[33] (last 4 elements)
    //         "vmovupd ymm12, ymmword ptr [{b_data_ptr} + 2176]",     // B[34] (first 4 elements)
    //         "vmovupd ymm13, ymmword ptr [{b_data_ptr} + 2208]",     // B[34] (last 4 elements)
    //         "vmovupd ymm14, ymmword ptr [{b_data_ptr} + 2240]",     // B[35] (first 4 elements)
    //         "vmovupd ymm15, ymmword ptr [{b_data_ptr} + 2272]",     // B[35] (last 4 elements)

    //         // Process all 64 A limbs with second half of B
    //         "mov {limb_idx}, 0",                     // limb_idx = 0

    //         "5:",                                    // Loop through A limbs (second pass)
    //         // Load current A limb
    //         "mov {a_limb}, [{a_data_ptr} + {limb_idx} * 8]",

    //         // Initialize result for this limb
    //         "vxorpd ymm0, ymm0, ymm0",               // result (first 4 elements)
    //         "vxorpd ymm1, ymm1, ymm1",               // result (last 4 elements)

    //         // Process nibbles 8-15 (second 32 bits) with B[32-63]
    //         "mov {nibble_idx}, 8",                   // Start from nibble 8

    //         "6:",                                    // Loop through nibbles 8-15
    //         // Extract current nibble
    //         "mov {nibble}, {a_limb}",
    //         "mov {shift_amount}, {nibble_idx}",
    //         "shl {shift_amount}, 2",                 // shift_amount = nibble_idx * 4
    //         "mov cl, {shift_amount:l}",
    //         "shr {nibble}, cl",
    //         "and {nibble}, 0xF",

    //         // Process nibble with current B rows (same pattern as before)
    //         "mov {bit_mask}, {nibble}",
    //         "and {bit_mask}, 1",
    //         "neg {bit_mask}",
    //         "vmovq xmm2, {bit_mask}",
    //         "vbroadcastsd ymm2, xmm2",
    //         "vpand ymm3, ymm8, ymm2",
    //         "vxorpd ymm0, ymm0, ymm3",
    //         "vpand ymm3, ymm9, ymm2",
    //         "vxorpd ymm1, ymm1, ymm3",

    //         "mov {bit_mask}, {nibble}",
    //         "shr {bit_mask}, 1",
    //         "and {bit_mask}, 1",
    //         "neg {bit_mask}",
    //         "vmovq xmm2, {bit_mask}",
    //         "vbroadcastsd ymm2, xmm2",
    //         "vpand ymm3, ymm10, ymm2",
    //         "vxorpd ymm0, ymm0, ymm3",
    //         "vpand ymm3, ymm11, ymm2",
    //         "vxorpd ymm1, ymm1, ymm3",

    //         "mov {bit_mask}, {nibble}",
    //         "shr {bit_mask}, 2",
    //         "and {bit_mask}, 1",
    //         "neg {bit_mask}",
    //         "vmovq xmm2, {bit_mask}",
    //         "vbroadcastsd ymm2, xmm2",
    //         "vpand ymm3, ymm12, ymm2",
    //         "vxorpd ymm0, ymm0, ymm3",
    //         "vpand ymm3, ymm13, ymm2",
    //         "vxorpd ymm1, ymm1, ymm3",

    //         "mov {bit_mask}, {nibble}",
    //         "shr {bit_mask}, 3",
    //         "and {bit_mask}, 1",
    //         "neg {bit_mask}",
    //         "vmovq xmm2, {bit_mask}",
    //         "vbroadcastsd ymm2, xmm2",
    //         "vpand ymm3, ymm14, ymm2",
    //         "vxorpd ymm0, ymm0, ymm3",
    //         "vpand ymm3, ymm15, ymm2",
    //         "vxorpd ymm1, ymm1, ymm3",

    //         // Load next 4 B rows for next nibble
    //         "inc {nibble_idx}",
    //         "cmp {nibble_idx}, 16",                  // Check if done with nibbles 8-15
    //         "jge 7f",

    //         // Calculate B offset: base (2048) + (nibble_idx - 8) * 4 * 64
    //         "mov {b_offset}, {nibble_idx}",
    //         "sub {b_offset}, 8",                     // Adjust for second half
    //         "shl {b_offset}, 8",                     // * 256
    //         "add {b_offset}, 2048",                  // Add base offset for B[32-63]
    //         "vmovupd ymm8, ymmword ptr [{b_data_ptr} + {b_offset}]",
    //         "vmovupd ymm9, ymmword ptr [{b_data_ptr} + {b_offset} + 32]",
    //         "vmovupd ymm10, ymmword ptr [{b_data_ptr} + {b_offset} + 64]",
    //         "vmovupd ymm11, ymmword ptr [{b_data_ptr} + {b_offset} + 96]",
    //         "vmovupd ymm12, ymmword ptr [{b_data_ptr} + {b_offset} + 128]",
    //         "vmovupd ymm13, ymmword ptr [{b_data_ptr} + {b_offset} + 160]",
    //         "vmovupd ymm14, ymmword ptr [{b_data_ptr} + {b_offset} + 192]",
    //         "vmovupd ymm15, ymmword ptr [{b_data_ptr} + {b_offset} + 224]",

    //         "jmp 6b",

    //         "7:",                                    // End of second pass for this limb
    //         // Horizontal XOR to reduce results
    //         "vperm2f128 ymm4, ymm0, ymm0, 0x01",
    //         "vxorpd ymm0, ymm0, ymm4",
    //         "vhaddpd ymm0, ymm0, ymm0",

    //         "vperm2f128 ymm4, ymm1, ymm1, 0x01",
    //         "vxorpd ymm1, ymm1, ymm4",
    //         "vhaddpd ymm1, ymm1, ymm1",
    //         "vxorpd ymm0, ymm0, ymm1",

    //         "vmovhlps xmm4, xmm4, xmm0",
    //         "vxorpd xmm0, xmm0, xmm4",

    //         // Add to existing C value (completing the GEMM: C += second_half)
    //         "vmovq xmm5, qword ptr [{c_data_ptr} + {limb_idx} * 8]",
    //         "vxorpd xmm0, xmm0, xmm5",
    //         "vmovq [{c_data_ptr} + {limb_idx} * 8], xmm0",

    //         // Next A limb
    //         "inc {limb_idx}",
    //         "cmp {limb_idx}, 64",
    //         "jl 5b",                                 // Continue with next limb

    //         // Constraints
    //         a_data_ptr = in(reg) a.limbs.as_ptr(),
    //         b_data_ptr = in(reg) b.limbs.as_ptr(),
    //         c_data_ptr = in(reg) c.limbs.as_mut_ptr(),

    //         // Loop variables
    //         limb_idx = out(reg) _,
    //         nibble_idx = out(reg) _,
    //         a_limb = out(reg) _,
    //         nibble = out(reg) _,
    //         bit_mask = out(reg) _,
    //         b_offset = out(reg) _,
    //         shift_amount = out(reg) _,

    //         // YMM registers
    //         out("ymm0") _, out("ymm1") _,     // Result accumulators
    //         out("ymm2") _, out("ymm3") _, out("ymm4") _, out("ymm5") _,     // Temp registers
    //         out("ymm8") _, out("ymm9") _, out("ymm10") _, out("ymm11") _, // B matrix rows
    //         out("ymm12") _, out("ymm13") _, out("ymm14") _, out("ymm15") _, // B matrix rows

    //         options(nostack, preserves_flags)
    //     );
    // }
}

pub fn setzero_block_avx(c: &mut MatrixBlockSliceMut) {
    unsafe {
        std::arch::asm! {
            "vpxor ymm0, ymm0, ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*0], ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*1], ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*2], ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*3], ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*4], ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*5], ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*6], ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*7], ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*8], ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*9], ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*10], ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*11], ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*12], ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*13], ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*14], ymm0",
            "vmovdqu ymmword ptr [{c_data_ptr} + 32*15], ymm0",

            c_data_ptr = in(reg) c.limbs,

            out("ymm0") _,

            options(nostack, preserves_flags)
        }
    }
}
