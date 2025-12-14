use std::{fmt, io, ops::Range};

use either::Either;
use itertools::Itertools;
use maybe_rayon::prelude::*;

use super::{QuasiInverse, Subspace};
use crate::{
    field::{Field, Fp, field_internal::FieldInternal},
    limb::Limb,
    matrix::m4ri::M4riTable,
    prime::{self, ValidPrime},
    vector::{FpSlice, FpSliceMut, FpVector},
};

/// A matrix! In particular, a matrix with values in F_p.
///
/// The way we store matrices means it is easier to perform row operations than column operations,
/// and the way we use matrices means we want our matrices to act on the right. Hence we think of
/// vectors as row vectors.
#[derive(Clone)]
pub struct Matrix {
    fp: Fp<ValidPrime>,
    rows: usize,
    columns: usize,
    data: Vec<Limb>,
    stride: usize,
    /// The pivot columns of the matrix. `pivots[n]` is `k` if column `n` is the `k`th pivot
    /// column, and a negative number otherwise. Said negative number is often -1 but this is not
    /// guaranteed.
    pub(crate) pivots: Vec<isize>,
}

impl PartialEq for Matrix {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl Eq for Matrix {}

impl Matrix {
    /// Produces a new matrix over F_p with the specified number of rows and columns, initialized
    /// to the 0 matrix.
    pub fn new(p: ValidPrime, rows: usize, columns: usize) -> Self {
        Self::new_with_capacity(p, rows, columns, rows, columns)
    }

    pub fn new_with_capacity(
        p: ValidPrime,
        rows: usize,
        columns: usize,
        rows_capacity: usize,
        columns_capacity: usize,
    ) -> Self {
        let fp = Fp::new(p);
        let stride = fp.number(columns_capacity);
        let mut data = Vec::with_capacity(rows_capacity * stride);
        data.resize(rows * stride, 0);

        Self {
            fp,
            rows,
            columns,
            data,
            stride,
            pivots: Vec::new(),
        }
    }

    pub fn identity(p: ValidPrime, dim: usize) -> Self {
        let mut matrix = Self::new(p, dim, dim);
        matrix.as_slice_mut().add_identity();
        matrix
    }

    pub fn from_bytes(
        p: ValidPrime,
        rows: usize,
        columns: usize,
        buffer: &mut impl io::Read,
    ) -> io::Result<Self> {
        let fp = Fp::new(p);
        let stride = fp.number(columns);
        let mut data: Vec<Limb> = vec![0; stride * rows];
        for row_idx in 0..rows {
            let limb_range = row_to_limb_range(row_idx, stride);
            crate::limb::from_bytes(&mut data[limb_range], buffer)?;
        }
        Ok(Self {
            fp,
            rows,
            columns,
            data,
            stride,
            pivots: Vec::new(),
        })
    }

    pub fn to_bytes(&self, data: &mut impl io::Write) -> io::Result<()> {
        let limbs_per_row = self.fp.number(self.columns);
        for row_idx in 0..self.rows() {
            let row_range = row_idx * self.stride..row_idx * self.stride + limbs_per_row;
            crate::limb::to_bytes(&self.data[row_range], data)?;
        }
        Ok(())
    }

    /// Read a vector of `isize`
    pub(crate) fn write_pivot(v: &[isize], buffer: &mut impl io::Write) -> io::Result<()> {
        if cfg!(all(target_endian = "little", target_pointer_width = "64")) {
            unsafe {
                let buf: &[u8] = std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 8);
                buffer.write_all(buf).unwrap();
            }
        } else {
            use byteorder::{LittleEndian, WriteBytesExt};
            for &i in v {
                buffer.write_i64::<LittleEndian>(i as i64)?;
            }
        }
        Ok(())
    }

    /// Read a vector of `isize` of length `dim`.
    pub(crate) fn read_pivot(dim: usize, data: &mut impl io::Read) -> io::Result<Vec<isize>> {
        if cfg!(all(target_endian = "little", target_pointer_width = "64")) {
            let mut image = vec![0; dim];
            unsafe {
                let buf: &mut [u8] =
                    std::slice::from_raw_parts_mut(image.as_mut_ptr() as *mut u8, dim * 8);
                data.read_exact(buf).unwrap();
            }
            Ok(image)
        } else {
            use byteorder::{LittleEndian, ReadBytesExt};
            let mut image = Vec::with_capacity(dim);
            for _ in 0..dim {
                image.push(data.read_i64::<LittleEndian>()? as isize);
            }
            Ok(image)
        }
    }
}

impl Matrix {
    pub fn prime(&self) -> ValidPrime {
        self.fp.characteristic()
    }

    /// Gets the number of rows in the matrix.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Gets the number of columns in the matrix.
    pub fn columns(&self) -> usize {
        self.columns
    }

    /// Set the pivots to -1 in every entry. This is called by [`Matrix::row_reduce`].
    pub fn initialize_pivots(&mut self) {
        self.pivots.clear();
        self.pivots.resize(self.columns, -1);
    }

    pub fn pivots(&self) -> &[isize] {
        &self.pivots
    }

    pub fn pivots_mut(&mut self) -> &mut [isize] {
        &mut self.pivots
    }

    /// Produces a Matrix from a vector of FpVectors. We pass in the number of columns because all
    /// `0 x n` matrices will have an empty Vec, and we have to distinguish between them.
    pub fn from_rows(p: ValidPrime, input: Vec<FpVector>, columns: usize) -> Self {
        let fp = Fp::new(p);
        let rows = input.len();
        let stride = fp.number(columns);
        let mut data = Vec::with_capacity(rows * stride);
        for row in &input {
            data.extend_from_slice(row.limbs());
        }
        Self {
            fp,
            rows,
            columns,
            data,
            stride,
            pivots: Vec::new(),
        }
    }

    /// Produces a `1 x n` matrix from a single FpVector. This is a convenience function.
    pub fn from_row(p: ValidPrime, row: FpVector, columns: usize) -> Self {
        Self::from_rows(p, vec![row], columns)
    }

