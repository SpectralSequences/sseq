use itertools::Itertools;
use crate::prime::{self, ValidPrime};
use crate::vector::{FpVector, FpVectorT, FpVectorSlice};
use super::{
    Subspace,
    QuasiInverse
};

use std::fmt;

/// A matrix! In particular, a matrix with values in F_p. The way we store matrices means it is
/// easier to perform row operations than column operations, and the way we use matrices means we
/// want our matrices to act on the right. Hence we think of vectors as row vectors.
///
/// Matrices can be *sliced*, i.e. restricted to a sub-matrix, and a sliced matrix behaves as if
/// the other rows and columns are not present for many purposes. For example, this affects the
/// values of `M[i]`, the `rows` and `columns` functions, as well as more "useful"
/// functions like `row_reduce` and `compute_kernel`. However, the row slicing is not taken into
/// account when dereferencing into `&[FpVector]` (even though the FpVectors still remember the
/// column slicing). This may or may not be a bug.
///
/// In general, before one uses a matrix, they must run
/// `fp_vector::initialize_limb_bit_index_table(p)`. This only has to be done once and will be
/// omitted from all examples.
#[derive(Clone)]
pub struct Matrix {
    p : ValidPrime,
    rows : usize,
    columns : usize,
    slice_row_start : usize,
    slice_row_end : usize,
    slice_col_start : usize,
    slice_col_end : usize,
    pub vectors : Vec<FpVector>,
    pivot_vec : Vec<isize>
}

impl PartialEq for Matrix {
    fn eq(&self, other: &Self) -> bool {
        self.vectors == other.vectors
            && self.slice_row_start == other.slice_row_start
            && self.slice_row_end == other.slice_row_end
            && self.slice_col_start == other.slice_col_start
            && self.slice_col_end == other.slice_col_end
    }
}

impl Eq for Matrix {}

impl Matrix {
    /// Produces a new matrix over F_p with the specified number of rows and columns, initialized
    /// to the 0 matrix.
    pub fn new(p : ValidPrime, rows : usize, columns : usize) -> Matrix {
        let mut vectors : Vec<FpVector> = Vec::with_capacity(rows);
        for _ in 0..rows {
            vectors.push(FpVector::new(p, columns));
        }
        Matrix {
            p, rows, columns,
            slice_row_start : 0, slice_row_end : rows,
            slice_col_start : 0, slice_col_end : columns,
            vectors,
            pivot_vec : Vec::new()
        }
    }
}

impl Matrix {
    pub fn prime(&self) -> ValidPrime {
        self.p
    }

    /// Gets the number of rows in the matrix.
    pub fn rows(&self) -> usize {
        self.slice_row_end - self.slice_row_start
    }

    /// Gets the number of columns in the matrix.
    pub fn columns(&self) -> usize {
        self.slice_col_end - self.slice_col_start
    }    

    pub fn initialize_pivots(&mut self) {
        if self.pivot_vec.len() < self.columns() {
            self.pivot_vec = vec![-1; self.columns()];
        } else {
            for i in 0..self.columns(){
                self.pivot_vec[i] = -1;
            }
        }
    }

    pub fn pivots(&self) -> &Vec<isize> {
        &self.pivot_vec
    }

    pub fn pivots_mut(&mut self) -> &mut Vec<isize> {
        &mut self.pivot_vec
    }

    pub fn replace_pivots(&mut self, new_pivots : Vec<isize>) -> Vec<isize> {
        std::mem::replace(&mut self.pivot_vec, new_pivots)
    }

    pub fn take_pivots(&mut self) -> Vec<isize> {
        self.replace_pivots(Vec::new())
    }

    pub fn set_pivots(&mut self, new_pivots : Vec<isize>) {
        self.pivot_vec = new_pivots;
    }

    /// Produces a matrix from a list of rows. If `vectors.len() == 0`, this returns a matrix
    /// with 0 rows and columns.  The function does not check if the rows have the same length,
    /// but please only input rows that do have the same length.
    pub fn from_rows(p : ValidPrime, vectors : Vec<FpVector>, columns : usize) -> Self {
        let rows = vectors.len();
        for row in &vectors {
            debug_assert_eq!(row.dimension(), columns);
        }

        Matrix {
            p,
            rows, columns,
            slice_row_start : 0, slice_row_end : rows,
            slice_col_start : 0, slice_col_end : columns,
            vectors,
            pivot_vec : Vec::new()
        }
    }

    /// Produces a Matrix from an `&[Vec<u32>]` object. If the number of rows is 0, the number
    /// of columns is also assumed to be zero.
    ///
    /// # Example
    /// ```
    /// # use fp::prime::ValidPrime;
    /// let p = ValidPrime::new(7);
    /// # use fp::matrix::Matrix;
    /// # fp::vector::initialize_limb_bit_index_table(p);
    /// let input  = [vec![1, 3, 6],
    ///               vec![0, 3, 4]];
    ///
    /// let m = Matrix::from_vec(p, &input);
    pub fn from_vec(p : ValidPrime, input : &[Vec<u32>]) -> Matrix {
        let rows = input.len();
        if rows == 0 {
            return Matrix::new(p, 0, 0);
        }
        let cols = input[0].len();
        let mut m = Matrix::new(p, rows, cols);
        for (i,x) in input.iter().enumerate(){
            m[i].pack(x);
        }
        m
    }

