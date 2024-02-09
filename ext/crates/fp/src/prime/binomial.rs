use crate::{
    constants::{BINOMIAL4_TABLE, BINOMIAL4_TABLE_SIZE, BINOMIAL_TABLE},
    PRIME_TO_INDEX_MAP,
};

use super::{Prime, ValidPrime};

/// This uses a lookup table for n choose k when n and k are both less than p.
/// Lucas's theorem reduces general binomial coefficients to this case.
///
/// Calling this function safely requires that `k, n < p`.  These invariants are often known
/// apriori because k and n are obtained by reducing mod p, so it is better to expose an unsafe
/// interface that avoids these checks.
unsafe fn direct_binomial(p: ValidPrime, n: usize, k: usize) -> u32 {
    *BINOMIAL_TABLE
        .get_unchecked(PRIME_TO_INDEX_MAP[p.as_usize()])
        .get_unchecked(n)
        .get_unchecked(k)
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
impl_binomial!(u16);
impl_binomial!(i32);
