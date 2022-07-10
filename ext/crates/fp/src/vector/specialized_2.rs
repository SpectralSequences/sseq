//! This module replaces `specialized` when `odd-primes` is disabled. Instead of producing enum
//! wrappers, it simply rexports `FooP<2>` as `Foo`.

use super::{
    base_generic::{BaseVectorMutP, BaseVectorP},
    generic::{FpVectorIterator, FpVectorNonZeroIteratorP, FpVectorP, SliceMutP, SliceP},
};
use crate::prime::ValidPrime;

pub type FpVector = FpVectorP<2>;
pub type Slice<'a> = SliceP<'a, 2>;
pub type SliceMut<'a> = SliceMutP<'a, 2>;
pub type FpVectorNonZeroIterator<'a> = FpVectorNonZeroIteratorP<'a, 2>;

pub trait BaseVector: BaseVectorP<2> {}
impl<T: BaseVectorP<2>> BaseVector for T {}

pub trait BaseVectorMut: BaseVectorMutP<2> {}
impl<T: BaseVectorMutP<2>> BaseVectorMut for T {}

impl std::ops::AddAssign<&FpVector> for FpVector {
    fn add_assign(&mut self, other: &FpVector) {
        self.add(other, 1);
    }
}

impl std::fmt::Display for FpVector {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl<'a> IntoIterator for &'a FpVector {
    type IntoIter = FpVectorIterator<'a>;
    type Item = u32;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

macro_rules! impl_try_into {
    ($var:tt, $p:literal) => {
        impl<'a> TryInto<&'a mut FpVectorP<$p>> for &'a mut FpVector {
            type Error = ();

            fn try_into(self) -> Result<&'a mut FpVectorP<$p>, ()> {
                Err(())
            }
        }
    };
}

impl_try_into!(_3, 3);
impl_try_into!(_5, 5);
impl_try_into!(_7, 7);

use itertools::Itertools;
#[cfg(feature = "json")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

impl FpVector {
    pub fn new(_p: ValidPrime, len: usize) -> FpVector {
        Self::new_(len)
    }

    pub fn new_with_capacity(_p: ValidPrime, len: usize, capacity: usize) -> FpVector {
        Self::new_with_capacity_(len, capacity)
    }

    pub fn from_slice(_p: ValidPrime, slice: &[u32]) -> Self {
        Self::from(&slice)
    }
}

impl<'a> std::fmt::Display for Slice<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            for v in self.iter() {
                write!(f, "{v}")?;
            }
            Ok(())
        } else {
            write!(f, "[{}]", self.iter().format(", "))
        }
    }
}