    pub fn to_vec(&self) -> Vec<Vec<u32>> {
        let mut result = Vec::with_capacity(self.columns());
        for i in 0 .. self.rows() {
            result.push(self[i].to_vector());
        }
        result
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
    /// # fp::vector::initialize_limb_bit_index_table(p);
    /// let input  = [vec![1, 3, 6],
    ///               vec![0, 3, 4]];
    ///
    /// let (n, m) = Matrix::augmented_from_vec(p, &input);
    /// assert_eq!(n, FpVector::padded_dimension(p, input[0].len()));
    pub fn augmented_from_vec(p : ValidPrime, input : &[Vec<u32>]) -> (usize, Matrix) {
        let rows = input.len();
        let cols = input[0].len();
        let padded_cols = FpVector::padded_dimension(p, cols);
        let mut m = Matrix::new(p, rows, padded_cols + rows);

        for i in 0..rows {
            for j in 0..cols {
                m[i].set_entry(j, input[i][j]);
            }
        }
        m.add_identity(rows, 0, padded_cols);
        (padded_cols, m)
    }

    pub fn add_identity(&mut self, size : usize, row : usize, column : usize) {
        for i in 0..size {
            self[row + i].add_basis_element(column + i, 1);
        }
    }

    pub fn set_to_zero(&mut self) {
        for i in 0..self.rows() {
            self[i].set_to_zero();
        }
    }

    pub fn assign(&mut self, other : &Matrix) {
        for i in 0..self.rows() {
            self[i].assign(&other[i]);
        }
    }

    /// Sets the slice on the matrix. Restricts to the submatrix consisting of the rows from
    /// `row_start` up to but not including `row_end`, and the columns from `col_start` up to but
    /// not including `col_end`.
    ///
    /// Slicing modifies the matrix in place.
    ///
    /// # Example
    /// ```
    /// # use fp::matrix::Matrix;
    /// # use fp::vector::FpVectorT;
    /// # use fp::prime::ValidPrime;
    /// let p = ValidPrime::new(3);
    /// # fp::vector::initialize_limb_bit_index_table(p);
    /// let input  = [vec![1, 2, 1, 1, 0],
    ///               vec![1, 0, 2, 1, 0],
    ///               vec![0, 1, 0, 2, 0]];
    ///
    /// let mut m = Matrix::from_vec(p, &input);
    /// m.set_slice(1, 4, 1, 3);
    ///
    /// assert_eq!(m.rows(), 3);
    /// assert_eq!(m.columns(), 2);
    /// assert_eq!(m[0].entry(0), 0);
    /// ```
    pub fn set_slice(&mut self, row_start : usize, row_end : usize, col_start : usize, col_end : usize) {
        for v in &mut self.vectors {
            v.restore_slice((col_start, col_end));
        }
        self.set_row_slice(row_start, row_end);
        self.slice_col_start = col_start;
        self.slice_col_end = col_end;
    }

    /// Un-slice the matrix.
    pub fn clear_slice(&mut self) {
        for v in &mut self.vectors {
            v.clear_slice();
        }
        self.clear_row_slice();
        self.slice_col_start = 0;
        self.slice_col_end = self.columns;
    }

    pub fn set_row_slice(&mut self, row_start: usize, row_end: usize) {
        self.slice_row_start = row_start;
        self.slice_row_end = row_end;
    }

    pub fn clear_row_slice(&mut self) {
        self.slice_row_start = 0;
        self.slice_row_end = self.rows;
    }

    pub fn into_slice(mut self) -> Self {
        self.rows = self.rows();
        self.columns = self.columns();
        self.vectors.truncate(self.slice_row_end);
        self.vectors.drain(0..self.slice_row_start);
        for v in &mut self.vectors {
            v.into_slice();
        }
        self.clear_slice();
        self
    }

    pub fn into_vec(self) -> Vec<FpVector> {
        self.into_slice().vectors
    }
}

impl std::ops::Deref for Matrix {
    type Target = [FpVector];

    fn deref(&self) -> &[FpVector] {
        &*self.vectors
    }
}

impl std::ops::DerefMut for Matrix {
    fn deref_mut(&mut self) -> &mut [FpVector] {
        &mut *self.vectors
    }
}

impl Matrix {
    pub fn iter(&self) -> std::slice::Iter<FpVector> {
        (*self.vectors)[self.slice_row_start .. self.slice_row_end].iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<FpVector> {
        (*self.vectors)[self.slice_row_start .. self.slice_row_end].iter_mut()
    }
}


impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let mut it = self.iter();
        if let Some(x) = it.next(){
            write!(f,"[\n    {}", x)?;
        } else {
            write!(f, "[]")?;
            return Ok(());
        }
        for x in it {
            write!(f, ",\n    {}", x)?;
        }
        write!(f,"\n]")?;
        Ok(())
    }
}

