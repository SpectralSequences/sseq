use super::{Prime, ValidPrime};
use crate::{PRIME_TO_INDEX_MAP, constants::BINOMIAL_TABLE};

/// This uses a lookup table for n choose k when n and k are both less than p.
/// Lucas's theorem reduces general binomial coefficients to this case.
///
/// Calling this function safely requires that `k, n < p`.  These invariants are often known
/// apriori because k and n are obtained by reducing mod p, so it is better to expose an unsafe
/// interface that avoids these checks.
unsafe fn direct_binomial(p: ValidPrime, n: usize, k: usize) -> u32 {
    unsafe {
        *BINOMIAL_TABLE
            .get_unchecked(PRIME_TO_INDEX_MAP[p.as_usize()])
            .get_unchecked(n)
            .get_unchecked(k)
    }
}

/// A number satisfying the Binomial trait supports computing various binomial coefficients. This
/// is implemented using a macro, since the implementation for all types is syntactically the same.
pub trait Binomial: Sized {
    /// mod 2 multinomial coefficient
    fn multinomial2(k: &[Self]) -> Self;

    /// mod 2 binomial coefficient n choose k
    fn binomial2(n: Self, k: Self) -> Self;

    /// Binomial coefficient n choose k mod 4. Zero when `k > n` or either is negative.
    ///
    /// By Kummer's theorem the 2-adic valuation of `n choose k` is the number of carries when
    /// adding `k` and `m = n - k` in base 2. When there are none the coefficient is odd, and its
    /// residue mod 4 is `-1` raised to the number of adjacent bit positions where one of `k` and
    /// `m` has a 1 directly above a 1 of the other.
    fn binomial4(n: Self, k: Self) -> Self;

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
        if p == 2 {
            Self::multinomial2(l)
        } else {
            Self::multinomial_odd(p, l)
        }
    }

    /// Binomial coefficient n choose k.
    fn binomial(p: ValidPrime, n: Self, k: Self) -> Self {
        if p == 2 {
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
                    // Checked before the addition, which could otherwise overflow.
                    #[allow(unused_comparisons)] // unused when ty is unsigned
                    if e < 0 {
                        return 0;
                    }
                    sum += e;
                    bit_or |= e;
                }
                if bit_or == sum { 1 } else { 0 }
            }

            #[inline]
            fn binomial2(n: Self, k: Self) -> Self {
                // The wrapped difference is only meaningful when `0 <= k <= n`, which the other two
                // conditions check. Combining with `&` rather than `&&` keeps this branchless.
                let d = n.wrapping_sub(k);
                #[allow(unused_comparisons)] // `k >= 0` is unused when ty is unsigned
                let ans = (k <= n) & (k >= 0) & (d & k == 0);
                ans as Self
            }

            #[inline]
            fn multinomial_odd(p_: ValidPrime, l: &mut [Self]) -> Self {
                let p = p_.as_u32() as Self;

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
                let p = p_.as_u32() as Self;

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
                let p = p.as_u32() as Self;

                while n > 0 {
                    if n % p < k % p {
                        return true;
                    }
                    n /= p;
                    k /= p;
                }
                false
            }

            #[inline]
            fn binomial4(n: Self, k: Self) -> Self {
                #[allow(unused_comparisons)] // unused when ty is unsigned
                if n < 0 || k < 0 {
                    return 0;
                }
                binomial4_u64(n as u64, k as u64) as Self
            }
        }
    };
}

impl_binomial!(u32);
impl_binomial!(u16);
impl_binomial!(i32);

/// Residues mod 4 packed two bits apiece, indexed by `2 * min(carries, 2) + parity`.
///
/// Only the no-carry entries depend on the parity: one carry gives 2 and more give 0.
const MOD4_BY_CARRIES_AND_PARITY: u32 = 0b00_00_10_10_11_01;

