use self::field_internal::FieldInternal;
use crate::prime::Prime;

pub mod element;
pub(crate) mod field_internal;

pub mod fp;
pub mod smallfq;

use element::FieldElement;
pub use fp::Fp;
pub use smallfq::SmallFq;

pub trait Field: FieldInternal + Sized {
    type Characteristic: Prime;

    fn characteristic(self) -> Self::Characteristic;

    fn degree(self) -> u32;

    fn q(self) -> u32 {
        self.characteristic().pow(self.degree())
    }

    fn zero(self) -> FieldElement<Self>;
    fn one(self) -> FieldElement<Self>;
}

// TODO: Figure out better tests
#[cfg(test)]
mod test {
    use super::{Field, SmallFq};
    use crate::prime::P2;

    #[test]
    fn test_f_4() {
        // Multiplication table generated by Sage.
        let f4 = SmallFq::new(P2, 2);
        let one = f4.one();
        let a = f4.a();

        let mut elements = vec![one];
        for _ in 1..f4.q() {
            let prev = elements.last().unwrap();
            elements.push(*prev * a);
        }

        let expansions = vec![one, a, a + one, one];

        assert_eq!(elements, expansions);
    }

    #[test]
    fn test_f_8() {
        // Multiplication table generated by Sage.
        let f8 = SmallFq::new(P2, 3);
        let one = f8.one();
        let a = f8.a();
        let a2 = a * a;

        let mut elements = vec![one];
        for _ in 1..f8.q() {
            let prev = elements.last().unwrap();
            elements.push(*prev * a);
        }

        let expansions = vec![one, a, a2, a + one, a2 + a, a2 + a + one, a2 + one, one];

        assert_eq!(elements, expansions);
    }

    #[test]
    fn test_f_16() {
        // Multiplication table generated by Sage.
        let f16 = SmallFq::new(P2, 4);
        let one = f16.one();
        let a = f16.a();
        let a2 = a * a;
        let a3 = a2 * a;

        let mut elements = vec![one];
        for _ in 1..f16.q() {
            let prev = elements.last().unwrap();
            elements.push(*prev * a);
        }

        let expansions = vec![
            one,
            a,
            a2,
            a3,
            a + one,
            a2 + a,
            a3 + a2,
            a3 + a + one,
            a2 + one,
            a3 + a,
            a2 + a + one,
            a3 + a2 + a,
            a3 + a2 + a + one,
            a3 + a2 + one,
            a3 + one,
            one,
        ];

        assert_eq!(elements, expansions);
    }

    #[cfg(feature = "odd-primes")]
    #[test]
    fn test_f_9() {
        use crate::prime::P3;

        // Multiplication table generated by Sage.
        let f9 = SmallFq::new(P3, 2);

        let one = f9.one();
        let two = one + one;
        let a = f9.a();

        let mut elements = vec![one];
        for _ in 1..f9.q() {
            let prev = elements.last().unwrap();
            elements.push(*prev * a);
        }

        let expansions = vec![
            one,
            a,
            a + one,
            two * a + one,
            two,
            two * a,
            two * a + two,
            a + two,
            one,
        ];

        assert_eq!(elements, expansions);
    }
}