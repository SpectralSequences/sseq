//! This module is provides wrappers around the contents of [`crate::vector::inner`]. The main
//! purpose is to put [`FpVectorP`] for different `p` into a single enum. It does the same for the
//! various slice structs.
//!
//! The main magic occurs in the macro `dispatch_vector_inner`, which we use to provide wrapper
//! functions around the `FpVectorP` functions.
//!
//! This module is only used when the `odd-primes` feature is enabled.

use std::{
    io::{Read, Write},
    mem::size_of,
};

use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::iter::{FpVectorIteratorP, FpVectorNonZeroIteratorP};
use crate::{
    field::{limb::LimbMethods, Fp},
    limb::Limb,
    prime::{Prime, ValidPrime, P2, P3, P5, P7},
    vector::inner::{FqVectorP, SliceMutP, SliceP},
};

macro_rules! dispatch_vector_inner {
    // other is a type, but marking it as a :ty instead of :tt means we cannot use it to access its
    // enum variants.
    ($vis:vis fn $method:ident(&self, other: &$other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        $vis fn $method(&self, other: &$other, $($arg: $ty),* ) $(-> $ret)* {
            match (self, other) {
                (Self::_2(x), $other::_2(y)) => x.$method(y, $($arg),*),
                (Self::_3(x), $other::_3(y)) => x.$method(y, $($arg),*),
                (Self::_5(x), $other::_5(y)) => x.$method(y, $($arg),*),
                (Self::_7(x), $other::_7(y)) => x.$method(y, $($arg),*),
                (Self::Big(x), $other::Big(y)) if x.prime() == y.prime() => x.$method(y, $($arg),*),
                (l, r) => {
                    panic!("Applying {} to vectors over different primes ({} and {})", stringify!($method), l.prime(), r.prime());
                }
            }
        }
    };
    ($vis:vis fn $method:ident(&mut self, other: &$other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&mut self, other: &$other, $($arg: $ty),* ) $(-> $ret)* {
            match (self, other) {
                (Self::_2(x), $other::_2(y)) => x.$method(y, $($arg),*),
                (Self::_3(x), $other::_3(y)) => x.$method(y, $($arg),*),
                (Self::_5(x), $other::_5(y)) => x.$method(y, $($arg),*),
                (Self::_7(x), $other::_7(y)) => x.$method(y, $($arg),*),
                (Self::Big(x), $other::Big(y)) if x.prime() == y.prime() => x.$method(y, $($arg),*),
                (l, r) => {
                    panic!("Applying {} to vectors over different primes ({} and {})", stringify!($method), l.prime(), r.prime());
                }
            }
        }
    };
    ($vis:vis fn $method:ident(&mut self, other: $other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        $vis fn $method(&mut self, other: $other, $($arg: $ty),* ) $(-> $ret)* {
            match (self, other) {
                (Self::_2(x), $other::_2(y)) => x.$method(y, $($arg),*),
                (Self::_3(x), $other::_3(y)) => x.$method(y, $($arg),*),
                (Self::_5(x), $other::_5(y)) => x.$method(y, $($arg),*),
                (Self::_7(x), $other::_7(y)) => x.$method(y, $($arg),*),
                (Self::Big(x), $other::Big(y)) if x.prime() == y.prime() => x.$method(y, $($arg),*),
                (l, r) => {
                    panic!("Applying {} to vectors over different primes ({} and {})", stringify!($method), l.prime(), r.prime());
                }
            }
        }
    };
    ($vis:vis fn $method:ident(&mut self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {
        #[must_use]
        $vis fn $method(&mut self, $($arg: $ty),* ) -> $ret {
            match self {
                Self::_2(x) => $ret::_2(x.$method($($arg),*)),
                Self::_3(x) => $ret::_3(x.$method($($arg),*)),
                Self::_5(x) => $ret::_5(x.$method($($arg),*)),
                Self::_7(x) => $ret::_7(x.$method($($arg),*)),
                Self::Big(x) => $ret::Big(x.$method($($arg),*)),
            }
        }
    };
    ($vis:vis fn $method:ident(&self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {
        #[must_use]
        $vis fn $method(&self, $($arg: $ty),* ) -> $ret {
            match self {
                Self::_2(x) => $ret::_2(x.$method($($arg),*)),
                Self::_3(x) => $ret::_3(x.$method($($arg),*)),
                Self::_5(x) => $ret::_5(x.$method($($arg),*)),
                Self::_7(x) => $ret::_7(x.$method($($arg),*)),
                Self::Big(x) => $ret::Big(x.$method($($arg),*)),
            }
        }
    };
    ($vis:vis fn $method:ident(self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {
        #[must_use]
        $vis fn $method(self, $($arg: $ty),* ) -> $ret {
            match self {
                Self::_2(x) => $ret::_2(x.$method($($arg),*)),
                Self::_3(x) => $ret::_3(x.$method($($arg),*)),
                Self::_5(x) => $ret::_5(x.$method($($arg),*)),
                Self::_7(x) => $ret::_7(x.$method($($arg),*)),
                Self::Big(x) => $ret::Big(x.$method($($arg),*)),
            }
        }
    };

    ($vis:vis fn $method:ident(self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt $lifetime:tt)) => {
        #[must_use]
        $vis fn $method(self, $($arg: $ty),* ) -> $ret<$lifetime> {
            match self {
                Self::_2(x) => $ret::_2(x.$method($($arg),*)),
                Self::_3(x) => $ret::_3(x.$method($($arg),*)),
                Self::_5(x) => $ret::_5(x.$method($($arg),*)),
                Self::_7(x) => $ret::_7(x.$method($($arg),*)),
                Self::Big(x) => $ret::Big(x.$method($($arg),*)),
            }
        }
    };

    ($vis:vis fn $method:ident(&mut self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&mut self, $($arg: $ty),* ) $(-> $ret)* {
            match self {
                Self::_2(x) => x.$method($($arg),*),
                Self::_3(x) => x.$method($($arg),*),
                Self::_5(x) => x.$method($($arg),*),
                Self::_7(x) => x.$method($($arg),*),
                Self::Big(x) => x.$method($($arg),*),
            }
        }
    };
    ($vis:vis fn $method:ident(&self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&self, $($arg: $ty),* ) $(-> $ret)* {
            match self {
                Self::_2(x) => x.$method($($arg),*),
                Self::_3(x) => x.$method($($arg),*),
                Self::_5(x) => x.$method($($arg),*),
                Self::_7(x) => x.$method($($arg),*),
                Self::Big(x) => x.$method($($arg),*),
            }
        }
    };
    ($vis:vis fn $method:ident(self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(self, $($arg: $ty),* ) $(-> $ret)* {
            match self {
                Self::_2(x) => x.$method($($arg),*),
                Self::_3(x) => x.$method($($arg),*),
                Self::_5(x) => x.$method($($arg),*),
                Self::_7(x) => x.$method($($arg),*),
                Self::Big(x) => x.$method($($arg),*),
            }
        }
    }
}

macro_rules! dispatch_vector {
    () => {};
    ($vis:vis fn $method:ident $tt:tt $(-> $ret:tt)?; $($tail:tt)*) => {
        dispatch_vector_inner! {
            $vis fn $method $tt $(-> $ret)*
        }
        dispatch_vector!{$($tail)*}
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum FpVector {
    _2(FqVectorP<Fp<P2>>),
    _3(FqVectorP<Fp<P3>>),
    _5(FqVectorP<Fp<P5>>),
    _7(FqVectorP<Fp<P7>>),
    Big(FqVectorP<Fp<ValidPrime>>),
}

#[derive(Debug, Copy, Clone)]
pub enum Slice<'a> {
    _2(SliceP<'a, Fp<P2>>),
    _3(SliceP<'a, Fp<P3>>),
    _5(SliceP<'a, Fp<P5>>),
    _7(SliceP<'a, Fp<P7>>),
    Big(SliceP<'a, Fp<ValidPrime>>),
}

#[derive(Debug)]
pub enum SliceMut<'a> {
    _2(SliceMutP<'a, Fp<P2>>),
    _3(SliceMutP<'a, Fp<P3>>),
    _5(SliceMutP<'a, Fp<P5>>),
    _7(SliceMutP<'a, Fp<P7>>),
    Big(SliceMutP<'a, Fp<ValidPrime>>),
}

pub enum FpVectorIterator<'a> {
    _2(FpVectorIteratorP<'a, Fp<P2>>),
    _3(FpVectorIteratorP<'a, Fp<P3>>),
    _5(FpVectorIteratorP<'a, Fp<P5>>),
    _7(FpVectorIteratorP<'a, Fp<P7>>),
    Big(FpVectorIteratorP<'a, Fp<ValidPrime>>),
}

pub enum FpVectorNonZeroIterator<'a> {
    _2(FpVectorNonZeroIteratorP<'a, Fp<P2>>),
    _3(FpVectorNonZeroIteratorP<'a, Fp<P3>>),
    _5(FpVectorNonZeroIteratorP<'a, Fp<P5>>),
    _7(FpVectorNonZeroIteratorP<'a, Fp<P7>>),
    Big(FpVectorNonZeroIteratorP<'a, Fp<ValidPrime>>),
}

impl FpVector {
    dispatch_vector! {
        pub fn prime(&self) -> ValidPrime;
        pub fn len(&self) -> usize;
        pub fn is_empty(&self) -> bool;
        pub fn scale(&mut self, c: u32);
        pub fn set_to_zero(&mut self);
        pub fn entry(&self, index: usize) -> u32;
        pub fn set_entry(&mut self, index: usize, value: u32);
        pub fn assign(&mut self, other: &Self);
        pub fn assign_partial(&mut self, other: &Self);
        pub fn add(&mut self, other: &Self, c: u32);
        pub fn add_offset(&mut self, other: &Self, c: u32, offset: usize);
        pub fn slice(&self, start: usize, end: usize) -> (dispatch Slice);
        pub fn as_slice(&self) -> (dispatch Slice);
        pub fn slice_mut(&mut self, start: usize, end: usize) -> (dispatch SliceMut);
        pub fn as_slice_mut(&mut self) -> (dispatch SliceMut);
        pub fn is_zero(&self) -> bool;
        pub fn iter(&self) -> (dispatch FpVectorIterator);
        pub fn iter_nonzero(&self) -> (dispatch FpVectorNonZeroIterator);
        pub fn extend_len(&mut self, dim: usize);
        pub fn set_scratch_vector_size(&mut self, dim: usize);
        pub fn add_basis_element(&mut self, index: usize, value: u32);
        pub fn copy_from_slice(&mut self, slice: &[u32]);
        pub(crate) fn trim_start(&mut self, n: usize);
        pub fn add_truncate(&mut self, other: &Self, c: u32) -> (Option<()>);
        pub fn sign_rule(&self, other: &Self) -> bool;
        pub fn add_carry(&mut self, other: &Self, c: u32, rest: &mut [FpVector]) -> bool;
        pub fn first_nonzero(&self) -> (Option<(usize, u32)>);
        pub fn density(&self) -> f32;

        pub(crate) fn limbs(&self) -> (&[Limb]);
        pub(crate) fn limbs_mut(&mut self) -> (&mut [Limb]);
    }

    pub fn new<P: Prime>(p: P, len: usize) -> FpVector {
        match p.as_u32() {
            2 => FpVector::_2(FqVectorP::new(Fp(P2), len)),
            3 => FpVector::_3(FqVectorP::new(Fp(P3), len)),
            5 => FpVector::_5(FqVectorP::new(Fp(P5), len)),
            7 => FpVector::_7(FqVectorP::new(Fp(P7), len)),
            _ => FpVector::Big(FqVectorP::new(Fp(p.to_dyn()), len)),
        }
    }

    pub fn new_with_capacity<P: Prime>(p: P, len: usize, capacity: usize) -> FpVector {
        match p.as_u32() {
            2 => FpVector::_2(FqVectorP::new_with_capacity(Fp(P2), len, capacity)),
            3 => FpVector::_3(FqVectorP::new_with_capacity(Fp(P3), len, capacity)),
            5 => FpVector::_5(FqVectorP::new_with_capacity(Fp(P5), len, capacity)),
            7 => FpVector::_7(FqVectorP::new_with_capacity(Fp(P7), len, capacity)),
            _ => FpVector::Big(FqVectorP::new_with_capacity(Fp(p.to_dyn()), len, capacity)),
        }
    }

    pub fn from_slice<P: Prime>(p: P, slice: &[u32]) -> Self {
        match p.as_u32() {
            2 => FpVector::_2(FqVectorP::from((Fp(P2), &slice))),
            3 => FpVector::_3(FqVectorP::from((Fp(P3), &slice))),
            5 => FpVector::_5(FqVectorP::from((Fp(P5), &slice))),
            7 => FpVector::_7(FqVectorP::from((Fp(P7), &slice))),
            _ => FpVector::Big(FqVectorP::from((Fp(p.to_dyn()), &slice))),
        }
    }

    pub(crate) fn num_limbs(p: ValidPrime, len: usize) -> usize {
        let entries_per_limb = Fp(p).entries_per_limb();
        (len + entries_per_limb - 1) / entries_per_limb
    }

    pub(crate) fn padded_len(p: ValidPrime, len: usize) -> usize {
        Self::num_limbs(p, len) * Fp(p).entries_per_limb()
    }

    pub fn update_from_bytes(&mut self, data: &mut impl Read) -> std::io::Result<()> {
        let limbs = self.limbs_mut();

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

    pub fn from_bytes(p: ValidPrime, len: usize, data: &mut impl Read) -> std::io::Result<Self> {
        let mut v = Self::new(p, len);
        v.update_from_bytes(data)?;
        Ok(v)
    }

    pub fn to_bytes(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        // self.limbs is allowed to have more limbs than necessary, but we only save the
        // necessary ones.
        let num_limbs = Self::num_limbs(self.prime(), self.len());

        if cfg!(target_endian = "little") {
            let num_bytes = num_limbs * size_of::<Limb>();
            unsafe {
                let buf: &[u8] =
                    std::slice::from_raw_parts_mut(self.limbs().as_ptr() as *mut u8, num_bytes);
                buffer.write_all(buf)?;
            }
        } else {
            for limb in &self.limbs()[0..num_limbs] {
                let bytes = limb.to_le_bytes();
                buffer.write_all(&bytes)?;
            }
        }
        Ok(())
    }
}

impl<'a> Slice<'a> {
    dispatch_vector! {
        pub fn prime(&self) -> ValidPrime;
        pub fn len(&self) -> usize;
        pub fn is_empty(&self) -> bool;
        pub fn entry(&self, index: usize) -> u32;
        pub fn iter(self) -> (dispatch FpVectorIterator 'a);
        pub fn iter_nonzero(self) -> (dispatch FpVectorNonZeroIterator 'a);
        pub fn is_zero(&self) -> bool;
        pub fn slice(self, start: usize, end: usize) -> (dispatch Slice 'a);
        pub fn to_owned(self) -> (dispatch FpVector);
    }
}

impl<'a> SliceMut<'a> {
    dispatch_vector! {
        pub fn prime(&self) -> ValidPrime;
        pub fn scale(&mut self, c: u32);
        pub fn set_to_zero(&mut self);
        pub fn add(&mut self, other: Slice, c: u32);
        pub fn assign(&mut self, other: Slice);
        pub fn set_entry(&mut self, index: usize, value: u32);
        pub fn as_slice(&self) -> (dispatch Slice);
        pub fn slice_mut(&mut self, start: usize, end: usize) -> (dispatch SliceMut);
        pub fn add_basis_element(&mut self, index: usize, value: u32);
        pub fn copy(&mut self) -> (dispatch SliceMut);
        pub fn add_masked(&mut self, other: Slice, c: u32, mask: &[usize]);
        pub fn add_unmasked(&mut self, other: Slice, c: u32, mask: &[usize]);
    }

    pub fn add_tensor(&mut self, offset: usize, coeff: u32, left: Slice, right: Slice) {
        match (self, left, right) {
            (SliceMut::_2(x), Slice::_2(y), Slice::_2(z)) => x.add_tensor(offset, coeff, y, z),
            (SliceMut::_3(x), Slice::_3(y), Slice::_3(z)) => x.add_tensor(offset, coeff, y, z),
            (SliceMut::_5(x), Slice::_5(y), Slice::_5(z)) => x.add_tensor(offset, coeff, y, z),
            (SliceMut::_7(x), Slice::_7(y), Slice::_7(z)) => x.add_tensor(offset, coeff, y, z),
            (SliceMut::Big(x), Slice::Big(y), Slice::Big(z)) => x.add_tensor(offset, coeff, y, z),
            _ => {
                panic!("Applying add_tensor to vectors over different primes");
            }
        }
    }
}

impl<'a> FpVectorIterator<'a> {
    dispatch_vector! {
        fn next(&mut self) -> (Option<u32>);
        pub fn skip_n(&mut self, n: usize);
    }
}

impl<'a> FpVectorNonZeroIterator<'a> {
    dispatch_vector! {
        fn next(&mut self) -> (Option<(usize, u32)>);
    }
}

impl std::fmt::Display for FpVector {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl<'a> std::fmt::Display for Slice<'a> {
    /// # Example
    /// ```
    /// # use fp::vector::FpVector;
    /// # use fp::prime::ValidPrime;
    /// let v = FpVector::from_slice(ValidPrime::new(2), &[0, 1, 0]);
    /// assert_eq!(&format!("{v}"), "[0, 1, 0]");
    /// assert_eq!(&format!("{v:#}"), "010");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            for v in self.iter() {
                // If self.p >= 11, this will look funky
                write!(f, "{v}")?;
            }
            Ok(())
        } else {
            write!(f, "[{}]", self.iter().format(", "))
        }
    }
}

impl From<&FpVector> for Vec<u32> {
    fn from(v: &FpVector) -> Vec<u32> {
        v.iter().collect()
    }
}

impl std::ops::AddAssign<&FpVector> for FpVector {
    fn add_assign(&mut self, other: &FpVector) {
        self.add(other, 1);
    }
}

impl<'a> Iterator for FpVectorIterator<'a> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        self.next()
    }
}

impl<'a> Iterator for FpVectorNonZeroIterator<'a> {
    type Item = (usize, u32);

    fn next(&mut self) -> Option<Self::Item> {
        self.next()
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
    ($var:tt, $p:ty) => {
        impl<'a> TryInto<&'a mut FqVectorP<Fp<$p>>> for &'a mut FpVector {
            type Error = ();

            fn try_into(self) -> Result<&'a mut FqVectorP<Fp<$p>>, ()> {
                match self {
                    FpVector::$var(x) => Ok(x),
                    _ => Err(()),
                }
            }
        }
    };
}

impl_try_into!(_2, P2);
impl_try_into!(_3, P3);
impl_try_into!(_5, P5);
impl_try_into!(_7, P7);
impl_try_into!(Big, ValidPrime);

impl Serialize for FpVector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Vec::<u32>::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FpVector {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        panic!("Deserializing FpVector not supported");
        // This is needed for ext-websocket/actions to be happy
    }
}

impl<'a, 'b> From<&'a mut SliceMut<'b>> for SliceMut<'a> {
    fn from(slice: &'a mut SliceMut<'b>) -> SliceMut<'a> {
        slice.copy()
    }
}

impl<'a, 'b> From<&'a Slice<'b>> for Slice<'a> {
    fn from(slice: &'a Slice<'b>) -> Slice<'a> {
        *slice
    }
}

impl<'a, 'b> From<&'a SliceMut<'b>> for Slice<'a> {
    fn from(slice: &'a SliceMut<'b>) -> Slice<'a> {
        slice.as_slice()
    }
}

impl<'a> From<&'a FpVector> for Slice<'a> {
    fn from(v: &'a FpVector) -> Slice<'a> {
        v.as_slice()
    }
}

impl<'a> From<&'a mut FpVector> for SliceMut<'a> {
    fn from(v: &'a mut FpVector) -> SliceMut<'a> {
        v.as_slice_mut()
    }
}
