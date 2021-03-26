#[allow(unused_imports)]
use std::arch::x86_64;

cfg_if::cfg_if! {
    if #[cfg(target_feature="avx512f")] {
        pub(crate) type SimdLimb = x86_64::__m512i;
    } else if #[cfg(target_feature="avx2")] {
        pub(crate) type SimdLimb = x86_64::__m256i;
    } else if #[cfg(target_feature="avx")] {
        pub(crate) type SimdLimb = x86_64::__m256;
    } else {
        pub(crate) type SimdLimb = u64;
    }
}

pub(crate) unsafe fn load(limb: *const u64) -> SimdLimb {
    cfg_if::cfg_if! {
        if #[cfg(target_feature="avx512f")] {
            x86_64::_mm512_loadu_si512(limb as *const i32)
        } else if #[cfg(target_feature="avx2")] {
            x86_64::_mm256_loadu_si256(limb as *const SimdLimb)
        } else if #[cfg(target_feature="avx")] {
            x86_64::_mm256_loadu_ps(limb as *const f32)
        } else {
            *limb
        }
    }
}

pub(crate) unsafe fn store(limb: *mut u64, val: SimdLimb) {
    cfg_if::cfg_if! {
        if #[cfg(target_feature="avx512f")] {
            x86_64::_mm512_storeu_si512(limb as *mut i32, val);
        } else if #[cfg(target_feature="avx2")] {
            x86_64::_mm256_storeu_si256(limb as *mut SimdLimb, val);
        } else if #[cfg(target_feature="avx")] {
            x86_64::_mm256_storeu_ps(limb as *mut f32, val);
        } else {
            *limb = val;
        }
    }
}

pub(crate) unsafe fn xor(left: SimdLimb, right: SimdLimb) -> SimdLimb {
    cfg_if::cfg_if! {
        if #[cfg(target_feature="avx512f")] {
            x86_64::_mm512_xor_si512(left, right)
        } else if #[cfg(target_feature="avx2")] {
            x86_64::_mm256_xor_si256(left, right)
        } else if #[cfg(target_feature="avx")] {
            x86_64::_mm256_xor_ps(left, right)
        } else {
            left ^ right
        }
    }
}