    /// Produces a Matrix from an `&[Vec<u32>]` object. If the number of rows is 0, the number
    /// of columns is also assumed to be zero.
    ///
    /// # Example
    /// ```
    /// # use fp::prime::ValidPrime;
    /// let p = ValidPrime::new(7);
    /// # use fp::matrix::Matrix;
    /// let input = [vec![1, 3, 6], vec![0, 3, 4]];
    ///
    /// let m = Matrix::from_vec(p, &input);
    /// ```
    pub fn from_vec(p: ValidPrime, input: &[Vec<u32>]) -> Self {
        let fp = Fp::new(p);
        let rows = input.len();
        if rows == 0 {
            return Self::new(p, 0, 0);
        }
        let columns = input[0].len();
        let stride = fp.number(columns);
        let mut data = Vec::with_capacity(rows * stride);
        for row in input {
            for chunk in row.chunks(fp.entries_per_limb()) {
                data.push(fp.pack(chunk.iter().map(|x| fp.element(*x))));
            }
        }
        Self {
            fp,
            rows,
            columns,
            data,
            stride,
            pivots: Vec::new(),
        }
    }

    /// ```
    /// # use fp::matrix::Matrix;
    /// # use fp::prime::TWO;
    ///
    /// let matrix_vec = vec![vec![1, 0], vec![0, 1]];
    ///
    /// assert_eq!(Matrix::from_vec(TWO, &matrix_vec).to_vec(), matrix_vec);
    /// ```
    pub fn to_vec(&self) -> Vec<Vec<u32>> {
        self.data
            .iter()
            .chunks(self.stride)
            .into_iter()
            .map(|row| {
                row.flat_map(|&limb| self.fp.unpack(limb).map(|x| x.val()))
                    .take(self.columns())
                    .collect()
            })
            .collect()
    }

    /// Produces a padded augmented matrix from an `&[Vec<u32>]` object (produces [A|0|I] from
    /// A). Returns the matrix and the first column index of I.
    ///
    /// # Example
    /// ```
    /// # use fp::prime::ValidPrime;
    /// let p = ValidPrime::new(7);
    /// # use fp::matrix::Matrix;
    /// # use fp::vector::FpVector;
    /// let input = [vec![1, 3, 6], vec![0, 3, 4]];
    ///
    /// let (n, m) = Matrix::augmented_from_vec(p, &input);
    /// assert!(n >= input[0].len());
    /// ```
    pub fn augmented_from_vec(p: ValidPrime, input: &[Vec<u32>]) -> (usize, Self) {
        let rows = input.len();
        let cols = input[0].len();
        let padded_cols = FpVector::padded_len(p, cols);
        let mut m = Self::new(p, rows, padded_cols + rows);

        for (i, row) in input.iter().enumerate() {
            for (j, &value) in row.iter().enumerate() {
                m.row_mut(i).set_entry(j, value);
            }
        }
        m.slice_mut(0, rows, padded_cols, padded_cols + rows)
            .add_identity();
        (padded_cols, m)
    }

    pub fn is_zero(&self) -> bool {
        self.data.iter().all(|limb| *limb == 0)
    }

    pub fn set_to_zero(&mut self) {
        for limb in self.data.iter_mut() {
            *limb = 0;
        }
    }

    pub fn assign(&mut self, other: &Self) {
        self.data = other.data.clone();
    }

    pub fn as_slice_mut(&mut self) -> MatrixSliceMut<'_> {
        self.slice_mut(0, self.rows(), 0, self.columns())
    }

    pub fn slice_mut(
        &mut self,
        row_start: usize,
        row_end: usize,
        col_start: usize,
        col_end: usize,
    ) -> MatrixSliceMut<'_> {
        let row_range = row_start..row_end;
        let limb_range = row_range_to_limb_range(&row_range, self.stride);
        MatrixSliceMut {
            fp: self.fp,
            rows: row_range.len(),
            data: &mut self.data[limb_range],
            col_start,
            col_end,
            stride: self.stride,
        }
    }

    pub fn row(&self, row: usize) -> FpSlice<'_> {
        let limb_range = row_to_limb_range(row, self.stride);
        FpSlice::new(self.prime(), &self.data[limb_range], 0, self.columns)
    }

    pub fn row_mut(&mut self, row: usize) -> FpSliceMut<'_> {
        let limb_range = row_to_limb_range(row, self.stride);
        FpSliceMut::new(self.prime(), &mut self.data[limb_range], 0, self.columns)
    }
}

impl Matrix {
    pub fn iter(&self) -> impl Iterator<Item = FpSlice<'_>> {
        (0..self.rows).map(move |row_idx| self.row(row_idx))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = FpSliceMut<'_>> {
        let p = self.prime();
        let columns = self.columns;

        if self.stride == 0 {
            Either::Left(std::iter::empty())
        } else {
            let rows = self
                .data
                .chunks_mut(self.stride)
                .map(move |row| FpSliceMut::new(p, row, 0, columns));
            Either::Right(rows)
        }
    }

    pub fn maybe_par_iter_mut(
        &mut self,
    ) -> impl MaybeIndexedParallelIterator<Item = FpSliceMut<'_>> {
        let p = self.prime();
        let columns = self.columns;

        if self.stride == 0 {
            Either::Left(maybe_rayon::empty())
        } else {
            let rows = self
                .data
                .maybe_par_chunks_mut(self.stride)
                .map(move |row| FpSliceMut::new(p, row, 0, columns));
            Either::Right(rows)
        }
    }
}

impl fmt::Display for Matrix {
    /// # Example
    /// ```
    /// # use fp::matrix::Matrix;
    /// # use fp::prime::ValidPrime;
    /// let m = Matrix::from_vec(ValidPrime::new(2), &[vec![0, 1, 0], vec![1, 1, 0]]);
    /// assert_eq!(&format!("{m}"), "[\n    [0, 1, 0],\n    [1, 1, 0]\n]");
    /// assert_eq!(&format!("{m:#}"), "010\n110");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "{:#}", self.iter().format("\n"))
        } else {
            let mut it = self.iter();
            if let Some(x) = it.next() {
                write!(f, "[\n    {x}")?;
            } else {
                return write!(f, "[]");
            }
            for x in it {
                write!(f, ",\n    {x}")?;
            }
            write!(f, "\n]")
        }
    }
}

impl fmt::Debug for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}

impl Matrix {
    /// A no-nonsense, safe, row operation. Adds `c * self[source]` to `self[target]`.
    pub fn safe_row_op(&mut self, target: usize, source: usize, c: u32) {
        assert_ne!(target, source);
        assert!(source < self.rows());
        assert!(target < self.rows());

        let (mut target, source) = unsafe { self.split_borrow(target, source) };
        target.add(source.as_slice(), c)
    }

