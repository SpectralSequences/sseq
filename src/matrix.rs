use crate::combinatorics;
use crate::fp_vector::{FpVector, FpVectorT};


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
#[derive(PartialEq, Eq, Clone)]
pub struct Matrix {
    p : u32,
    rows : usize,
    columns : usize,
    slice_row_start : usize,
    slice_row_end : usize,
    slice_col_start : usize,
    slice_col_end : usize,
    vectors : Vec<FpVector>
}

impl Matrix {
    /// Produces a new matrix over F_p with the specified number of rows and columns, intiialized
    /// to the 0 matrix.
    pub fn new(p : u32, rows : usize, columns : usize) -> Matrix {
        let mut vectors : Vec<FpVector> = Vec::with_capacity(rows);
        for _ in 0..rows {
            vectors.push(FpVector::new(p, columns));
        }
        Matrix { 
            p, rows, columns, 
            slice_row_start : 0, slice_row_end : rows,
            slice_col_start : 0, slice_col_end : columns,
            vectors
        }
    }

    /// Produces a matrix from a list of rows. If `vectors.len() == 0`, this returns a matrix
    /// with 0 rows and columns.  The function does not check if the rows have the same length,
    /// but please only input rows that do have the same length.
    pub fn from_rows(p : u32, vectors : Vec<FpVector>, columns : usize) -> Self {
        let rows = vectors.len();
        for row in vectors.iter() {
            debug_assert_eq!(row.dimension(), columns);
        }

        Matrix {
            p,
            rows, columns,
            slice_row_start : 0, slice_row_end : rows,
            slice_col_start : 0, slice_col_end : columns,
            vectors
        }
    }

