use crate::prime::ValidPrime;
use crate::vector::{FpVector, FpVectorT};
use super::{Matrix, AugmentedMatrix2};

#[derive(Clone)]
pub struct Basis {
    pub matrix : Matrix,
    pub inverse : AugmentedMatrix2
}

impl Basis {
    pub fn new(p : ValidPrime, dimension : usize) -> Self {
        let mut result = Basis {
            matrix :  Matrix::new(p, dimension, dimension),
            inverse : AugmentedMatrix2::new(p, dimension, &[dimension, dimension])
        };
        result.matrix.add_identity(dimension, 0, 0);
        result.inverse.segment(0,0).add_identity(dimension, 0, 0);        
        result.calculate_inverse();
        result
    }

    pub fn prime(&self) -> ValidPrime {
        self.matrix.prime()
    }

    pub fn dimension(&self) -> usize {
        self.matrix.rows()
    }

    pub fn is_inverse_calculated(&self) -> bool {
        self.inverse.pivots()[0] == 0
    }

    pub fn invalidate_inverse(&mut self) {
        self.inverse.pivots_mut()[0] = -1;
    }

    pub fn take_matrix(&mut self) -> Matrix {
        let p = self.prime(); // Need this separate for borrow rules
        std::mem::replace(&mut self.matrix, Matrix::new(p, 0, 0))
    }

    pub fn set_matrix(&mut self, m : Matrix){
        self.matrix = m;
    }

    pub fn is_singular(&self) -> bool {
        for i in 0..self.matrix.columns() {
            if self.inverse.pivots()[i] != i as isize {
                return true;
            }
        }
        return false;
    }


    pub fn calculate_inverse(&mut self) {
        let matrix = self.take_matrix();
        self.inverse.clear_slice();
        self.inverse.segment(0,0).assign(&matrix);
        self.set_matrix(matrix);
        self.inverse.segment(1,1).set_to_zero();
        self.inverse.segment(1,1).add_identity(self.matrix.rows(), 0, 0);
        self.inverse.initialize_pivots();
        self.inverse.row_reduce();
        std::mem::forget(self.inverse.segment(1,1));
    }

    pub fn apply(&self, result : &mut FpVector, coeff : u32, v : &FpVector) {
        assert!(v.dimension() == self.matrix.columns());
        for i in 0 .. v.dimension() {
            result.add(&self.matrix[i], (v.entry(i) * coeff) % *self.prime());
        }
    }

    pub fn apply_inverse(&self, result : &mut FpVector, coeff : u32, v : &FpVector) {
        assert!(v.dimension() == self.matrix.columns());
        for i in 0 .. v.dimension() {
            result.add(&self.inverse[i], (v.entry(i) * coeff) % *self.prime());
        }
    }

    // pub fn replace_entry(&mut self, row : usize, v : &FpVector) -> Result<(), ()>{
    //     assert!(v.dimension() == self.matrix.columns());
    //     self.matrix[row].assign(v);
    //     self.calculate_inverse();
    // }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basis() {
        let p = ValidPrime::new(2);
        let mut basis = Basis::new(p, 4);
        let matrix = &[
            vec![1, 0, 1, 0],
            vec![1, 1, 0, 0],
            vec![1, 0, 1, 1],
            vec![0, 1, 0, 1],
        ];

        let inverse = &[
            vec![1, 1, 1, 1],
            vec![1, 0, 1, 1],
            vec![0, 1, 1, 1],
            vec![1, 0, 1, 0]
        ];

        for (i, row) in basis.matrix.iter_mut().enumerate() {
            row.pack(&matrix[i]);
        }
        basis.calculate_inverse();
        basis.inverse.assert_list_eq(inverse);
        let mut result = FpVector::new(p, 4);
        let mut input = FpVector::new(p, 4);
        input.pack(&[1,1,1,1]);
        basis.apply(&mut result, 1, &input);
        println!("result : {}", result);
        result.set_to_zero();
        basis.apply_inverse(&mut result, 1, &input);
        println!("inverse_result : {}", result);
        result.set_to_zero();
        println!("basis : {}", basis.matrix);
        println!("inverse : {}", *basis.inverse);        
        input.pack(&[1,1,0,1]);
        println!("  inverse columns 111 : {}", basis.inverse.columns());
        basis.apply_inverse(&mut result, 1, &input);
        println!("inverse_result : {}", result);
        result.set_to_zero();        
        // basis.replace_entry(2, &input);
        // println!("basis : {}", basis.matrix);
        // println!("inverse : {}", *basis.inverse);
    }

}