    /// Performs a row operation using `pivot_column` as the pivot column. This assumes that the
    /// source row is zero in all columns before the pivot column.
    ///
    /// # Safety
    /// `target` and `source` must be distinct and less that `vectors.len()`
    pub unsafe fn row_op(
        &mut self,
        target: usize,
        source: usize,
        pivot_column: usize,
        prime: ValidPrime,
    ) {
        debug_assert_ne!(target, source);
        let coef = self.row(target).entry(pivot_column);
        if coef == 0 {
            return;
        }
        let (mut target, source) = unsafe { self.split_borrow(target, source) };
        target.add_offset(source.as_slice(), prime - coef, pivot_column);
    }

    /// A version of [`Matrix::row_op`] without the zero assumption.
    ///
    /// # Safety
    /// `target` and `source` must be distinct and less that `vectors.len()`
    pub unsafe fn row_op_naive(
        &mut self,
        target: usize,
        source: usize,
        pivot_column: usize,
        prime: ValidPrime,
    ) {
        debug_assert_ne!(target, source);
        let coef = self.row(target).entry(pivot_column);
        if coef == 0 {
            return;
        }
        let (mut target, source) = unsafe { self.split_borrow(target, source) };
        target.add(source.as_slice(), prime - coef);
    }

    /// Mutably borrows `x[i]` and `x[j]`.
    ///
    /// # Safety
    /// `i` and `j` must be distinct and not out of bounds.
    pub(crate) unsafe fn split_borrow(
        &mut self,
        i: usize,
        j: usize,
    ) -> (FpSliceMut<'_>, FpSliceMut<'_>) {
        let ptr = self.data.as_mut_ptr();
        let row1 = unsafe { std::slice::from_raw_parts_mut(ptr.add(i * self.stride), self.stride) };
        let row2 = unsafe { std::slice::from_raw_parts_mut(ptr.add(j * self.stride), self.stride) };
        (
            FpSliceMut::new(self.prime(), row1, 0, self.columns),
            FpSliceMut::new(self.prime(), row2, 0, self.columns),
        )
    }

    pub fn swap_rows(&mut self, i: usize, j: usize) {
        for limb_idx in 0..self.stride {
            self.data
                .swap(i * self.stride + limb_idx, j * self.stride + limb_idx);
        }
    }

    /// This is very similar to row_reduce, except we only need to get to row echelon form, not
    /// *reduced* row echelon form. It also returns the list of pivots instead.
    pub fn find_pivots_permutation<T: Iterator<Item = usize>>(
        &mut self,
        permutation: T,
    ) -> Vec<usize> {
        let p = self.prime();
        let rows = self.rows();
        let mut pivots = Vec::with_capacity(rows);

        if rows == 0 {
            return pivots;
        }

        let mut pivot: usize = 0;
        for pivot_column in permutation {
            // Search down column for a nonzero entry.
            let mut pivot_row = rows;
            for i in pivot..rows {
                if self.row(i).entry(pivot_column) != 0 {
                    pivot_row = i;
                    break;
                }
            }
            if pivot_row == rows {
                continue;
            }

            // Record position of pivot.
            pivots.push(pivot_column);

            // Pivot_row contains a row with a pivot in current column.
            // Swap pivot row up.
            self.swap_rows(pivot, pivot_row);
            // println!("({}) <==> ({}): \n{}", pivot, pivot_row, self);

            // // Divide pivot row by pivot entry
            let c = self.row(pivot).entry(pivot_column);
            let c_inv = prime::inverse(p, c);
            self.row_mut(pivot).scale(c_inv);
            // println!("({}) <== {} * ({}): \n{}", pivot, c_inv, pivot, self);

            for i in pivot_row + 1..rows {
                // Safety requires i != pivot, which follows from i > pivot_row >= pivot. They are
                // both less than rows by construction
                unsafe { self.row_op_naive(i, pivot, pivot_column, p) };
            }
            pivot += 1;
        }
        pivots
    }

    /// Perform row reduction to reduce it to reduced row echelon form. This modifies the matrix in
    /// place and records the pivots in `column_to_pivot_row`. The way the pivots are recorded is
    /// that `column_to_pivot_row[i]` is the row of the pivot if the `i`th row contains a pivot,
    /// and `-1` otherwise.
    ///
    /// # Returns
    /// The number of non-empty rows in the matrix
    ///
    /// # Arguments
    ///  * `column_to_pivot_row` - A vector for the function to write the pivots into. The length
    ///    should be at least as long as the number of columns (and the extra entries are ignored).
    ///
    /// # Example
    /// ```
    /// # use fp::prime::ValidPrime;
    /// let p = ValidPrime::new(7);
    /// # use fp::matrix::Matrix;
    ///
    /// let input = [vec![1, 3, 6], vec![0, 3, 4]];
    ///
    /// let result = [vec![1, 0, 2], vec![0, 1, 6]];
    ///
    /// let mut m = Matrix::from_vec(p, &input);
    /// m.row_reduce();
    ///
    /// assert_eq!(m, Matrix::from_vec(p, &result));
    /// ```
    pub fn row_reduce(&mut self) -> usize {
        let p = self.prime();
        self.initialize_pivots();

        let mut empty_rows = Vec::with_capacity(self.rows());

        if p == 2 {
            // the m4ri C library uses a similar formula but with a hard cap of 7 instead of 8
            let k = std::cmp::min(8, crate::prime::log2(1 + self.rows()) * 3 / 4);
            let mut table = M4riTable::new(k, self.columns());

            for i in 0..self.rows() {
                table.reduce_naive(&mut *self, i);

                if let Some((c, _)) = self.row(i).first_nonzero() {
                    self.pivots[c] = i as isize;
                    for &row in table.rows() {
                        unsafe {
                            self.row_op(row, i, c, p);
                        }
                    }
                    table.add(c, i);

                    if table.len() == k {
                        table.generate(self);
                        for j in 0..table.rows()[0] {
                            table.reduce(self.row_mut(j).limbs_mut());
                        }
                        for j in i + 1..self.rows() {
                            table.reduce(self.row_mut(j).limbs_mut());
                        }
                        table.clear();
                    }
                } else {
                    empty_rows.push(i);
                }
            }
            if !table.is_empty() {
                table.generate(self);
                for j in 0..table.rows()[0] {
                    table.reduce(self.row_mut(j).limbs_mut());
                }
                table.clear();
            }
        } else {
            for i in 0..self.rows() {
                if let Some((c, v)) = self.row(i).first_nonzero() {
                    self.pivots[c] = i as isize;
                    self.row_mut(i).scale(prime::inverse(p, v));
                    for j in 0..self.rows() {
                        if i == j {
                            continue;
                        }
                        unsafe {
                            self.row_op(j, i, c, p);
                        }
                    }
                } else {
                    empty_rows.push(i);
                }
            }
        }

        // Now reorder the vectors. There are O(n) in-place permutation algorithms but the way we
        // get the permutation makes the naive strategy easier.
        let old_len = self.data.len();
        let old_data = std::mem::replace(&mut self.data, vec![0; old_len]);

        let mut new_row_idx = 0;
        for old_row in self.pivots.iter_mut().filter(|row| **row >= 0) {
            let old_row_idx = *old_row as usize;
            let old_limb_range = row_to_limb_range(old_row_idx, self.stride);
            let new_limb_range = row_to_limb_range(new_row_idx, self.stride);
            self.data[new_limb_range].copy_from_slice(&old_data[old_limb_range]);
            *old_row = new_row_idx as isize;
            new_row_idx += 1;
        }

        new_row_idx
    }
}

