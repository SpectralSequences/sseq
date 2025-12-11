macro_rules! use_primes {
    () => {
        use crate::{
            field::fp::F2,
            prime::{P2, ValidPrime},
        };
    };
}

macro_rules! dispatch_struct {
    ($(#[derive $derives:tt])? $vis:vis $name:ident $(<$life:lifetime>)? from $fq_name:ident) => {
        $(#[derive $derives])*
        $vis enum $name $(<$life>)? {
            _2($fq_name<$($life,)? Fp<P2>>),
        }
    };
}

macro_rules! dispatch_vector_inner {
    // other is a type, but marking it as a :ty instead of :tt means we cannot use it to access its
    // enum variants.
    ($vis:vis fn $method:ident $helper_method:ident(&self, other: &$other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        $vis fn $method(&self, other: &$other, $($arg: $ty),* ) $(-> $ret)* {
            let Self::_2(self_0) = self;
            let $other::_2(other_0) = other;
            self_0.$helper_method(&other_0, $($arg),*)
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&mut self, other: &$other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&mut self, other: &$other, $($arg: $ty),* ) $(-> $ret)* {
            let Self::_2(self_0) = self;
            let $other::_2(other_0) = other;
            self_0.$helper_method(&other_0, $($arg),*)
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&mut self, other: $other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        $vis fn $method(&mut self, other: $other, $($arg: $ty),* ) $(-> $ret)* {
            let Self::_2(self_0) = self;
            let $other::_2(other_0) = other;
            self_0.$helper_method(other_0, $($arg),*)
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&mut self $(, $arg:ident: $ty:ty )*, @left: $other1:tt, right: $other2:tt ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&mut self, $($arg: $ty),* , left: $other1, right: $other2 ) $(-> $ret)* {
            let Self::_2(self_0) = self;
            let $other1::_2(left_0) = left;
            let $other2::_2(right_0) = right;
            self_0.$helper_method($($arg),*, left_0, right_0)
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&mut self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:path)) => {
        #[must_use]
        $vis fn $method(&mut self, $($arg: $ty),* ) -> $ret {
            let Self::_2(self_0) = self;
            <$ret>::_2(self_0.$helper_method($($arg),*))
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:path)) => {
        #[must_use]
        $vis fn $method(&self, $($arg: $ty),* ) -> $ret {
            let Self::_2(self_0) = self;
            <$ret>::_2(self_0.$helper_method($($arg),*))
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:path)) => {
        #[must_use]
        $vis fn $method(self, $($arg: $ty),* ) -> $ret {
            let Self::_2(self_0) = self;
            <$ret>::_2(self_0.$helper_method($($arg),*))
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&mut self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&mut self, $($arg: $ty),* ) $(-> $ret)* {
            let Self::_2(self_0) = self;
            self_0.$helper_method($($arg),*)
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&self, $($arg: $ty),* ) $(-> $ret)* {
            let Self::_2(self_0) = self;
            self_0.$helper_method($($arg),*)
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(self, $($arg: $ty),* ) $(-> $ret)* {
            let Self::_2(self_0) = self;
            self_0.$helper_method($($arg),*)
        }
    }
}

macro_rules! dispatch_vector {
    () => {};
    ($vis:vis fn $method:ident $tt:tt $(-> $ret:tt)?; $($tail:tt)*) => {
        dispatch_vector_inner! {
            $vis fn $method $method $tt $(-> $ret)*
        }
        dispatch_vector!{$($tail)*}
    };
    ($vis:vis fn @$method:ident $tt:tt $(-> $ret:tt)?; $($tail:tt)*) => {
        paste::paste! {
            dispatch_vector_inner! {
                $vis fn $method [<$method _helper>] $tt $(-> $ret)*
            }
        }
        dispatch_vector!{$($tail)*}
    };
    // Special-case the constructors
    ($vis:vis fn $method:ident <P: Prime> (p: P $(, $arg:ident: $ty:ty )*) -> (from $fq_name:tt); $($tail:tt)*) => {
        $vis fn $method<P: Prime>(_p: P, $($arg: $ty),*) -> Self {
            Self::_2($fq_name::$method(F2, $($arg),*))
        }
        dispatch_vector!{$($tail)*}
    };
    // Special-case update_from_bytes
    ($vis:vis fn $method:ident <P: Prime> (p: P $(, $arg:ident: $ty:ty )*) -> (from io $fq_name:tt); $($tail:tt)*) => {
        $vis fn $method<P: Prime>(_p: P, $($arg: $ty),*) -> std::io::Result<Self> {
            Ok(Self::_2($fq_name::$method(F2, $($arg),*)?))
        }
        dispatch_vector!{$($tail)*}
    }
}

macro_rules! impl_from {
    () => {
        impl From<FqVector<Fp<P2>>> for FpVector {
            fn from(other: FqVector<Fp<P2>>) -> Self {
                Self::_2(other)
            }
        }
    };
}

macro_rules! impl_try_into {
    () => {
        impl<'a> TryInto<&'a mut FqVector<Fp<P2>>> for &'a mut FpVector {
            type Error = std::convert::Infallible;

            fn try_into(self) -> Result<&'a mut FqVector<Fp<P2>>, Self::Error> {
                let FpVector::_2(self_0) = self;
                Ok(self_0)
            }
        }
    };
}

pub(super) use dispatch_struct;
pub(super) use dispatch_vector;
pub(super) use impl_try_into;
pub(super) use use_primes;
