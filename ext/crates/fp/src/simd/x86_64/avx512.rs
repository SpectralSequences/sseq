use std::arch::x86_64;

use crate::limb::Limb;

type SimdLimb = x86_64::__m512i;

#[target_feature(enable = "avx512f")]
fn load(limb: *const Limb) -> SimdLimb {
    unsafe { x86_64::_mm512_loadu_si512(limb as *const SimdLimb) }
}

#[target_feature(enable = "avx512f")]
fn store(limb: *mut Limb, val: SimdLimb) {
    unsafe { x86_64::_mm512_storeu_si512(limb as *mut SimdLimb, val) }
}

#[target_feature(enable = "avx512f")]
fn xor(left: SimdLimb, right: SimdLimb) -> SimdLimb {
    x86_64::_mm512_xor_si512(left, right)
}

super::add_simd_arch!("avx512f");
