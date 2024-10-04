macro_rules! use_primes {
    () => {
        use crate::{
            field::fp::F2,
            prime::{ValidPrime, P2},
        };
    };
}

macro_rules! dispatch_struct {
    ($(#[derive $derives:tt])? $vis:vis $name:ident $(<$life:lifetime>)? from $fq_name:ident) => {
        $(#[derive $derives])?
        $vis struct $name $(<$life>)? ($fq_name<$($life,)? Fp<P2>>);
    };
}

macro_rules! dispatch_vector_inner {
    // other is a type, but marking it as a :ty instead of :tt means we cannot use it to access its
    // enum variants.
    ($vis:vis fn $method:ident $helper_method:ident(&self, other: &$other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        $vis fn $method(&self, other: &$other, $($arg: $ty),* ) $(-> $ret)* {
            self.0.$helper_method(&other.0, $($arg),*)
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&mut self, other: &$other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&mut self, other: &$other, $($arg: $ty),* ) $(-> $ret)* {
            self.0.$helper_method(&other.0, $($arg),*)
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&mut self, other: $other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        $vis fn $method(&mut self, other: $other, $($arg: $ty),* ) $(-> $ret)* {
            self.0.$helper_method(other.0, $($arg),*)
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&mut self $(, $arg:ident: $ty:ty )*, @left: $other1:tt, right: $other2:tt ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&mut self, $($arg: $ty),* , left: $other1, right: $other2 ) $(-> $ret)* {
            self.0.$helper_method($($arg),*, left.0, right.0)
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&mut self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {
        #[must_use]
        $vis fn $method(&mut self, $($arg: $ty),* ) -> $ret {
            $ret(self.0.$helper_method($($arg),*))
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {
        #[must_use]
        $vis fn $method(&self, $($arg: $ty),* ) -> $ret {
            $ret(self.0.$helper_method($($arg),*))
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {
        #[must_use]
        $vis fn $method(self, $($arg: $ty),* ) -> $ret {
            $ret(self.0.$helper_method($($arg),*))
        }
    };

    ($vis:vis fn $method:ident $helper_method:ident(self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt $lifetime:tt)) => {
        #[must_use]
        $vis fn $method(self, $($arg: $ty),* ) -> $ret<$lifetime> {
            $ret(self.0.$helper_method($($arg),*))
        }
    };

    ($vis:vis fn $method:ident $helper_method:ident(&mut self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&mut self, $($arg: $ty),* ) $(-> $ret)* {
            self.0.$helper_method($($arg),*)
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&self, $($arg: $ty),* ) $(-> $ret)* {
            self.0.$helper_method($($arg),*)
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(self, $($arg: $ty),* ) $(-> $ret)* {
            self.0.$helper_method($($arg),*)
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
            Self($fq_name::$method(F2, $($arg),*))
        }
        dispatch_vector!{$($tail)*}
    };
    // Special-case update_from_bytes
    ($vis:vis fn $method:ident <P: Prime> (p: P $(, $arg:ident: $ty:ty )*) -> (from io $fq_name:tt); $($tail:tt)*) => {
        $vis fn $method<P: Prime>(_p: P, $($arg: $ty),*) -> std::io::Result<Self> {
            Ok(Self($fq_name::$method(F2, $($arg),*)?))
        }
        dispatch_vector!{$($tail)*}
    }
}

macro_rules! impl_from {
    () => {
        impl From<FqVector<Fp<P2>>> for FpVector {
            fn from(other: FqVector<Fp<P2>>) -> Self {
                Self(other)
            }
        }
    };
}

macro_rules! impl_try_into {
    () => {
        impl<'a> TryInto<&'a mut FqVector<Fp<P2>>> for &'a mut FpVector {
            type Error = std::convert::Infallible;

            fn try_into(self) -> Result<&'a mut FqVector<Fp<P2>>, Self::Error> {
                Ok(&mut self.0)
            }
        }
    };
}

pub(super) use dispatch_struct;
pub(super) use dispatch_vector;
pub(super) use impl_try_into;
pub(super) use use_primes;
