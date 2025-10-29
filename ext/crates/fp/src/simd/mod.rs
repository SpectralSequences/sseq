use crate::limb::Limb;

mod generic;

#[cfg(target_arch = "x86_64")]
mod x86_64;

pub(crate) fn add_simd(target: &mut [Limb], source: &[Limb], min_limb: usize) {
    cfg_if::cfg_if! {
        if #[cfg(any(target_arch = "x86", target_arch = "x86_64"))] {
            x86_64::add_simd(target, source, min_limb)
        } else {
            generic::add_simd(target, source, min_limb)
        }
    }
}
