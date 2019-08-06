use crate::combinatorics;
use crate::fp_vector::{FpVector, FpVectorT};


use std::fmt;

pub struct Matrix {
    p : u32,
    rows : usize,
    columns : usize,
    slice_row_start : usize,
    slice_row_end : usize,
    slice_col_start : usize,
    slice_col_end : usize,
    vectors : Vec<FpVector>,
    row_permutation : Vec<usize>
}

impl Matrix {
    pub fn new(p : u32, rows : usize, columns : usize) -> Matrix {
        let mut vectors : Vec<FpVector> = Vec::with_capacity(rows);
        for _ in 0..rows {
            vectors.push(FpVector::new(p, columns, 0));
        }
        let mut row_permutation : Vec<usize> = Vec::with_capacity(columns);
        for i in 0..rows {
            row_permutation.push(i);
        }
        Matrix { 
            p, rows, columns, 
            slice_row_start : 0, slice_row_end : rows,
            slice_col_start : 0, slice_col_end : columns,
            vectors, 
            row_permutation
        }
    }

    pub fn get_prime(&self) -> u32 {
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

    pub fn set_slice(&mut self, row_start : usize, row_end : usize, col_start : usize, col_end : usize) {
        for v in self.vectors.iter_mut() {
            v.set_slice(col_start, col_end);
        }
        self.slice_row_start = row_start;
        self.slice_row_end = row_end;
        self.slice_col_start = col_start;
        self.slice_col_end = col_end;
    }

    pub fn clear_slice(&mut self) {
        for v in self.vectors.iter_mut() {
            v.clear_slice();
        }        
        self.slice_row_start = 0;
        self.slice_row_end = self.rows;
        self.slice_col_start = 0;
        self.slice_col_end = self.columns;
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
        self.row_permutation.swap(i + self.slice_row_start, j + self.slice_row_start);
    }

    pub fn apply_permutation(&mut self, permutation : &Vec<usize>, scratch_space : &mut Vec<FpVector>){
        assert!(permutation.len() < self.vectors.len());
        assert!(permutation.len() < scratch_space.len());
        unsafe {
            for i in 0..permutation.len(){
                std::ptr::swap(scratch_space.as_mut_ptr().offset(i as isize), self.vectors.as_mut_ptr().offset(permutation[i] as isize));
            }
            for i in 0..permutation.len(){
                std::ptr::swap(self.vectors.as_mut_ptr().offset(i as isize), scratch_space.as_mut_ptr().offset(i as isize));
            }            
        }
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

    pub fn row_reduce(&mut self, column_to_pivot_row: &mut Vec<isize>){
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
        for pivot_column in 0 .. columns {
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

#[derive(Debug)]
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
}

pub struct QuasiInverse {
    pub image : Option<Subspace>,
    pub preimage : Matrix
}


impl QuasiInverse {
    pub fn get_prime(&self) -> u32 {
        self.preimage.get_prime()
    }

    pub fn apply(&self, target : &mut FpVector, coeff : u32, input : &FpVector){
        let p = self.get_prime();
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

    pub fn reduce(&self, vector : &mut FpVector){
        let p = self.get_prime();
        let mut row = 0;
        let columns = vector.get_dimension();
        let image = self.image.as_ref().unwrap();
        for i in 0 .. columns {
            if image.column_to_pivot_row[i] < 0 {
                continue;
            }
            let c = vector.get_entry(i);
            if c != 0 {
                vector.add(&image.matrix[row], p - c);
            }
            row += 1;
        }
    }
}

impl Matrix {
    pub fn find_first_row_in_block(&self, pivots : &Vec<isize>, first_column_in_block : usize) -> usize {
        for i in first_column_in_block .. self.get_columns() {
            if pivots[i] >= 0 {
                return pivots[i] as usize;
            }
        }
        return self.get_rows();
    }

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
            for i in 0..source_dimension {
                kernel.column_to_pivot_row[i] = -1;
            }
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

    pub fn compute_quasi_inverse(&mut self, pivots : &Vec<isize>, last_target_col : usize, first_source_col : usize) -> QuasiInverse {
        let p = self.get_prime();
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

    pub fn compute_quasi_inverses(&mut self, pivots : &Vec<isize>, first_res_col : usize, last_res_col : usize,  first_source_col : usize) -> (QuasiInverse, QuasiInverse) {
        let p = self.get_prime();
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
        return (cm_qi, res_qi);
    }
    

    /// Take an augmented row reduced matrix representation of a map and adds rows to it to hit the complement
    /// of complement_pivots in desired_image. Does so by walking through the columns and if it finds a target column
    /// that has a pivot in desired_image but no pivot in current_pivots or complement_pivots, add that the row in desired_image
    /// to the matrix.
    ///    self -- An augmented, row reduced matrix to be modified to extend it's image.
    ///    first_source_column : Where does the source comppstart in the augmented matrix?
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
            matrix_row.set_to_zero();
            matrix_row.set_entry(i, 1);
            first_empty_row += 1;
        }
        return added_pivots;
    }

    fn extend_image_to_desired_image(&mut self,
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

    /// Take an augmented row reduced matrix representation of a map and adds rows to it to hit the complement
    /// of complement_pivots in desired_image. Does so by walking through the columns and if it finds a target column
    /// that has a pivot in desired_image but no pivot in current_pivots or complement_pivots, add that the row in desired_image
    /// to the matrix.
    ///    self -- An augmented, row reduced matrix to be modified to extend it's image.
    ///    first_source_column : Where does the source comppstart in the augmented matrix?
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
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_reduce_2(){
        let p = 2;
        combinatorics::initialize_prime(p);
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
