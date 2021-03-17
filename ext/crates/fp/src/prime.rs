pub const NUM_PRIMES: usize = 8;
pub const MAX_PRIME: usize = 19;
#[cfg(feature = "odd-primes")]
const NOT_A_PRIME: usize = !1;
pub const MAX_MULTINOMIAL_LEN: usize = 10;
#[cfg(feature = "json")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[macro_export]
macro_rules! const_for {
    ($i:ident in $a:literal .. $b:ident $contents:block) => {
        let mut $i = $a;
        while $i < $b {
            $contents;
            $i += 1;
        }
    };
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ValidPrime {
    #[cfg(feature = "odd-primes")]
    p: u32,
}

impl ValidPrime {
    pub const fn new(p: u32) -> Self {
        assert!(is_valid_prime(p));

        #[cfg(feature = "odd-primes")]
        {
            Self { p }
        }

        #[cfg(not(feature = "odd-primes"))]
        {
            Self {}
        }
    }

    pub fn try_new(p: u32) -> Option<Self> {
        if is_valid_prime(p) {
            Some(Self::new(p))
        } else {
            None
        }
    }

    /// Get the underlying prime. This is the same function as deref but
    /// 1. This is a const fn
    /// 2. This does not inform the compiler about properties of p via unreachable_unchecked.
    ///
    /// Use this function in a const context where you would expect it to be evaluated at
    /// compile-time.
    pub const fn value(&self) -> u32 {
        #[cfg(feature = "odd-primes")]
        {
            self.p
        }
        #[cfg(not(feature = "odd-primes"))]
        {
            2
        }
    }
}

impl std::ops::Deref for ValidPrime {
    type Target = u32;

    #[cfg(not(feature = "odd-primes"))]
    fn deref(&self) -> &Self::Target {
        &2
    }

    #[cfg(feature = "odd-primes")]
    fn deref(&self) -> &Self::Target {
        let p = self.p;
        unsafe {
            if !is_valid_prime(p) || p == 0 || PRIME_TO_INDEX_MAP[p as usize] >= NUM_PRIMES {
                std::hint::unreachable_unchecked()
            }
        }
        &self.p
    }
}

impl std::fmt::Display for ValidPrime {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        (**self).fmt(f)
    }
}

#[cfg(feature = "json")]
impl Serialize for ValidPrime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (**self).serialize(serializer)
    }
}

#[cfg(feature = "json")]
impl<'de> Deserialize<'de> for ValidPrime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let p: u32 = u32::deserialize(deserializer)?;
        Ok(ValidPrime::new(p))
    }
}

#[cfg(feature = "odd-primes")]
pub const fn is_valid_prime(p: u32) -> bool {
    (p as usize) <= MAX_PRIME && PRIME_TO_INDEX_MAP[p as usize] != NOT_A_PRIME
}

#[cfg(not(feature = "odd-primes"))]
pub const fn is_valid_prime(p: u32) -> bool {
    p == 2
}

// Uses a the lookup table we initialized.
pub fn inverse(p: ValidPrime, k: u32) -> u32 {
    assert!(k > 0 && k < *p);
    // LLVM doesn't understand the inequality is transitive
    unsafe { *INVERSE_TABLE[PRIME_TO_INDEX_MAP[*p as usize]].get_unchecked(k as usize) }
}

pub const fn minus_one_to_the_n(p: u32, i: i32) -> u32 {
    if i % 2 == 0 {
        1
    } else {
        p - 1
    }
}

/// This uses a lookup table for n choose k when n and k are both less than p.
/// Lucas's theorem reduces general binomial coefficients to this case.
///
/// Calling this function safely requires that `k, n < p`.  These invariants are often known
/// apriori because k and n are obtained by reducing mod p, so it is better to expose an unsafe
/// interface that avoids these checks.
unsafe fn direct_binomial(p: ValidPrime, n: usize, k: usize) -> u32 {
    *BINOMIAL_TABLE
        .get_unchecked(PRIME_TO_INDEX_MAP[*p as usize])
        .get_unchecked(n)
        .get_unchecked(k)
}

/// Computes b^e.
pub const fn integer_power(mut b: u32, mut e: u32) -> u32 {
    let mut result: u32 = 1;
    while e > 0 {
        // b is b^{2^i}
        // if the current bit of e is odd, mutliply b^{2^i} into result.
        if e & 1 == 1 {
            result *= b;
        }
        b *= b;
        e >>= 1;
    }
    result
}

/// Compute b^e mod p.
pub const fn power_mod(p: u32, mut b: u32, mut e: u32) -> u32 {
    assert!(p > 0);
    let mut result: u32 = 1;
    while e > 0 {
        if (e & 1) == 1 {
            result = (result * b) % p;
        }
        b = (b * b) % p;
        e >>= 1;
    }
    result
}

// Discrete log base p of n.
pub const fn logp(p: u32, mut n: u32) -> u32 {
    let mut result = 0u32;
    while n > 0 {
        n /= p;
        result += 1;
    }
    result
}

