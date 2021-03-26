pub(crate) type SimdLimb = u64;

pub(crate) unsafe fn load(limb: *const u64) -> SimdLimb {
    *limb
}

pub(crate) unsafe fn store(limb: *mut u64, val: SimdLimb) {
    *limb = val;
}

pub(crate) unsafe fn xor(left: SimdLimb, right: SimdLimb) -> SimdLimb {
    left ^ right
}