impl Matrix {
    /// Given a row reduced matrix, find the first row whose pivot column is after (or at)
    /// `first_column`.
    pub fn find_first_row_in_block(&self, first_column: usize) -> usize {
        self.pivots[first_column..]
            .iter()
            .find(|&&x| x >= 0)
            .map(|x| *x as usize)
            .unwrap_or_else(|| self.rows())
    }

    /// Computes the quasi-inverse of a matrix given a rref of [A|0|I], where 0 is the zero padding
    /// as usual.
    ///
    /// # Arguments
    ///  * `last_target_col` - the last column of A
    ///  * `first_source_col` - the first column of I
    ///
    /// # Example
    /// ```
    /// # use fp::prime::ValidPrime;
    /// let p = ValidPrime::new(3);
    /// # use fp::matrix::Matrix;
    /// # use fp::vector::FpVector;
    /// let input = [
    ///     vec![1, 2, 1, 1, 0],
    ///     vec![1, 0, 2, 1, 1],
    ///     vec![2, 2, 0, 2, 1],
    /// ];
    ///
    /// let (padded_cols, mut m) = Matrix::augmented_from_vec(p, &input);
    /// m.row_reduce();
    /// let qi = m.compute_quasi_inverse(input[0].len(), padded_cols);
    ///
    /// let preimage = [vec![0, 1, 0], vec![0, 2, 2]];
    /// assert_eq!(qi.preimage(), &Matrix::from_vec(p, &preimage));
    /// ```
    pub fn compute_quasi_inverse(
        &self,
        last_target_col: usize,
        first_source_col: usize,
    ) -> QuasiInverse {
        let p = self.prime();
        let columns = self.columns();
        let source_columns = columns - first_source_col;
        let first_kernel_row = self.find_first_row_in_block(first_source_col);
        let mut preimage = Self::new(p, first_kernel_row, source_columns);
        for i in 0..first_kernel_row {
            preimage
                .row_mut(i)
                .assign(self.row(i).slice(first_source_col, columns));
        }
        QuasiInverse::new(Some(self.pivots()[..last_target_col].to_vec()), preimage)
    }

    /// Computes the quasi-inverse of a matrix given a rref of [A|0|I], where 0 is the zero padding
    /// as usual.
    ///
    /// # Arguments
    ///  * `last_target_col` - the last column of A
    ///  * `first_source_col` - the first column of I
    ///
    /// # Example
    /// ```
    /// # use fp::prime::ValidPrime;
    /// let p = ValidPrime::new(3);
    /// # use fp::matrix::Matrix;
    /// # use fp::vector::FpVector;
    /// let input = [
    ///     vec![1, 2, 1, 1, 0],
    ///     vec![1, 0, 2, 1, 1],
    ///     vec![2, 2, 0, 2, 1],
    /// ];
    ///
    /// let (padded_cols, mut m) = Matrix::augmented_from_vec(p, &input);
    /// m.row_reduce();
    ///
    /// let computed_image = m.compute_image(input[0].len(), padded_cols);
    ///
    /// let image = [vec![1, 0, 2, 1, 1], vec![0, 1, 1, 0, 1]];
    /// assert_eq!(*computed_image, Matrix::from_vec(p, &image));
    /// assert_eq!(computed_image.pivots(), &vec![0, 1, -1, -1, -1]);
    /// ```
    pub fn compute_image(&self, last_target_col: usize, first_source_col: usize) -> Subspace {
        let p = self.prime();
        let first_kernel_row = self.find_first_row_in_block(first_source_col);
        let mut image_matrix = Self::new(p, first_kernel_row, last_target_col);
        for i in 0..first_kernel_row {
            image_matrix
                .row_mut(i)
                .assign(self.row(i).slice(0, last_target_col));
        }
        image_matrix.pivots = self.pivots()[..last_target_col].to_vec();
        Subspace::from_matrix(image_matrix)
    }

