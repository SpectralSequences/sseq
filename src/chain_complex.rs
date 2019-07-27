use crate::memory::CVec;
use crate::module::Module;
use crate::module_homomorphism::ModuleHomomorphism;

pub trait ChainComplex {
    fn get_prime(&self) -> u32;
    fn get_min_degree(&self) -> i32;
    fn get_module(&self, homological_degree : usize) -> &Module;
    fn get_differential(&self, homological_degree : usize) -> &ModuleHomomorphism;
    fn compute_through_bidegree(&mut self, homological_degree : usize, degree : i32) {}

    fn compute_kernel(&mut self, degree : i32, homological_degree : usize){
        if homological_degree == 0 {
            let module = self.get_module(0);
            let dim = module.get_dimension(degree);
            let mut matrix = Matrix::new(p, dim, dim);
            let mut pivots = CVec::new(dim);
            for i in 0..dim {
                matrix.vectors[i].set_entry(i, 1);
                pivots[i] = i;
            }
            let subspace = Subspace::new(matrix, pivots);
        }
        let d = self.get_differential(homological_degree - 1);
        let source_dimension = d.get_source().get_dimension(degree);
        let target_dimension = d.get_target().get_dimension(degree);
        let padded_target_dimension = FpVector::get_padded_dimension(p, target_dimension, 0);
        let mut matrix = Matrix::new(p, source_dimension, padded_target_dimension + source_dimension);
        d.get_matrix(matrix, degree, 0, 0);
        for i in 0..source_dimension {
            matrix.vectors[i].set_entry(padded_target_dimension + i, 1);
        }
        matrix.row_reduce()
        d.set_kernel(degree, kernel);
    }
    fn compute_quasi_inverse(&mut self, degree : i32, subspace : &Subspace){
        
        // self.set_quasi_inverse(degree, quasi_inverse);
    }
}