use fp::prime::{Prime, ValidPrime, iter::BitflagIterator};
use itertools::Itertools;

use super::{PPart, PPartEntry};
use crate::algebra::combinatorics;

/// A Milnor basis element.
#[derive(Debug, Clone, Copy, Default)]
pub struct MilnorBasisElement {
    pub q_part: u32,
    pub p_part: PPart,
    pub degree: i32,
}

impl MilnorBasisElement {
    pub(super) fn from_p(p_part: PPart, degree: i32) -> Self {
        Self {
            q_part: 0,
            p_part,
            degree,
        }
    }

    pub(super) fn excess(&self, p: ValidPrime) -> u32 {
        if p == 2 {
            self.p_part.iter().sum::<PPartEntry>()
        } else {
            self.q_part.count_ones() + 2 * self.p_part.iter().sum::<PPartEntry>()
        }
    }

    pub fn clone_into(&self, other: &mut Self) {
        *other = *self;
    }

    /// Update the degree component to the correct degree
    pub fn compute_degree(&mut self, p: ValidPrime) {
        let q = if p == 2 { 1 } else { 2 * (p.as_i32() - 1) };
        let xi_degrees = combinatorics::xi_degrees(p);
        let tau_degrees = combinatorics::tau_degrees(p);

        self.degree = q * std::iter::zip(xi_degrees, self.p_part.iter())
            .map(|(&a, b)| a * b as i32)
            .sum::<i32>()
            + BitflagIterator::set_bit_iterator(self.q_part as u64)
                .map(|k| tau_degrees[k])
                .sum::<i32>();
    }
}

impl std::cmp::PartialEq for MilnorBasisElement {
    fn eq(&self, other: &Self) -> bool {
        #[cfg(feature = "odd-primes")]
        return self.p_part == other.p_part && self.q_part == other.q_part;

        #[cfg(not(feature = "odd-primes"))]
        return self.p_part == other.p_part;
    }
}

impl std::cmp::Eq for MilnorBasisElement {}

impl std::hash::Hash for MilnorBasisElement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.p_part.hash(state);
        #[cfg(feature = "odd-primes")]
        self.q_part.hash(state);
    }
}

impl std::fmt::Display for MilnorBasisElement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.degree == 0 {
            write!(f, "1")?;
            return Ok(());
        }
        if self.q_part != 0 {
            let q_part = BitflagIterator::set_bit_iterator(self.q_part as u64)
                .map(|idx| format!("Q_{idx}"))
                .format(" ");
            write!(f, "{q_part}")?;
        }
        if !self.p_part.is_empty() {
            if self.q_part != 0 {
                write!(f, " ")?;
            }
            write!(f, "P({})", self.p_part.iter().format(", "))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone_into() {
        let mut other = MilnorBasisElement::default();

        let mut check = |a: &MilnorBasisElement| {
            a.clone_into(&mut other);
            assert_eq!(a, &other);
        };

        check(&MilnorBasisElement {
            q_part: 3,
            p_part: PPart::from_slice(&[3, 2]),
            degree: 12,
        });
        check(&MilnorBasisElement {
            q_part: 1,
            p_part: PPart::from_slice(&[3]),
            degree: 11,
        });
        check(&MilnorBasisElement {
            q_part: 5,
            p_part: PPart::from_slice(&[1, 3, 5, 2]),
            degree: 7,
        });
        check(&MilnorBasisElement {
            q_part: 0,
            p_part: PPart::zero(),
            degree: 2,
        });
    }
}
