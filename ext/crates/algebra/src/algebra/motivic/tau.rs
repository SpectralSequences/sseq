//! The coefficient ring $\mathbb{F}_2[\tau]$, represented by $\tau$-valuations.
//!
//! Every computation over the C-motivic Steenrod algebra that we perform is
//! **homogeneous** — differentials, products, and coproducts all preserve the
//! motivic weight — and a homogeneous element of $\mathbb{F}_2[\tau]$ in a fixed
//! bidegree is exactly $\tau^k$ for a single $k$ (or $0$). So we never need a
//! polynomial: a coefficient is one integer, its $\tau$-valuation, with a
//! sentinel for zero. That is [`Tau`].
//!
//! This is a small, self-contained scalar type for the motivic layer only. It is
//! deliberately **not** an [`fp`](fp) replacement threaded through the resolution
//! engine: the engine stays $\mathbb{F}_p$-only (see the `motivic` module docs),
//! and `Tau` is used solely by the motivic algebra and the Phase 2 lift.

/// A homogeneous $\mathbb{F}_2[\tau]$ coefficient: either $0$ or $\tau^k$ for a
/// unique $k \ge 0$, stored as the $\tau$-valuation with a zero sentinel.
///
/// Over $\mathbb{F}_2$ a nonzero homogeneous coefficient has unit $\mathbb{F}_2$
/// part, so the valuation is the whole story: `None` is $0$, `Some(k)` is
/// $\tau^k$. Multiplication adds valuations; addition of two coefficients in the
/// *same* bidegree can only ever combine equal powers (homogeneity), so it
/// cancels mod 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Tau(Option<u32>);

impl Tau {
    /// The zero coefficient.
    pub const ZERO: Self = Self(None);

    /// The unit $\tau^0 = 1$.
    pub const ONE: Self = Self(Some(0));

    /// The zero coefficient (method form, for symmetry with [`Tau::one`]).
    pub fn zero() -> Self {
        Self::ZERO
    }

    /// The unit $1 = \tau^0$.
    pub fn one() -> Self {
        Self::ONE
    }

    /// The coefficient $\tau^k$.
    pub fn power(k: u32) -> Self {
        Self(Some(k))
    }

    /// Whether this coefficient is zero.
    pub fn is_zero(self) -> bool {
        self.0.is_none()
    }

    /// The $\tau$-valuation: `Some(k)` if this is $\tau^k$, `None` if zero.
    pub fn valuation(self) -> Option<u32> {
        self.0
    }

    /// The $n$-th power $(\tau^k)^n = \tau^{kn}$, with the convention $x^0 = 1$.
    pub fn pow(self, n: u32) -> Self {
        if n == 0 {
            Self::ONE
        } else {
            Self(self.0.map(|k| k * n))
        }
    }

    /// Multiply by $\tau^k$, i.e. raise the valuation by `k` (zero stays zero).
    pub fn shift(self, k: u32) -> Self {
        Self(self.0.map(|v| v + k))
    }
}

impl std::ops::Mul for Tau {
    type Output = Self;

    /// Product: $\tau^i \cdot \tau^j = \tau^{i+j}$; zero absorbs.
    // Multiplying τ-powers genuinely *adds* their valuations, so the `+` here is correct.
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, other: Self) -> Self {
        match (self.0, other.0) {
            (Some(i), Some(j)) => Self(Some(i + j)),
            _ => Self::ZERO,
        }
    }
}

impl std::ops::Add for Tau {
    type Output = Self;

    /// Sum of two homogeneous coefficients in one bidegree. By homogeneity the
    /// two nonzero powers must be equal, so $\tau^k + \tau^k = 0$ over
    /// $\mathbb{F}_2$; a zero summand is absorbed. Adding *distinct* nonzero
    /// powers is inhomogeneous and cannot arise (checked in debug builds).
    fn add(self, other: Self) -> Self {
        match (self.0, other.0) {
            (None, x) | (x, None) => Self(x),
            (Some(i), Some(j)) => {
                debug_assert_eq!(i, j, "adding inhomogeneous tau powers τ^{i} + τ^{j}");
                Self::ZERO
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_one() {
        assert!(Tau::zero().is_zero());
        assert!(!Tau::one().is_zero());
        assert_eq!(Tau::one().valuation(), Some(0));
        assert_eq!(Tau::power(3).valuation(), Some(3));
        assert_eq!(Tau::zero().valuation(), None);
    }

    #[test]
    fn test_mul_adds_valuations() {
        assert_eq!(Tau::power(2) * Tau::power(3), Tau::power(5));
        assert_eq!(Tau::one() * Tau::power(4), Tau::power(4));
        assert_eq!(Tau::zero() * Tau::power(4), Tau::zero());
        assert_eq!(Tau::power(4) * Tau::zero(), Tau::zero());
    }

    #[test]
    fn test_add_cancels_equal_powers() {
        assert_eq!(Tau::power(2) + Tau::power(2), Tau::zero());
        assert_eq!(Tau::power(2) + Tau::zero(), Tau::power(2));
        assert_eq!(Tau::zero() + Tau::power(5), Tau::power(5));
        assert_eq!(Tau::zero() + Tau::zero(), Tau::zero());
    }

    #[test]
    fn test_pow_and_shift() {
        assert_eq!(Tau::power(3).pow(2), Tau::power(6));
        assert_eq!(Tau::power(3).pow(0), Tau::one());
        assert_eq!(Tau::zero().pow(0), Tau::one());
        assert_eq!(Tau::zero().pow(3), Tau::zero());
        assert_eq!(Tau::power(2).shift(3), Tau::power(5));
        assert_eq!(Tau::zero().shift(3), Tau::zero());
    }
}
