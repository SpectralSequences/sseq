use std::arch::x86_64;

use crate::limb::Limb;

pub(crate) type SimdLimb = x86_64::__m128i;

pub(crate) unsafe fn load(limb: *const Limb) -> SimdLimb {
    x86_64::_mm_loadu_si128(limb as *const SimdLimb)
}

pub(crate) unsafe fn store(limb: *mut Limb, val: SimdLimb) {
    x86_64::_mm_storeu_si128(limb as *mut SimdLimb, val)
}

pub(crate) unsafe fn xor(left: SimdLimb, right: SimdLimb) -> SimdLimb {
    x86_64::_mm_xor_si128(left, right)
}

super::add_simd_arch!("sse2");
