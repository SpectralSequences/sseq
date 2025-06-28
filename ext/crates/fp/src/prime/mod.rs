use std::{
    fmt::{Debug, Display},
    hash::Hash,
    num::NonZeroU32,
    ops::{
        Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr,
        ShrAssign, Sub, SubAssign,
    },
};

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

pub mod binomial;
pub mod iter;

#[cfg(not(feature = "odd-primes"))]
pub mod primes_2;
#[cfg(feature = "odd-primes")]
pub mod primes_generic;

pub use binomial::Binomial;
#[cfg(not(feature = "odd-primes"))]
pub use primes_2::*;
#[cfg(feature = "odd-primes")]
pub use primes_generic::*;

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
#[allow(private_bounds)]
pub trait Prime:
    Debug
    + Clone
    + Copy
    + Display
    + Hash
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
    + crate::MaybeArbitrary<Option<NonZeroU32>>
    + 'static
{
    fn as_i32(self) -> i32;
    fn to_dyn(self) -> ValidPrime;

    fn as_u32(self) -> u32 {
        self.as_i32() as u32
    }

    fn as_usize(self) -> usize {
        self.as_u32() as usize
    }

    /// Computes the sum mod p. This takes care of overflow.
    fn sum(self, n1: u32, n2: u32) -> u32 {
        let n1 = n1 as u64;
        let n2 = n2 as u64;
        let p = self.as_u32() as u64;
        let sum = (n1 + n2) % p;
        sum as u32
    }

    /// Computes the product mod p. This takes care of overflow.
    fn product(self, n1: u32, n2: u32) -> u32 {
        let n1 = n1 as u64;
        let n2 = n2 as u64;
        let p = self.as_u32() as u64;
        let product = (n1 * n2) % p;
        product as u32
    }

    fn inverse(self, k: u32) -> u32 {
        inverse(self, k)
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
            Self::NotAnInteger(s) => write!(f, "Not an integer: {s}"),
            Self::InvalidPrime(p) => write!(f, "{p} is not a valid prime"),
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

        #[cfg(feature = "proptest")]
        impl proptest::arbitrary::Arbitrary for $pn {
            type Parameters = Option<NonZeroU32>;
            type Strategy = proptest::strategy::Just<$pn>;

            fn arbitrary_with(_max: Self::Parameters) -> Self::Strategy {
                // This doesn't honor the max parameter, but that should be fine as long as the
                // static primes are small enough and/or the max is large enough. There's no such
                // thing as an empty strategy, so the best we could do is return a strategy that
                // always rejects. This would cause local failures that may make tests fail.
                proptest::strategy::Just($pn)
            }
        }

        impl crate::MaybeArbitrary<Option<NonZeroU32>> for $pn {}
    };
}

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

macro_rules! impl_try_from {
    ($p:tt) => {
        impl_try_from!(@ $p, $p);
    };
    // We need the type both as a type and as an expression.
    (@ $pn:ty, $pn_ex:expr) => {
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

// Strange but required to export macro properly
use def_prime_static;
use impl_op_pn_u32;
use impl_prime_ops;
use impl_try_from;

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
    while n.is_multiple_of(p.as_u32()) {
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
pub(crate) mod tests {
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
