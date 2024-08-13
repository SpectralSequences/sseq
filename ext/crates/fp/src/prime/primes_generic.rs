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

impl_try_from!(P2, P2);
impl_try_from!(P3, P3);
impl_try_from!(P5, P5);
impl_try_from!(P7, P7);

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
