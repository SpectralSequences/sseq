use std::{
    fmt::Debug,
    ops::{
        Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr,
        ShrAssign, Sub, SubAssign,
    },
};

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

pub mod binomial;
pub mod iter;

pub use binomial::Binomial;

pub mod primes {
    pub use super::{ValidPrime, P2, P3, P5, P7};
}

pub const TWO: ValidPrime = ValidPrime::new(2);

/// A trait that represents a prime number. There are currently two kinds of structs that implement
/// this trait: static primes and `ValidPrime`, the dynamic prime.
///
/// The methods in this trait take a `self` receiver so that the dynamic prime `ValidPrime` can
/// implement it. We could also have a `&self` receiver, but since `Prime` is a supertrait of
/// `Copy`, the two are equivalent. Using `self` might also be useful in the future if we ever want
/// to play with autoref specialization.
///
/// The fact that e.g. `P2::to_u32` is hardcoded to return 2 means that a condition like `p.to_u32()
/// == 2` (or even better, just `p == 2`) will reduce to `true` at compile time, and allow the
/// compiler to eliminate an entire branch, while also leaving that check in for when the prime is
/// chosen at runtime.
pub trait Prime:
    Debug
    + Copy
    + PartialEq
    + Eq
    + PartialEq<u32>
    + PartialOrd<u32>
    + Add<u32, Output = u32>
    + Sub<u32, Output = u32>
    + Mul<u32, Output = u32>
    + Div<u32, Output = u32>
    + Rem<u32, Output = u32>
    + Shl<u32, Output = u32>
    + Shr<u32, Output = u32>
    + Serialize
    + for<'de> Deserialize<'de>
{
    fn as_i32(self) -> i32;
    fn to_dyn(self) -> ValidPrime;

    fn as_u32(self) -> u32 {
        self.as_i32() as u32
    }

    fn as_usize(self) -> usize {
        self.as_u32() as usize
    }

    /// Computes the product mod p. This takes care of overflow.
    fn product(self, n1: u32, n2: u32) -> u32 {
        ((n1 as u64) * (n2 as u64) % (self.as_u32() as u64)) as u32
    }

    fn pow(self, exp: u32) -> u32 {
        self.as_u32().pow(exp)
    }

    fn pow_mod(self, mut b: u32, mut e: u32) -> u32 {
        assert!(self.as_u32() > 0);
        let mut result: u32 = 1;
        while e > 0 {
            if (e & 1) == 1 {
                result = self.product(result, b);
            }
            b = self.product(b, b);
            e >>= 1;
        }
        result
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimeError {
    NotAnInteger(std::num::ParseIntError),
    InvalidPrime(u32),
}

impl std::fmt::Display for PrimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimeError::NotAnInteger(s) => write!(f, "Not an integer: {}", s),
            PrimeError::InvalidPrime(p) => write!(f, "{} is not a valid prime", p),
        }
    }
}

macro_rules! def_prime_static {
    ($pn:ident, $p:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $pn;

        impl Prime for $pn {
            #[inline]
            fn as_i32(self) -> i32 {
                $p
            }

            #[inline]
            fn to_dyn(self) -> ValidPrime {
                ValidPrime::new($p)
            }
        }

        impl Serialize for $pn {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.as_u32().serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $pn {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let p: u32 = u32::deserialize(deserializer)?;
                $pn::try_from(p).map_err(D::Error::custom)
            }
        }
    };
}

def_prime_static!(P2, 2);
def_prime_static!(P3, 3);
def_prime_static!(P5, 5);
def_prime_static!(P7, 7);