impl fmt::Debug for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let mut it = self.iter();
        if let Some(x) = it.next(){
            write!(f,"[\n    {}", x)?;
        } else {
            write!(f, "[]")?;
            return Ok(());
        }
        for x in it {
            write!(f, ",\n    {}", x)?;
        }
        write!(f,"\n]")?;
        Ok(())
    }
}

impl std::ops::Index<usize> for Matrix {
    type Output = FpVector;
    fn index(&self, i : usize) -> &Self::Output {
        &self.vectors[self.slice_row_start + i]
    }
}

impl std::ops::IndexMut<usize> for Matrix {
    fn index_mut(&mut self, i : usize) -> &mut Self::Output {
        &mut self.vectors[self.slice_row_start + i]
    }
}


impl Matrix {
    pub fn swap_rows(&mut self, i : usize, j : usize){
        self.vectors.swap(i + self.slice_row_start, j + self.slice_row_start);
    }

    pub fn row_op(&mut self, target : usize, source : usize, coeff : u32) {
        debug_assert!(target != source);
        unsafe {
            // Can't take two mutable borrows from one vector, so instead just cast
            // them to their raw pointers to do the swap
            let ptarget: *mut FpVector = &mut self[target];
            let psource: *const FpVector = &mut self[source];
            // Use the optimized variant of add that ignores slicing (profiling shows this cuts out ~ 2% of runtime)
            (*ptarget).add_shift_none_pure(&*psource, coeff);
        }
    }

    /// Perform row reduction to reduce it to reduced row echelon form. This modifies the matrix in
    /// place and records the pivots in `column_to_pivot_row`. The way the pivots are recorded is
    /// that `column_to_pivot_row[i]` is the row of the pivot if the `i`th row contains a pivot,
    /// and `-1` otherwise.
    ///
    /// One has to call `fp_vector::initialize_limb_bit_index_table(p)`. This step will be skipped in
    /// future examples.
    ///
    /// # Arguments
    ///  * `column_to_pivot_row` - A vector for the function to write the pivots into. The length
    ///  should be at least as long as the number of columns (and the extra entries are ignored).
    ///
    /// # Example
    /// `#`#`
    /// # use fp::prime::ValidPrime;
    /// let p = ValidPrime::new(7);
    /// # use fp::matrix::Matrix;
    /// # fp::vector::initialize_limb_bit_index_table(p);
    ///
    /// let input  = [vec![1, 3, 6],
    ///               vec![0, 3, 4]];
    ///
    /// let result = [vec![1, 0, 2],
    ///               vec![0, 1, 6]];
    ///
    /// let mut m = Matrix::from_vec(p, &input);
    /// m.initialize_pivots();
    /// m.row_reduce();
    ///
    /// assert_eq!(m, Matrix::from_vec(p, &result));
    /// `#`#`
    pub fn row_reduce(&mut self) {
        let mut column_to_pivot_row = self.take_pivots();
        self.row_reduce_into_pivots(&mut column_to_pivot_row);
        self.set_pivots(column_to_pivot_row);
    }
    
    pub fn row_reduce_into_pivots(&mut self, column_to_pivot_row: &mut Vec<isize>) {
        self.row_reduce_offset_into_pivots(column_to_pivot_row, 0);
    }

    pub fn row_reduce_offset_into_pivots(&mut self, column_to_pivot_row: &mut Vec<isize>, offset : usize) {
        self.row_reduce_permutation_into_pivots(column_to_pivot_row, offset..self.columns());
    }

