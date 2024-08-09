use std::str::FromStr;

use super::*;

def_prime_static!(P2, 2);
impl_prime_ops!(P2);
impl_try_from!(P2, P2);

pub type ValidPrime = P2;

pub(crate) mod fp {
    use super::P2;
    use crate::field::Fp;

    pub const F2: Fp<P2> = Fp::new(P2);
}

pub const fn is_prime(p: u32) -> bool {
    p == 2
}

impl ValidPrime {
    pub const fn new(_p: u32) -> Self {
        // Disregard the argument, assume the prime is 2. This has the advantage of us being
        // able to use the same tests independently of whether odd-primes is enabled or not.
        //
        // This is sound but can cause some problems for the user that could be hard to
        // diagnose. Maybe use debug_assert! and fix the tests?
        Self
    }

    pub const fn new_unchecked(_p: u32) -> Self {
        Self
    }
}

impl FromStr for ValidPrime {
    type Err = PrimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = s.parse().map_err(PrimeError::NotAnInteger)?;
        if p == 2 {
            Ok(Self)
        } else {
            Err(PrimeError::InvalidPrime(p))
        }
    }
}
