use crate::combinatorics;
use crate::fp_vector::{FpVector, FpVectorT};


use std::fmt;

/// A matrix! In particular, a matrix with values in F_p. The way we store matrices means it is
/// easier to perform row operations than column operations, and the way we use matrices means we
/// want our matrices to act on the right. Hence we think of vectors as row vectors.
///
/// Matrices can be *sliced*, i.e. restricted to a sub-matrix, and a sliced matrix behaves as if
/// the other rows and columns are not present for many purposes. For example, this affects the
/// values of `M[i]`, the `get_rows` and `get_columns` functions, as well as more "useful"
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
    vectors : Vec<FpVector>,
//    row_permutation : Vec<usize>
}

impl Matrix {
    /// Produces a new matrix over F_p with the specified number of rows and columns, intiialized
    /// to the 0 matrix.
    pub fn new(p : u32, rows : usize, columns : usize) -> Matrix {
        let mut vectors : Vec<FpVector> = Vec::with_capacity(rows);
        for _ in 0..rows {
            vectors.push(FpVector::new(p, columns));
        }
//        let mut row_permutation : Vec<usize> = Vec::with_capacity(columns);
//        for i in 0..rows {
//            row_permutation.push(i);
//        }
        Matrix { 
            p, rows, columns, 
            slice_row_start : 0, slice_row_end : rows,
            slice_col_start : 0, slice_col_end : columns,
            vectors, 
//            row_permutation
        }
    }

    /// Produces a matrix from a list of rows. If `vectors.len() == 0`, this returns a matrix
    /// with 0 rows and columns.  The function does not check if the rows have the same length,
    /// but please only input rows that do have the same length.
    pub fn from_rows(p : u32, vectors : Vec<FpVector>) -> Self {
        let rows = vectors.len();
        if rows == 0 {
            return Matrix::new(p, 0, 0);
        }
        let columns = vectors[0].get_dimension();
        Matrix {
            p,
            rows, columns,
            slice_row_start : 0, slice_row_end : rows,
            slice_col_start : 0, slice_col_end : columns,
            vectors
        }
    }