    /// This is very similar to row_reduce, except we only need to get to row echelon form, not
    /// *reduced* row echelon form. It also returns the list of pivots instead.
    pub fn find_pivots_permutation<T : Iterator<Item = usize>>(&mut self, permutation : T) -> Vec<usize> {
        let p = self.p;
        let rows = self.rows();
        let mut pivots = Vec::with_capacity(rows);

        if rows == 0 {
            return pivots;
        }

        let mut pivot : usize = 0;
        for pivot_column in permutation {
            // Search down column for a nonzero entry.
            let mut pivot_row = rows;
            for i in pivot..rows {
                if self[i].entry(pivot_column) != 0 {
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
            let c = self[pivot].entry(pivot_column);
            let c_inv = prime::inverse(p, c);
            self[pivot].scale(c_inv);
            // println!("({}) <== {} * ({}): \n{}", pivot, c_inv, pivot, self);

            for i in pivot_row + 1 .. rows {
                let pivot_column_entry = self[i].entry(pivot_column);
                if pivot_column_entry == 0 {
                    continue;
                }
                let row_op_coeff = *p - pivot_column_entry;
                // Do row operation
                self.row_op(i, pivot, row_op_coeff);
            }
            pivot += 1;
        }
        pivots
    }

    pub fn row_reduce_permutation_into_pivots<T>(&mut self, column_to_pivot_row: &mut Vec<isize>, permutation : T)
        where T : Iterator<Item = usize> {
        debug_assert!(self.columns() <= column_to_pivot_row.len());
        let p = self.p;
        let rows = self.rows();
        for x in column_to_pivot_row.iter_mut() {
            *x = -1;
        }
        if rows == 0 {
            return;
        }
        let mut pivot : usize = 0;
        for pivot_column in permutation {
            // Search down column for a nonzero entry.
            let mut pivot_row = rows;
            for i in pivot..rows {
                if self[i].entry(pivot_column) != 0 {
                    pivot_row = i;
                    break;
                }
            }
            if pivot_row == rows {
                continue;
            }

            // Record position of pivot.
            column_to_pivot_row[pivot_column] = pivot as isize;

            // Pivot_row contains a row with a pivot in current column.
            // Swap pivot row up.
            self.swap_rows(pivot, pivot_row);
            // println!("({}) <==> ({}): \n{}", pivot, pivot_row, self);

            // // Divide pivot row by pivot entry
            let c = self[pivot].entry(pivot_column);
            let c_inv = prime::inverse(p, c);
            self[pivot].scale(c_inv);
            // println!("({}) <== {} * ({}): \n{}", pivot, c_inv, pivot, self);
            // We would say:
            // for i in 0..rows { // but we want to skip a few rows so we can't use for.
            let mut i = 0;
            while i < rows {
                if i as usize == pivot {
                    // Between pivot and pivot_row, we already checked that the pivot column is 0, 
                    // so we skip ahead a bit.
                    i = pivot_row + 1;
                    continue;
                }
                let pivot_column_entry = self[i].entry(pivot_column);
                if pivot_column_entry == 0 {
                    i += 1; // loop control structure.
                    continue;
                }
                let row_op_coeff = *p - pivot_column_entry;
                self.row_op(i, pivot, row_op_coeff);
                i += 1; // loop control structure.
            }
            pivot += 1;
        }
    }
}


impl Matrix {
    pub fn find_first_row_in_block(&self, first_column_in_block : usize) -> usize {
        self.find_first_row_in_block_with_pivots(self.pivots(), first_column_in_block)
    }

    pub fn find_first_row_in_block_with_pivots(&self, column_to_pivot_row : &[isize],  first_column_in_block : usize) -> usize {
        for &pivot in &column_to_pivot_row[first_column_in_block..] {
            if pivot >= 0 {
                return pivot as usize;
            }
        }
        self.rows()
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
    /// # use fp::vector::{FpVector, FpVectorT};
    /// # fp::vector::initialize_limb_bit_index_table(p);
    /// let input  = [vec![1, 2, 1, 1, 0],
    ///               vec![1, 0, 2, 1, 1],
    ///               vec![2, 2, 0, 2, 1]];
    ///
    /// let (padded_cols, mut m) = Matrix::augmented_from_vec(p, &input);
    /// m.initialize_pivots();
    /// m.row_reduce();
    /// let ker = m.compute_kernel(padded_cols);
    ///
    /// let mut target = vec![0; 3];
    /// ker.matrix[0].unpack(&mut target);
    /// assert_eq!(target, vec![1, 1, 2]);
    /// ```
    pub fn compute_kernel(&mut self, first_source_column : usize) -> Subspace {
        let column_to_pivot_row = self.take_pivots();
        let result = self.compute_kernel_from_pivots(&column_to_pivot_row, first_source_column);
        self.set_pivots(column_to_pivot_row);
        result
    }
    
    pub fn compute_kernel_from_pivots(&mut self, column_to_pivot_row : &[isize], first_source_column : usize) -> Subspace {
        let p = self.p;
        let rows = self.rows();
        let columns = self.columns();
        let source_dimension = columns - first_source_column;

        // Find the first kernel row
        let first_kernel_row = self.find_first_row_in_block_with_pivots(column_to_pivot_row, first_source_column);
        // Every row after the first kernel row is also a kernel row, so now we know how big it is and can allocate space.
        let kernel_dimension = rows - first_kernel_row;
        let mut kernel = Subspace::new(p, kernel_dimension, source_dimension);
        if kernel_dimension == 0 {
            return kernel;
        }
        // Write pivots into kernel
        for i in 0 .. source_dimension {
            // Turns -1 into some negative number... make sure to check <0 for no pivot in column...
            kernel.pivots_mut()[i] = column_to_pivot_row[i + first_source_column] - first_kernel_row as isize;
        }
        // Copy kernel matrix into kernel
        for (i, row) in kernel.matrix.iter_mut().enumerate() {
            row.assign(&*self[first_kernel_row + i].borrow_slice(first_source_column, first_source_column + source_dimension));
        }
        kernel
    }

    /// Computes the quasi-inverse of a matrix given a rref of [A|0|I], where 0 is the zero padding
    /// as usual.
    ///
    /// # Arguments
    ///  * `pivots` - Pivots returned by `row_reduce`
    ///  * `last_target_col` - the last column of A
    ///  * `first_source_col` - the first column of I
    ///
    /// # Example
    /// ```
    /// # use fp::prime::ValidPrime;
    /// let p = ValidPrime::new(3);
    /// # use fp::matrix::Matrix;
    /// # use fp::vector::{FpVector, FpVectorT};
    /// # fp::vector::initialize_limb_bit_index_table(p);
    /// let input  = [vec![1, 2, 1, 1, 0],
    ///               vec![1, 0, 2, 1, 1],
    ///               vec![2, 2, 0, 2, 1]];
    ///
    /// let (padded_cols, mut m) = Matrix::augmented_from_vec(p, &input);
    /// m.initialize_pivots();
    /// m.row_reduce();
    /// let qi = m.compute_quasi_inverse(input[0].len(), padded_cols);
    ///
    /// let image = [vec![1, 0, 2, 1, 1],
    ///              vec![0, 1, 1, 0, 1]];
    /// let computed_image = qi.image.unwrap();
    /// assert_eq!(computed_image.matrix, Matrix::from_vec(p, &image));
    /// assert_eq!(computed_image.pivots(), &vec![0, 1, -1, -1, -1]);
    ///
    /// let preimage = [vec![0, 1, 0],
    ///                 vec![0, 2, 2]];
    /// assert_eq!(qi.preimage, Matrix::from_vec(p, &preimage));
    /// ```
    pub fn compute_quasi_inverse(&mut self, last_target_col : usize, first_source_col : usize) -> QuasiInverse {
        let p = self.prime();
        let columns = self.columns();
        let source_columns = columns - first_source_col;
        let first_kernel_row = self.find_first_row_in_block(first_source_col);
        let mut image_matrix = Matrix::new(p, first_kernel_row, last_target_col);
        let mut preimage = Matrix::new(p, first_kernel_row, source_columns);
        for i in 0 .. first_kernel_row {
            let old_slice = self[i].slice();
            self[i].set_slice(0, last_target_col);
            image_matrix[i].assign(&self[i]);
            self[i].restore_slice(old_slice);
            self[i].set_slice(first_source_col, columns);
            preimage[i].assign(&self[i]);
            self[i].restore_slice(old_slice);
        }
        image_matrix.set_pivots(self.pivots()[..last_target_col].to_vec());
        let image = Subspace {
            matrix : image_matrix
        };
        QuasiInverse {
            image : Some(image),
            preimage
        }
    }

    /// This function computes quasi-inverses for matrices A, B given a reduced row echelon form of
    /// [A|0|B|0|I] such that the [A|0] and [B|0] blocks have number of columns a multiple of
    /// `entries_per_64_bit`, and A is surjective. Moreover, if Q is the quasi-inverse of A, it is
    /// guaranteed that the image of QB and B|_{ker A} are disjoint.
    ///
    /// # Arguments
    ///  * `pivots` - the pivots produced by `row_reduce`
    ///  * `first_res_column` - the first column of B
    ///  * `last_res_col` - the last column of B
    ///  * `first_source_col` - the first column of I
    pub fn compute_quasi_inverses(&mut self, first_res_col : usize, last_res_col : usize,  first_source_col : usize) -> (QuasiInverse, QuasiInverse) {
        let p = self.prime();
        let columns = self.columns();
        let source_columns = columns - first_source_col;
        let res_columns = last_res_col - first_res_col;
        let first_res_row = self.find_first_row_in_block(first_res_col);
        let first_kernel_row = self.find_first_row_in_block(first_source_col);
        let mut cc_preimage = Matrix::new(p, first_res_row, source_columns);
        for i in 0..first_res_row {
            let old_slice = self[i].slice();
            self[i].set_slice(first_source_col, columns);
            cc_preimage[i].assign(&self[i]);
            self[i].restore_slice(old_slice);
        }
        let mut new_pivots = vec![-1; columns - first_res_col];
        let res_image_rows;
        if first_res_row == 0 {
            new_pivots[0 .. (columns - first_res_col)]
                .clone_from_slice(&self.pivots()[first_res_col..columns]);
            res_image_rows = first_kernel_row;
        } else {
            self.set_slice(0, first_kernel_row, first_res_col, columns);
            self.row_reduce_into_pivots(&mut new_pivots);
            res_image_rows = self.find_first_row_in_block(first_source_col - first_res_col);
            self.clear_slice();
        }
        let mut res_preimage = Matrix::new(p, res_image_rows, source_columns);
        let mut res_image = Subspace::new(p, res_image_rows, res_columns);
        for i in 0..res_image_rows {
            res_image[i].assign(&*self[i].borrow_slice(first_res_col, last_res_col));
            res_image.pivots_mut().copy_from_slice(&new_pivots[..res_columns]);
            res_preimage[i].assign(&*self[i].borrow_slice(first_source_col, columns));
        }
        let cm_qi = QuasiInverse {
            image : None,
            preimage  : cc_preimage
        };
        let res_qi = QuasiInverse {
            image : Some(res_image),
            preimage : res_preimage
        };
        (cm_qi, res_qi)
    }

    pub fn get_image(&mut self, image_rows : usize, target_dimension : usize, pivots : &[isize]) -> Subspace {
        let mut image = Subspace::new(self.p, image_rows, target_dimension);
        for i in 0 .. image_rows {
            image.pivots_mut()[i] = pivots[i];
            let vector_to_copy = &mut self[i];
            let old_slice = vector_to_copy.slice();
            vector_to_copy.set_slice(0, target_dimension);
            image[i].assign(vector_to_copy);
            vector_to_copy.restore_slice(old_slice);
        }
        image
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
    ///  our new rows. This is a mutable borrow and by the end of the function, `first_empty_row`
    ///  will be updated to the new first empty row.
    ///  * `current_pivots` - The current pivots of the matrix.
    ///
    /// # Panics
    /// The function panics if there are not enough empty rows.
    pub fn extend_to_surjection(&mut self,
        mut first_empty_row : usize,
        start_column : usize, end_column : usize
    ) -> Vec<usize> {
        let mut added_pivots = Vec::new();
        let pivots = self.take_pivots();
        for (i, &pivot) in pivots[start_column .. end_column].iter().enumerate() {
            if pivot >= 0 {
                continue;
            }
            // Look up the cycle that we're missing and add a generator hitting it.
            let matrix_row = &mut self[first_empty_row];
            // We're trying to make a surjection so we just set the output equal to 1
            added_pivots.push(i);
//            matrix_row.set_to_zero();
            matrix_row.set_entry(i, 1);
            first_empty_row += 1;
        }
        self.set_pivots(pivots);
        added_pivots
    }

    /// Given a matrix in rref, say [A|B|C], where B lies between columns `start_column` and
    /// `end_columns`, and a subspace of the image of B, add rows to the matrix such that the image
    /// of B becomes this subspace. This doesn't change the size of the matrix. Rather, it adds the
    /// new row to the next empty row in the matrix. This will panic if there are not enough empty
    /// rows.
    ///
    /// The rows added are basis vectors of the desired image as specified in the Subspace object.
    /// The function returns the list of new pivot columns.
    ///
    /// # Arguments
    ///  * `first_empty_row` - The first row in the matrix that is empty. This is where we will add
    ///  our new rows. This is a mutable borrow and by the end of the function, `first_empty_row`
    ///  will be updated to the new first empty row.
    ///  * `current_pivots` - The current pivots of the matrix.
    ///
    /// # Panics
    /// The function panics if there are not enough empty rows. It *may* panic if the current image
    /// is not contained in `desired_image`, but is not guaranteed to do so.
    pub fn extend_image_to_desired_image(&mut self,
        mut first_empty_row : usize,
        start_column : usize, end_column : usize,
        desired_image : &Subspace
    ) -> Vec<usize> {
        let mut added_pivots = Vec::new();
        let desired_pivots = desired_image.matrix.pivots();
        let early_end_column = std::cmp::min(end_column, desired_pivots.len() + start_column);
        for i in start_column .. early_end_column {
            debug_assert!(self.pivots()[i] < 0 || desired_pivots[i - start_column] >= 0,
                "current_pivots : {:?}, desired_pivots : {:?}", self.pivots(), desired_pivots);
            if self.pivots()[i] >= 0 || desired_pivots[i - start_column] < 0 {
                continue;
            }
            // Look up the cycle that we're missing and add a generator hitting it.
            let kernel_vector_row = desired_pivots[i - start_column] as usize;
            let new_image = &desired_image[kernel_vector_row];
            let matrix_row = &mut self[first_empty_row];
            added_pivots.push(i);
            matrix_row.set_to_zero();
            let old_slice = matrix_row.slice();
            matrix_row.set_slice(start_column, start_column + desired_image.matrix.columns);
            matrix_row.assign(&new_image);
            matrix_row.restore_slice(old_slice);
            first_empty_row += 1;
        }
        added_pivots
    }

    /// Extends the image of a matrix to either the whole codomain, or the desired image specified
    /// by `desired_image`. It simply calls `extends_image_to_surjection` or
    /// `extend_image_to_surjection` depending on the value of `desired_image`. Refer to these
    /// functions for documentation.
    pub fn extend_image(&mut self,
        first_empty_row : usize,
        start_column : usize, end_column : usize,
        desired_image : Option<&Subspace>
    ) -> Vec<usize> {
        if let Some(image) = desired_image.as_ref() {
            self.extend_image_to_desired_image(first_empty_row, start_column, end_column, image)
        } else {
            self.extend_to_surjection(first_empty_row, start_column, end_column)
        }
    }

    /// Applies a matrix to a vector.
    ///
    /// # Example
    /// #`#`#`
    /// # use fp::prime::ValidPrime;
    /// let p = ValidPrime::new(7);
    /// # use fp::matrix::Matrix;
    /// # use fp::vector::{FpVector, FpVectorT};
    /// # fp::vector::initialize_limb_bit_index_table(p);
    /// let input  = [vec![1, 3, 6],
    ///               vec![0, 3, 4]];
    ///
    /// let m = Matrix::from_vec(p, &input);
    /// let mut v = FpVector::new(p, 2);
    /// v.pack(&vec![3, 1]);
    /// let mut result = FpVector::new(p, 3);
    /// let mut desired_result = FpVector::new(p, 3);
    /// desired_result.pack(&vec![3, 5, 1]);
    /// m.apply(&mut result, 1, &v);
    /// assert_eq!(result, desired_result);
    /// `#`#`
    pub fn apply(&self, result : &mut FpVector, coeff : u32, input : &FpVector) {
        debug_assert_eq!(input.dimension(), self.rows());
        for i in 0 .. input.dimension() {
            result.add(&self.vectors[i], (coeff * input.entry(i)) % *self.p);
        }
    }
}

impl std::ops::Mul for &Matrix {
    type Output = Matrix;

    fn mul(self, rhs : Self) -> Matrix {
        assert_eq!(self.prime(), rhs.prime());
        assert_eq!(self.columns(), rhs.rows());

        let mut result = Matrix::new(self.prime(), self.rows(), rhs.columns());
        for i in 0 .. self.rows() {
            for j in 0 .. rhs.columns() {
                for k in 0 .. self.columns() {
                    result[i].add_basis_element(j, self[i].entry(k) * rhs[k].entry(j));
                }
            }
        }
        result
    }
}

#[allow(clippy::suspicious_op_assign_impl)]
impl std::ops::MulAssign<u32> for Matrix {
    fn mul_assign(&mut self, rhs : u32) {
        let rhs = rhs % *self.p;
        for row in self.iter_mut() {
            row.scale(rhs);
        }
    }
}

impl std::ops::AddAssign<&Matrix> for Matrix {
    fn add_assign(&mut self, rhs : &Matrix) {
        assert_eq!(self.prime(), rhs.prime());
        assert_eq!(self.columns(), rhs.columns());
        assert_eq!(self.rows(), rhs.rows());

        for (i, row) in self.iter_mut().enumerate() {
            row.add(&rhs[i], 1);
        }
    }
}
macro_rules! augmented_matrix {
    ( $($N:expr, $name:ident), * ) => {
        $(
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
            pub struct $name {
                pub end: [usize; $N],
                pub start: [usize; $N],
                pub inner: Matrix,
            }

            impl $name {
                pub fn new(p: ValidPrime, rows: usize, columns: &[usize]) -> Self {
                    let mut start = [0; $N];
                    let mut end = [0; $N];
                    for i in 1 .. $N {
                        start[i] = start[i - 1] + FpVector::padded_dimension(p, columns[i - 1]);
                    }
                    for i in 0 .. $N {
                        end[i] = start[i] + columns[i];
                    }

                    Self {
                        inner: Matrix::new(p, rows, end[$N - 1]),
                        start,
                        end,
                    }
                }

                pub fn segment(&mut self, start: usize, end: usize) -> MatrixSlice<'_> {
                    let start_idx = self.start[start];
                    let end_idx = self.end[end];
                    let old_slice = if self.rows() == 0 {
                        (0, self.columns())
                    } else {
                        self[0].slice()
                    };
                    for v in &mut *self.inner {
                        v.restore_slice((start_idx, end_idx));
                    }
                    self.inner.slice_col_start = start_idx;
                    self.inner.slice_col_end = end_idx;
                    MatrixSlice(&mut self.inner, old_slice)
                }

                pub fn row_segment(&mut self, i: usize, start: usize, end: usize) -> FpVectorSlice<'_> {
                    let start_idx = self.start[start];
                    let end_idx = self.end[end];
                    self[i].borrow_slice(start_idx, end_idx)
                }

                pub fn into_matrix(self) -> Matrix {
                    self.inner
                }
            }

            impl std::ops::Deref for $name {
                type Target = Matrix;

                fn deref(&self) -> &Matrix {
                    &self.inner
                }
            }

            impl std::ops::DerefMut for $name {
                fn deref_mut(&mut self) -> &mut Matrix {
                    &mut self.inner
                }
            }
        )*
    }
}

augmented_matrix!(3, AugmentedMatrix3, 2, AugmentedMatrix2);

impl AugmentedMatrix2 {
    pub fn compute_quasi_inverse(&mut self) -> QuasiInverse {
        self.inner.compute_quasi_inverse(self.end[0], self.start[1])
    }
    pub fn compute_kernel(&mut self) -> Subspace {
        self.inner.compute_kernel(self.start[1])
    }
}

impl AugmentedMatrix3 {
    pub fn compute_quasi_inverses(&mut self) -> (QuasiInverse, QuasiInverse) {
        self.inner.compute_quasi_inverses(self.start[1], self.end[1], self.start[2])
    }
}

pub struct MatrixSlice<'a>(&'a mut Matrix, (usize, usize));

impl<'a> Drop for MatrixSlice<'a> {
    fn drop(&mut self) {
        self.0.slice_col_start = 0;
        self.0.slice_col_end = self.0.columns;
        for v in &mut **self.0 {
            v.restore_slice(self.1);
        }
    }
}

impl std::ops::Deref for MatrixSlice<'_> {
    type Target = Matrix;

