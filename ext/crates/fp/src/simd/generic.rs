use crate::limb::Limb;

pub(crate) type SimdLimb = Limb;

pub(crate) unsafe fn load(limb: *const Limb) -> SimdLimb {
    *limb
}

pub(crate) unsafe fn store(limb: *mut Limb, val: SimdLimb) {
    *limb = val;
}

pub(crate) unsafe fn xor(left: SimdLimb, right: SimdLimb) -> SimdLimb {
    left ^ right
}