    /// Computes the kernel from an augmented matrix in rref. To compute the kernel of a matrix
    /// A, produce an augmented matrix of the form
    /// ```text
    /// [A | I]
    /// ```
    /// An important thing to note is that the number of columns of `A` should be a multiple of the
    /// number of entries per limb in an FpVector, and this is often achieved by padding columns
    /// with 0. The padded length can be obtained from `FpVector::padded_dimension`.
    ///
    /// After this matrix is set up, perform row reduction with `Matrix::row_reduce`, and then
    /// apply `compute_kernel`.
    ///
    /// # Arguments
    ///  * `column_to_pivot_row` - This is the list of pivots `row_reduce` gave you.
    ///  * `first_source_column` - The column where the `I` part of the augmented matrix starts.
    ///
    /// # Example
    /// ```
    /// # use fp::prime::ValidPrime;
    /// let p = ValidPrime::new(3);
    /// # use fp::matrix::Matrix;
    /// # use fp::vector::FpVector;
    /// let input = [
    ///     vec![1, 2, 1, 1, 0],
    ///     vec![1, 0, 2, 1, 1],
    ///     vec![2, 2, 0, 2, 1],
    /// ];
    ///
    /// let (padded_cols, mut m) = Matrix::augmented_from_vec(p, &input);
    /// m.row_reduce();
    /// let ker = m.compute_kernel(padded_cols);
    ///
    /// let mut target = vec![0; 3];
    /// assert_eq!(ker.row(0).iter().collect::<Vec<u32>>(), vec![1, 1, 2]);
    /// ```
    pub fn compute_kernel(&self, first_source_column: usize) -> Subspace {
        let p = self.prime();
        let rows = self.rows();
        let columns = self.columns();
        let source_dimension = columns - first_source_column;
        let column_to_pivot_row = self.pivots();

        // Find the first kernel row
        let first_kernel_row = self.find_first_row_in_block(first_source_column);
        // Every row after the first kernel row is also a kernel row, so now we know how big it is and can allocate space.
        let kernel_dimension = rows - first_kernel_row;
        let mut kernel = Self::new(p, kernel_dimension, source_dimension);
        kernel.initialize_pivots();

        if kernel_dimension == 0 {
            return Subspace::from_matrix(kernel);
        }

        // Write pivots into kernel
        for i in 0..source_dimension {
            // Turns -1 into some negative number... make sure to check <0 for no pivot in column...
            kernel.pivots_mut()[i] =
                column_to_pivot_row[i + first_source_column] - first_kernel_row as isize;
        }
        // Copy kernel matrix into kernel
        for (i, mut row) in kernel.iter_mut().enumerate() {
            row.assign(
                self.row(first_kernel_row + i)
                    .slice(first_source_column, first_source_column + source_dimension),
            );
        }
        Subspace::from_matrix(kernel)
    }

    pub fn extend_column_dimension(&mut self, columns: usize) {
        if columns > self.columns {
            self.extend_column_capacity(columns);
            self.columns = columns;
            self.pivots.resize(columns, -1);
        }
    }

    pub fn extend_column_capacity(&mut self, columns: usize) {
        let new_stride = self.fp.number(columns);
        if new_stride > self.stride {
            self.data.resize(new_stride * self.rows, 0);
            // Shift row data backwards, starting from the end to avoid overwriting data.
            for row_idx in (0..self.rows).rev() {
                let old_row_start = row_idx * self.stride;
                let new_row_start = row_idx * new_stride;
                let new_row_zero_part =
                    row_idx * new_stride + self.stride..(row_idx + 1) * new_stride;
                // Safety: we already resized the data, and limbs are always aligned.
                unsafe {
                    std::ptr::copy(
                        &raw const self.data[old_row_start],
                        &raw mut self.data[new_row_start],
                        self.stride,
                    );
                }
                for limb in &mut self.data[new_row_zero_part] {
                    *limb = 0;
                }
            }
            self.stride = new_stride;
        }
    }

    /// Add a row to the matrix and return a mutable reference to it.
    pub fn add_row(&mut self) -> FpSliceMut<'_> {
        self.data.resize((self.rows + 1) * self.stride, 0);
        self.rows += 1;
        self.row_mut(self.rows - 1)
    }

    /// Given a matrix M in rref, add rows to make the matrix surjective when restricted to the
    /// columns between `start_column` and `end_column`. That is, if M = [*|B|*] where B is between
    /// columns `start_column` and `end_column`, we want the new B to be surjective. This doesn't
    /// change the size of the matrix. Rather, it adds the new row to the next empty row in the
    /// matrix. This will panic if there are not enough empty rows.
    ///
    /// The rows added are all zero except in a single column, where it is 1. The function returns
    /// the list of such columns.
    ///
    /// # Arguments
    ///  * `first_empty_row` - The first row in the matrix that is empty. This is where we will add
    ///    our new rows. This is a mutable borrow and by the end of the function, `first_empty_row`
    ///    will be updated to the new first empty row.
    ///  * `current_pivots` - The current pivots of the matrix.
    ///
    /// # Panics
    /// The function panics if there are not enough empty rows.
    pub fn extend_to_surjection(
        &mut self,
        start_column: usize,
        end_column: usize,
        extra_column_capacity: usize,
    ) -> Vec<usize> {
        let mut added_pivots = Vec::new();
        self.extend_column_capacity(self.columns + extra_column_capacity);

        for (i, &pivot) in self.pivots.clone()[start_column..end_column]
            .iter()
            .enumerate()
        {
            if pivot >= 0 {
                continue;
            }
            let mut new_row = self.add_row();
            new_row.set_entry(i, 1);
            added_pivots.push(i);
        }
        added_pivots
    }

    /// Given a matrix in rref, say [A|B|C], where B lies between columns `start_column` and
    /// `end_columns`, and a superspace of the image of B, add rows to the matrix such that the
    /// image of B becomes this superspace.
    ///
    /// The rows added are basis vectors of the desired image as specified in the Subspace object.
    /// The function returns the list of new pivot columns.
    ///
    /// # Panics
    /// It *may* panic if the current image is not contained in `desired_image`, but is not
    /// guaranteed to do so.
    pub fn extend_image(
        &mut self,
        start_column: usize,
        end_column: usize,
        desired_image: &Subspace,
        extra_column_capacity: usize,
    ) -> Vec<usize> {
        let mut added_pivots = Vec::new();
        let desired_pivots = desired_image.pivots();
        let early_end_column = std::cmp::min(end_column, desired_pivots.len() + start_column);

        self.extend_column_capacity(self.columns + extra_column_capacity);

        for i in start_column..early_end_column {
            debug_assert!(
                self.pivots()[i] < 0 || desired_pivots[i - start_column] >= 0,
                "current_pivots : {:?}, desired_pivots : {:?}",
                self.pivots(),
                desired_pivots
            );
            if self.pivots()[i] >= 0 || desired_pivots[i - start_column] < 0 {
                continue;
            }
            // Look up the cycle that we're missing and add a generator hitting it.
            let kernel_vector_row = desired_pivots[i - start_column] as usize;
            let new_image = desired_image.row(kernel_vector_row);

            let mut new_row = self.add_row();
            new_row
                .slice_mut(
                    start_column,
                    start_column + desired_image.ambient_dimension(),
                )
                .assign(new_image);

            added_pivots.push(i);
        }
        added_pivots
    }

    /// Applies a matrix to a vector.
    ///
    /// # Example
    /// ```
    /// # use fp::prime::ValidPrime;
    /// let p = ValidPrime::new(7);
    /// # use fp::matrix::Matrix;
    /// # use fp::vector::FpVector;
    /// let input = [vec![1, 3, 6], vec![0, 3, 4]];
    ///
    /// let m = Matrix::from_vec(p, &input);
    /// let v = FpVector::from_slice(p, &vec![3, 1]);
    /// let mut result = FpVector::new(p, 3);
    /// let desired_result = FpVector::from_slice(p, &vec![3, 5, 1]);
    /// m.apply(result.as_slice_mut(), 1, v.as_slice());
    /// assert_eq!(result, desired_result);
    /// ```
    pub fn apply(&self, mut result: FpSliceMut, coeff: u32, input: FpSlice) {
        debug_assert_eq!(input.len(), self.rows());
        for i in 0..input.len() {
            result.add(self.row(i), (coeff * input.entry(i)) % self.prime());
        }
    }

    pub fn trim(&mut self, row_start: usize, row_end: usize, col_start: usize) {
        self.rows = row_end - row_start;
        self.data.truncate(row_end * self.stride);
        self.data.drain(0..row_start * self.stride);
        for mut row in self.iter_mut() {
            row.shl_assign(col_start);
        }
        self.columns -= col_start;
    }

    /// Rotate the rows downwards in the range `range`.
    pub fn rotate_down(&mut self, range: Range<usize>, shift: usize) {
        let limb_range = row_range_to_limb_range(&range, self.stride);
        self.data[limb_range].rotate_right(shift * self.stride)
    }
}

