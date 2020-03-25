use crate::prime::ValidPrime;
use super::{Matrix, AugmentedMatrix2};

pub struct Basis {
    matrix : Matrix,
    inverse : AugmentedMatrix2
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
}