use crate::prime::ValidPrime;
pub use crate::vector_inner::initialize_limb_bit_index_table;
use crate::vector_inner::{
    entries_per_64_bits, FpVectorNonZeroIteratorP, FpVectorP, SliceMutP, SliceP,
};
use itertools::Itertools;
#[cfg(feature = "json")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub type FpVector = FpVectorP<2>;
pub type Slice<'a> = SliceP<'a, 2>;
pub type SliceMut<'a> = SliceMutP<'a, 2>;
pub type FpVectorNonZeroIterator<'a> = FpVectorNonZeroIteratorP<'a, 2>;

impl FpVector {
    pub fn new(_p: ValidPrime, dim: usize) -> FpVector {
        FpVector::new_(dim)
    }

    pub fn from_slice(_p: ValidPrime, slice: &[u32]) -> Self {
        Self::from(&slice)
    }

    fn from_limbs(_p: ValidPrime, dim: usize, limbs: Vec<u64>) -> Self {
        Self::from_limbs_(dim, limbs)
    }

    pub fn padded_dimension(p: ValidPrime, dimension: usize) -> usize {
        let entries_per_limb = entries_per_64_bits(p);
        ((dimension + entries_per_limb - 1) / entries_per_limb) * entries_per_limb
    }
}

impl std::fmt::Display for FpVector {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        self.as_slice().fmt(f)
    }
}

impl<'a> std::fmt::Display for Slice<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "[{}]", self.iter().join(", "))?;
        Ok(())
    }
}

impl std::ops::AddAssign<&FpVector> for FpVector {
    fn add_assign(&mut self, other: &FpVector) {
        self.add(other, 1);
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

use saveload::{Load, Save};
use std::io;
use std::io::{Read, Write};

impl Save for FpVector {
    fn save(&self, buffer: &mut impl Write) -> io::Result<()> {
        self.dimension().save(buffer)?;
        for limb in self.limbs() {
            limb.save(buffer)?;
        }
        Ok(())
    }
}

impl Load for FpVector {
    type AuxData = ValidPrime;

    fn load(buffer: &mut impl Read, p: &ValidPrime) -> io::Result<Self> {
        let p = *p;

        let dimension = usize::load(buffer, &())?;

        if dimension == 0 {
            return Ok(FpVector::new(p, 0));
        }

        let entries_per_64_bits = entries_per_64_bits(p);
        let num_limbs = (dimension - 1) / entries_per_64_bits + 1;
        let mut limbs: Vec<u64> = Vec::with_capacity(num_limbs);

        for _ in 0..num_limbs {
            limbs.push(u64::load(buffer, &())?);
        }

        Ok(FpVector::from_limbs(p, dimension, limbs))
    }
}