/// A number satisfying the Binomial trait supports computing various binomial coefficients. This
/// is implemented using a macro, since the implementation for all types is syntactically the same.
pub trait Binomial: Sized {
    /// mod 2 multinomial coefficient
    fn multinomial2(k: &[Self]) -> Self;

    /// mod 2 binomial coefficient n choose k
    fn binomial2(n: Self, k: Self) -> Self;

    /// Binomial coefficients mod 4. We pre-compute the coefficients for small values of n. For large
    /// n, we recursively use the fact that if n = 2^k + l, l < 2^k, then
    ///
    ///    n choose r = l choose r + 2 (l choose (r - 2^{k - 1})) + (l choose (r - 2^k))
    ///
    /// This is easy to verify using the fact that
    ///
    ///    (x + y)^{2^k} = x^{2^k} + 2 x^{2^{k - 1}} y^{2^{k - 1}} + y^{2^k}
    ///
    fn binomial4(n: Self, k: Self) -> Self;

    /// Compute binomial coefficients mod 4 using the recursion relation in the documentation of
    /// [Binomial::binomial4]. This calls into binomial4 instead of binomial4_rec. The main purpose
    /// of this is to separate out the logic for testing.
    fn binomial4_rec(n: Self, k: Self) -> Self;

    /// Computes the multinomial coefficient mod p using Lucas' theorem. This modifies the
    /// underlying list. For p = 2 it is more efficient to use multinomial2
    fn multinomial_odd(p: ValidPrime, l: &mut [Self]) -> Self;

    /// Compute odd binomial coefficients mod p, where p is odd. For p = 2 it is more efficient to
    /// use binomial2
    fn binomial_odd(p: ValidPrime, n: Self, k: Self) -> Self;

    /// Checks whether n choose k is zero mod p. Since we don't have to compute the value, this is
    /// faster than binomial_odd.
    fn binomial_odd_is_zero(p: ValidPrime, n: Self, k: Self) -> bool;

    /// Multinomial coefficient of the list l
    fn multinomial(p: ValidPrime, l: &mut [Self]) -> Self {
        if *p == 2 {
            Self::multinomial2(l)
        } else {
            Self::multinomial_odd(p, l)
        }
    }

    /// Binomial coefficient n choose k.
    fn binomial(p: ValidPrime, n: Self, k: Self) -> Self {
        if *p == 2 {
            Self::binomial2(n, k)
        } else {
            Self::binomial_odd(p, n, k)
        }
    }
}

macro_rules! impl_binomial {
    ($T:ty) => {
        impl Binomial for $T {
            #[inline]
            fn multinomial2(l: &[Self]) -> Self {
                let mut bit_or: Self = 0;
                let mut sum: Self = 0;
                for &e in l {
                    sum += e;
                    bit_or |= e;
                }
                if bit_or == sum {
                    1
                } else {
                    0
                }
            }

            #[inline]
            fn binomial2(n: Self, k: Self) -> Self {
                if n < k {
                    0
                } else if (n - k) & k == 0 {
                    1
                } else {
                    0
                }
            }
            #[inline]
            fn multinomial_odd(p_: ValidPrime, l: &mut [Self]) -> Self {
                let p = *p_ as Self;

                let mut n: Self = l.iter().sum();
                if n == 0 {
                    return 1;
                }
                let mut answer = 1;

                while n > 0 {
                    let mut multi: Self = 1;

                    let total_entry = n % p;
                    n /= p;

                    let mut partial_sum: Self = l[0] % p;
                    l[0] /= p;

                    for ll in l.iter_mut().skip(1) {
                        let entry = *ll % p;
                        *ll /= p;

                        partial_sum += entry;
                        if partial_sum > total_entry {
                            // This early return is necessary because direct_binomial only works when
                            // partial_sum < 19
                            return 0;
                        }
                        // This is safe because p < 20, partial_sum <= total_entry < p and entry < p.
                        multi *=
                            unsafe { direct_binomial(p_, partial_sum as usize, entry as usize) }
                                as Self;
                        multi %= p;
                    }
                    answer *= multi;
                    answer %= p;
                }
                answer
            }

            #[inline]
            fn binomial_odd(p_: ValidPrime, mut n: Self, mut k: Self) -> Self {
                let p = *p_ as Self;

                // We have both signed and unsigned types
                #[allow(unused_comparisons)]
                if n < k || k < 0 {
                    return 0;
                }

                let mut answer = 1;

                while n > 0 {
                    // This is safe because p < 20 and anything mod p is < p.
                    answer *=
                        unsafe { direct_binomial(p_, (n % p) as usize, (k % p) as usize) } as Self;
                    answer %= p;
                    n /= p;
                    k /= p;
                }
                answer
            }

            #[inline]
            fn binomial_odd_is_zero(p: ValidPrime, mut n: Self, mut k: Self) -> bool {
                let p = *p as Self;

                while n > 0 {
                    if n % p < k % p {
                        return true;
                    }
                    n /= p;
                    k /= p;
                }
                false
            }
            fn binomial4(n: Self, j: Self) -> Self {
                if (n as usize) < BINOMIAL4_TABLE_SIZE {
                    return BINOMIAL4_TABLE[n as usize][j as usize] as Self;
                }
                if (n - j) & j == 0 {
                    // Answer is odd
                    Self::binomial4_rec(n, j)
                } else if (n - j).count_ones() + j.count_ones() - n.count_ones() == 1 {
                    2
                } else {
                    0
                }
            }

            #[inline]
            fn binomial4_rec(n: Self, j: Self) -> Self {
                let k = (std::mem::size_of::<Self>() * 8) as u32 - n.leading_zeros() - 1;
                let l = n - (1 << k);
                let mut ans = 0;
                if j <= l {
                    ans += Self::binomial4(l, j)
                }
                let pow = 1 << (k - 1);
                if pow <= j && j <= l + pow {
                    ans += 2 * Self::binomial2(l, j - pow);
                }
                if j >= (1 << k) {
                    ans += Self::binomial4(l, j - (1 << k));
                }
                ans % 4
            }
        }
    };
}

