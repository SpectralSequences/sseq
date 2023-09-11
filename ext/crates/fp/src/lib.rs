mod constants;
mod limb;

pub use constants::{MAX_MULTINOMIAL_LEN, NUM_PRIMES, PRIMES, PRIME_TO_INDEX_MAP};

pub mod matrix;
pub mod prime;
pub mod vector;

pub(crate) mod simd;

#[cfg(feature = "odd-primes")]
pub const ODD_PRIMES: bool = true;
#[cfg(not(feature = "odd-primes"))]
pub const ODD_PRIMES: bool = false;
