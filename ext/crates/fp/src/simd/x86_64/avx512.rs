use std::arch::x86_64;

use crate::limb::Limb;

type SimdLimb = x86_64::__m512i;

unsafe fn load(limb: *const Limb) -> SimdLimb {
    x86_64::_mm512_loadu_si512(limb as *const SimdLimb)
}

unsafe fn store(limb: *mut Limb, val: SimdLimb) {
    x86_64::_mm512_storeu_si512(limb as *mut SimdLimb, val);
}

unsafe fn xor(left: SimdLimb, right: SimdLimb) -> SimdLimb {
    x86_64::_mm512_xor_si512(left, right)
}

super::add_simd_arch!("avx512f");
