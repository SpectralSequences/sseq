use std::str::FromStr;

use super::*;

def_prime_static!(P2, 2);
def_prime_static!(P3, 3);
def_prime_static!(P5, 5);
def_prime_static!(P7, 7);

impl_prime_ops!(P2);
impl_prime_ops!(P3);
impl_prime_ops!(P5);
impl_prime_ops!(P7);

impl_try_from!(P2);
impl_try_from!(P3);
impl_try_from!(P5);
impl_try_from!(P7);

pub(crate) mod fp {
    use super::{P2, P3, P5, P7};
    use crate::field::Fp;

    pub const F2: Fp<P2> = Fp::new(P2);
    pub const F3: Fp<P3> = Fp::new(P3);
    pub const F5: Fp<P5> = Fp::new(P5);
    pub const F7: Fp<P7> = Fp::new(P7);
}

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
    pub const fn new(p: u32) -> Self {
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
        Self { p }
    }

    pub const fn new_unchecked(p: u32) -> Self {
        Self { p }
    }
}

impl Prime for ValidPrime {
    fn as_i32(self) -> i32 {
        self.p as i32
    }

    fn to_dyn(self) -> Self {
        self
    }
}

impl_prime_ops!(ValidPrime);

impl TryFrom<u32> for ValidPrime {
    type Error = PrimeError;

    fn try_from(p: u32) -> Result<Self, PrimeError> {
        if is_prime(p) {
            Ok(Self { p })
        } else {
            Err(PrimeError::InvalidPrime(p))
        }
    }
}

impl FromStr for ValidPrime {
    type Err = PrimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p: u32 = s.parse().map_err(PrimeError::NotAnInteger)?;
        Self::try_from(p)
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
        Self::try_from(p).map_err(D::Error::custom)
    }
}

#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for ValidPrime {
    type Parameters = Option<std::num::NonZeroU32>;
    type Strategy = proptest::sample::Select<Self>;

    /// An arbitrary `ValidPrime` in the range `2..(1 << 24)`, plus the largest prime that we
    /// support. If `max` is specified, the primes are restricted to be less than `max`.
    fn arbitrary_with(max: Self::Parameters) -> Self::Strategy {
        use std::sync::OnceLock;

        static TEST_PRIMES: OnceLock<Vec<ValidPrime>> = OnceLock::new();
        let test_primes = TEST_PRIMES.get_or_init(|| {
            // Sieve of erathosthenes
            const MAX: usize = 1 << 24;
            let mut is_prime = Vec::new();
            is_prime.resize_with(MAX, || true);
            is_prime[0] = false;
            is_prime[1] = false;
            for i in 2..MAX {
                if is_prime[i] {
                    for j in ((2 * i)..MAX).step_by(i) {
                        is_prime[j] = false;
                    }
                }
            }
            (0..MAX)
                .filter(|&i| is_prime[i])
                .map(|p| Self::new_unchecked(p as u32))
                .chain(std::iter::once(Self::new_unchecked(2147483647)))
                .collect()
        });
        let restricted_slice = if let Some(max) = max {
            let max_index = test_primes
                .iter()
                .position(|&p| p >= max.get())
                .unwrap_or(test_primes.len());

            &test_primes[..max_index]
        } else {
            test_primes
        };
        proptest::sample::select(restricted_slice)
    }
}

impl crate::MaybeArbitrary<Option<NonZeroU32>> for ValidPrime {}