impl_binomial!(u32);
impl_binomial!(u8);
impl_binomial!(i32);

pub struct BitflagIterator {
    remaining: u8,
    flag: u64,
}

impl BitflagIterator {
    pub fn new(flag: u64) -> Self {
        Self {
            remaining: u8::max_value(),
            flag,
        }
    }

    pub fn new_fixed_length(flag: u64, remaining: usize) -> Self {
        assert!(remaining <= 64);
        let remaining = remaining as u8;
        Self { remaining, flag }
    }

    pub fn set_bit_iterator(flag: u64) -> impl Iterator<Item = usize> {
        Self::new(flag)
            .enumerate()
            .filter_map(|(idx, v)| if v { Some(idx) } else { None })
    }
}

impl Iterator for BitflagIterator {
    type Item = bool;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining > 64 && self.flag == 0 || self.remaining == 0 {
            None
        } else {
            self.remaining -= 1;
            let result = self.flag & 1 == 1;
            self.flag >>= 1;
            Some(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_test() {
        for &p in PRIMES.iter() {
            let p = ValidPrime::new(p);
            for k in 1..*p {
                assert_eq!((inverse(p, k) * k) % *p, 1);
            }
        }
    }

    #[test]
    fn binomial_test() {
        let entries = [[2, 2, 1, 0], [2, 3, 1, 1], [3, 1090, 730, 1], [7, 3, 2, 3]];

        for entry in &entries {
            assert_eq!(
                entry[3] as u32,
                u32::binomial(ValidPrime::new(entry[0] as u32), entry[1], entry[2])
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
                        "{} choose {} mod {}",
                        n,
                        j,
                        p
                    );
                }
                assert_eq!(u32::binomial4(n, j), ans % 4, "{} choose {} mod 4", n, j);
                // binomial4_rec is only called on large n. It does not handle the n = 0, 1 cases
                // correctly.
                if n > 1 {
                    assert_eq!(
                        u32::binomial4_rec(n, j),
                        ans % 4,
                        "{} choose {} mod 4 rec",
                        n,
                        j
                    );
                }
            }
        }
    }
}

pub const PRIMES: [u32; NUM_PRIMES] = [2, 3, 5, 7, 11, 13, 17, 19];

pub const PRIME_TO_INDEX_MAP: [usize; MAX_PRIME + 1] = [
    !1, !1, 0, 1, !1, 2, !1, 3, !1, !1, !1, 4, !1, 5, !1, !1, !1, 6, !1, 7,
];

const INVERSE_TABLE: [[u32; MAX_PRIME]; NUM_PRIMES] = {
    let mut result = [[0; MAX_PRIME]; NUM_PRIMES];
    const_for! { i in 0 .. NUM_PRIMES {
        let p = PRIMES[i];
        const_for! { k in 1 .. p {
            result[i as usize][k as usize] = power_mod(p, k, p - 2);
        }}
    }};
    result
};

macro_rules! populate_binomial_table {
    ($res:expr, $size:ident, $mod:expr) => {
        const_for! { n in 0 .. $size {
            $res[n][0] = 1;
            const_for! { k in 0 .. n {
                $res[n][k + 1] = ($res[n - 1][k] + $res[n - 1][k + 1]) % $mod;
            }}
        }}
    };
}
const BINOMIAL4_TABLE_SIZE: usize = 50;

const BINOMIAL4_TABLE: [[u32; BINOMIAL4_TABLE_SIZE]; BINOMIAL4_TABLE_SIZE] = {
    let mut res = [[0; BINOMIAL4_TABLE_SIZE]; BINOMIAL4_TABLE_SIZE];
    populate_binomial_table!(res, BINOMIAL4_TABLE_SIZE, 4);
    res
};

static BINOMIAL_TABLE: [[[u32; MAX_PRIME]; MAX_PRIME]; NUM_PRIMES] = {
    let mut result = [[[0; MAX_PRIME]; MAX_PRIME]; NUM_PRIMES];
    const_for! { i in 0 .. NUM_PRIMES {
        let p = PRIMES[i];
        let pu = p as usize;
        populate_binomial_table!(result[i], pu, p);
    }}
    result
};