    /// Produces a Matrix from an `&[Vec<u32>]` object
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
        let mut result = Vec::with_capacity(self.get_columns());
        for i in 0 .. self.get_rows() {
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
    /// assert_eq!(n, FpVector::get_padded_dimension(p, input[0].len()));
    pub fn augmented_from_vec(p : u32, input : &[Vec<u32>]) -> (usize, Matrix) {
        let rows = input.len();
        let cols = input[0].len();
        let padded_cols = FpVector::get_padded_dimension(p, cols);
        let mut m = Matrix::new(p, rows, padded_cols + rows);

        for i in 0..rows {
            for j in 0..cols {
                m[i].set_entry(j, input[i][j]);
            }
            m[i].set_entry(padded_cols + i, 1);
        }
        (padded_cols, m)
    }

    pub fn prime(&self) -> u32 {
        self.p
    }

    /// Gets the number of rows in the matrix.
    pub fn get_rows(&self) -> usize {
        self.slice_row_end - self.slice_row_start
    }

    /// Gets the number of columns in the matrix.
    pub fn get_columns(&self) -> usize {
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
    /// assert_eq!(m.get_rows(), 3);
    /// assert_eq!(m.get_columns(), 2);
    /// assert_eq!(m[0].get_entry(0), 0);
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

    pub fn set_row(&mut self, row_idx : usize, row : &FpVector) {
        self.vectors[row_idx] = row.clone();
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
    fn iter(&self) -> std::slice::Iter<FpVector> {
        (*self.vectors)[self.slice_row_start .. self.slice_row_end].iter()
    }

    fn iter_mut(&mut self) -> std::slice::IterMut<FpVector> {
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

// void Matrix_serialize(char **buffer, Matrix *M){
//     size_t size = Matrix_getSize(M->p, M->rows, M->columns);
//     memcpy(*buffer, M, sizeof(Matrix));
//     *buffer += sizeof(Matrix);
//     *buffer += M->rows * sizeof(Vector*);
//     for(uint row = 0; row < M->rows; row++){
//         Vector_serialize(buffer, M->vectors[row]);
//     }
// }

// Matrix *Matrix_deserialize(char **buffer){
//     Matrix *M = (Matrix*)*buffer;
//     char *start_ptr = *buffer;
//     *buffer += sizeof(Matrix);
//     Vector **vector_ptr = (Vector**)*buffer;
//     M->vectors = vector_ptr;
//     uint rows = M->rows;    
//     *buffer += rows * sizeof(Vector*);
//     for(uint row = 0; row < rows; row++){
//         *vector_ptr = Vector_deserialize(M->p, buffer);
//         vector_ptr ++;
//     }
//     assert(start_ptr + Matrix_getSize(M->p, M->rows, M->columns) == *buffer);
//     return M;
// }


// Matrix *Matrix_slice(Matrix *M, char *memory, uint row_min, uint row_max, uint column_min, uint column_max){
//     assert(row_min <= row_max && row_max <= M->rows);
//     assert(column_min <= column_max && column_max <= M->columns);
//     Matrix *result = (Matrix*)memory;
//     uint num_rows = row_max - row_min;
//     uint num_cols = column_max - column_min;
//     result->p = M->p;
//     result->rows = num_rows;
//     result->columns = num_cols;
//     result->vectors = (Vector**)(result + 1);
//     Vector **matrix_ptr = (Vector**)result->vectors; 
//     char *vector_ptr = (char*)(matrix_ptr + num_rows);
//     Vector *initialized_vector_ptr;
//     for(uint i = 0; i < num_rows; i++){
//         initialized_vector_ptr = Vector_initialize(M->p, &vector_ptr, 0, 0);
//         *matrix_ptr = initialized_vector_ptr;
//         Vector_slice(initialized_vector_ptr, M->vectors[i], column_min, column_max);
//         matrix_ptr ++;
//     }
//     assert(matrix_ptr == (Vector **)(result->vectors + num_rows));
//     assert(vector_ptr == (char*)matrix_ptr + num_rows * VECTOR_CONTAINER_SIZE);
//     return result;
// }



// void Matrix_getRowPermutation(Matrix *M, uint *result){
//     Vector *first_vector = (Vector*)(M->vectors + M->rows);
//     for(uint i=0; i < M->rows; i++){
//         uint j = ((uint64)M->vectors[i] - (uint64)first_vector)/VECTOR_CONTAINER_SIZE; // why is this sizeof(VectorPrivate)??
//         result[i] = j;
//     }
// }

// void Matrix_applyRowPermutation(Matrix *M, uint *permutation, uint rows){
//     Vector *temp[rows];
//     for(uint i=0; i < rows; i++){
//         temp[i] = M->vectors[permutation[i]];
//     }
//     memcpy(M->vectors, temp, rows * sizeof(Vector*));
// }


impl Matrix {
    pub fn swap_rows(&mut self, i : usize, j : usize){
        self.vectors.swap(i + self.slice_row_start, j + self.slice_row_start);
//        self.row_permutation.swap(i + self.slice_row_start, j + self.slice_row_start);
    }

//    pub fn apply_permutation(&mut self, permutation : &Vec<usize>, scratch_space : &mut Vec<FpVector>){
//        assert!(permutation.len() < self.vectors.len());
//        assert!(permutation.len() < scratch_space.len());
//        unsafe {
//            for i in 0..permutation.len(){
//                std::ptr::swap(scratch_space.as_mut_ptr().offset(i as isize), self.vectors.as_mut_ptr().offset(permutation[i] as isize));
//            }
//            for i in 0..permutation.len(){
//                std::ptr::swap(self.vectors.as_mut_ptr().offset(i as isize), scratch_space.as_mut_ptr().offset(i as isize));
//            }
//        }
//    }

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
    /// let mut output_pivots_cvec = vec![-1; m.get_columns()];
    /// m.row_reduce(&mut output_pivots_cvec);
    ///
    /// assert_eq!(m, Matrix::from_vec(p, &result));
    /// ```
    pub fn row_reduce(&mut self, column_to_pivot_row: &mut Vec<isize>) {
        self.row_reduce_offset(column_to_pivot_row, 0);
    }

    pub fn row_reduce_offset(&mut self, column_to_pivot_row: &mut Vec<isize>, offset : usize) {
        assert!(self.get_columns() <= column_to_pivot_row.len());
        let p = self.p;
        let columns = self.get_columns();
        let rows = self.get_rows();
        for x in column_to_pivot_row.iter_mut() {
            *x = -1;
        }
        if rows == 0 {
            return;
        }
        let mut pivot : usize = 0;
        for pivot_column in offset .. columns {
            // Search down column for a nonzero entry.
            let mut pivot_row = rows;
            for i in pivot..rows {
                if self[i].get_entry(pivot_column) != 0 {
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
            let c = self[pivot].get_entry(pivot_column);
            let c_inv = combinatorics::inverse(p, c);
            self[pivot].scale(c_inv);
            // println!("({}) <== {} * ({}): \n{}", pivot, c_inv, pivot, self);

            // if(col_end > 0){
            //     printf("row(%d) *= %d\n", pivot, c_inv);
            //     Matrix_printSlice(M, col_end, col_start);
            // }
            for i in 0 .. rows {
                // Between pivot and pivot_row, we already checked that the pivot column is 0, so we could skip ahead a bit.
                // But Rust doesn't make this as easy as C.
                if i == pivot {
                    // i = pivot_row;
                    continue;
                }
                let pivot_column_entry = self[i].get_entry(pivot_column);
                if pivot_column_entry == 0 {
                    continue;
                }
                let row_op_coeff = p - pivot_column_entry;
                // Do row operation
                self.row_op(i, pivot, row_op_coeff);
            }
            pivot += 1;
        }
        return;
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
        self.matrix.set_row(self.matrix.get_rows() - 1, row);
        self.matrix.row_reduce(&mut self.column_to_pivot_row);
    }

    /// Projects a vector to a complement of the subspace. The complement is the set of vectors
    /// that have a 0 in every column where there is a pivot in `matrix`
    pub fn reduce(&self, vector : &mut FpVector){
        let p = self.matrix.prime();
        let mut row = 0;
        let columns = vector.get_dimension();
        for i in 0 .. columns {
            if self.column_to_pivot_row[i] < 0 {
                continue;
            }
            let c = vector.get_entry(i);
            if c != 0 {
                vector.add(&self.matrix[row], p - c);
            }
            row += 1;
        }
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

    /// Returns a basis of the subspace
    pub fn get_basis(&self) -> &[FpVector] {
        &self.matrix.vectors[..self.dimension()]
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
        let columns = input.get_dimension();
        for i in 0 .. columns {
            if let Some(image) = &self.image { if image.column_to_pivot_row[i] < 0 {
                continue;
            }}
            let c = input.get_entry(i);
            target.add(&self.preimage[row], (coeff * c) % p);
            row += 1;
        }
    }
}

impl Matrix {
    pub fn set_to_zero(&mut self) {
        for row in 0..self.get_rows() {
            self.vectors[row].set_to_zero();
        }
    }

    pub fn find_first_row_in_block(&self, pivots : &Vec<isize>, first_column_in_block : usize) -> usize {
        for i in first_column_in_block .. self.get_columns() {
            if pivots[i] >= 0 {
                return pivots[i] as usize;
            }
        }
        return self.get_rows();
    }

    /// Computes the kernel from an augmented matrix in rref. To compute the kernel of a matrix
    /// A, produce an augmented matrix of the form
    /// ```text
    /// [A | I]
    /// ```
    /// An important thing to note is that the number of columns of `A` should be a multiple of the
    /// number of entries per limb in an FpVector, and this is often achieved by padding columns
    /// with 0. The padded length can be obtained from `FpVector::get_padded_dimension`.
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
    /// let mut pivots = vec![-1; m.get_columns()];
    /// m.row_reduce(&mut pivots);
    /// let ker = m.compute_kernel(&pivots, padded_cols);
    ///
    /// let mut target = vec![0; 3];
    /// ker.matrix[0].unpack(&mut target);
    /// assert_eq!(target, vec![1, 1, 2]);
    /// ```
    pub fn compute_kernel(&mut self, column_to_pivot_row : &Vec<isize>, first_source_column : usize) -> Subspace {
        let p = self.p;
        let rows = self.get_rows();
        let columns = self.get_columns();
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
            let old_slice = vector.get_slice();
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
    /// let mut pivots = vec![-1; m.get_columns()];
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
        let columns = self.get_columns();
        let source_columns = columns - first_source_col;
        let first_kernel_row = self.find_first_row_in_block(&pivots, first_source_col);
        let mut image_matrix = Matrix::new(p, first_kernel_row, last_target_col);
        let mut preimage = Matrix::new(p, first_kernel_row, source_columns);
        for i in 0 .. first_kernel_row {
            let old_slice = self[i].get_slice();
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
        let columns = self.get_columns();
        let source_columns = columns - first_source_col;
        let res_columns = last_res_col - first_res_col;
        let first_res_row = self.find_first_row_in_block(&pivots, first_res_col);
        let first_kernel_row = self.find_first_row_in_block(&pivots, first_source_col);
        let mut cc_preimage = Matrix::new(p, first_res_row, source_columns);
        for i in 0..first_res_row {
            let old_slice = self[i].get_slice();
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
            let old_slice = self[i].get_slice();
            self[i].set_slice(first_res_col, last_res_col);
            res_image.matrix[i].assign(&self[i]);
            res_image.column_to_pivot_row.copy_from_slice(&new_pivots[first_res_col..last_res_col]);
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
            let old_slice = vector_to_copy.get_slice();
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
        current_pivots : &Vec<isize>, desired_image : Subspace
    ) -> Vec<usize> {
        let mut added_pivots = Vec::new();
        let desired_pivots = &desired_image.column_to_pivot_row;
        let early_end_column = std::cmp::min(end_column, desired_pivots.len() + start_column);
        for i in start_column .. early_end_column {
            assert!(current_pivots[i] < 0 || desired_pivots[i - start_column] >= 0, 
                format!("current_pivots : {:?}, desired_pivots : {:?}", current_pivots, desired_pivots));
            if current_pivots[i] >= 0 || desired_pivots[i - start_column] < 0 {
                continue;
            }
            // Look up the cycle that we're missing and add a generator hitting it.
            let kernel_vector_row = desired_pivots[i] as usize;
            let new_image = &desired_image.matrix[kernel_vector_row];
            let matrix_row = &mut self[first_empty_row];
            added_pivots.push(i);
            matrix_row.set_to_zero();
            let old_slice = matrix_row.get_slice();
            matrix_row.set_slice(0, desired_image.matrix.columns);
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
        current_pivots : &Vec<isize>, desired_image : Option<Subspace>
    ) -> Vec<usize> {
        if let Some(image) = desired_image {
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
        assert_eq!(input.get_dimension(), self.get_rows());
        for i in 0 .. input.get_dimension() {
            result.add(&self.vectors[i], (coeff * input.get_entry(i)) % self.p);
        }
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
            for (i,x) in input.iter().enumerate(){
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