/// [`Binomial::binomial4`] on `u64`, without the sign check.
///
/// The carries of `k + m` are the bits of `n ^ k ^ m`. Reading the result from
/// [`MOD4_BY_CARRIES_AND_PARITY`] rather than branching on the carry count keeps the cost
/// independent of the inputs, which otherwise mispredict on mixed odd and even coefficients.
#[inline]
fn binomial4_u64(n: u64, k: u64) -> u8 {
    let m = n.wrapping_sub(k);
    let carries = (n ^ k ^ m).count_ones().min(2);
    let parity = ((k & (m >> 1)) | (m & (k >> 1))).count_ones() & 1;
    let ans = (MOD4_BY_CARRIES_AND_PARITY >> (2 * (2 * carries + parity))) & 3;
    (ans as u8) & 0u8.wrapping_sub((k <= n) as u8)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// Upper bound, exclusive, on the `n` and `k` checked exhaustively.
    const EXHAUSTIVE: u32 = 1024;

    /// Slow model of n choose k mod 4, independent of Kummer's theorem.
    ///
    /// For `n = 2^t + l` with `l < 2^t`, expanding `(x + y)^{2^t}` mod 4 as
    /// `x^{2^t} + 2 x^{2^{t-1}} y^{2^{t-1}} + y^{2^t}` gives
    ///
    /// ```text
    /// n choose k = l choose k + 2 (l choose (k - 2^{t-1})) + l choose (k - 2^t)
    /// ```
    ///
    /// where the middle term only matters mod 2.
    fn binomial4_model(n: u32, k: u32) -> u32 {
        if k > n {
            return 0;
        }
        if n <= 1 {
            return 1;
        }
        let t = u32::BITS - 1 - n.leading_zeros();
        let l = n - (1 << t);
        let half = 1 << (t - 1);
        let mut ans = binomial4_model(l, k);
        if k >= half {
            ans += 2 * u32::binomial2(l, k - half);
        }
        if k >= 1 << t {
            ans += binomial4_model(l, k - (1 << t));
        }
        ans % 4
    }

    /// The model agrees with Pascal's triangle mod 4.
    #[test]
    fn model_vs_pascal() {
        let mut row = vec![1u32];
        for n in 0..EXHAUSTIVE {
            for k in 0..EXHAUSTIVE {
                let want = row.get(k as usize).copied().unwrap_or(0);
                assert_eq!(binomial4_model(n, k), want, "{n} choose {k}");
            }
            let mut next = vec![1; row.len() + 1];
            for k in 1..row.len() {
                next[k] = (row[k - 1] + row[k]) % 4;
            }
            row = next;
        }
    }

    /// [`Binomial::binomial4`] agrees with the model for every type, including where `k > n`.
    #[test]
    fn binomial4_vs_model() {
        for n in 0..EXHAUSTIVE {
            for k in 0..EXHAUSTIVE {
                let want = binomial4_model(n, k);
                assert_eq!(u32::binomial4(n, k), want, "{n} choose {k}");
                assert_eq!(
                    u16::binomial4(n as u16, k as u16),
                    want as u16,
                    "{n} choose {k}"
                );
                assert_eq!(
                    i32::binomial4(n as i32, k as i32),
                    want as i32,
                    "{n} choose {k}"
                );
            }
        }
    }

    proptest! {
        /// [`Binomial::binomial4`] agrees with the model for large `n`.
        #[test]
        fn binomial4_vs_model_large(n in 0u32..1 << 20, k_bits: u32, subset: bool) {
            // A bit-subset of `n` makes the coefficient odd, which uniform `k` rarely does.
            let k = if subset { n & k_bits } else { k_bits % (n + 1) };
            prop_assert_eq!(u32::binomial4(n, k), binomial4_model(n, k));
        }

        /// [`Binomial::binomial4`] is zero whenever `n` or `k` is negative.
        #[test]
        fn binomial4_negative(n: i32, k: i32) {
            prop_assume!(n < 0 || k < 0);
            prop_assert_eq!(i32::binomial4(n, k), 0);
        }
    }
}
