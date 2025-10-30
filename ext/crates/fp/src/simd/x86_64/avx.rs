use std::arch::x86_64;

use crate::limb::Limb;

type SimdLimb = x86_64::__m256;

#[target_feature(enable = "avx")]
fn load(limb: *const Limb) -> SimdLimb {
    unsafe { x86_64::_mm256_loadu_ps(limb as *const f32) }
}

#[target_feature(enable = "avx")]
fn store(limb: *mut Limb, val: SimdLimb) {
    unsafe { x86_64::_mm256_storeu_ps(limb as *mut f32, val) }
}

#[target_feature(enable = "avx")]
fn xor(left: SimdLimb, right: SimdLimb) -> SimdLimb {
    x86_64::_mm256_xor_ps(left, right)
}

super::add_simd_arch!("avx");
