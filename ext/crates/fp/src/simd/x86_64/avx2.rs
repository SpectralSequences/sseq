use std::arch::x86_64;

use crate::limb::Limb;

type SimdLimb = x86_64::__m256i;

#[target_feature(enable = "avx2")]
fn load(limb: *const Limb) -> SimdLimb {
    unsafe { x86_64::_mm256_loadu_si256(limb as *const SimdLimb) }
}

#[target_feature(enable = "avx2")]
fn store(limb: *mut Limb, val: SimdLimb) {
    unsafe { x86_64::_mm256_storeu_si256(limb as *mut SimdLimb, val) }
}

#[target_feature(enable = "avx2")]
fn xor(left: SimdLimb, right: SimdLimb) -> SimdLimb {
    x86_64::_mm256_xor_si256(left, right)
}

super::add_simd_arch!("avx2");