    /// Produces a Matrix from an `&[Vec<u32>]` object. If the number of rows is 0, the number
    /// of columns is also assumed to be zero.
    ///
    /// # Example
    /// ```
    /// let p = 7;
    /// # use rust_ext::matrix::Matrix;
    /// # rust_ext::fp_vector::initialize_limb_bit_index_table(p);
    /// let input  = [vec![1, 3, 6],
    ///               vec![0, 3, 4]];
    ///
    /// let m = Matrix::from_vec(p, &input);
    pub fn from_vec(p : u32, input : &[Vec<u32>]) -> Matrix {
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
    /// let p = 7;
    /// # use rust_ext::matrix::Matrix;
    /// # use rust_ext::fp_vector::FpVector;
    /// # rust_ext::fp_vector::initialize_limb_bit_index_table(p);
    /// let input  = [vec![1, 3, 6],
    ///               vec![0, 3, 4]];
    ///
    /// let (n, m) = Matrix::augmented_from_vec(p, &input);
    /// assert_eq!(n, FpVector::padded_dimension(p, input[0].len()));
    pub fn augmented_from_vec(p : u32, input : &[Vec<u32>]) -> (usize, Matrix) {
        let rows = input.len();
        let cols = input[0].len();
        let padded_cols = FpVector::padded_dimension(p, cols);
        let mut m = Matrix::new(p, rows, padded_cols + rows);

        for i in 0..rows {
            for j in 0..cols {
                m[i].set_entry(j, input[i][j]);
            }
        }
        m.set_identity(rows, 0, padded_cols);
        (padded_cols, m)
    }

    pub fn set_identity(&mut self, size : usize, row : usize, column : usize) {
        for i in 0..size {
            self[row + i].set_entry(column + i, 1);
        }
    }

    pub fn prime(&self) -> u32 {
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

    /// Sets the slice on the matrix. Restricts to the submatrix consisting of the rows from
    /// `row_start` up to but not including `row_end`, and the columns from `col_start` up to but
    /// not including `col_end`.
    ///
    /// Slicing modifies the matrix in place.
    ///
    /// # Example
    /// ```
    /// let p = 3;
    /// # use rust_ext::matrix::Matrix;
    /// # use rust_ext::fp_vector::FpVectorT;
    /// # rust_ext::fp_vector::initialize_limb_bit_index_table(p);
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
        for v in self.vectors.iter_mut() {
            v.set_slice(col_start, col_end);
        }
        self.slice_row_start = row_start;
        self.slice_row_end = row_end;
        self.slice_col_start = col_start;
        self.slice_col_end = col_end;
    }

    /// Un-slice the matrix.
    pub fn clear_slice(&mut self) {
        for v in self.vectors.iter_mut() {
            v.clear_slice();
        }        
        self.slice_row_start = 0;
        self.slice_row_end = self.rows;
        self.slice_col_start = 0;
        self.slice_col_end = self.columns;
    }

    pub fn into_slice(&mut self) {
        self.rows = self.rows();
        self.columns = self.columns();
        self.vectors.drain(0..self.slice_row_start);
        self.slice_row_end -= self.slice_row_start;
        self.vectors.truncate(self.slice_row_end);
        for v in self.vectors.iter_mut() {
            v.into_slice();
        }
        self.clear_slice();
    }

    pub fn into_vec(mut self) -> Vec<FpVector> {
        self.into_slice();
        self.vectors
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

    pub fn row_op(&mut self, target : usize, source : usize, coeff : u32){
    unsafe {
            // Can't take two mutable borrows from one vector, so instead just cast
            // them to their raw pointers to do the swap
            let ptarget: *mut FpVector = &mut self[target];
            let psource: *const FpVector = &mut self[source];
            (*ptarget).add(&*psource, coeff);
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
    /// ```
    /// let p = 7;
    /// # use rust_ext::matrix::Matrix;
    /// # rust_ext::fp_vector::initialize_limb_bit_index_table(p);
    ///
    /// let input  = [vec![1, 3, 6],
    ///               vec![0, 3, 4]];
    ///
    /// let result = [vec![1, 0, 2],
    ///               vec![0, 1, 6]];
    ///
    /// let mut m = Matrix::from_vec(p, &input);
    /// let mut output_pivots_cvec = vec![-1; m.columns()];
    /// m.row_reduce(&mut output_pivots_cvec);
    ///
    /// assert_eq!(m, Matrix::from_vec(p, &result));
    /// ```
    pub fn row_reduce(&mut self, column_to_pivot_row: &mut Vec<isize>) {
        self.row_reduce_offset(column_to_pivot_row, 0);
    }

    pub fn row_reduce_offset(&mut self, column_to_pivot_row: &mut Vec<isize>, offset : usize) {
        self.row_reduce_permutation(column_to_pivot_row, offset..self.columns());
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
            let c_inv = combinatorics::inverse(p, c);
            self[pivot].scale(c_inv);
            // println!("({}) <== {} * ({}): \n{}", pivot, c_inv, pivot, self);

            for i in pivot_row + 1 .. rows {
                let pivot_column_entry = self[i].entry(pivot_column);
                if pivot_column_entry == 0 {
                    continue;
                }
                let row_op_coeff = p - pivot_column_entry;
                // Do row operation
                self.row_op(i, pivot, row_op_coeff);
            }
            pivot += 1;
        }
        pivots
    }

    pub fn row_reduce_permutation<T>(&mut self, column_to_pivot_row: &mut Vec<isize>, permutation : T)
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
            let c_inv = combinatorics::inverse(p, c);
            self[pivot].scale(c_inv);
            // println!("({}) <== {} * ({}): \n{}", pivot, c_inv, pivot, self);

            for i in 0 .. rows {
                // Between pivot and pivot_row, we already checked that the pivot column is 0, so we could skip ahead a bit.
                // But Rust doesn't make this as easy as C.
                if i == pivot {
                    continue;
                }
                let pivot_column_entry = self[i].entry(pivot_column);
                if pivot_column_entry == 0 {
                    continue;
                }
                let row_op_coeff = p - pivot_column_entry;
                // Do row operation
                self.row_op(i, pivot, row_op_coeff);
            }
            pivot += 1;
        }
    }
}

/// A subspace of a vector space.
/// # Fields
///  * `matrix` - A matrix in reduced row echelon, whose number of columns is the dimension of the
///  ambient space and each row is a basis vector of the subspace.
///  * `column_to_pivot_row` - If the column is a pivot column, the entry is the row the pivot
///  corresponds to. If the column is not a pivot column, this is some negative number &mdash; not
///  necessarily -1!
#[derive(Debug, Clone)]
pub struct Subspace {
    pub matrix : Matrix,
    pub column_to_pivot_row : Vec<isize>
}

impl Subspace {
    pub fn new(p : u32, rows : usize, columns : usize) -> Self {
        Self {
            matrix : Matrix::new(p, rows, columns),
            column_to_pivot_row : vec![-1; columns]
        }
    }

