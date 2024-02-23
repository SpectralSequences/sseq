use std::{
    io::{Read, Write},
    ops::Deref,
};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use itertools::Itertools;

use super::Matrix;
use crate::{
    prime::ValidPrime,
    vector::{FpVector, Slice, SliceMut},
};

/// A subspace of a vector space.
///
/// In general, a method is defined on the [`Subspace`] if it is a meaningful property of the
/// subspace itself. Otherwise, users can dereference the subspace to gain read-only access to the
/// underlying matrix object.
///
/// # Fields
///  * `matrix` - A matrix in reduced row echelon, whose number of columns is the dimension of the
///  ambient space and each row is a basis vector of the subspace.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct Subspace {
    matrix: Matrix,
}

// We implement `Deref` to make it easier to access the methods of the underlying matrix. Since we
// don't implement `DerefMut`, we still ensure that the matrix stays row reduced.
impl Deref for Subspace {
    type Target = Matrix;

    fn deref(&self) -> &Self::Target {
        &self.matrix
    }
}

impl Subspace {
    pub fn new(p: ValidPrime, dim: usize) -> Self {
        // We add an extra row to the matrix to allow for adding vectors to the subspace. This way,
        // even if the subspace is already the entire ambient space, we still have the space to add
        // one more vector, which will then be reduced to zero by the row reduction.
        let mut matrix = Matrix::new(p, dim + 1, dim);
        matrix.initialize_pivots();
        Self::from_matrix(matrix)
    }

    /// Create a new subspace from a matrix. The matrix does not have to be in row echelon form.
    pub fn from_matrix(mut matrix: Matrix) -> Self {
        matrix.row_reduce();
        Self { matrix }
    }

    /// Run a closure on the matrix and then ensure it is row-reduced.
    pub fn update_then_row_reduce<T, F: FnOnce(&mut Matrix) -> T>(&mut self, f: F) -> T {
        let ret = f(&mut self.matrix);
        self.matrix.row_reduce();
        ret
    }

    pub fn from_bytes(p: ValidPrime, data: &mut impl Read) -> std::io::Result<Self> {
        let rows = data.read_u64::<LittleEndian>()? as usize;
        let ambient_dimension = data.read_u64::<LittleEndian>()? as usize;

        let mut matrix = Matrix::from_bytes(p, rows, ambient_dimension, data)?;

        matrix.pivots = Matrix::read_pivot(matrix.columns(), data)?;

        Ok(Self { matrix })
    }

    pub fn to_bytes(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        buffer.write_u64::<LittleEndian>(self.matrix.rows() as u64)?;
        buffer.write_u64::<LittleEndian>(self.ambient_dimension() as u64)?;

        self.matrix.to_bytes(buffer)?;
        Matrix::write_pivot(self.pivots(), buffer)
    }

