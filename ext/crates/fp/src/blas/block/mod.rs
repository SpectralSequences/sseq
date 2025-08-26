pub mod avx;
pub mod avx512;
pub mod scalar;

pub mod blocks;

pub use blocks::{MatrixBlock, MatrixBlockSlice, MatrixBlockSliceMut};

pub fn gemm_block(
    alpha: bool,
    a: MatrixBlock,
    b: MatrixBlock,
    beta: bool,
    c: MatrixBlock,
) -> MatrixBlock {
    // Call the appropriate BLAS implementation based on the target architecture.
    #[cfg(not(target_arch = "x86_64"))]
    {
        // Fallback to scalar implementation if not on x86_64.
        // TODO: Implement NEON gemm for ARM.
        return scalar::gemm_block_scalar(alpha, a, b, beta, c);
    }

    if is_x86_feature_detected!("avx512f") {
        avx512::gemm_block_avx512(alpha, a, b, beta, c)
    // } else if is_x86_feature_detected!("avx") {
    //     avx::gemm_block_avx(alpha, a, b, beta, c);
    } else {
        // Fallback to scalar implementation if no SIMD support is detected.
        scalar::gemm_block_scalar(alpha, a, b, beta, c)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::{
        matrix::{arbitrary::MatrixArbParams, Matrix},
        prime::TWO,
    };

    proptest! {
        #[test]
        fn test_avx512_is_gemm(
            a in Matrix::arbitrary_with(MatrixArbParams {
                p: Some(TWO),
                rows: Just(64).boxed(),
                columns: Just(64).boxed(),
            }),
            b in Matrix::arbitrary_with(MatrixArbParams {
                p: Some(TWO),
                rows: Just(64).boxed(),
                columns: Just(64).boxed(),
            }),
            mut c in Matrix::arbitrary_with(MatrixArbParams {
                p: Some(TWO),
                rows: Just(64).boxed(),
                columns: Just(64).boxed(),
            }),
            alpha: bool,
            beta: bool,
        ) {
            let mut c2 = c.clone();
            scalar::gemm_block_scalar(
                alpha,
                a.as_tile().block_at(0, 0).gather(),
                b.as_tile().block_at(0, 0).gather(),
                beta,
                c.as_tile_mut().block_mut_at(0, 0).as_slice().gather(),
            );
            avx512::gemm_block_avx512(
                alpha,
                a.as_tile().block_at(0, 0).gather(),
                b.as_tile().block_at(0, 0).gather(),
                beta,
                c2.as_tile_mut().block_mut_at(0, 0).as_slice().gather(),
            );
            prop_assert_eq!(c, c2);
        }

    }
}