macro_rules! impl_op_pn_u32 {
    ($pn:ty, $trt:ident, $mth:ident, $trt_assign:ident, $mth_assign:ident, $operator:tt) => {
        impl $trt<$pn> for u32 {
            type Output = u32;

            fn $mth(self, other: $pn) -> Self::Output {
                self $operator other.as_u32()
            }
        }

        impl $trt<u32> for $pn {
            type Output = u32;

            fn $mth(self, other: u32) -> Self::Output {
                self.as_u32() $operator other
            }
        }

        impl $trt<$pn> for $pn {
            type Output = u32;

            fn $mth(self, other: $pn) -> Self::Output {
                self.as_u32() $operator other.as_u32()
            }
        }

        impl $trt_assign<$pn> for u32 {
            fn $mth_assign(&mut self, other: $pn) {
                *self = *self $operator other;
            }
        }
    };
}

macro_rules! impl_prime_ops {
    ($pn:ty) => {
        impl_op_pn_u32!($pn, Add, add, AddAssign, add_assign, +);
        impl_op_pn_u32!($pn, Sub, sub, SubAssign, sub_assign, -);
        impl_op_pn_u32!($pn, Mul, mul, MulAssign, mul_assign, *);
        impl_op_pn_u32!($pn, Div, div, DivAssign, div_assign, /);
        impl_op_pn_u32!($pn, Rem, rem, RemAssign, rem_assign, %);
        impl_op_pn_u32!($pn, Shl, shl, ShlAssign, shl_assign, <<);
        impl_op_pn_u32!($pn, Shr, shr, ShrAssign, shr_assign, >>);

        impl PartialEq<u32> for $pn {
            fn eq(&self, other: &u32) -> bool {
                self.as_u32() == *other
            }
        }

        impl PartialOrd<u32> for $pn {
            fn partial_cmp(&self, other: &u32) -> Option<std::cmp::Ordering> {
                self.as_u32().partial_cmp(other)
            }
        }

        impl From<$pn> for u32 {
            fn from(value: $pn) -> u32 {
                value.as_u32()
            }
        }

        impl std::fmt::Display for $pn {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                <u32 as std::fmt::Display>::fmt(&self.as_u32(), f)
            }
        }
    };
}

impl_prime_ops!(P2);
impl_prime_ops!(P3);
impl_prime_ops!(P5);
impl_prime_ops!(P7);

macro_rules! impl_try_from {
    // We need the type both as a type and as an expression.
    ($pn:ty, $pn_ex:expr) => {
        impl std::convert::TryFrom<u32> for $pn {
            type Error = PrimeError;

            fn try_from(p: u32) -> Result<Self, PrimeError> {
                if $pn_ex == p {
                    Ok($pn_ex)
                } else {
                    Err(PrimeError::InvalidPrime(p))
                }
            }
        }
    };
}

impl_try_from!(P2, P2);
impl_try_from!(P3, P3);
impl_try_from!(P5, P5);
impl_try_from!(P7, P7);

#[cfg(feature = "odd-primes")]
mod validprime {
    use std::str::FromStr;

    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ValidPrime {
        p: u32,
    }

    pub const fn is_prime(p: u32) -> bool {
        // (2..p).all(|k| p % k != 0), but make it const
        let mut k = 2;
        while k < p {
            if p % k == 0 {
                return false;
            }
            k += 1;
        }
        true
    }

    impl ValidPrime {
        pub const fn new(p: u32) -> ValidPrime {
            // We need the size restriction for a few reasons.
            //
            // First, we need `bit_length(p)` to be smaller than 64. Otherwise, shifting a u64 by 64
            // bits is considered an overflow. We could special case these shifts to be setting to
            // 0, but that doesn't seem worth it.
            //
            // More importantly, the existence of `Prime::as_i32` means that we need `p` to fit in
            // an i32. We want this method because there are a few places in the codebase that
            // use it. It might be possible to go and change all of those to use `as_u32` instead,
            // but it doesn't seem worth it for now.
            assert!(p < (1 << 31), "Tried to construct a prime larger than 2^31");
            assert!(is_prime(p), "Tried to construct a composite dynamic prime");
            ValidPrime { p }
        }

        pub const fn new_unchecked(p: u32) -> ValidPrime {
            ValidPrime { p }
        }
    }

    impl Prime for ValidPrime {
        fn as_i32(self) -> i32 {
            self.p as i32
        }