impl std::ops::Mul for &Matrix {
    type Output = Matrix;

    fn mul(self, rhs: Self) -> Matrix {
        assert_eq!(self.prime(), rhs.prime());
        assert_eq!(self.columns(), rhs.rows());

        let mut result = Matrix::new(self.prime(), self.rows(), rhs.columns());
        for i in 0..self.rows() {
            for j in 0..rhs.columns() {
                for k in 0..self.columns() {
                    result
                        .row_mut(i)
                        .add_basis_element(j, self.row(i).entry(k) * rhs.row(k).entry(j));
                }
            }
        }
        result
    }
}

impl std::ops::MulAssign<u32> for Matrix {
    fn mul_assign(&mut self, rhs: u32) {
        #[allow(clippy::suspicious_op_assign_impl)]
        let rhs = rhs % self.prime();
        for mut row in self.iter_mut() {
            row.scale(rhs);
        }
    }
}

impl std::ops::AddAssign<&Self> for Matrix {
    fn add_assign(&mut self, rhs: &Self) {
        assert_eq!(self.prime(), rhs.prime());
        assert_eq!(self.columns(), rhs.columns());
        assert_eq!(self.rows(), rhs.rows());

        for (i, mut row) in self.iter_mut().enumerate() {
            row.add(rhs.row(i), 1);
        }
    }
}

#[cfg(feature = "proptest")]
pub mod arbitrary {
    use proptest::prelude::*;

    use super::*;
    use crate::{
        field::Fp,
        vector::{FqVector, arbitrary::FqVectorArbParams},
    };

    pub const MAX_ROWS: usize = 100;
    pub const MAX_COLUMNS: usize = 100;

    #[derive(Debug, Clone)]
    pub struct MatrixArbParams {
        pub p: Option<ValidPrime>,
        pub rows: BoxedStrategy<usize>,
        pub columns: BoxedStrategy<usize>,
    }

    impl Default for MatrixArbParams {
        fn default() -> Self {
            Self {
                p: None,
                rows: (1..=MAX_ROWS).boxed(),
                columns: (1..=MAX_COLUMNS).boxed(),
            }
        }
    }

    impl Arbitrary for Matrix {
        type Parameters = MatrixArbParams;
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
            let p = match args.p {
                Some(p) => Just(p).boxed(),
                None => any::<ValidPrime>().boxed(),
            };

            (p, args.rows, args.columns)
                .prop_flat_map(|(p, rows, columns)| {
                    let row_strategy = any_with::<FqVector<_>>(FqVectorArbParams {
                        fq: Some(Fp::new(p)),
                        len: Just(columns).boxed(),
                    })
                    .prop_map(|v| -> FpVector { v.into() });

                    let rows = proptest::collection::vec(row_strategy, rows);
                    (Just(p), rows, Just(columns))
                })
                .prop_map(|(p, rows, columns)| Self::from_rows(p, rows, columns))
                .boxed()
        }
    }

    impl Matrix {
        /// Generate an arbitrary row-reduced matrix.
        ///
        /// This is more interesting than just generating an arbitrary matrix and row-reducing. If
        /// we pick a matrix uniformly at random in the space of all $n \times m$ matrices, it has a
        /// very high probability of having full rank with all its pivots in the first $n$ columns.
        /// This implies that, after projecting to the space of row-reduced matrices, the output is
        /// very likely to be an identity matrix augmented by a random matrix. If $m$ is
        /// significantly larger than $n$, this is only a tiny subspace of the space of all
        /// row-reduced matrices.
        ///
        /// While a search through *all* $n \times m$ matrices will also cover all row-reduced
        /// matrices, in practice this space is so large that we only test a vanishingly small
        /// fraction of it. Therefore, if a method that is sensitive to the pivot structure of the
        /// input matrix is proptested using `arbitrary_with`, it is unlikely that the tests will
        /// cover many matrices with interesting pivots, while those are the most likely to cause
        /// bugs. This function attempts to generate a matrix that is chosen uniformly at random
        /// directly in the space of all row-reduced matrices.
        ///
        /// In practice, this is not quite right. There is no randomness in the code; instead we
        /// generate a `Strategy` that samples from only the space of row-reduced matrices. Also,
        /// depending on the parameters, the strategy may output matrices that are not all of the
        /// same size or even over the same ground field, so using the word "space" is slightly
        /// improper, mathematically speaking.
        pub fn arbitrary_rref_with(args: MatrixArbParams) -> impl Strategy<Value = Self> {
            Self::arbitrary_with(args)
                .prop_flat_map(|m| {
                    let column_vec = (0..m.columns()).collect::<Vec<_>>();
                    let smallest_dim = std::cmp::min(m.rows(), m.columns());
                    let pivot_cols = proptest::sample::subsequence(column_vec, 0..=smallest_dim);
                    (Just(m), pivot_cols)
                })
                .prop_map(|(mut m, pivot_cols)| {
                    // Ensure rows start with 0s followed by a 1 in their pivot column
                    for (row_idx, mut row) in m.iter_mut().enumerate() {
                        if let Some(&col_idx) = pivot_cols.get(row_idx) {
                            row.slice_mut(0, col_idx).set_to_zero();
                            row.set_entry(col_idx, 1);
                        } else {
                            row.set_to_zero();
                        }
                    }
                    // Set all other entries in the pivot columns to 0
                    for (row_idx, &col_idx) in pivot_cols.iter().enumerate() {
                        for mut row in m.iter_mut().take(row_idx) {
                            row.set_entry(col_idx, 0);
                        }
                    }
                    m
                })
        }

        pub fn arbitrary_rref() -> impl Strategy<Value = Self> {
            Self::arbitrary_rref_with(MatrixArbParams::default())
        }
    }
}