    /// Given a chain of subspaces `subspace` < `space` < k^`ambient_dimension`, compute the
    /// subquotient `space`/`subspace`. The answer is expressed as a list of basis vectors of
    /// `space` whose image in `space`/`subspace` forms a basis, and a basis vector of `space` is
    /// described by its index in the list of basis vectors of `space` (not the ambient space).
    ///
    /// # Arguments
    ///  * `space` - If this is None, it is the whole space k^`ambient_dimension`
    ///  * `subspace` - If this is None, it is empty
    pub fn subquotient(space : Option<&Subspace>, subspace : Option<&Subspace>, ambient_dimension : usize) -> Vec<usize> {
        match subspace {
            None => {
                if let Some(sp) = space {
                    return sp.column_to_pivot_row.iter().filter( |i| **i >= 0).map(|i| *i as usize).collect();
                } else {
                    return (0..ambient_dimension).collect();
                }
            },
            Some(subsp) => {
                if let Some(sp) = space {
                    return sp.column_to_pivot_row.iter().zip(subsp.column_to_pivot_row.iter())
                      .filter(|(x,y)| {
                          debug_assert!(**x >= 0 || **y < 0);
                          **x >= 0 && **y < 0
                        }).map(|(x,_)| *x as usize).collect();
                } else {
                    return (0..ambient_dimension).filter( |i| subsp.column_to_pivot_row[*i] < 0).collect();
                }
            }
        }
    }

    pub fn entire_space(p : u32, dim : usize) -> Self {
        let mut result = Self::new(p, dim, dim);
        for i in 0..dim {
            result.matrix[i].set_entry(i, 1);
            result.column_to_pivot_row[i] = i as isize;
        }
        return result;
    }

    /// This adds a vector to the subspace. This function assumes that the last row of the
    /// matrix is zero, i.e. the dimension of the current subspace is strictly less than the number
    /// of rows. This can be achieved by setting the number of rows to be the dimension plus one
    /// when creating the subspace.
    pub fn add_vector(&mut self, row : &FpVector) {
        let last_row = self.matrix.rows() - 1;
        self.matrix[last_row].assign(row);
        self.matrix.row_reduce(&mut self.column_to_pivot_row);
    }

    pub fn add_vectors(&mut self, mut rows : impl std::iter::Iterator<Item=FpVector>) {
        let num_rows = self.matrix.rows();
        'outer: loop {
            let mut first_row = num_rows;
            for i in 0 .. num_rows {
                if self.matrix[i].is_zero() {
                    first_row = i;
                    break;
                }
            }
            if first_row == num_rows {
                return;
            }

            for i in first_row .. num_rows {
                if let Some(v) = rows.next() {
                    assert_eq!(v.dimension(), self.matrix.columns());
                    self.matrix[i] = v;
                } else {
                    break 'outer;
                }
            }
            self.row_reduce();
        }
        self.row_reduce();
    }

