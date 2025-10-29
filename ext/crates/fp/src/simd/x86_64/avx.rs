use std::arch::x86_64;

use crate::limb::Limb;

pub(crate) type SimdLimb = x86_64::__m256;

pub(crate) unsafe fn load(limb: *const Limb) -> SimdLimb {
    x86_64::_mm256_loadu_ps(limb as *const f32)
}

pub(crate) unsafe fn store(limb: *mut Limb, val: SimdLimb) {
    x86_64::_mm256_storeu_ps(limb as *mut f32, val);
}

pub(crate) unsafe fn xor(left: SimdLimb, right: SimdLimb) -> SimdLimb {
    x86_64::_mm256_xor_ps(left, right)
}

super::add_simd_arch!("avx");