    pub fn entire_space(p: ValidPrime, dim: usize) -> Self {
        let mut result = Self::new(p, dim);
        for i in 0..dim {
            result.matrix.row_mut(i).set_entry(i, 1);
            result.matrix.pivots_mut()[i] = i as isize;
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
        self.matrix.row_mut(last_row).assign(row);
        self.matrix.row_reduce()
    }

    /// This adds some rows to the subspace
    ///
    /// # Arguments
    ///  - `rows`: A function that writes the row to be added to the given SliceMut. This returns
    ///     `None` if it runs out of rows, `Some(())` otherwise.
    pub fn add_vectors(&mut self, mut rows: impl for<'a> FnMut(SliceMut<'a>) -> Option<()>) {
        let num_rows = self.matrix.rows();
        'outer: loop {
            let first_row = self.dimension();
            if first_row == num_rows {
                return;
            }

            for i in first_row..num_rows {
                if rows(self.matrix.row_mut(i)).is_none() {
                    break 'outer;
                }
            }
            self.matrix.row_reduce();
        }
        self.matrix.row_reduce();
    }

    pub fn add_basis_elements(&mut self, mut rows: impl std::iter::Iterator<Item = usize>) {
        self.add_vectors(|mut row| {
            row.set_entry(rows.next()?, 1);
            Some(())
        });
    }

    /// Projects a vector to a complement of the subspace. The complement is the set of vectors
    /// that have a 0 in every column where there is a pivot in `matrix`
    pub fn reduce(&self, mut vector: SliceMut) {
        assert_eq!(vector.as_slice().len(), self.ambient_dimension());
        if self.matrix.rows() == 0 {
            return;
        }
        let p = self.prime();
        let iter = self
            .pivots()
            .iter()
            .enumerate()
            .filter(|(_, x)| **x >= 0)
            .map(|(col, _)| col)
            .zip(self.iter());
        for (col, row) in iter {
            let c = vector.as_slice().entry(col);
            if c != 0 {
                vector.add(row, p - c);
            }
        }
    }

    pub fn contains(&self, vector: Slice) -> bool {
        let mut vector: FpVector = vector.to_owned();
        self.reduce(vector.as_slice_mut());
        vector.is_zero()
    }

    pub fn contains_space(&self, other: &Self) -> bool {
        other.iter().all(|row| self.contains(row))
    }

    pub fn dimension(&self) -> usize {
        self.pivots()
            .iter()
            .rev()
            .find(|&&i| i >= 0)
            .map_or(0, |&i| i as usize + 1)
    }

    /// Whether the subspace is empty. This assumes the subspace is row reduced.
    pub fn is_empty(&self) -> bool {
        self.matrix.rows() == 0 || self.matrix[0].is_zero()
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
            self.matrix.row_mut(i).set_entry(i, 1);
            self.matrix.pivots_mut()[i] = i as isize;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Slice> {
        self.matrix
            .iter()
            .map(FpVector::as_slice)
            .take(self.dimension())
    }

    /// Iterate over all vectors in the subspace.
    /// ```
    /// # use fp::{prime::ValidPrime, matrix::{Matrix, Subspace}, vector::FpVector};
    /// let subspace = Subspace::from_matrix(Matrix::from_vec(
    ///     ValidPrime::new(3),
    ///     &[vec![1, 0, 0], vec![0, 1, 2]],
    /// ));
    /// let all_vectors = subspace
    ///     .iter_all_vectors()
    ///     .map(|v| (&v).into())
    ///     .collect::<Vec<Vec<u32>>>();
    ///
    /// assert_eq!(
    ///     all_vectors,
    ///     vec![
    ///         vec![0, 0, 0],
    ///         vec![0, 1, 2],
    ///         vec![0, 2, 1],
    ///         vec![1, 0, 0],
    ///         vec![1, 1, 2],
    ///         vec![1, 2, 1],
    ///         vec![2, 0, 0],
    ///         vec![2, 1, 2],
    ///         vec![2, 2, 1],
    ///     ]
    /// );
    /// ```
    pub fn iter_all_vectors(&self) -> impl Iterator<Item = FpVector> + '_ {
        crate::prime::iter::combinations(self.prime(), self.dimension()).map(|combination| {
            let mut vector = FpVector::new(self.prime(), self.ambient_dimension());
            for (i, &v) in combination.iter().enumerate() {
                vector.add(&self.matrix[i], v);
            }
            vector
        })
    }

    pub fn sum(&self, other: &Self) -> Self {
        let self_rows = self.matrix.clone().into_iter();
        let other_rows = other.matrix.clone().into_iter();
        let new_rows = self_rows.chain(other_rows).collect();

        let mut ret = Self::from_matrix(Matrix::from_rows(self.prime(), new_rows));
        ret.matrix.trim(0, self.matrix.columns() + 1, 0);
        ret
    }
}

impl std::fmt::Display for Subspace {
    /// # Example
    /// ```
    /// # use expect_test::expect;
    /// # use fp::matrix::Subspace;
    /// # use fp::prime::TWO;
    /// let subspace = Subspace::entire_space(TWO, 3);
    ///
    /// expect![[r#"
    ///     [1, 0, 0]
    ///     [0, 1, 0]
    ///     [0, 0, 1]
    /// "#]]
    /// .assert_eq(&format!("{}", subspace));
    ///
    /// assert_eq!(
    ///     "[1, 0, 0], [0, 1, 0], [0, 0, 1]",
    ///     &format!("{:#}", subspace)
    /// );
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let dim = self.dimension();
        let mut rows = self.matrix.iter().take(dim);

        let output = if f.alternate() {
            rows.join(", ")
        } else {
            rows.fold(String::new(), |mut output, row| {
                use std::fmt::Write;
                let _ = writeln!(output, "{}", row);
                output
            })
        };

        write!(f, "{output}")?;
        Ok(())
    }
}
