/// Use all primes. It isn't possible to have this import when `odd-primes` is disabled because
/// primes other than P2 and ValidPrime (which happen to be the same type) do not exist.
macro_rules! use_primes {
    () => {
        use crate::{
            field::fp::{F2, F3, F5, F7},
            prime::{ValidPrime, P2, P3, P5, P7},
        };
    };
}

/// Define a struct that wraps some field-dependent type in an enum. Since `Fp<P2>` through `Fp<P7>`
/// are all ZSTs, working with them under the hood is in theory much faster than working with an
/// arbitrary prime field, let alone finite field.
macro_rules! dispatch_struct {
    ($(#[derive $derives:tt])? $vis:vis $name:ident $(<$life:lifetime>)? from $fq_name:ident) => {
        $(#[derive $derives])*
        $vis enum $name $(<$life>)? {
            _2($fq_name<$($life,)? Fp<P2>>),
            _3($fq_name<$($life,)? Fp<P3>>),
            _5($fq_name<$($life,)? Fp<P5>>),
            _7($fq_name<$($life,)? Fp<P7>>),
            Big($fq_name<$($life,)? Fp<ValidPrime>>),
        }
    };
}

macro_rules! dispatch_vector_inner {
    // other is a type, but marking it as a :ty instead of :tt means we cannot use it to access its
    // enum variants.
    ($vis:vis fn $method:ident $helper_method:ident(&self, other: &$other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        $vis fn $method(&self, other: &$other, $($arg: $ty),* ) $(-> $ret)* {
            match (self, other) {
                (Self::_2(x), $other::_2(y)) => x.$helper_method(y, $($arg),*),
                (Self::_3(x), $other::_3(y)) => x.$helper_method(y, $($arg),*),
                (Self::_5(x), $other::_5(y)) => x.$helper_method(y, $($arg),*),
                (Self::_7(x), $other::_7(y)) => x.$helper_method(y, $($arg),*),
                (Self::Big(x), $other::Big(y)) if x.prime() == y.prime() => x.$helper_method(y, $($arg),*),
                (l, r) => {
                    panic!("Applying {} to vectors over different primes ({} and {})", stringify!($method), l.prime(), r.prime());
                }
            }
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&mut self, other: &$other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&mut self, other: &$other, $($arg: $ty),* ) $(-> $ret)* {
            match (self, other) {
                (Self::_2(x), $other::_2(y)) => x.$helper_method(y, $($arg),*),
                (Self::_3(x), $other::_3(y)) => x.$helper_method(y, $($arg),*),
                (Self::_5(x), $other::_5(y)) => x.$helper_method(y, $($arg),*),
                (Self::_7(x), $other::_7(y)) => x.$helper_method(y, $($arg),*),
                (Self::Big(x), $other::Big(y)) if x.prime() == y.prime() => x.$helper_method(y, $($arg),*),
                (l, r) => {
                    panic!("Applying {} to vectors over different primes ({} and {})", stringify!($method), l.prime(), r.prime());
                }
            }
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&mut self, other: $other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        $vis fn $method(&mut self, other: $other, $($arg: $ty),* ) $(-> $ret)* {
            match (self, other) {
                (Self::_2(x), $other::_2(y)) => x.$helper_method(y, $($arg),*),
                (Self::_3(x), $other::_3(y)) => x.$helper_method(y, $($arg),*),
                (Self::_5(x), $other::_5(y)) => x.$helper_method(y, $($arg),*),
                (Self::_7(x), $other::_7(y)) => x.$helper_method(y, $($arg),*),
                (Self::Big(x), $other::Big(y)) if x.prime() == y.prime() => x.$helper_method(y, $($arg),*),
                (l, r) => {
                    panic!("Applying {} to vectors over different primes ({} and {})", stringify!($method), l.prime(), r.prime());
                }
            }
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&mut self $(, $arg:ident: $ty:ty )*, @left: $other1:tt, right: $other2:tt ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&mut self, $($arg: $ty),* , left: $other1, right: $other2 ) $(-> $ret)* {
            match (self, left, right) {
                (Self::_2(x), $other1::_2(y), $other2::_2(z)) => x.$helper_method($($arg),*, y, z),
                (Self::_3(x), $other1::_3(y), $other2::_3(z)) => x.$helper_method($($arg),*, y, z),
                (Self::_5(x), $other1::_5(y), $other2::_5(z)) => x.$helper_method($($arg),*, y, z),
                (Self::_7(x), $other1::_7(y), $other2::_7(z)) => x.$helper_method($($arg),*, y, z),
                (Self::Big(x), $other1::Big(y), $other2::Big(z)) if x.prime() == y.prime() && y.prime() == z.prime() => x.$helper_method($($arg),*, y, z),
                _ => {
                    panic!("Applying {} to vectors over different primes", stringify!($method));
                }
            }
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&mut self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {
        #[must_use]
        $vis fn $method(&mut self, $($arg: $ty),* ) -> $ret {
            match self {
                Self::_2(x) => $ret::_2(x.$helper_method($($arg),*)),
                Self::_3(x) => $ret::_3(x.$helper_method($($arg),*)),
                Self::_5(x) => $ret::_5(x.$helper_method($($arg),*)),
                Self::_7(x) => $ret::_7(x.$helper_method($($arg),*)),
                Self::Big(x) => $ret::Big(x.$helper_method($($arg),*)),
            }
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {
        #[must_use]
        $vis fn $method(&self, $($arg: $ty),* ) -> $ret {
            match self {
                Self::_2(x) => $ret::_2(x.$helper_method($($arg),*)),
                Self::_3(x) => $ret::_3(x.$helper_method($($arg),*)),
                Self::_5(x) => $ret::_5(x.$helper_method($($arg),*)),
                Self::_7(x) => $ret::_7(x.$helper_method($($arg),*)),
                Self::Big(x) => $ret::Big(x.$helper_method($($arg),*)),
            }
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {
        #[must_use]
        $vis fn $method(self, $($arg: $ty),* ) -> $ret {
            match self {
                Self::_2(x) => $ret::_2(x.$helper_method($($arg),*)),
                Self::_3(x) => $ret::_3(x.$helper_method($($arg),*)),
                Self::_5(x) => $ret::_5(x.$helper_method($($arg),*)),
                Self::_7(x) => $ret::_7(x.$helper_method($($arg),*)),
                Self::Big(x) => $ret::Big(x.$helper_method($($arg),*)),
            }
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt $lifetime:tt)) => {
        #[must_use]
        $vis fn $method(self, $($arg: $ty),* ) -> $ret<$lifetime> {
            match self {
                Self::_2(x) => $ret::_2(x.$helper_method($($arg),*)),
                Self::_3(x) => $ret::_3(x.$helper_method($($arg),*)),
                Self::_5(x) => $ret::_5(x.$helper_method($($arg),*)),
                Self::_7(x) => $ret::_7(x.$helper_method($($arg),*)),
                Self::Big(x) => $ret::Big(x.$helper_method($($arg),*)),
            }
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&mut self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&mut self, $($arg: $ty),* ) $(-> $ret)* {
            match self {
                Self::_2(x) => x.$helper_method($($arg),*),
                Self::_3(x) => x.$helper_method($($arg),*),
                Self::_5(x) => x.$helper_method($($arg),*),
                Self::_7(x) => x.$helper_method($($arg),*),
                Self::Big(x) => x.$helper_method($($arg),*),
            }
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(&self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&self, $($arg: $ty),* ) $(-> $ret)* {
            match self {
                Self::_2(x) => x.$helper_method($($arg),*),
                Self::_3(x) => x.$helper_method($($arg),*),
                Self::_5(x) => x.$helper_method($($arg),*),
                Self::_7(x) => x.$helper_method($($arg),*),
                Self::Big(x) => x.$helper_method($($arg),*),
            }
        }
    };
    ($vis:vis fn $method:ident $helper_method:ident(self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(self, $($arg: $ty),* ) $(-> $ret)* {
            match self {
                Self::_2(x) => x.$helper_method($($arg),*),
                Self::_3(x) => x.$helper_method($($arg),*),
                Self::_5(x) => x.$helper_method($($arg),*),
                Self::_7(x) => x.$helper_method($($arg),*),
                Self::Big(x) => x.$helper_method($($arg),*),
            }
        }
    };
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
        $vis fn $method<P: Prime>(p: P, $($arg: $ty),*) -> Self {
            match p.as_u32() {
                2 => Self::_2($fq_name::$method(F2, $($arg),*)),
                3 => Self::_3($fq_name::$method(F3, $($arg),*)),
                5 => Self::_5($fq_name::$method(F5, $($arg),*)),
                7 => Self::_7($fq_name::$method(F7, $($arg),*)),
                _ => Self::Big($fq_name::$method(Fp::new(p.to_dyn()), $($arg),*)),
            }
        }
        dispatch_vector!{$($tail)*}
    };
    // Special-case update_from_bytes
    ($vis:vis fn $method:ident <P: Prime> (p: P $(, $arg:ident: $ty:ty )*) -> (from io $fq_name:tt); $($tail:tt)*) => {
        $vis fn $method<P: Prime>(p: P, $($arg: $ty),*) -> std::io::Result<Self> {
            Ok(match p.as_u32() {
                2 => Self::_2($fq_name::$method(F2, $($arg),*)?),
                3 => Self::_3($fq_name::$method(F3, $($arg),*)?),
                5 => Self::_5($fq_name::$method(F5, $($arg),*)?),
                7 => Self::_7($fq_name::$method(F7, $($arg),*)?),
                _ => Self::Big($fq_name::$method(Fp::new(p.to_dyn()), $($arg),*)?),
            })
        }
        dispatch_vector!{$($tail)*}
    }
}

macro_rules! impl_from_inner {
    ($var:tt, $p:ty) => {
        impl<'a> From<FqVector<Fp<$p>>> for FpVector {
            fn from(x: FqVector<Fp<$p>>) -> Self {
                FpVector::$var(x)
            }
        }
    };
}

macro_rules! impl_from {
    () => {
        impl_from_inner!(_2, P2);
        impl_from_inner!(_3, P3);
        impl_from_inner!(_5, P5);
        impl_from_inner!(_7, P7);
        impl_from_inner!(Big, ValidPrime);
    };
}

macro_rules! impl_try_into_inner {
    ($var:tt, $p:ty) => {
        impl<'a> TryInto<&'a mut FqVector<Fp<$p>>> for &'a mut FpVector {
            type Error = ();

            fn try_into(self) -> Result<&'a mut FqVector<Fp<$p>>, Self::Error> {
                match self {
                    FpVector::$var(x) => Ok(x),
                    _ => Err(()),
                }
            }
        }
    };
}

macro_rules! impl_try_into {
    () => {
        impl_try_into_inner!(_2, P2);
        impl_try_into_inner!(_3, P3);
        impl_try_into_inner!(_5, P5);
        impl_try_into_inner!(_7, P7);
        impl_try_into_inner!(Big, ValidPrime);
    };
}

pub(super) use dispatch_struct;
pub(super) use dispatch_vector;
pub(super) use impl_try_into;
pub(super) use use_primes;
