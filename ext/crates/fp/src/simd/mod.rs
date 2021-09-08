cfg_if::cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        mod x86_64;
        use x86_64::*;
    } else {
        mod generic;
        use generic::*;
    }
}

use super::limb::Limb;

const LIMBS_PER_SIMD: usize = std::mem::size_of::<SimdLimb>() / crate::constants::BYTES_PER_LIMB;

pub(crate) fn add_simd(target: &mut [Limb], source: &[Limb], min_limb: usize) {
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