fn row_to_limb_range(row: usize, stride: usize) -> Range<usize> {
    row_range_to_limb_range(&(row..row + 1), stride)
}

fn row_range_to_limb_range(row_range: &Range<usize>, stride: usize) -> Range<usize> {
    row_range.start * stride..row_range.end * stride
}

/// This models an augmented matrix.
///
/// In an ideal world, this will have no public fields. The inner matrix
/// can be accessed via deref, and there are functions that expose `end`
/// and `start`. However, in the real world, the borrow checker exists, and there are
/// cases where directly accessing these fields is what it takes to let you pass the
/// borrow checker.
///
/// In particular, if `m` is an augmented matrix and `f` is a function
/// that takes in `&mut Matrix`, trying to run `m.f(m.start[0])` produces an error
/// because it is not clear if we first do the `deref_mut` then retrieve `start[0]`.
/// (since `deref_mut` takes in a mutable borrow, it could in theory modify `m`
/// non-trivially)
#[derive(Clone)]
pub struct AugmentedMatrix<const N: usize> {
    pub end: [usize; N],
    pub start: [usize; N],
    pub inner: Matrix,
}

impl<const N: usize> AugmentedMatrix<N> {
    pub fn new(p: ValidPrime, rows: usize, columns: [usize; N]) -> Self {
        let mut start = [0; N];
        let mut end = [0; N];
        for i in 1..N {
            start[i] = start[i - 1] + FpVector::padded_len(p, columns[i - 1]);
        }
        for i in 0..N {
            end[i] = start[i] + columns[i];
        }

        Self {
            inner: Matrix::new(p, rows, end[N - 1]),
            start,
            end,
        }
    }

    pub fn new_with_capacity(
        p: ValidPrime,
        rows: usize,
        columns: &[usize],
        row_capacity: usize,
        extra_column_capacity: usize,
    ) -> Self {
        let mut start = [0; N];
        let mut end = [0; N];
        for i in 1..N {
            start[i] = start[i - 1] + FpVector::padded_len(p, columns[i - 1]);
        }
        for i in 0..N {
            end[i] = start[i] + columns[i];
        }

        Self {
            inner: Matrix::new_with_capacity(
                p,
                rows,
                end[N - 1],
                row_capacity,
                end[N - 1] + extra_column_capacity,
            ),
            start,
            end,
        }
    }

    pub fn segment(&mut self, start: usize, end: usize) -> MatrixSliceMut<'_> {
        let rows = self.inner.rows();
        let start = self.start[start];
        let end = self.end[end];
        self.slice_mut(0, rows, start, end)
    }

    pub fn row_segment_mut(&mut self, i: usize, start: usize, end: usize) -> FpSliceMut<'_> {
        let start_idx = self.start[start];
        let end_idx = self.end[end];
        let limb_range = row_to_limb_range(i, self.stride);
        FpSliceMut::new(self.prime(), &mut self.data[limb_range], start_idx, end_idx)
    }

    pub fn row_segment(&self, i: usize, start: usize, end: usize) -> FpSlice<'_> {
        let start_idx = self.start[start];
        let end_idx = self.end[end];
        self.row(i).slice(start_idx, end_idx)
    }

    pub fn into_matrix(self) -> Matrix {
        self.inner
    }

    pub fn into_tail_segment(
        mut self,
        row_start: usize,
        row_end: usize,
        segment_start: usize,
    ) -> Matrix {
        self.inner
            .trim(row_start, row_end, self.start[segment_start]);
        self.inner
    }

    pub fn compute_kernel(&self) -> Subspace {
        self.inner.compute_kernel(self.start[N - 1])
    }

    pub fn extend_column_dimension(&mut self, columns: usize) {
        if columns > self.columns {
            self.end[N - 1] += columns - self.columns;
            self.inner.extend_column_dimension(columns);
        }
    }
}

impl<const N: usize> std::ops::Deref for AugmentedMatrix<N> {
    type Target = Matrix;

    fn deref(&self) -> &Matrix {
        &self.inner
    }
}

impl<const N: usize> std::ops::DerefMut for AugmentedMatrix<N> {
    fn deref_mut(&mut self) -> &mut Matrix {
        &mut self.inner
    }
}

impl AugmentedMatrix<2> {
    pub fn compute_image(&self) -> Subspace {
        self.inner.compute_image(self.end[0], self.start[1])
    }

    pub fn compute_quasi_inverse(&self) -> QuasiInverse {
        self.inner.compute_quasi_inverse(self.end[0], self.start[1])
    }
}

impl AugmentedMatrix<3> {
    pub fn drop_first(mut self) -> AugmentedMatrix<2> {
        let offset = self.start[1];
        for mut row in self.inner.iter_mut() {
            row.shl_assign(offset);
        }
        self.inner.columns -= offset;
        AugmentedMatrix::<2> {
            inner: self.inner,
            start: [self.start[1] - offset, self.start[2] - offset],
            end: [self.end[1] - offset, self.end[2] - offset],
        }
    }

