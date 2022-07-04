use super::Subspace;
use crate::matrix::Matrix;
use crate::prime::ValidPrime;
use crate::vector::{prelude::*, FpVector, Slice, SliceMut};

#[derive(Clone)]
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
            gens: Subspace::new(p, dim + 1, dim),
            quotient: Subspace::new(p, dim + 1, dim),
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
    pub fn reduce(&self, mut elt: SliceMut) -> Vec<u32> {
        self.quotient.reduce(elt.copy());
        let mut result = Vec::with_capacity(self.gens.ambient_dimension());
        for i in 0..self.gens.ambient_dimension() {
            if self.gens.pivots()[i] < 0 {
                continue;
            }
            let c = elt.entry(i);
            result.push(c);
            if c != 0 {
                elt.add(
                    self.gens.matrix.row(self.gens.pivots()[i] as usize),
                    ((*elt.prime() - 1) * c) % *elt.prime(),
                );
            }
        }
        result
    }

    /// Project the vector onto the complement of the quotient part of the subquotient.
    pub fn reduce_by_quotient(&self, elt: SliceMut) {
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

    pub fn gens(&self) -> impl Iterator<Item = Slice> {
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
    pub fn subspace_gens(&self) -> impl Iterator<Item = Slice> {
        self.gens().chain(self.quotient.iter())
    }

    /// The pivot columns of the complement to the subspace
    pub fn complement_pivots(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.ambient_dimension()).filter(|&i| {
            !self.quotient.pivots().contains(&(i as isize))
                && !self.gens.pivots().contains(&(i as isize))
        })
    }

    pub fn quotient(&mut self, elt: Slice) {
        self.quotient.add_vector(elt);
        for elt in self.gens.matrix.iter_mut().take(self.dimension) {
            self.quotient.reduce(elt.as_slice_mut());
        }
        self.gens.matrix.row_reduce();
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

    pub fn add_gen(&mut self, gen: Slice) {
        let mut new_row = self.gens.matrix.row_mut(self.dimension);
        new_row.assign(gen);
        self.quotient.reduce(new_row);
        self.gens.matrix.row_reduce();
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

    /// Given a chain of subspaces `subspace` < `space` < k^`ambient_dimension`, compute the
    /// subquotient `space`/`subspace`. The answer is expressed as a list of basis vectors of
    /// `space` whose image in `space`/`subspace` forms a basis, and a basis vector of `space` is
    /// described by its index in the list of basis vectors of `space` (not the ambient space).
    ///
    /// # Arguments
    ///  * `space` - If this is None, it is the whole space k^`ambient_dimension`
    ///  * `subspace` - If this is None, it is empty
    pub fn from_parts(mut sub: Subspace, quotient: Subspace) -> Self {
        let dim = sub.dimension();
        for row in sub.matrix.iter_mut().take(dim) {
            quotient.reduce(row.as_slice_mut());
        }
        sub.matrix.row_reduce();
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

#[cfg(feature = "odd-primes")]
#[cfg(test)]
mod test {
    use super::*;
    use expect_test::expect;

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
}
