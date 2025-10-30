use crate::limb::Limb;

pub(super) fn add_simd(target: &mut [Limb], source: &[Limb], min_limb: usize) {
    for (target_limb, source_limb) in target.iter_mut().zip(source.iter()).skip(min_limb) {
        *target_limb ^= source_limb
    }
}