        fn to_dyn(self) -> ValidPrime {
            self
        }
    }

    impl_prime_ops!(ValidPrime);

    impl TryFrom<u32> for ValidPrime {
        type Error = PrimeError;

        fn try_from(p: u32) -> Result<Self, PrimeError> {
            if is_prime(p) {
                Ok(ValidPrime { p })
            } else {
                Err(PrimeError::InvalidPrime(p))
            }
        }
    }

    impl FromStr for ValidPrime {
        type Err = PrimeError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let p: u32 = s.parse().map_err(PrimeError::NotAnInteger)?;
            ValidPrime::try_from(p)
        }
    }

    impl Serialize for ValidPrime {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            self.as_u32().serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for ValidPrime {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let p: u32 = u32::deserialize(deserializer)?;
            ValidPrime::try_from(p).map_err(D::Error::custom)
        }
    }
}

#[cfg(not(feature = "odd-primes"))]
mod validprime {
    use std::str::FromStr;

    use super::PrimeError;

    pub type ValidPrime = super::P2;

    pub const fn is_prime(p: u32) -> bool {
        p == 2
    }

    impl ValidPrime {
        pub const fn new(_p: u32) -> ValidPrime {
            // Disregard the argument, assume the prime is 2. This has the advantage of us being
            // able to use the same tests independently of whether odd-primes is enabled or not.
            //
            // This is sound but can cause some problems for the user that could be hard to
            // diagnose. Maybe use debug_assert! and fix the tests?
            super::P2
        }

        pub const fn new_unchecked(_p: u32) -> ValidPrime {
            super::P2
        }
    }

    impl FromStr for ValidPrime {
        type Err = PrimeError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let p = s.parse().map_err(PrimeError::NotAnInteger)?;
            if p == 2 {
                Ok(super::P2)
            } else {
                Err(PrimeError::InvalidPrime(p))
            }
        }
    }
}

pub use validprime::{is_prime, ValidPrime};

/// Compute b^e mod p. This is a const version of `Prime::pow_mod`.
pub const fn power_mod(p: u32, mut b: u32, mut e: u32) -> u32 {
    // We can't use Prime::product because const traits are still unstable
    assert!(p > 0);
    let mut result: u32 = 1;
    while e > 0 {
        if (e & 1) == 1 {
            result = ((result as u64) * (b as u64) % (p as u64)) as u32;
        }
        b = (((b as u64) * (b as u64)) % (p as u64)) as u32;
        e >>= 1;
    }
    result
}

/// Compute the base 2 log of a number, rounded down to the nearest integer.
///
/// # Example
/// ```
/// # use fp::prime::log2;
/// assert_eq!(0, log2(0b1));
/// assert_eq!(1, log2(0b10));
/// assert_eq!(1, log2(0b11));
/// assert_eq!(3, log2(0b1011));
/// ```
pub const fn log2(n: usize) -> usize {
    std::mem::size_of::<usize>() * 8 - 1 - n.leading_zeros() as usize
}

/// Discrete log base p of n.
pub fn logp<P: Prime>(p: P, mut n: u32) -> u32 {
    let mut result = 0u32;
    while n > 0 {
        n /= p.as_u32();
        result += 1;
    }
    result
}

/// Factor $n$ as $p^k m$. Returns $(k, m)$.
pub fn factor_pk<P: Prime>(p: P, mut n: u32) -> (u32, u32) {
    if n == 0 {
        return (0, 0);
    }
    let mut k = 0;
    while n % p.as_u32() == 0 {
        n /= p.as_u32();
        k += 1;
    }
    (k, n)
}

// Uses a the lookup table we initialized.
pub fn inverse<P: Prime>(p: P, k: u32) -> u32 {
    use crate::constants::{INVERSE_TABLE, MAX_PRIME, PRIME_TO_INDEX_MAP};
    assert!(k > 0 && p > k);

    if p <= MAX_PRIME as u32 {
        // LLVM doesn't understand the inequality is transitive
        unsafe { *INVERSE_TABLE[PRIME_TO_INDEX_MAP[p.as_usize()]].get_unchecked(k as usize) }
    } else {
        power_mod(p.as_u32(), k, p.as_u32() - 2)
    }
}

