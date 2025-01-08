use crate::constants::BITS_PER_LIMB;
pub(crate) use crate::constants::Limb;

/// A struct containing the information required to access a specific entry in an array of `Limb`s.
#[derive(Debug, Copy, Clone)]
pub(crate) struct LimbBitIndexPair {
    pub(crate) limb: usize,
    pub(crate) bit_index: usize,
}

/// Read an array of `Limb`s.
pub(crate) fn from_bytes(limbs: &mut [Limb], data: &mut impl std::io::Read) -> std::io::Result<()> {
    if cfg!(target_endian = "little") {
        let num_bytes = std::mem::size_of_val(limbs);
        unsafe {
            let buf: &mut [u8] =
                std::slice::from_raw_parts_mut(limbs.as_mut_ptr() as *mut u8, num_bytes);
            data.read_exact(buf).unwrap();
        }
    } else {
        for entry in limbs {
            let mut bytes: [u8; size_of::<Limb>()] = [0; size_of::<Limb>()];
            data.read_exact(&mut bytes)?;
            *entry = Limb::from_le_bytes(bytes);
        }
    };
    Ok(())
}

/// Store an array of `Limb`s.
pub(crate) fn to_bytes(limbs: &[Limb], data: &mut impl std::io::Write) -> std::io::Result<()> {
    let num_limbs = limbs.len();

    if cfg!(target_endian = "little") {
        let num_bytes = std::mem::size_of_val(limbs);
        unsafe {
            let buf: &[u8] = std::slice::from_raw_parts_mut(limbs.as_ptr() as *mut u8, num_bytes);
            data.write_all(buf)?;
        }
    } else {
        for limb in &limbs[0..num_limbs] {
            let bytes = limb.to_le_bytes();
            data.write_all(&bytes)?;
        }
    }
    Ok(())
}

pub(crate) fn sign_rule(mut target: Limb, mut source: Limb) -> u32 {
    let mut result = 0;
    let mut n = 1;
    // Empirically, the compiler unrolls this loop because BITS_PER_LIMB is a constant.
    while 2 * n < BITS_PER_LIMB {
        // This is 1 every 2n bits.
        let mask: Limb = !0 / ((1 << (2 * n)) - 1);
        result ^= (mask & (source >> n) & target).count_ones() % 2;
        source = source ^ (source >> n);
        target = target ^ (target >> n);
        n *= 2;
    }
    result ^= (1 & (source >> (BITS_PER_LIMB / 2)) & target) as u32;
    result
}
