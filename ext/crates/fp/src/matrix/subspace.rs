use super::Matrix;
use crate::prime::ValidPrime;
use crate::vector::{FpVector, Slice, SliceMut};

/// A subspace of a vector space.
/// # Fields
///  * `matrix` - A matrix in reduced row echelon, whose number of columns is the dimension of the
///  ambient space and each row is a basis vector of the subspace.
///  * `pivots` - If the column is a pivot column, the entry is the row the pivot
///  corresponds to. If the column is not a pivot column, this is some negative number &mdash; not
///  necessarily -1!
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct Subspace {
    pub matrix: Matrix,
}

impl std::ops::Deref for Subspace {
    type Target = Matrix;

    fn deref(&self) -> &Matrix {
        &self.matrix
    }
}

impl std::ops::DerefMut for Subspace {
    fn deref_mut(&mut self) -> &mut Matrix {
        &mut self.matrix
    }
}

impl Subspace {
    pub fn new(p: ValidPrime, rows: usize, columns: usize) -> Self {
        let mut matrix = Matrix::new(p, rows, columns);
        matrix.initialize_pivots();
        Self { matrix }
    }

    pub fn empty_space(p: ValidPrime, dim: usize) -> Self {
        Self::new(p, 0, dim)
    }

    pub fn entire_space(p: ValidPrime, dim: usize) -> Self {
        let mut result = Self::new(p, dim, dim);
        for i in 0..dim {
            result[i].set_entry(i, 1);
            result.pivots_mut()[i] = i as isize;
        }
        result
    }

    /// This adds a vector to the subspace. This function assumes that the last row of the
    /// matrix is zero, i.e. the dimension of the current subspace is strictly less than the number
    /// of rows. This can be achieved by setting the number of rows to be the dimension plus one
    /// when creating the subspace.
    ///
    /// # Returns
    /// The new dimension of the subspace
    pub fn add_vector(&mut self, row: Slice) -> usize {
        let last_row = self.matrix.rows() - 1;
        self[last_row].as_slice_mut().assign(row);
        self.row_reduce()
    }

    pub fn add_vectors(&mut self, mut rows: impl std::iter::Iterator<Item = FpVector>) {
        let num_rows = self.matrix.rows();
        'outer: loop {
            let mut first_row = num_rows;
            for i in 0..num_rows {
                if self[i].is_zero() {
                    first_row = i;
                    break;
                }
            }
            if first_row == num_rows {
                return;
            }

            for i in first_row..num_rows {
                if let Some(v) = rows.next() {
                    assert_eq!(v.len(), self.matrix.columns());
                    self[i] = v;
                } else {
                    break 'outer;
                }
            }
            self.row_reduce();
        }
        self.row_reduce();
    }

    pub fn add_basis_elements(&mut self, mut rows: impl std::iter::Iterator<Item = usize>) {
        let num_rows = self.matrix.rows();
        'outer: loop {
            let mut first_row = num_rows;
            for i in 0..num_rows {
                if self[i].is_zero() {
                    first_row = i;
                    break;
                }
            }
            if first_row == num_rows {
                return;
            }

            for i in first_row..num_rows {
                if let Some(v) = rows.next() {
                    self[i].set_entry(v, 1);
                } else {
                    break 'outer;
                }
            }
            self.row_reduce();
        }
        self.row_reduce();
    }

    /// Projects a vector to a complement of the subspace. The complement is the set of vectors
    /// that have a 0 in every column where there is a pivot in `matrix`
    pub fn reduce(&self, mut vector: SliceMut) {
        assert_eq!(vector.as_slice().len(), self.columns());
        if self.rows() == 0 {
            return;
        }
        let p = self.prime();
        let iter = self
            .pivots()
            .iter()
            .enumerate()
            .filter(|(_, x)| **x >= 0)
            .map(|(i, _)| i)
            .enumerate();
        for (row, i) in iter {
            let c = vector.as_slice().entry(i);
            if c != 0 {
                vector.add(self[row].as_slice(), *p - c);
            }
        }
    }

    pub fn row_reduce(&mut self) -> usize {
        self.matrix.row_reduce()
    }

    pub fn contains(&self, vector: Slice) -> bool {
        let mut vector: FpVector = vector.to_owned();
        self.reduce(vector.as_slice_mut());
        vector.is_zero()
    }

    pub fn dimension(&self) -> usize {
        self.matrix
            .pivots()
            .iter()
            .rev()
            .find(|&&i| i >= 0)
            .map(|&i| i as usize + 1)
            .unwrap_or(0)
    }

    pub fn ambient_dimension(&self) -> usize {
        self.matrix.columns()
    }

    /// Returns a basis of the subspace.
    pub fn basis(&self) -> &[FpVector] {
        &self.matrix[..self.dimension()]
    }

    /// Sets the subspace to be the zero subspace.
    pub fn set_to_zero(&mut self) {
        self.matrix.set_to_zero();
        for x in self.matrix.pivots_mut() {
            *x = -1;
        }
    }

    /// Sets the subspace to be the entire subspace.
    pub fn set_to_entire(&mut self) {
        self.matrix.set_to_zero();
        for i in 0..self.matrix.columns() {
            self[i].set_entry(i, 1);
            self.pivots_mut()[i] = i as isize;
        }
    }
}

impl std::fmt::Display for Subspace {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let dim = self.dimension();
        for row in self.matrix.iter().take(dim) {
            writeln!(f, "{}", row)?;
        }
        Ok(())
    }
}
