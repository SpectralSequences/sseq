use std::io::{Read, Write};

use itertools::Itertools;

use super::{
    base_generic::{BaseVectorMutP, BaseVectorP},
    generic::{FpVectorIterator, FpVectorNonZeroIteratorP, FpVectorP, SliceMutP, SliceP},
};
use crate::{limb::Limb, prime::ValidPrime};

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum FpVector {
    _2(FpVectorP<2>),
    _3(FpVectorP<3>),
    _5(FpVectorP<5>),
    _7(FpVectorP<7>),
}

#[derive(Debug, Copy, Clone)]
pub enum Slice<'a> {
    _2(SliceP<'a, 2>),
    _3(SliceP<'a, 3>),
    _5(SliceP<'a, 5>),
    _7(SliceP<'a, 7>),
}

#[derive(Debug)]
pub enum SliceMut<'a> {
    _2(SliceMutP<'a, 2>),
    _3(SliceMutP<'a, 3>),
    _5(SliceMutP<'a, 5>),
    _7(SliceMutP<'a, 7>),
}

pub enum FpVectorNonZeroIterator<'a> {
    _2(FpVectorNonZeroIteratorP<'a, 2>),
    _3(FpVectorNonZeroIteratorP<'a, 3>),
    _5(FpVectorNonZeroIteratorP<'a, 5>),
    _7(FpVectorNonZeroIteratorP<'a, 7>),
}

macro_rules! dispatch_prime_generic_inner {
    (fn $method:ident(&mut self $(, $arg:ident: $ty:ty )*) $(-> $ret:ty)?) => {
        fn $method<'b, T: Into<Slice<'b>>>(&mut self, other: T $(,$arg: $ty)*) $(-> $ret)? {
            match (self, other.into()) {
                (Self::_2(ref mut x), Slice::_2(y)) => x.$method(y $(,$arg)*),
                (Self::_3(ref mut x), Slice::_3(y)) => x.$method(y $(,$arg)*),
                (Self::_5(ref mut x), Slice::_5(y)) => x.$method(y $(,$arg)*),
                (Self::_7(ref mut x), Slice::_7(y)) => x.$method(y $(,$arg)*),
                (l, r) => panic!(
                    "Applying add to vectors over different primes ({} and {})",
                    l.prime(),
                    r.prime()
                ),
            }
        }
    }
}

/// Macro to implement the generic addition methods.
macro_rules! dispatch_prime_generic {
    () => {};
    (fn $method:ident(&mut self $(, $arg:ident: $ty:ty )*) $(-> $ret:ty)?; $($tail:tt)*) => {
        dispatch_prime_generic_inner! {
            fn $method(&mut self $(, $arg: $ty )*) $(-> $ret)?
        }
        dispatch_prime_generic!{$($tail)*}
    }
}

macro_rules! dispatch_basevector {
    () => {
        dispatch_prime! {
            fn prime(&self) -> ValidPrime;
            fn len(&self) -> usize;
            fn is_empty(&self) -> bool;
            fn entry(&self, index: usize) -> u32;
            fn as_slice(&self) -> (dispatch Slice);
            fn is_zero(&self) -> bool;
            fn iter(&self) -> FpVectorIterator;
            fn iter_nonzero(&self) -> (dispatch FpVectorNonZeroIterator);
            fn first_nonzero(&self) -> (Option<(usize, u32)>);
            fn sign_rule(&self, other: &Self) -> bool;
            fn density(&self) -> f32;
        }

        fn slice<'b>(&self, start: usize, end: usize) -> Slice<'b>
        where
            Self: 'b,
        {
            match self {
                Self::_2(x) => Slice::_2(x.slice(start, end)),
                Self::_3(x) => Slice::_3(x.slice(start, end)),
                Self::_5(x) => Slice::_5(x.slice(start, end)),
                Self::_7(x) => Slice::_7(x.slice(start, end)),
            }
        }
    };
}

macro_rules! dispatch_basevectormut {
    () => {
        dispatch_prime! {
            fn scale(&mut self, c: u32);
            fn set_to_zero(&mut self);
            fn set_entry(&mut self, index: usize, value: u32);
            fn slice_mut(&mut self, start: usize, end: usize) -> (dispatch SliceMut);
            fn as_slice_mut(&mut self) -> (dispatch SliceMut);
            fn add_basis_element(&mut self, index: usize, value: u32);
            fn copy_from_slice(&mut self, slice: &[u32]);
        }

        dispatch_prime_generic! {
            fn assign(&mut self);
            fn add(&mut self, c: u32);
            fn add_offset(&mut self, c: u32, offset: usize);
            fn add_masked(&mut self, c: u32, mask: &[usize]);
            fn add_unmasked(&mut self, c: u32, mask: &[usize]);
            fn add_truncate(&mut self, c: u32) -> Option<()>;
        }
    };
}

/// Trait for common methods on vector-type structs.
pub trait BaseVector {
    fn prime(&self) -> ValidPrime;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn entry(&self, index: usize) -> u32;
    fn slice<'a>(&self, start: usize, end: usize) -> Slice<'a>
    where
        Self: 'a;
    fn as_slice(&self) -> Slice;
    fn into_owned(self) -> FpVector;
    fn is_zero(&self) -> bool;
    fn iter(&self) -> FpVectorIterator;
    fn iter_nonzero(&self) -> FpVectorNonZeroIterator;
    fn first_nonzero(&self) -> Option<(usize, u32)>;
    fn sign_rule(&self, other: &Self) -> bool;
    fn density(&self) -> f32;
}

/// Trait for common methods on mutable vector-type structs.
pub trait BaseVectorMut: BaseVector {
    fn scale(&mut self, c: u32);
    fn set_to_zero(&mut self);
    fn set_entry(&mut self, index: usize, value: u32);
    fn assign<'a, T: Into<Slice<'a>>>(&mut self, other: T);
    fn add<'a, T: Into<Slice<'a>>>(&mut self, other: T, c: u32);
    fn add_offset<'a, T: Into<Slice<'a>>>(&mut self, other: T, c: u32, offset: usize);
    fn slice_mut(&mut self, start: usize, end: usize) -> SliceMut;
    fn as_slice_mut(&mut self) -> SliceMut;
    fn add_basis_element(&mut self, index: usize, value: u32);
    fn copy_from_slice(&mut self, slice: &[u32]);
    fn add_masked<'a, T: Into<Slice<'a>>>(&mut self, other: T, c: u32, mask: &[usize]);
    fn add_unmasked<'a, T: Into<Slice<'a>>>(&mut self, other: T, c: u32, mask: &[usize]);
    fn add_truncate<'a, T: Into<Slice<'a>>>(&mut self, other: T, c: u32) -> Option<()>;
}

// impls for `FpVector`

impl BaseVector for FpVector {
    dispatch_basevector!();

    fn into_owned(self) -> FpVector {
        self
    }
}

impl BaseVectorMut for FpVector {
    dispatch_basevectormut!();
}

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
                match self {
                    FpVector::$var(ref mut x) => Ok(x),
                    _ => Err(()),
                }
            }
        }
    };
}

impl_try_into!(_2, 2);
impl_try_into!(_3, 3);
impl_try_into!(_5, 5);
impl_try_into!(_7, 7);

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
    dispatch_prime! {
        pub fn assign_partial(&mut self, other: &Self);
        pub fn extend_len(&mut self, len: usize);
        pub fn set_scratch_vector_size(&mut self, len: usize);
        pub(crate) fn trim_start(&mut self, n: usize);
        pub fn add_carry(&mut self, other: &Self, c: u32, rest: &mut [FpVector]) -> bool;
        pub fn update_from_bytes(&mut self, data: &mut impl Read) -> (std::io::Result<()>);
        pub fn to_bytes(&self, buffer: &mut impl Write) -> (std::io::Result<()>);
        pub(crate) fn limbs(&self) -> (&[Limb]);
        pub(crate) fn limbs_mut(&mut self) -> (&mut [Limb]);
    }

    pub fn new(p: ValidPrime, len: usize) -> FpVector {
        match_p!(p, FpVectorP::new_(len))
    }

    pub fn new_with_capacity(p: ValidPrime, len: usize, capacity: usize) -> FpVector {
        match_p!(p, FpVectorP::new_with_capacity_(len, capacity))
    }

    pub fn from_slice(p: ValidPrime, slice: &[u32]) -> Self {
        match_p!(p, FpVectorP::from(&slice))
    }

    pub fn from_bytes(p: ValidPrime, len: usize, data: &mut impl Read) -> std::io::Result<Self> {
        Ok(match_p!(p, FpVectorP::from_bytes(p, len, data)?))
    }
}

// impls for `SliceMut`

impl<'a> BaseVector for SliceMut<'a> {
    dispatch_basevector!();

    dispatch_prime! {
        fn into_owned(self) -> (dispatch FpVector);
    }
}

impl<'a> BaseVectorMut for SliceMut<'a> {
    dispatch_basevectormut!();
}

impl<'a> From<&'a mut FpVector> for SliceMut<'a> {
    fn from(vec: &'a mut FpVector) -> Self {
        vec.as_slice_mut()
    }
}

impl<'a> SliceMut<'a> {
    dispatch_prime! {
        pub fn copy(&mut self) -> (dispatch SliceMut);
    }

    pub fn add_tensor(&mut self, offset: usize, coeff: u32, left: Slice, right: Slice) {
        match (self, left, right) {
            (SliceMut::_2(ref mut x), Slice::_2(y), Slice::_2(z)) => {
                x.add_tensor(offset, coeff, y, z)
            }
            (SliceMut::_3(ref mut x), Slice::_3(y), Slice::_3(z)) => {
                x.add_tensor(offset, coeff, y, z)
            }
            (SliceMut::_5(ref mut x), Slice::_5(y), Slice::_5(z)) => {
                x.add_tensor(offset, coeff, y, z)
            }
            (SliceMut::_7(ref mut x), Slice::_7(y), Slice::_7(z)) => {
                x.add_tensor(offset, coeff, y, z)
            }
            _ => {
                panic!("Applying add_tensor to vectors over different primes");
            }
        }
    }
}

// impls for `Slice`

impl<'a> BaseVector for Slice<'a> {
    dispatch_basevector!();

    dispatch_prime! {
        fn into_owned(self) -> (dispatch FpVector);
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

impl<'a> From<&'a FpVector> for Slice<'a> {
    fn from(vec: &'a FpVector) -> Self {
        vec.as_slice()
    }
}

impl<'a> From<&'a mut FpVector> for Slice<'a> {
    fn from(vec: &'a mut FpVector) -> Self {
        (vec as &'a FpVector).as_slice()
    }
}

// impls for `FpVectorNonZeroIterator`

impl<'a> Iterator for FpVectorNonZeroIterator<'a> {
    type Item = (usize, u32);

    fn next(&mut self) -> Option<Self::Item> {
        self.next()
    }
}

impl<'a> FpVectorNonZeroIterator<'a> {
    dispatch_prime! {
        fn next(&mut self) -> (Option<(usize, u32)>);
    }
}

// other trait impls

macro_rules! impl_from_ref {
    ($t1:tt, $t2:tt, $t2p:tt $(, $m:tt)?) => {
        impl<'a, 'b> From<&'a $($m)* $t1<'b>> for $t2<'a> {
            fn from(slice: &'a $($m)* $t1<'b>) -> $t2<'a> {
                match slice {
                    $t1::_2(x) => $t2::_2($t2p::<'a, 2>::from(x)),
                    $t1::_3(x) => $t2::_3($t2p::<'a, 3>::from(x)),
                    $t1::_5(x) => $t2::_5($t2p::<'a, 5>::from(x)),
                    $t1::_7(x) => $t2::_7($t2p::<'a, 7>::from(x)),
                }
            }
        }
    };
}

impl_from_ref!(SliceMut, SliceMut, SliceMutP, mut);
impl_from_ref!(Slice, Slice, SliceP);
impl_from_ref!(SliceMut, Slice, SliceP);

impl From<&FpVector> for Vec<u32> {
    fn from(v: &FpVector) -> Vec<u32> {
        v.iter().collect()
    }
}

// Tautological impls

macro_rules! dispatch_prime_tauto_inner {
    (fn $method:ident(&self $(, $arg:ident: $ty:ty )*) $(-> $ret:ty)?) => {
        fn $method(&self $(,$arg: $ty)*) $(-> $ret)? {
            T::$method(self $(,$arg)*)
        }
    };
    (fn $method:ident (&mut self $(, $arg:ident: $ty:ty )*) $(-> $ret:ty)?) => {
        fn $method (&mut self $(,$arg: $ty)*) $(-> $ret)? {
            T::$method(self $(,$arg)*)
        }
    };
    (fn $method:ident <'a, S: Into<Slice<'a>>> (&mut self $(, $arg:ident: $ty:ty )*) $(-> $ret:ty)?) => {
        fn $method <'a, S: Into<Slice<'a>>> (&mut self $(,$arg: $ty)*) $(-> $ret)? {
            T::$method(self $(,$arg)*)
        }
    };
}

macro_rules! dispatch_prime_tauto {
    () => {};
    (fn $method:ident $tt:tt $(-> $ret:ty)?; $($tail:tt)*) => {
        dispatch_prime_tauto_inner! {
            fn $method $tt $(-> $ret)?
        }
        dispatch_prime_tauto!{$($tail)*}
    };
    (fn $method:ident <'a, S: Into<Slice<'a>>> $tt:tt $(-> $ret:ty)?; $($tail:tt)*) => {
        dispatch_prime_tauto_inner! {
            fn $method <'a, S: Into<Slice<'a>>> $tt $(-> $ret)?
        }
        dispatch_prime_tauto!{$($tail)*}
    }
}

impl<T: BaseVector> BaseVector for &T {
    dispatch_prime_tauto! {
        fn prime(&self) -> ValidPrime;
        fn len(&self) -> usize;
        fn is_empty(&self) -> bool;
        fn entry(&self, index: usize) -> u32;
        fn as_slice(&self) -> Slice;
        fn is_zero(&self) -> bool;
        fn iter(&self) -> FpVectorIterator;
        fn iter_nonzero(&self) -> FpVectorNonZeroIterator;
        fn first_nonzero(&self) -> Option<(usize, u32)>;
        fn sign_rule(&self, other: &Self) -> bool;
        fn density(&self) -> f32;
    }

    fn slice<'b>(&self, start: usize, end: usize) -> Slice<'b>
    where
        Self: 'b,
    {
        T::slice(self, start, end)
    }

    fn into_owned(self) -> FpVector {
        T::as_slice(self).into_owned()
    }
}

impl<T: BaseVector> BaseVector for &mut T {
    dispatch_prime_tauto! {
        fn prime(&self) -> ValidPrime;
        fn len(&self) -> usize;
        fn is_empty(&self) -> bool;
        fn entry(&self, index: usize) -> u32;
        fn as_slice(&self) -> Slice;
        fn is_zero(&self) -> bool;
        fn iter(&self) -> FpVectorIterator;
        fn iter_nonzero(&self) -> FpVectorNonZeroIterator;
        fn first_nonzero(&self) -> Option<(usize, u32)>;
        fn sign_rule(&self, other: &Self) -> bool;
        fn density(&self) -> f32;
    }

    fn slice<'b>(&self, start: usize, end: usize) -> Slice<'b>
    where
        Self: 'b,
    {
        T::slice(self, start, end)
    }

    fn into_owned(self) -> FpVector {
        T::as_slice(self).into_owned()
    }
}

impl<T: BaseVectorMut> BaseVectorMut for &mut T {
    dispatch_prime_tauto! {
        fn scale(&mut self, c: u32);
        fn set_to_zero(&mut self);
        fn set_entry(&mut self, index: usize, value: u32);
        fn slice_mut(&mut self, start: usize, end: usize) -> SliceMut;
        fn as_slice_mut(&mut self) -> SliceMut;
        fn add_basis_element(&mut self, index: usize, value: u32);
        fn copy_from_slice(&mut self, slice: &[u32]);
        fn assign<'a, S: Into<Slice<'a>>>(&mut self, other: S);
        fn add<'a, S: Into<Slice<'a>>>(&mut self, other: S, c: u32);
        fn add_offset<'a, S: Into<Slice<'a>>>(&mut self, other: S, c: u32, offset: usize);
        fn add_masked<'a, S: Into<Slice<'a>>>(&mut self, other: S, c: u32, mask: &[usize]);
        fn add_unmasked<'a, S: Into<Slice<'a>>>(&mut self, other: S, c: u32, mask: &[usize]);
        fn add_truncate<'a, S: Into<Slice<'a>>>(&mut self, other: S, c: u32) -> Option<()>;
    }
}
