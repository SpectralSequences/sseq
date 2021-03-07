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
        let mut $i = 0;
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
unsafe fn direct_binomial(p: ValidPrime, n: u32, k: u32) -> u32 {
    *BINOMIAL_TABLE
        .get_unchecked(PRIME_TO_INDEX_MAP[*p as usize])
        .get_unchecked(n as usize)
        .get_unchecked(k as usize)
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

// We next have separate implementations of binomial and multinomial coefficients for p = 2 and odd
// primes.

/// Multinomial coefficient of the list l
pub fn multinomial2(l: &[u32]) -> u32 {
    let mut bit_or = 0u32;
    let mut sum = 0u32;
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

/// Mod 2 binomial coefficient n choose k
#[inline]
pub fn binomial2(n: i32, k: i32) -> u32 {
    if n < k {
        0
    } else if (n - k) & k == 0 {
        1
    } else {
        0
    }
}

/// Mod p multinomial coefficient of l. If p is 2, more efficient to use Multinomial2.
/// This uses Lucas's theorem to reduce to n choose k for n, k < p.
pub fn multinomial_odd(p_: ValidPrime, l: &mut [u32]) -> u32 {
    let p = *p_;

    let mut n: u32 = l.iter().sum();
    if n == 0 {
        return 1;
    }
    let mut answer: u32 = 1;

    while n > 0 {
        let mut multi: u32 = 1;

        let total_entry = n % p;
        n /= p;

        let mut partial_sum: u32 = l[0] % p;
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
            multi *= unsafe { direct_binomial(p_, partial_sum, entry) };
            multi %= p;
        }
        answer *= multi;
        answer %= p;
    }
    answer
}

/// Mod p binomial coefficient n choose k. If p is 2, more efficient to use Binomial2.
#[inline]
pub fn binomial_odd(p_: ValidPrime, n: i32, k: i32) -> u32 {
    let p = *p_;

    if n < k || k < 0 {
        return 0;
    }

    let mut k = k as u32;
    let mut n = n as u32;

    let mut answer: u32 = 1;

    while n > 0 {
        // This is safe because p < 20 and anything mod p is < p.
        answer *= unsafe { direct_binomial(p_, n % p, k % p) };
        answer %= p;
        n /= p;
        k /= p;
    }
    answer
}

#[inline]
pub fn binomial_odd_is_zero(p: ValidPrime, mut n: u32, mut k: u32) -> bool {
    let p = *p;

    while n > 0 {
        if n % p < k % p {
            return true;
        }
        n /= p;
        k /= p;
    }
    false
}

/// This computes the multinomial coefficient $\binom{n}{l_1 \ldots l_k}\bmod p$, where $n$
/// is the sum of the entries of l. This function modifies the entries of l.
pub fn multinomial(p: ValidPrime, l: &mut [u32]) -> u32 {
    if *p == 2 {
        multinomial2(l)
    } else {
        multinomial_odd(p, l)
    }
}

/// Dispatch to binomial2 or binomial_odd
pub fn binomial(p: ValidPrime, n: i32, k: i32) -> u32 {
    if *p == 2 {
        binomial2(n, k)
    } else {
        binomial_odd(p, n, k)
    }
}

/// Binomial coefficients mod 4. We pre-compute the coefficients for small values of n. For large
/// n, we recursively use the fact that if n = 2^k + l, l < 2^k, then
///
///    n choose r = l choose r + 2 (l choose (r - 2^{k - 1})) + (l choose (r - 2^k))
///
/// This is easy to verify using the fact that
///
///    (x + y)^{2^k} = x^{2^k} + 2 x^{2^{k - 1}} y^{2^{k - 1}} + y^{2^k}
///
pub fn binomial4(n: u32, j: u32) -> u32 {
    if (n as usize) < BINOMIAL4_TABLE_SIZE {
        return BINOMIAL4_TABLE[n as usize][j as usize];
    }
    #[allow(clippy::collapsible_else_if)]
    if (n - j) & j == 0 {
        // Answer is odd
        binomial4_rec(n, j)
    } else if (n - j).count_ones() + j.count_ones() - n.count_ones() == 1 {
        2
    } else {
        0
    }
}

/// Separate out the recursive logic for binomial4 for testing
#[inline]
fn binomial4_rec(n: u32, j: u32) -> u32 {
    let k = 32 - n.leading_zeros() - 1;
    let l = n - (1 << k);
    let mut ans = 0;
    if j <= l {
        ans += binomial4(l, j)
    }
    let diff = j as i32 - (1u32 << (k - 1)) as i32;
    if 0 <= diff && diff as u32 <= l {
        ans += 2 * binomial2(l as i32, diff);
    }
    if j >= (1 << k) {
        ans += binomial4(l, j - (1 << k));
    }
    ans % 4
}

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
                binomial(ValidPrime::new(entry[0] as u32), entry[1], entry[2])
            );
        }
    }

    #[test]
    fn binomial_vs_monomial() {
        for &p in &[2, 3, 5, 7, 11] {
            let p = ValidPrime::new(p);
            for l in 0..20 {
                for m in 0..20 {
                    assert_eq!(
                        binomial(p, (l + m) as i32, m as i32),
                        multinomial(p, &mut [l, m])
                    )
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
                        binomial(ValidPrime::new(p), n as i32, j as i32),
                        ans % p,
                        "{} choose {} mod {}",
                        n,
                        j,
                        p
                    );
                }
                assert_eq!(binomial4(n, j), ans % 4, "{} choose {} mod 4", n, j);
                // binomial4_rec is only called on large n. It does not handle the n = 0, 1 cases
                // correctly.
                if n > 1 {
                    assert_eq!(binomial4_rec(n, j), ans % 4, "{} choose {} mod 4 rec", n, j);
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