    fn deref(&self) -> &Matrix {
        &self.0
    }
}

impl std::ops::DerefMut for MatrixSlice<'_> {
    fn deref_mut(&mut self) -> &mut Matrix {
        &mut self.0
    }
}

use std::io;
use std::io::{Read, Write};
use saveload::{Save, Load};

impl Save for Matrix {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        self.columns.save(buffer)?;
        self.vectors.save(buffer)?;
        self.pivots().save(buffer)?;
        Ok(())
    }
}

impl Load for Matrix {
    type AuxData = ValidPrime;

    fn load(buffer : &mut impl Read, p : &ValidPrime) -> io::Result<Self> {
        let columns = usize::load(buffer, &())?;
        let vectors : Vec<FpVector> = Load::load(buffer, p)?;
        let pivots : Vec<isize> = Load::load(buffer, &())?;
        let mut result = Matrix::from_rows(*p, vectors, columns);
        result.set_pivots(pivots);
        Ok(result)
    }
}

impl Save for Subspace {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        self.matrix.save(buffer)?;
        Ok(())
    }
}

impl Load for Subspace {
    type AuxData = ValidPrime;

    fn load(buffer : &mut impl Read, p : &ValidPrime) -> io::Result<Self> {
        let matrix : Matrix = Matrix::load(buffer, p)?;
        Ok(Subspace { matrix })
    }
}

