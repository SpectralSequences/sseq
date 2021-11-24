//! This module replaces `vector` when `odd-primes` is disabled. Instead of producing enum
//! wrappers, it simply rexports `FooP<2>` as `Foo`.

use crate::limb::{entries_per_limb_const, Limb};
use crate::prime::ValidPrime;
use crate::vector_inner::{FpVectorNonZeroIteratorP, FpVectorP, SliceMutP, SliceP};
use itertools::Itertools;
#[cfg(feature = "json")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::io::{Read, Write};
use std::mem::size_of;

pub type FpVector = FpVectorP<2>;
pub type Slice<'a> = SliceP<'a, 2>;
pub type SliceMut<'a> = SliceMutP<'a, 2>;
pub type FpVectorNonZeroIterator<'a> = FpVectorNonZeroIteratorP<'a, 2>;

impl FpVector {
    pub fn new(_p: ValidPrime, len: usize) -> FpVector {
        FpVector::new_(len)
    }

    pub fn new_with_capacity(_p: ValidPrime, len: usize, capacity: usize) -> FpVector {
        FpVector::new_with_capacity_(len, capacity)
    }

    pub fn from_slice(_p: ValidPrime, slice: &[u32]) -> Self {
        Self::from(&slice)
    }

    pub fn num_limbs(_p: ValidPrime, len: usize) -> usize {
        let entries_per_limb = entries_per_limb_const::<2>();
        (len + entries_per_limb - 1) / entries_per_limb
    }

    pub fn padded_len(p: ValidPrime, len: usize) -> usize {
        Self::num_limbs(p, len) * entries_per_limb_const::<2>()
    }

    pub fn from_bytes(p: ValidPrime, len: usize, data: &mut impl Read) -> std::io::Result<Self> {
        let num_limbs = Self::num_limbs(p, len);
        let mut limbs = Vec::with_capacity(num_limbs);

        for _ in 0..num_limbs {
            let mut bytes: [u8; size_of::<Limb>()] = [0; size_of::<Limb>()];
            data.read_exact(&mut bytes)?;
            limbs.push(Limb::from_le_bytes(bytes));
        }
        Ok(Self::from_raw_parts(len, limbs))
    }

    pub fn to_bytes(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        let num_limbs = Self::num_limbs(self.prime(), self.len());
        // self.limbs is allowed to have more limbs than necessary, but we only save the
        // necessary ones.

        for limb in &self.limbs()[0..num_limbs] {
            let bytes = limb.to_le_bytes();
            buffer.write_all(&bytes)?;
        }
        Ok(())
    }
}

impl std::fmt::Display for FpVector {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl<'a> std::fmt::Display for Slice<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[{}]", self.iter().format(", "))?;
        Ok(())
    }
}

impl std::ops::AddAssign<&FpVector> for FpVector {
    fn add_assign(&mut self, other: &FpVector) {
        self.add(other, 1);
    }
}

impl<'a> IntoIterator for &'a FpVector {
    type IntoIter = crate::vector_inner::FpVectorIterator<'a>;
    type Item = u32;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(feature = "json")]
impl Serialize for FpVector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Vec::<u32>::from(self).serialize(serializer)
    }
}

#[cfg(feature = "json")]
impl<'de> Deserialize<'de> for FpVector {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        panic!("Deserializing FpVector not supported");
        // This is needed for ext-websocket/actions to be happy
    }
}
