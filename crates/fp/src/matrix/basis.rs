use crate::prime::ValidPrime;
use crate::vector::{FpVector, FpVectorT};
use super::{Matrix, AugmentedMatrix2};

pub struct Basis {
    pub matrix : Matrix,
    pub inverse : AugmentedMatrix2
}

impl Basis {
    pub fn new(p : ValidPrime, dimension : usize) -> Self {
        let mut matrix = Matrix::new(p, dimension, dimension);
        let mut inverse = AugmentedMatrix2::new(p, dimension, &[dimension, dimension]);
        matrix.add_identity(dimension, 0, 0);
        inverse.segment(0,0).add_identity(dimension, 0, 0);
        inverse.segment(1,1).add_identity(dimension, 0, 0);
        Basis {
            matrix,
            inverse
        }
    }

    pub fn is_inverse_calculated(&self) -> bool {
        self.inverse.pivots()[0] == 0
    }

    pub fn invalidate_inverse(&mut self) {
        self.inverse.pivots_mut()[0] = -1;
    }

    pub fn take_matrix(&mut self) -> Matrix {
        let p = self.matrix.prime();
        std::mem::replace(&mut self.matrix, Matrix::new(p, 0, 0))
    }

    pub fn set_matrix(&mut self, m : Matrix){
        self.matrix = m;
    }

    pub fn calculate_inverse(&mut self) {
        let matrix = self.take_matrix();
        self.inverse.segment(0,0).assign(&matrix);
        self.set_matrix(matrix);
        self.inverse.initialize_pivots();
        self.inverse.row_reduce();
    }

    pub fn apply(&self, result : &mut FpVector, v : &FpVector) {
        assert!(v.dimension() == self.matrix.columns());
        for i in 0 .. v.dimension() {
            result.add(&self.matrix[i], v.entry(i));
        }
    }

    pub fn apply_inverse(&mut self, result : &mut FpVector, v : &FpVector) {
        assert!(v.dimension() == self.matrix.columns());
        for i in 0 .. v.dimension() {
            result.add(&self.inverse.segment(1,1)[i], v.entry(i));
        }
    }    
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
        for (i, row) in basis.matrix.iter_mut().enumerate() {
            row.pack(&matrix[i]);
        }
        basis.calculate_inverse();
        println!("inverse: {}", *basis.inverse.segment(1,1));
        // Should be:
        // [
        //     [1, 1, 1, 1],
        //     [1, 0, 1, 1],
        //     [0, 1, 1, 1],
        //     [1, 0, 1, 0]
        // ]
        let mut result = FpVector::new(p, 4);
        let mut input = FpVector::new(p, 4);
        input.pack(&[1,1,1,1]);
        basis.apply(&mut result, &input);
        println!("result : {}", result);
        result.set_to_zero();
        basis.apply_inverse(&mut result, &input);
        println!("inverse_result : {}", result);
        result.set_to_zero();
    }

}
