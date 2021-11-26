macro_rules! dispatch_prime_inner {
    // other is a type, but marking it as a :ty instead of :tt means we cannot use it to access its
    // enum variants.
    ($vis:vis fn $method:ident(&self, other: &$other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        $vis fn $method(&self, other: &$other, $($arg: $ty),* ) $(-> $ret)* {
            match (self, other) {
                (Self::_2(ref x), $other::_2(ref y)) => x.$method(y, $($arg),*),
                (Self::_3(ref x), $other::_3(ref y)) => x.$method(y, $($arg),*),
                (Self::_5(ref x), $other::_5(ref y)) => x.$method(y, $($arg),*),
                (Self::_7(ref x), $other::_7(ref y)) => x.$method(y, $($arg),*),
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
                (Self::_2(ref mut x), $other::_2(ref y)) => x.$method(y, $($arg),*),
                (Self::_3(ref mut x), $other::_3(ref y)) => x.$method(y, $($arg),*),
                (Self::_5(ref mut x), $other::_5(ref y)) => x.$method(y, $($arg),*),
                (Self::_7(ref mut x), $other::_7(ref y)) => x.$method(y, $($arg),*),
                (l, r) => {
                    panic!("Applying {} to vectors over different primes ({} and {})", stringify!($method), l.prime(), r.prime());
                }
            }
        }
    };
    ($vis:vis fn $method:ident(&mut self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {
        $vis fn $method(&mut self, $($arg: $ty),* ) -> $ret {
            match self {
                Self::_2(ref mut x) => $ret::_2(x.$method($($arg),*)),
                Self::_3(ref mut x) => $ret::_3(x.$method($($arg),*)),
                Self::_5(ref mut x) => $ret::_5(x.$method($($arg),*)),
                Self::_7(ref mut x) => $ret::_7(x.$method($($arg),*)),
            }
        }
    };
    ($vis:vis fn $method:ident(&self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {
        $vis fn $method(&self, $($arg: $ty),* ) -> $ret {
            match self {
                Self::_2(ref x) => $ret::_2(x.$method($($arg),*)),
                Self::_3(ref x) => $ret::_3(x.$method($($arg),*)),
                Self::_5(ref x) => $ret::_5(x.$method($($arg),*)),
                Self::_7(ref x) => $ret::_7(x.$method($($arg),*)),
            }
        }
    };
    ($vis:vis fn $method:ident(self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {
        $vis fn $method(self, $($arg: $ty),* ) -> $ret {
            match self {
                Self::_2(x) => $ret::_2(x.$method($($arg),*)),
                Self::_3(x) => $ret::_3(x.$method($($arg),*)),
                Self::_5(x) => $ret::_5(x.$method($($arg),*)),
                Self::_7(x) => $ret::_7(x.$method($($arg),*)),
            }
        }
    };
    ($vis:vis fn $method:ident(&mut self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&mut self, $($arg: $ty),* ) $(-> $ret)* {
            match self {
                Self::_2(ref mut x) => x.$method($($arg),*),
                Self::_3(ref mut x) => x.$method($($arg),*),
                Self::_5(ref mut x) => x.$method($($arg),*),
                Self::_7(ref mut x) => x.$method($($arg),*),
            }
        }
    };
    ($vis:vis fn $method:ident(&self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&self, $($arg: $ty),* ) $(-> $ret)* {
            match self {
                Self::_2(ref x) => x.$method($($arg),*),
                Self::_3(ref x) => x.$method($($arg),*),
                Self::_5(ref x) => x.$method($($arg),*),
                Self::_7(ref x) => x.$method($($arg),*),
            }
        }
    };
}

macro_rules! dispatch_prime {
    () => {};
    ($vis:vis fn $method:ident $tt:tt $(-> $ret:tt)?; $($tail:tt)*) => {
        dispatch_prime_inner! {
            $vis fn $method $tt $(-> $ret)*
        }
        dispatch_prime!{$($tail)*}
    }
}

macro_rules! match_p {
    ($p:ident, $($val:tt)*) => {
        match *$p {
            2 => Self::_2($($val)*),
            3 => Self::_3($($val)*),
            5 => Self::_5($($val)*),
            7 => Self::_7($($val)*),
            _ => panic!("Prime not supported: {}", *$p)
        }
    }
}
