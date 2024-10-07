use super::Subspace;
use crate::{
    matrix::Matrix,
    prime::ValidPrime,
    vector::{FpSlice, FpSliceMut, FpVector},
};

#[derive(Debug, Clone)]
pub struct Subquotient {
    gens: Subspace,
    quotient: Subspace,
    dimension: usize,
}

impl std::fmt::Display for Subquotient {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Generators:\n{}", self.gens)?;
        writeln!(f, "Zeros:\n{}", self.quotient)
    }
}

impl Subquotient {
    /// Create a new subquotient of an ambient space of dimension `dim`. This defaults to the zero
    /// subspace.
    pub fn new(p: ValidPrime, dim: usize) -> Self {
        Self {
            gens: Subspace::new(p, dim),
            quotient: Subspace::new(p, dim),
            dimension: 0,
        }
    }

    /// Create a new subquotient of an ambient space of dimension `dim`, where the subspace is the
    /// full subspace and quotient is trivial.
    pub fn new_full(p: ValidPrime, dim: usize) -> Self {
        let mut result = Self::new(p, dim);
        result.gens.set_to_entire();
        result.dimension = dim;
        result
    }

    /// Given a vector `elt`, project `elt` to the complement and express
    /// as a linear combination of the basis. The result is returned as a list of coefficients.
    /// If elt is nonzero afterwards, this means the vector was not in the subspace to begin with.
    pub fn reduce(&self, mut elt: FpSliceMut) -> Vec<u32> {
        self.quotient.reduce(elt.copy());
        let mut result = Vec::with_capacity(self.gens.ambient_dimension());
        for i in 0..self.gens.ambient_dimension() {
            if self.gens.pivots()[i] < 0 {
                continue;
            }
            let c = elt.as_slice().entry(i);
            result.push(c);
            if c != 0 {
                elt.add(
                    self.gens.row(self.gens.pivots()[i] as usize),
                    ((elt.prime() - 1) * c) % elt.prime(),
                );
            }
        }
        result
    }

    /// Project the vector onto the complement of the quotient part of the subquotient.
    pub fn reduce_by_quotient(&self, elt: FpSliceMut) {
        self.quotient.reduce(elt)
    }

    /// Set the subquotient to be the full ambient space quotiented by zero
    pub fn set_to_full(&mut self) {
        self.quotient.set_to_zero();
        self.gens.set_to_entire();
    }

    pub fn zeros(&self) -> &Subspace {
        &self.quotient
    }

    pub fn gens(&self) -> impl Iterator<Item = FpSlice> {
        self.gens.iter()
    }

    pub fn quotient_dimension(&self) -> usize {
        self.ambient_dimension() - self.quotient.dimension()
    }

    /// The dimension of the subspace part of the subquotient.
    pub fn subspace_dimension(&self) -> usize {
        self.dimension + self.quotient.dimension()
    }

    /// The generators of the subspace part of the subquotient.
    pub fn subspace_gens(&self) -> impl Iterator<Item = FpSlice> {
        self.gens().chain(self.quotient.iter())
    }