#[inline(always)]
pub fn minus_one_to_the_n<P: Prime>(p: P, i: i32) -> u32 {
    if i % 2 == 0 {
        1
    } else {
        p - 1
    }
}

#[cfg(test)]
mod tests {
    use super::{binomial::Binomial, inverse, iter::BinomialIterator, Prime, ValidPrime};
    use crate::{
        constants::PRIMES,
        prime::{is_prime, PrimeError},
    };

    #[test]
    fn validprime_test() {
        for p in (0..(1 << 16)).filter(|&p| is_prime(p)) {
            assert_eq!(ValidPrime::new(p), p);
        }
    }

    #[test]
    fn validprime_invalid() {
        assert_eq!(
            ValidPrime::try_from(4).unwrap_err(),
            PrimeError::InvalidPrime(4)
        );
        assert_eq!(
            "4".parse::<ValidPrime>().unwrap_err(),
            PrimeError::InvalidPrime(4)
        );
        assert_eq!(
            "4.0".parse::<ValidPrime>().unwrap_err(),
            PrimeError::NotAnInteger("4.0".parse::<u32>().unwrap_err())
        );
    }

    #[test]
    fn inverse_test() {
        for &p in PRIMES.iter() {
            let p = ValidPrime::new(p);
            for k in 1..p.as_u32() {
                assert_eq!((inverse(p, k) * k) % p, 1);
            }
        }
    }

    #[test]
    fn binomial_test() {
        let entries = [[2, 2, 1, 0], [2, 3, 1, 1], [3, 1090, 730, 1], [7, 3, 2, 3]];

        for entry in &entries {
            assert_eq!(
                entry[3],
                u32::binomial(ValidPrime::new(entry[0]), entry[1], entry[2])
            );
        }
    }

    #[test]
    fn binomial_vs_monomial() {
        for &p in &[2, 3, 5, 7, 11] {
            let p = ValidPrime::new(p);
            for l in 0..20 {
                for m in 0..20 {
                    assert_eq!(u32::binomial(p, l + m, m), u32::multinomial(p, &mut [l, m]))
                }
            }
        }
    }

    fn binomial_full(n: u32, j: u32) -> u32 {
        let mut res = 1;
        for k in j + 1..=n {
            res *= k;
        }
        for k in 1..=(n - j) {
            res /= k;
        }
        res
    }

    #[test]
    fn binomial_cmp() {
        for n in 0..12 {
            for j in 0..=n {
                let ans = binomial_full(n, j);
                for &p in &[2, 3, 5, 7, 11] {
                    assert_eq!(
                        u32::binomial(ValidPrime::new(p), n, j),
                        ans % p,
                        "{n} choose {j} mod {p}"
                    );
                }
                assert_eq!(u32::binomial4(n, j), ans % 4, "{n} choose {j} mod 4");
                // binomial4_rec is only called on large n. It does not handle the n = 0, 1 cases
                // correctly.
                if n > 1 {
                    assert_eq!(
                        u32::binomial4_rec(n, j),
                        ans % 4,
                        "{n} choose {j} mod 4 rec"
                    );
                }
            }
        }
    }

    #[test]
    fn binomial_iterator() {
        let mut iter = BinomialIterator::new(4);
        assert_eq!(iter.next(), Some(0b1111));
        assert_eq!(iter.next(), Some(0b10111));
        assert_eq!(iter.next(), Some(0b11011));
        assert_eq!(iter.next(), Some(0b11101));
        assert_eq!(iter.next(), Some(0b11110));
        assert_eq!(iter.next(), Some(0b100111));
        assert_eq!(iter.next(), Some(0b101011));
        assert_eq!(iter.next(), Some(0b101101));
        assert_eq!(iter.next(), Some(0b101110));
        assert_eq!(iter.next(), Some(0b110011));
    }
}