    pub fn add_basis_elements(&mut self, mut rows : impl std::iter::Iterator<Item=usize>) {
        let num_rows = self.matrix.rows();
        'outer: loop {
            let mut first_row = num_rows;
            for i in 0 .. num_rows {
                if self.matrix[i].is_zero() {
                    first_row = i;
                    break;
                }
            }
            if first_row == num_rows {
                return;
            }

            for i in first_row .. num_rows {
                if let Some(v) = rows.next() {
                    self.matrix[i].set_entry(v, 1);
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
    pub fn reduce(&self, vector : &mut FpVector){
        assert_eq!(vector.dimension(), self.matrix.columns());
        let p = self.matrix.prime();
        let mut row = 0;
        let columns = vector.dimension();
        for i in 0 .. columns {
            if self.column_to_pivot_row[i] < 0 {
                continue;
            }
            let c = vector.entry(i);
            if c != 0 {
                vector.add(&self.matrix[row], p - c);
            }
            row += 1;
        }
    }

    /// A version of `reduce` that doesn't require the vectors to be aligned.
    pub fn shift_reduce(&self, vector : &mut FpVector){
        let p = self.matrix.prime();
        let mut row = 0;
        let columns = vector.dimension();
        for i in 0 .. columns {
            if self.column_to_pivot_row[i] < 0 {
                continue;
            }
            let c = vector.entry(i);
            if c != 0 {
                vector.shift_add(&self.matrix[row], p - c);
            }
            row += 1;
        }
    }

    pub fn row_reduce(&mut self) {
        self.matrix.row_reduce(&mut self.column_to_pivot_row);
    }

    pub fn contains(&self, vector : &FpVector) -> bool {
        let mut vector = vector.clone();
        self.reduce(&mut vector);
        vector.is_zero()
    }

    pub fn dimension(&self) -> usize {
        for &i in self.column_to_pivot_row.iter().rev() {
            if i >= 0 {
                return i as usize + 1 ;
            }
        }
        return 0;
    }

    /// Returns a basis of the subspace.
    pub fn basis(&self) -> &[FpVector] {
        &self.matrix.vectors[..self.dimension()]
    }

    /// Sets the subspace to be the zero subspace.
    pub fn set_to_zero(&mut self) {
        self.matrix.set_to_zero();
        for x in self.column_to_pivot_row.iter_mut() {
            *x = -1;
        }
    }

    /// Sets the subspace to be the entire subspace.
    pub fn set_to_entire(&mut self) {
        self.matrix.set_to_zero();
        for i in 0..self.matrix.columns() {
            self.matrix[i].set_entry(i, 1);
            self.column_to_pivot_row[i] = i as isize;
        }
    }
}

/// Given a matrix M, a quasi-inverse Q is a map from the co-domain to the domain such that xQM = x
/// for all x in the image (recall our matrices act on the right).
///
/// # Fields
///  * `image` - The image of the original matrix. If the image is omitted, it is assumed to be
///  everything (with the standard basis).
///  * `preimage` - The actual quasi-inverse, where the basis of the image is that given by
///  `image`.
#[derive(Debug)]
pub struct QuasiInverse {
    pub image : Option<Subspace>,
    pub preimage : Matrix
}


impl QuasiInverse {
    pub fn prime(&self) -> u32 {
        self.preimage.prime()
    }

    /// Apply the quasi-inverse to an input vector and add a constant multiple of the result
    /// to an output vector
    ///
    /// # Arguments
    ///  * `target` - The output vector
    ///  * `coeff` - The constant multiple above
    ///  * `input` - The input vector, expressed in the basis of the ambient space
    pub fn apply(&self, target : &mut FpVector, coeff : u32, input : &FpVector){
        let p = self.prime();
        let mut row = 0;
        let columns = input.dimension();
        for i in 0 .. columns {
            if let Some(image) = &self.image { if image.column_to_pivot_row[i] < 0 {
                continue;
            }}
            let c = input.entry(i);
            if c != 0 {
                target.add(&self.preimage[row], (coeff * c) % p);
            }
            row += 1;
        }
    }
}

impl Matrix {
    pub fn set_to_zero(&mut self) {
        for row in 0..self.rows() {
            self.vectors[row].set_to_zero();
        }
    }

    pub fn find_first_row_in_block(&self, pivots : &Vec<isize>, first_column_in_block : usize) -> usize {
        for i in first_column_in_block .. self.columns() {
            if pivots[i] >= 0 {
                return pivots[i] as usize;
            }
        }
        return self.rows();
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
    /// let p = 3;
    /// # use rust_ext::matrix::Matrix;
    /// # use rust_ext::fp_vector::FpVectorT;
    /// # use rust_ext::fp_vector::FpVector;
    /// # rust_ext::fp_vector::initialize_limb_bit_index_table(p);
    /// let input  = [vec![1, 2, 1, 1, 0],
    ///               vec![1, 0, 2, 1, 1],
    ///               vec![2, 2, 0, 2, 1]];
    ///
    /// let (padded_cols, mut m) = Matrix::augmented_from_vec(p, &input);
    /// let mut pivots = vec![-1; m.columns()];
    /// m.row_reduce(&mut pivots);
    /// let ker = m.compute_kernel(&pivots, padded_cols);
    ///
    /// let mut target = vec![0; 3];
    /// ker.matrix[0].unpack(&mut target);
    /// assert_eq!(target, vec![1, 1, 2]);
    /// ```
    pub fn compute_kernel(&mut self, column_to_pivot_row : &Vec<isize>, first_source_column : usize) -> Subspace {
        let p = self.p;
        let rows = self.rows();
        let columns = self.columns();
        let source_dimension = columns - first_source_column;

        // Find the first kernel row
        let first_kernel_row = self.find_first_row_in_block(&column_to_pivot_row, first_source_column);
        // Every row after the first kernel row is also a kernel row, so now we know how big it is and can allocate space.
        let kernel_dimension = rows - first_kernel_row;
        let mut kernel = Subspace::new(p, kernel_dimension, source_dimension);
        if kernel_dimension == 0 {
            return kernel;
        }
        // Write pivots into kernel
        for i in 0 .. source_dimension {
            // Turns -1 into some negative number... make sure to check <0 for no pivot in column...
            kernel.column_to_pivot_row[i] = column_to_pivot_row[i + first_source_column] - first_kernel_row as isize;
        }
        // Copy kernel matrix into kernel
        for row in 0 .. kernel_dimension {
            // Reading from slice, alright.
            let vector = &mut self[first_kernel_row + row];
            let old_slice = vector.slice();
            vector.set_slice(first_source_column, first_source_column + source_dimension);
            kernel.matrix[row].assign(&vector);
            vector.restore_slice(old_slice);
        }
        return kernel;
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
    /// let p = 3;
    /// # use rust_ext::matrix::Matrix;
    /// # use rust_ext::fp_vector::FpVectorT;
    /// # use rust_ext::fp_vector::FpVector;
    /// # rust_ext::fp_vector::initialize_limb_bit_index_table(p);
    /// let input  = [vec![1, 2, 1, 1, 0],
    ///               vec![1, 0, 2, 1, 1],
    ///               vec![2, 2, 0, 2, 1]];
    ///
    /// let (padded_cols, mut m) = Matrix::augmented_from_vec(p, &input);
    /// let mut pivots = vec![-1; m.columns()];
    /// m.row_reduce(&mut pivots);
    /// let qi = m.compute_quasi_inverse(&pivots, input[0].len(), padded_cols);
    ///
    /// let image = [vec![1, 0, 2, 1, 1],
    ///              vec![0, 1, 1, 0, 1]];
    /// let computed_image = qi.image.unwrap();
    /// assert_eq!(computed_image.matrix, Matrix::from_vec(p, &image));
    /// assert_eq!(computed_image.column_to_pivot_row, vec![0, 1, -1, -1, -1]);
    ///
    /// let preimage = [vec![0, 1, 0],
    ///                 vec![0, 2, 2]];
    /// assert_eq!(qi.preimage, Matrix::from_vec(p, &preimage));
    /// ```
    pub fn compute_quasi_inverse(&mut self, pivots : &Vec<isize>, last_target_col : usize, first_source_col : usize) -> QuasiInverse {
        let p = self.prime();
        let columns = self.columns();
        let source_columns = columns - first_source_col;
        let first_kernel_row = self.find_first_row_in_block(&pivots, first_source_col);
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
        let image_pivots = pivots[..last_target_col].to_vec();
        let image = Subspace {
            matrix : image_matrix,
            column_to_pivot_row : image_pivots
        };
        return QuasiInverse {
            image : Some(image),
            preimage
        };
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
    pub fn compute_quasi_inverses(&mut self, pivots : &Vec<isize>, first_res_col : usize, last_res_col : usize,  first_source_col : usize) -> (QuasiInverse, QuasiInverse) {
        let p = self.prime();
        let columns = self.columns();
        let source_columns = columns - first_source_col;
        let res_columns = last_res_col - first_res_col;
        let first_res_row = self.find_first_row_in_block(&pivots, first_res_col);
        let first_kernel_row = self.find_first_row_in_block(&pivots, first_source_col);
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
            for i in first_res_col..columns {
                new_pivots[i - first_res_col] = pivots[i];
            }
            res_image_rows = first_kernel_row;
        } else {
            self.set_slice(0, first_kernel_row, first_res_col, columns);
            self.row_reduce(&mut new_pivots);
            res_image_rows = self.find_first_row_in_block(&pivots, first_source_col - first_res_col);
            self.clear_slice();
        }
        let mut res_preimage = Matrix::new(p, res_image_rows, source_columns);
        let mut res_image = Subspace::new(p, res_image_rows, res_columns);
        for i in 0..res_image_rows {
            let old_slice = self[i].slice();
            self[i].set_slice(first_res_col, last_res_col);
            res_image.matrix[i].assign(&self[i]);
            res_image.column_to_pivot_row.copy_from_slice(&new_pivots[..res_columns]);
            self[i].restore_slice(old_slice);
            self[i].set_slice(first_source_col, columns);
            res_preimage[i].assign(&self[i]);
            self[i].restore_slice(old_slice);
        }
        let cm_qi = QuasiInverse {
            image : None,
            preimage  : cc_preimage
        };
        let res_qi = QuasiInverse {
            image : Some(res_image),
            preimage : res_preimage
        };
        // println!("{:?}", self);
        // println!("{:?}", res_qi);
        return (cm_qi, res_qi);
    }
    
    pub fn get_image(&mut self, image_rows : usize, target_dimension : usize, pivots : &Vec<isize>) -> Subspace {
        let mut image = Subspace::new(self.p, image_rows, target_dimension);
        for i in 0 .. image_rows {
            image.column_to_pivot_row[i] = pivots[i];
            let vector_to_copy = &mut self[i];
            let old_slice = vector_to_copy.slice();
            vector_to_copy.set_slice(0, target_dimension);
            image.matrix[i].assign(vector_to_copy);
            vector_to_copy.restore_slice(old_slice);
        }
        return image;
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
        start_column : usize, end_column : usize,        
        current_pivots : &Vec<isize>
    ) -> Vec<usize> {
        let mut added_pivots = Vec::new();
        for i in start_column .. end_column {
            if current_pivots[i] >= 0 {
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
        return added_pivots;
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
        current_pivots : &Vec<isize>, desired_image : &Subspace
    ) -> Vec<usize> {
        let mut added_pivots = Vec::new();
        let desired_pivots = &desired_image.column_to_pivot_row;
        let early_end_column = std::cmp::min(end_column, desired_pivots.len() + start_column);
        for i in start_column .. early_end_column {
            debug_assert!(current_pivots[i] < 0 || desired_pivots[i - start_column] >= 0,
                format!("current_pivots : {:?}, desired_pivots : {:?}", current_pivots, desired_pivots));
            if current_pivots[i] >= 0 || desired_pivots[i - start_column] < 0 {
                continue;
            }
            // Look up the cycle that we're missing and add a generator hitting it.
            let kernel_vector_row = desired_pivots[i - start_column] as usize;
            let new_image = &desired_image.matrix[kernel_vector_row];
            let matrix_row = &mut self[first_empty_row];
            added_pivots.push(i);
            matrix_row.set_to_zero();
            let old_slice = matrix_row.slice();
            matrix_row.set_slice(start_column, start_column + desired_image.matrix.columns);
            matrix_row.assign(&new_image);
            matrix_row.restore_slice(old_slice);
            first_empty_row += 1;
        }
        return added_pivots;
    }

    /// Extends the image of a matrix to either the whole codomain, or the desired image specified
    /// by `desired_image`. It simply calls `extends_image_to_surjection` or
    /// `extend_image_to_surjection` depending on the value of `desired_image`. Refer to these
    /// functions for documentation.
    pub fn extend_image(&mut self, 
        first_empty_row : usize, 
        start_column : usize, end_column : usize, 
        current_pivots : &Vec<isize>, desired_image : &Option<Subspace>
    ) -> Vec<usize> {
        if let Some(image) = desired_image.as_ref() {
            return self.extend_image_to_desired_image(first_empty_row, start_column, end_column, current_pivots, image);
        } else {
            return self.extend_to_surjection(first_empty_row, start_column, end_column, current_pivots);
        }
    }

    /// Applies a matrix to a vector.
    ///
    /// # Example
    /// let p = 7;
    /// # use rust_ext::matrix::Matrix;
    /// # rust_ext::fp_vector::initialize_limb_bit_index_table(p);
    /// let input  = [vec![1, 3, 6],
    ///               vec![0, 3, 4]];
    ///
    /// let m = Matrix::from_vec(p, &input);
    /// let mut v = FpVector::new(p, 2);
    /// v.pack(vec![3, 1]);
    /// let mut result = FpVector::new(p, 3);
    /// let mut desired_result = FpVector::new(p, 3);
    /// result.pack(vec![3, 5, 1]);
    /// m.apply(&mut result, 1, &v);
    /// assert_eq!(result, desired_result);
    /// ```
    pub fn apply(&self, result : &mut FpVector, coeff : u32, input : &FpVector) {
        debug_assert_eq!(input.dimension(), self.rows());
        for i in 0 .. input.dimension() {
            result.add(&self.vectors[i], (coeff * input.entry(i)) % self.p);
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

use std::io;
use std::io::{Read, Write};
use saveload::{Save, Load};

impl Save for Matrix {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        self.columns.save(buffer)?;
        self.vectors.save(buffer)
    }
}

impl Load for Matrix {
    type AuxData = u32;

    fn load(buffer : &mut impl Read, p : &u32) -> io::Result<Self> {
        let columns = usize::load(buffer, &())?;

        let vectors : Vec<FpVector> = Load::load(buffer, p)?;
        Ok(Matrix::from_rows(*p, vectors, columns))
    }
}

impl Save for Subspace {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        self.matrix.save(buffer)?;
        self.column_to_pivot_row.save(buffer)
    }
}

impl Load for Subspace {
    type AuxData = u32;

    fn load(buffer : &mut impl Read, p : &u32) -> io::Result<Self> {
        let matrix : Matrix = Matrix::load(buffer, p)?;
        let column_to_pivot_row : Vec<isize> = Load::load(buffer, &())?;

        Ok(Subspace { matrix, column_to_pivot_row })
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_reduce_2(){
        let p = 2;
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
            let mut output_pivots_cvec = vec![-1; cols];
            m.row_reduce(&mut output_pivots_cvec);
            let mut unpacked_row : Vec<u32> = vec![0; cols];
            for i in 0 .. input.len() {
                m[i].unpack(&mut unpacked_row);
                assert_eq!(unpacked_row, goal_output[i]);
            }
            let mut output_pivots_vec = Vec::with_capacity(cols);
            for i in 0..cols {
                output_pivots_vec.push(output_pivots_cvec[i]);
            }
            assert_eq!(output_pivots_vec, goal_pivots)
        }
    }
}
