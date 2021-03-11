use super::Subspace;
use crate::matrix::Matrix;
use crate::prime::ValidPrime;
use crate::vector::{FpVector, FpVectorT};

#[derive(Clone)]
pub struct Subquotient {
    gens: Subspace,
    quotient: Subspace,
    dimension: usize,
}

impl std::fmt::Display for Subquotient {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
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
    pub fn reduce(&self, elt: &mut FpVector) -> Vec<u32> {
        self.quotient.reduce(elt);
        let mut result = Vec::with_capacity(self.gens.columns());
        for i in 0..self.gens.columns() {
            if self.gens.pivots()[i] < 0 {
                continue;
            }
            let c = elt.entry(i);
            result.push(c);
            if c != 0 {
                elt.add(
                    &self.gens[self.gens.pivots()[i] as usize],
                    ((*elt.prime() - 1) * c) % *elt.prime(),
                );
            }
        }
        result
    }

    pub fn set_to_zero(&mut self) {
        self.quotient.set_to_zero();
        self.gens.set_to_zero();
    }

    pub fn zeros(&self) -> &Subspace {
        &self.quotient
    }

    pub fn gens(&self) -> impl Iterator<Item = &FpVector> {
        self.gens.iter().take(self.dimension)
    }

    pub fn quotient(&mut self, elt: &FpVector) {
        self.quotient.add_vector(elt);
        for elt in self.gens.iter_mut() {
            self.quotient.reduce(elt);
        }
        self.gens.row_reduce();
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

    pub fn add_gen(&mut self, gen: &FpVector) {
        let new_row = &mut self.gens[self.dimension];
        new_row.assign(gen);
        self.quotient.reduce(new_row);
        self.gens.row_reduce();
        self.dimension = self.gens.dimension();
    }

    pub fn reduce_matrix(matrix: &Matrix, source: &Self, target: &Self) -> Vec<Vec<u32>> {
        let mut result = Vec::with_capacity(source.dimension());
        let mut temp = FpVector::new(source.prime(), target.ambient_dimension());
        for v in source.gens() {
            matrix.apply(&mut temp, 1, v);
            result.push(target.reduce(&mut temp));
            temp.set_to_zero_pure()
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
    pub fn subquotient(
        space: Option<&Subspace>,
        subspace: Option<&Subspace>,
        ambient_dimension: usize,
    ) -> Vec<usize> {
        match subspace {
            None => {
                if let Some(sp) = space {
                    sp.pivots()
                        .iter()
                        .filter(|i| **i >= 0)
                        .map(|i| *i as usize)
                        .collect()
                } else {
                    (0..ambient_dimension).collect()
                }
            }
            Some(subsp) => {
                if let Some(sp) = space {
                    sp.pivots()
                        .iter()
                        .zip(subsp.pivots().iter())
                        .filter(|(x, y)| {
                            debug_assert!(**x >= 0 || **y < 0);
                            **x >= 0 && **y < 0
                        })
                        .map(|(x, _)| *x as usize)
                        .collect()
                } else {
                    (0..ambient_dimension)
                        .filter(|i| subsp.pivots()[*i] < 0)
                        .collect()
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use expect_test::expect;

    #[test]
    fn test_add_gen() {
        let p = ValidPrime::new(3);
        crate::vector::initialize_limb_bit_index_table(p);

        let mut sq = Subquotient::new(p, 5);
        sq.quotient(&FpVector::from_vec(p, &[1, 1, 0, 0, 1]));
        sq.quotient(&FpVector::from_vec(p, &[0, 2, 0, 0, 1]));
        sq.add_gen(&FpVector::from_vec(p, &[1, 1, 0, 0, 0]));
        sq.add_gen(&FpVector::from_vec(p, &[0, 1, 0, 0, 0]));

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
        .assert_debug_eq(&sq.reduce(&mut FpVector::from_vec(p, &[2, 0, 0, 0, 0])));

        assert_eq!(sq.gens().count(), 1);
    }
}
