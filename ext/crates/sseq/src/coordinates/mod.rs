pub mod bidegree;
pub mod element;
pub mod generator;
pub mod iter;
pub mod lattice;
pub mod ordered_bidegrees;

pub use bidegree::Bidegree;
pub use element::BidegreeElement;
pub use generator::BidegreeGenerator;
pub use ordered_bidegrees::{ClassicalBidegree, StemBidegree};

#[cfg(test)]
mod test {
    use std::cmp::{max, min};

    use fp::{prime::ValidPrime, vector::FpVector};
    use rand::Rng;

    use super::{Bidegree, BidegreeElement, BidegreeGenerator};

    fn random_bidegree_pair<F: Fn(u32, i32) -> Bidegree>(f: F) -> (Bidegree, Bidegree) {
        let mut rng = rand::thread_rng();
        let s1 = rng.gen_range(0..1000);
        let s2 = rng.gen_range(0..1000);
        let h1 = rng.gen_range(-1000..1000);
        let h2 = rng.gen_range(-1000..1000);
        (f(min(s1, s2), min(h1, h2)), f(max(s1, s2), max(h1, h2)))
    }

    #[test]
    fn test_classical_iterator() {
        let (begin, end) = random_bidegree_pair(Bidegree::classical);
        let mut classical_iter = begin.iter_classical_to(end);
        for t in begin.t()..=end.t() {
            for s in begin.s()..=end.s() {
                assert_eq!(
                    classical_iter.next(),
                    Some(Bidegree::classical(s, t).into())
                );
            }
        }
        assert_eq!(classical_iter.next(), None);
    }

    #[test]
    fn test_stem_iterator() {
        let (begin, end) = random_bidegree_pair(Bidegree::stem);
        let mut stem_iter = begin.iter_stem_to(end);
        for n in begin.n()..=end.n() {
            for s in begin.s()..=end.s() {
                assert_eq!(stem_iter.next(), Some(Bidegree::stem(s, n).into()));
            }
        }
        assert_eq!(stem_iter.next(), None);
    }

    #[test]
    fn test_bidegree_generator_try_from_element() {
        let b = Bidegree::stem(9, 23);
        let mut vec = FpVector::new(ValidPrime::new(2), 2);
        vec.set_entry(1, 1);
        let h1_pd0 = BidegreeElement::new(b, vec.clone());
        assert_eq!(
            Ok(BidegreeGenerator::new(Bidegree::stem(9, 23), 1)),
            h1_pd0.try_into()
        );
        vec.set_entry(0, 1);
        let h0_squared_i = BidegreeElement::new(b, vec);
        assert_eq!(
            Result::<BidegreeGenerator, ()>::Err(()),
            h0_squared_i.try_into()
        );
    }
}