    /// This function computes quasi-inverses for matrices A, B given a reduced row echelon form of
    /// [A|0|B|0|I] such that A is surjective. Moreover, if Q is the quasi-inverse of A, it is
    /// guaranteed that the image of QB and B|_{ker A} are disjoint.
    ///
    /// This takes ownership of the matrix since it heavily modifies the matrix. This is not
    /// strictly necessary but is fine in most applications.
    pub fn compute_quasi_inverses(mut self) -> (QuasiInverse, QuasiInverse) {
        let p = self.prime();
        let stride = self.stride;

        let source_columns = self.end[2] - self.start[2];

        if self.end[0] == 0 {
            let cc_qi = QuasiInverse::new(None, Matrix::new(p, 0, source_columns));
            let res_qi = Matrix::compute_quasi_inverse(&self, self.end[1], self.start[2]);
            (cc_qi, res_qi)
        } else {
            let mut cc_preimage = Matrix::new(p, self.end[0], source_columns);
            for i in 0..self.end[0] {
                cc_preimage
                    .row_mut(i)
                    .assign(self.row(i).slice(self.start[2], self.end[2]));
            }
            let cm_qi = QuasiInverse::new(None, cc_preimage);

            let first_kernel_row = self.find_first_row_in_block(self.start[2]);
            self.rows = first_kernel_row;
            self.data.truncate(first_kernel_row * stride);

            let mut res_matrix = self.drop_first();
            res_matrix.row_reduce();
            let res_qi = res_matrix.compute_quasi_inverse();

            (cm_qi, res_qi)
        }
    }
}

pub struct MatrixSliceMut<'a> {
    fp: Fp<ValidPrime>,
    data: &'a mut [Limb],
    rows: usize,
    col_start: usize,
    col_end: usize,
    stride: usize,
}

impl<'a> MatrixSliceMut<'a> {
    pub fn prime(&self) -> ValidPrime {
        self.fp.characteristic()
    }

    pub fn columns(&self) -> usize {
        self.col_end - self.col_start
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn row_slice<'b: 'a>(&'b mut self, row_start: usize, row_end: usize) -> MatrixSliceMut<'b> {
        let limb_range = row_range_to_limb_range(&(row_start..row_end), self.stride);
        Self {
            fp: self.fp,
            data: &mut self.data[limb_range],
            rows: row_end - row_start,
            col_start: self.col_start,
            col_end: self.col_end,
            stride: self.stride,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = FpSlice<'_>> + '_ {
        let start = self.col_start;
        let end = self.col_end;
        (0..self.rows).map(move |row_idx| {
            let limb_range = row_to_limb_range(row_idx, self.stride);
            FpSlice::new(self.prime(), &self.data[limb_range], start, end)
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = FpSliceMut<'_>> + '_ {
        let p = self.prime();
        let start = self.col_start;
        let end = self.col_end;

        if self.stride == 0 {
            Either::Left(std::iter::empty())
        } else {
            let rows = self
                .data
                .chunks_mut(self.stride)
                .map(move |row| FpSliceMut::new(p, row, start, end));
            Either::Right(rows)
        }
    }

    pub fn maybe_par_iter_mut(
        &mut self,
    ) -> impl MaybeIndexedParallelIterator<Item = FpSliceMut<'_>> + '_ {
        let p = self.prime();
        let start = self.col_start;
        let end = self.col_end;

        if self.stride == 0 {
            Either::Left(maybe_rayon::empty())
        } else {
            let rows = self
                .data
                .maybe_par_chunks_mut(self.stride)
                .map(move |row| FpSliceMut::new(p, row, start, end));
            Either::Right(rows)
        }
    }

    pub fn row(&mut self, row: usize) -> FpSlice<'_> {
        let limb_range = row_to_limb_range(row, self.stride);
        FpSlice::new(
            self.prime(),
            &self.data[limb_range],
            self.col_start,
            self.col_end,
        )
    }

    pub fn row_mut(&mut self, row: usize) -> FpSliceMut<'_> {
        let limb_range = row_to_limb_range(row, self.stride);
        FpSliceMut::new(
            self.prime(),
            &mut self.data[limb_range],
            self.col_start,
            self.col_end,
        )
    }

    pub fn add_identity(&mut self) {
        debug_assert_eq!(self.rows(), self.columns());
        for row_idx in 0..self.rows {
            self.row_mut(row_idx).add_basis_element(row_idx, 1);
        }
    }

    /// For each row, add the `v[i]`th entry of `other` to `self`.
    pub fn add_masked(&mut self, other: &Matrix, mask: &[usize]) {
        assert_eq!(self.rows(), other.rows());

        for (mut l, r) in self.iter_mut().zip(other.iter()) {
            l.add_masked(r, 1, mask);
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn test_augmented_matrix() {
        test_augmented_matrix_inner([1, 0, 5]);
        test_augmented_matrix_inner([4, 6, 2]);
        test_augmented_matrix_inner([129, 4, 64]);
        test_augmented_matrix_inner([64, 64, 102]);
    }

    fn test_augmented_matrix_inner(cols: [usize; 3]) {
        let mut aug = AugmentedMatrix::<3>::new(ValidPrime::new(2), 3, cols);
        assert_eq!(aug.segment(0, 0).columns(), cols[0]);
        assert_eq!(aug.segment(1, 1).columns(), cols[1]);
        assert_eq!(aug.segment(2, 2).columns(), cols[2]);
    }

    #[test]
    fn test_row_reduce_2() {
        let p = ValidPrime::new(2);
        let tests = [(
            [
                vec![0, 1, 1, 0, 1, 1, 0, 1, 0, 0, 0, 1, 0, 1, 1],
                vec![0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0],
                vec![0, 0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1, 0, 1, 1],
                vec![1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0],
                vec![1, 1, 0, 0, 1, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0],
                vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1],
                vec![0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 1, 1, 1, 1],
            ],
            [
                [1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 0, 1, 1],
                [0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1, 1],
                [0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0],
                [0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0],
                [0, 0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 1, 0, 0, 1],
                [0, 0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1, 0, 1, 1],
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1],
            ],
            [0, 1, 2, 3, 4, 5, -1, -1, -1, -1, -1, -1, 6, -1, -1],
        )];
        for test in &tests {
            let input = &test.0;
            let goal_output = test.1;
            let goal_pivots = test.2;

            let mut m = Matrix::from_vec(p, input);
            println!("{m}");
            m.row_reduce();
            for (i, goal_row) in goal_output.iter().enumerate() {
                assert_eq!(m.row(i).iter().collect::<Vec<_>>(), *goal_row);
            }
            assert_eq!(m.pivots(), &goal_pivots)
        }
    }

    proptest! {
        // Test that `arbitrary_rref` generates matrices in rref.
        #[test]
        fn test_arbitrary_rref(m in Matrix::arbitrary_rref()) {
            let mut m_red = m.clone();
            m_red.row_reduce();
            prop_assert_eq!(m, m_red);
        }
    }
}