use crate::vector::VectorDiffEntry;
pub struct MatrixDiffEntry {
    pos : (usize, usize),
    left : u32,
    right : u32
}

impl Matrix {
    pub fn diff_list(&self, other : &[Vec<u32>]) -> Vec<MatrixDiffEntry> {
        assert!(self.rows() == other.len());
        if self.rows() > 0 {
            assert!(self.columns() == other[0].len());
        }
        let mut result = Vec::new();
        for row in 0 .. self.rows() {
            result.extend(
                self[row].diff_list(&other[row]).iter()
                .map(|&VectorDiffEntry {index, left, right}| 
                    MatrixDiffEntry {
                        pos : (row, index),
                        left,
                        right
                    }
                )
            )
        }
        result
    }

    pub fn diff_matrix(&self, other : &Matrix) -> Vec<MatrixDiffEntry> {
        assert!(self.rows() == other.rows());
        assert!(self.columns() == other.columns());
        let mut result = Vec::new();
        for row in 0 .. self.rows() {
            result.extend(
                self[row].diff_vec(&other[row]).iter()
                .map(|&VectorDiffEntry {index, left, right}| 
                    MatrixDiffEntry {
                        pos : (row, index),
                        left,
                        right
                    }
                )
            )
        }
        result
    }
    
    pub fn format_diff(diff : Vec<MatrixDiffEntry>) -> String {
        let data_formatter = diff.iter().format_with("\n ", |MatrixDiffEntry {pos, left, right}, f| 
            f(&format_args!("  At index {:?}: {}!={}", pos, left, right))
        );
        format!("{}", data_formatter)
    }

