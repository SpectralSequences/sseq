use crate::memory::CVec;
use crate::memory::MemoryAllocator;
use crate::combinatorics;
use crate::fp_vector::FpVector;


use std::fmt;

pub struct Matrix {
    p : u32,
    pub rows : usize,
    pub columns : usize,
    pub vectors : CVec<FpVector>,
    row_permutation : CVec<usize>
}

impl Matrix {
    pub fn new(p : u32, rows : usize, columns : usize) -> Matrix {
        let mut vectors : Vec<FpVector> = Vec::with_capacity(columns);
        for i in 0..rows {
            vectors.push(FpVector::new(p, columns, 0));
        }
        let mut permutation : Vec<usize> = Vec::with_capacity(columns);
        for i in 0..rows {
            permutation.push(i);
        }
        Matrix { p, rows, columns, vectors : CVec::from_vec(vectors), row_permutation : CVec::from_vec(permutation) }
    }

    pub fn new_from_allocator<T : MemoryAllocator + std::fmt::Display>(allocator: &T, p : u32, rows : usize, columns : usize) -> Matrix {
        let mut vectors : CVec<FpVector> = allocator.alloc_vec(rows);
        for v in vectors.iter_mut() {
            *v = FpVector::new_from_allocator(allocator, p, columns, 0);
        }
        let mut row_permutation : CVec<usize> = allocator.alloc_vec(rows);
        for i in 0..rows {
            row_permutation[i] = i;
        }
        Matrix { p, rows, columns, vectors, row_permutation }
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
        (*self.vectors).iter()
    }

    fn iter_mut(&mut self) -> std::slice::IterMut<FpVector> {
        (*self.vectors).iter_mut()
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

impl std::ops::Index<usize> for Matrix {
    type Output = FpVector;
    fn index(&self, i : usize) -> &Self::Output {
        &self.vectors[i]
    }
}

impl std::ops::IndexMut<usize> for Matrix {
    fn index_mut(&mut self, i : usize) -> &mut Self::Output {
        &mut self.vectors[i]
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
        self.vectors.swap(i, j);
        self.row_permutation.swap(i, j);
    }

    pub fn apply_permutation(&mut self, permutation : CVec<usize>, scratch_space : CVec<FpVector>){
        self.vectors.apply_permutation(permutation, scratch_space);
    }

    pub fn row_op(&mut self, target : usize, source : usize, coeff : u32){
        unsafe {
            // Can't take two mutable loans from one vector, so instead just cast
            // them to their raw pointers to do the swap
            let ptarget: *mut FpVector = &mut self[target];
            let psource: *const FpVector = &mut self[source];
            (*ptarget).add(&*psource, coeff);
        }
    }

    pub fn row_reduce(&mut self, column_to_pivot_row: &mut CVec<isize>){
        assert!(self.columns <= column_to_pivot_row.len());
        let p = self.p;
        let columns = self.columns;
        let rows = self.rows;
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
                if self.vectors[i].get_entry(pivot_column) != 0 {
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
            let c = self.vectors[pivot].get_entry(pivot_column);
            let c_inv = combinatorics::inverse(p, c);
            self.vectors[pivot].scale(c_inv);
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
                let pivot_column_entry = self.vectors[i].get_entry(pivot_column);
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

pub struct Subspace {
    matrix : Matrix,
    column_to_pivot_row : Vec<i32>
}

impl Subspace {
    pub fn new(p : u32, rows : usize, columns : usize) -> Self {
        Self {
            matrix : Matrix::new(p, rows, columns),
            column_to_pivot_row : Vec::with_capacity(columns)
        }
    }
}

// matrix -- a row reduced augmented matrix
// column_to_pivot_row -- the pivots in matrix (also returned by row_reduce)
// first_source_column -- which block of the matrix is the source of the map
impl Matrix {
    pub fn compute_kernel(&mut self, column_to_pivot_row : Vec<i32>, first_source_column : usize) -> Subspace {
        let p = self.p;
        let source_dimension = self.columns - first_source_column;

        // Find the first kernel row
        let mut first_kernel_row = self.rows;
        for i in first_source_column .. self.columns {
            if column_to_pivot_row[i] >= 0 {
                first_kernel_row = column_to_pivot_row[i] as usize;
                break;
            }
        }
        // Every row after the first kernel row is also a kernel row, so now we know how big it is and can allocate space.
        let kernel_dimension = self.rows - first_kernel_row;
        let mut kernel = Subspace::new(p, kernel_dimension, source_dimension);
        if kernel_dimension == 0 {
            for i in 0..source_dimension {
                kernel.column_to_pivot_row.push(-1);
            }
            return kernel;
        }
        // Write pivots into kernel
        for i in 0 .. source_dimension {
            // Turns -1 into some negative number... make sure to check <0 for no pivot in column...
            kernel.column_to_pivot_row.push(column_to_pivot_row[i + first_source_column] - first_kernel_row as i32);
        }
        // Copy kernel matrix into kernel
        for row in 0 .. kernel_dimension {
            // Reading from slice, alright.
            let slice = self.vectors[first_kernel_row + row].
                            slice(first_source_column, first_source_column + source_dimension);
            kernel.matrix.vectors[row].assign(&slice);
        }
        return kernel;
    }

    /// Take an augmented row reduced matrix representation of a map and adds rows to it to hit the complement
    /// of complement_pivots in desired_image. Does so by walking through the columns and if it finds a target column
    /// that has a pivot in desired_image but no pivot in current_pivots or complement_pivots, add that the row in desired_image
    /// to the matrix.
    ///    self -- An augmented, row reduced matrix to be modified to extend it's image.
    ///    first_source_column : Where does the source comppstart in the augmented matrix?
    pub fn extend_image(&mut self, 
        mut first_empty_row : usize, current_pivots : Vec<i32>, 
        desired_image : Subspace, complement_pivots : Option<Vec<i32>>
    ) -> u32 {
        let p = self.p;
        let mut homology_dimension = 0;
        let desired_pivots = desired_image.column_to_pivot_row;
        for i in 0 .. desired_image.matrix.columns {
            assert!(current_pivots[i] < 0 || desired_pivots[i] >= 0);
            if current_pivots[i] >= 0 || desired_pivots[i] < 0 {
                continue;
            }
            if let Some(l) = &complement_pivots {
                if l[i] >= 0 { continue; }
            }
            // Look up the cycle that we're missing and add a generator hitting it.
            let kernel_vector_row = desired_pivots[i] as usize;
            let new_image = &desired_image.matrix.vectors[kernel_vector_row];
            let matrix_row = &mut self.vectors[first_empty_row];
            // Writing into slice -- leaving rest of Vector alone would be alright in this case.
            let mut slice = matrix_row.slice(0, desired_image.matrix.columns);
            slice.assign(&new_image);
            first_empty_row += 1;
            homology_dimension += 1;
        }
        return homology_dimension;
    }
}

pub struct QuasiInverse<'a> {
    matrix : Matrix,
    image : &'a Subspace
}

impl QuasiInverse<'_> {
    pub fn apply(&self, target : &mut FpVector, input : &FpVector){
        let mut row = 0;
        for i in 0 .. self.image.matrix.columns {
            if self.image.column_to_pivot_row[i] < 0 {
                continue;
            }
            let coeff = input.get_entry(i);
            target.add(&self.matrix.vectors[row], coeff);
            row += 1;
        }
    }
}