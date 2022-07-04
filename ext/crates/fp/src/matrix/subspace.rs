use super::Matrix;
use crate::prime::ValidPrime;
use crate::vector::{prelude::*, FpVector, Slice, SliceMut};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// A subspace of a vector space.
///
/// In general, a method is defined on the [`Subspace`] if it is a meaningful property of the
/// subspace itself. Otherwise, users are expected to access the matrix object directly. When the
/// user directly modifies the matrix, they are expected to ensure the matrix is row reduced after
/// the operations conclude.
///
/// # Fields
///  * `matrix` - A matrix in reduced row echelon, whose number of columns is the dimension of the
///  ambient space and each row is a basis vector of the subspace.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct Subspace {
    pub matrix: Matrix,
}

impl Subspace {
    pub fn new(p: ValidPrime, rows: usize, columns: usize) -> Self {
        let mut matrix = Matrix::new(p, rows, columns);
        matrix.initialize_pivots();
        Self { matrix }
    }

    pub fn prime(&self) -> ValidPrime {
        self.matrix.prime()
    }

    pub fn pivots(&self) -> &[isize] {
        self.matrix.pivots()
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

    pub fn empty_space(p: ValidPrime, dim: usize) -> Self {
        Self::new(p, 0, dim)
    }

    pub fn entire_space(p: ValidPrime, dim: usize) -> Self {
        let mut result = Self::new(p, dim, dim);
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
    pub fn reduce<T: BaseVectorMut>(&self, mut vector: T) {
        assert_eq!(vector.len(), self.ambient_dimension());
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
            let c = vector.entry(col);
            if c != 0 {
                vector.add(row, *p - c);
            }
        }
    }

    pub fn contains(&self, vector: Slice) -> bool {
        let mut vector: FpVector = vector.into_owned();
        self.reduce(vector.as_slice_mut());
        vector.is_zero()
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
}

impl std::fmt::Display for Subspace {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let dim = self.dimension();
        for row in self.matrix.iter().take(dim) {
            if f.alternate() {
                writeln!(f, "{row:#}")?;
            } else {
                writeln!(f, "{row}")?;
            }
        }
        Ok(())
    }
}