    pub fn assert_list_eq(&self, other : &[Vec<u32>]){
        let diff = self.diff_list(other);
        if diff.len() == 0 {
            return;
        }
        println!("assert {} == {:?}", self,other);
        println!("{}", Matrix::format_diff(diff));
    }

    pub fn assert_matrix_eq(&self, other : &Matrix){
        let diff = self.diff_matrix(other);
        if diff.len() == 0 {
            return;
        }
        println!("assert {} == {}", self, other);
        println!("{}", Matrix::format_diff(diff));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use rstest::rstest;

    #[test]
    fn test_augmented_matrix() {
        test_augmented_matrix_inner(&[1, 0, 5]);
        test_augmented_matrix_inner(&[4, 6, 2]);
        test_augmented_matrix_inner(&[129, 4, 64]);
        test_augmented_matrix_inner(&[64, 64, 102]);
    }

    fn test_augmented_matrix_inner(cols: &[usize]) {
        let mut aug = AugmentedMatrix3::new(ValidPrime::new(2), 3, cols);
        assert_eq!(aug.segment(0, 0).columns(), cols[0]);
        assert_eq!(aug.segment(1, 1).columns(), cols[1]);
        assert_eq!(aug.segment(2, 2).columns(), cols[2]);
    }

    #[test]
    fn test_row_reduce_2(){
        let p = ValidPrime::new(2);
        let tests = [(
            [
                [0, 1, 1, 0, 1, 1, 0, 1, 0, 0, 0, 1, 0, 1, 1],
                [0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0],
                [0, 0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1, 0, 1, 1],
                [1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0],
                [1, 1, 0, 0, 1, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0],
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1],
                [0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 1, 1, 1, 1]
            ], [
                [1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 0, 1, 1],
                [0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1, 1],
                [0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0],
                [0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0],
                [0, 0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 1, 0, 0, 1],
                [0, 0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1, 0, 1, 1],
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1]
            ],
            [0, 1, 2, 3, 4, 5, -1, -1, -1, -1, -1, -1, 6, -1, -1]
        )];
        for test in &tests {
            let input = test.0;
            let goal_output = test.1;
            let goal_pivots = test.2;
            let rows = input.len();
            let cols = input[0].len();
            let mut m = Matrix::new(p, rows, cols);
            for (i,x) in input.iter().enumerate(){
                m[i].pack(x);
            }
            m.initialize_pivots();
            m.row_reduce();
            let mut unpacked_row : Vec<u32> = vec![0; cols];
            for i in 0 .. input.len() {
                m[i].unpack(&mut unpacked_row);
                assert_eq!(unpacked_row, goal_output[i]);
            }
            assert_eq!(m.pivots(), &goal_pivots)
        }
    }

    fn random_vector(p : u32, v : &mut Vec<u32>){
        let mut rng = rand::thread_rng();
        for i in 0 .. v.len() {
            v[i] = rng.gen::<u32>() % p;
        }
    }
    
}
