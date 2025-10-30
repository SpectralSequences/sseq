use std::arch::x86_64;

use crate::limb::Limb;

type SimdLimb = x86_64::__m128i;

#[target_feature(enable = "sse2")]
fn load(limb: *const Limb) -> SimdLimb {
    unsafe { x86_64::_mm_loadu_si128(limb as *const SimdLimb) }
}

#[target_feature(enable = "sse2")]
fn store(limb: *mut Limb, val: SimdLimb) {
    unsafe { x86_64::_mm_storeu_si128(limb as *mut SimdLimb, val) }
}

#[target_feature(enable = "sse2")]
fn xor(left: SimdLimb, right: SimdLimb) -> SimdLimb {
    x86_64::_mm_xor_si128(left, right)
}

super::add_simd_arch!("sse2");
