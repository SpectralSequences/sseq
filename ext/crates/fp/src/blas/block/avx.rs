use super::{MatrixBlock, MatrixBlockSliceMut};

pub fn gemm_block_avx(
    _alpha: bool,
    _a: MatrixBlock,
    _b: MatrixBlock,
    _beta: bool,
    _c: MatrixBlockSliceMut,
) {
    todo!("Implement AVX GEMM block multiplication");
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
