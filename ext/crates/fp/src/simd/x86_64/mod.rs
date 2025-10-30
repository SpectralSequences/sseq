mod avx;
mod avx2;
mod avx512;
mod sse2;

use crate::limb::Limb;

macro_rules! add_simd_arch {
    ($arch:tt) => {
        const LIMBS_PER_SIMD: usize =
            std::mem::size_of::<SimdLimb>() / crate::constants::BYTES_PER_LIMB;

        #[target_feature(enable = $arch)]
        pub(super) fn add_simd(target: &mut [Limb], source: &[Limb], min_limb: usize) {
            let max_limb = target.len();
            let target = target.as_mut_ptr();
            let source = source.as_ptr();
            let chunks = (max_limb - min_limb) / LIMBS_PER_SIMD;
            for i in 0..chunks {
                unsafe {
                    let mut target_chunk = load(target.add(LIMBS_PER_SIMD * i + min_limb));
                    let source_chunk = load(source.add(LIMBS_PER_SIMD * i + min_limb));
                    target_chunk = xor(target_chunk, source_chunk);
                    store(target.add(LIMBS_PER_SIMD * i + min_limb), target_chunk);
                }
            }
            for i in (min_limb + LIMBS_PER_SIMD * chunks)..max_limb {
                unsafe {
                    // pointer arithmetic
                    *target.add(i) = *target.add(i) ^ *source.add(i);
                }
            }
        }
    };
}

use add_simd_arch;

pub(super) fn add_simd(target: &mut [Limb], source: &[Limb], min_limb: usize) {
    if is_x86_feature_detected!("avx512f") {
        unsafe { avx512::add_simd(target, source, min_limb) }
    } else if is_x86_feature_detected!("avx2") {
        unsafe { avx2::add_simd(target, source, min_limb) }
    } else if is_x86_feature_detected!("avx") {
        unsafe { avx::add_simd(target, source, min_limb) }
    } else if is_x86_feature_detected!("sse2") {
        unsafe { sse2::add_simd(target, source, min_limb) }
    } else {
        super::generic::add_simd(target, source, min_limb)
    }
}