    /// The pivot columns of the complement to the subspace
    pub fn complement_pivots(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.ambient_dimension())
            .filter(|&i| self.quotient.pivots()[i] < 0 && self.gens.pivots()[i] < 0)
    }

    pub fn quotient(&mut self, elt: FpSlice) {
        self.quotient.add_vector(elt);

        self.gens.update_then_row_reduce(|gens_matrix| {
            for elt in gens_matrix.iter_mut().take(self.dimension) {
                self.quotient.reduce(elt.as_slice_mut());
            }
        });

        self.dimension = self.gens.dimension();
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn ambient_dimension(&self) -> usize {
        self.gens.ambient_dimension()
    }

    pub fn prime(&self) -> ValidPrime {
        self.gens.prime()
    }

    pub fn is_empty(&self) -> bool {
        self.dimension == 0
    }

    pub fn clear_gens(&mut self) {
        self.gens.set_to_zero();
        self.dimension = 0;
    }

    pub fn add_gen(&mut self, gen: FpSlice) {
        self.gens.update_then_row_reduce(|gens_matrix| {
            let mut new_row = gens_matrix.row_mut(self.dimension);
            new_row.assign(gen);
            self.quotient.reduce(new_row);
        });
        self.dimension = self.gens.dimension();
    }

    pub fn reduce_matrix(matrix: &Matrix, source: &Self, target: &Self) -> Vec<Vec<u32>> {
        let mut result = Vec::with_capacity(source.dimension());
        let mut temp = FpVector::new(source.prime(), target.ambient_dimension());
        for v in source.gens() {
            matrix.apply(temp.as_slice_mut(), 1, v);
            result.push(target.reduce(temp.as_slice_mut()));
            temp.set_to_zero()
        }
        result
    }

    /// Given a chain of subspaces `quotient` < `sub` in some ambient space, compute the subquotient
    /// `sub`/`quotient`. The answer is expressed as a list of basis vectors of `sub` whose image in
    /// `sub`/`quotient` forms a basis, and a basis vector of `sub` is described by its index in the
    /// list of basis vectors of `sub` (not the ambient space).
    ///
    /// Note that the `quotient` argument does not need to be a subspace of the `sub` argument, nor
    /// do they need to be disjoint. Mathematically, this method constructs the space `(sub +
    /// quotient) / quotient`.
    pub fn from_parts(mut sub: Subspace, quotient: Subspace) -> Self {
        let dim = sub.dimension();

        sub.update_then_row_reduce(|sub_matrix| {
            for row in sub_matrix.iter_mut().take(dim) {
                quotient.reduce(row.as_slice_mut());
            }
        });

        Self {
            dimension: sub.dimension(),
            gens: sub,
            quotient,
        }
    }

    pub fn quotient_pivots(&self) -> &[isize] {
        self.quotient.pivots()
    }
}

#[cfg(feature = "proptest")]
pub mod arbitrary {
    use proptest::prelude::*;

    use super::*;
    use crate::matrix::subspace::arbitrary::SubspaceArbParams;
    pub use crate::matrix::subspace::arbitrary::MAX_DIM;

    #[derive(Debug, Clone)]
    pub struct SubquotientArbParams {
        pub p: Option<ValidPrime>,
        pub dim: BoxedStrategy<usize>,
    }

    impl Default for SubquotientArbParams {
        fn default() -> Self {
            Self {
                p: None,
                dim: (0..=MAX_DIM).boxed(),
            }
        }
    }

    impl Arbitrary for Subquotient {
        type Parameters = SubquotientArbParams;
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
            let p = match args.p {
                Some(p) => Just(p).boxed(),
                None => any::<ValidPrime>().boxed(),
            };

            (p, args.dim)
                .prop_flat_map(|(p, dim)| {
                    let sub = Subspace::arbitrary_with(SubspaceArbParams {
                        p: Some(p),
                        dim: Just(dim).boxed(),
                    });
                    let quotient = Subspace::arbitrary_with(SubspaceArbParams {
                        p: Some(p),
                        dim: Just(dim).boxed(),
                    });

                    (sub, quotient)
                })
                .prop_map(|(sub, quotient)| Self::from_parts(sub, quotient))
                .boxed()
        }
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn test_add_gen() {
        let p = ValidPrime::new(3);

        let mut sq = Subquotient::new(p, 5);
        sq.quotient(FpVector::from_slice(p, &[1, 1, 0, 0, 1]).as_slice());
        sq.quotient(FpVector::from_slice(p, &[0, 2, 0, 0, 1]).as_slice());
        sq.add_gen(FpVector::from_slice(p, &[1, 1, 0, 0, 0]).as_slice());
        sq.add_gen(FpVector::from_slice(p, &[0, 1, 0, 0, 0]).as_slice());

        expect![[r#"
            Generators:
            [0, 0, 0, 0, 1]

            Zeros:
            [1, 0, 0, 0, 2]
            [0, 1, 0, 0, 2]

        "#]]
        .assert_eq(&sq.to_string());

        expect![[r#"
            [
                2,
            ]
        "#]]
        .assert_debug_eq(&sq.reduce(FpVector::from_slice(p, &[2, 0, 0, 0, 0]).as_slice_mut()));

        assert_eq!(sq.gens().count(), 1);
    }

    proptest! {
        #[test]
        fn test_sum_quotient_gens_complement_is_ambient(sq: Subquotient) {
            let quotient_dim = sq.zeros().dimension();
            let gens_dim = sq.gens().count();
            let complement_dim = sq.complement_pivots().count();
            assert_eq!(quotient_dim + gens_dim + complement_dim, sq.ambient_dimension());
        }
    }
}
