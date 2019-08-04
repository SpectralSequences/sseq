use crate::fp_vector::{FpVector, FpVectorT};
// use crate::once::OnceRefOwned;
use crate::matrix::{Matrix, Subspace }; //QuasiInverse
use crate::algebra::Algebra;
use crate::module::{Module, ZeroModule};
use crate::module_homomorphism::{ModuleHomomorphism, ZeroHomomorphism};
use std::rc::Rc;


pub trait ChainComplex {
    fn get_prime(&self) -> u32 {
        self.get_algebra().get_prime()
    }
    fn get_algebra(&self) -> Rc<dyn Algebra>;
    fn get_min_degree(&self) -> i32;
    fn get_module(&self, homological_degree : u32) -> Rc<dyn Module>;
    fn get_differential(&self, homological_degree : u32) -> &dyn ModuleHomomorphism;
    fn compute_through_bidegree(&self, homological_degree : u32, degree : i32) {}
    fn computed_through_bidegree_q(&self, homological_degree : u32, degree : i32) -> bool { true }
    // fn set_kernel(&self, homological_degree : usize, degree : i32, kernel : Subspace);
    // fn set_image(&self, degree : i32, homological_degree : usize, image : Subspace);
    // fn get_kernel(&self, homological_degree : usize, degree : i32) -> &Subspace;
    // fn get_image(&self, homological_degree : usize, degree : i32) -> Option<&Subspace>;
    // fn get_quasi_inverse(&self, degree : i32, homological_degree : usize) -> &QuasiInverse;

    fn compute_kernel_and_image(&self,  homological_degree : u32, degree : i32){
        let p = self.get_prime();
        let d = self.get_differential(homological_degree);
        let lock = d.get_lock();
        if homological_degree == 0 {
            let module = self.get_module(0);
            let dim = module.get_dimension(degree);
            let kernel = Subspace::entire_space(p, dim);
            d.set_kernel(&lock, degree, kernel);
        }
        let source_dimension = d.get_source().get_dimension(degree);
        let target_dimension = d.get_target().get_dimension(degree);
        let padded_target_dimension = FpVector::get_padded_dimension(p, target_dimension, 0);
        let columns = padded_target_dimension + source_dimension;
        let mut matrix = Matrix::new(p, source_dimension, columns);
        d.get_matrix(&mut matrix, degree, 0, 0);
        for i in 0..source_dimension {
            matrix[i].set_entry(padded_target_dimension + i, 1);
        }
        let mut pivots = vec![-1;columns];
        matrix.row_reduce(&mut pivots);
        let kernel = matrix.compute_kernel(&pivots, padded_target_dimension);
        let kernel_rows = kernel.matrix.get_rows();
        d.set_kernel(&lock, degree, kernel);        
        let image_rows = matrix.get_rows() - kernel_rows;
        // let quasi_inverse = matrix.compute_quasi_inverses();
        // d.copy_image_from_matrix(degree, &mut matrix, &pivots, image_rows, target_dimension);
        // d.copy_quasi_inverse_from_matrix(degree, &mut matrix, image_rows, padded_target_dimension);
    }
}

pub struct ChainComplexConcentratedInDegreeZero {
    module : Rc<dyn Module>,
    zero_module : Rc<dyn Module>,
    d0 : ZeroHomomorphism,
    d1 : ZeroHomomorphism,
    other_ds : ZeroHomomorphism
}

impl ChainComplexConcentratedInDegreeZero {
    pub fn new(module : Rc<dyn Module>) -> Self {
        let p = module.get_prime();
        let zero_module = Rc::new(ZeroModule::new(Rc::clone(&module.get_algebra()))) as Rc<dyn Module>;

        Self {
            d0 : ZeroHomomorphism::new(Rc::clone(&module), Rc::clone(&zero_module)),
            d1 : ZeroHomomorphism::new(Rc::clone(&zero_module), Rc::clone(&module)),
            other_ds : ZeroHomomorphism::new(Rc::clone(&zero_module), Rc::clone(&zero_module)),
            module,
            zero_module
        }
    }
}

impl ChainComplex for ChainComplexConcentratedInDegreeZero {
    fn get_algebra(&self) -> Rc<dyn Algebra> {
        self.module.get_algebra()
    }

    fn get_module(&self, homological_degree : u32) -> Rc<dyn Module> {
        if homological_degree == 0 {
            Rc::clone(&self.module)
        } else {
            Rc::clone(&self.zero_module)
        }
    }

    fn get_min_degree(&self) -> i32 {
        self.module.get_min_degree()
    }

    // fn get_max_degree(&self) -> i32 {
    //     self.ccdz.head().module.get_max_degree()
    // }

    fn get_differential(&self, homological_degree : u32) -> &dyn ModuleHomomorphism {
        match homological_degree {
            0 => &self.d0,
            1 => &self.d1,
            _ => &self.other_ds
        }
    }

    // fn get_quasi_inverse(&self, degree : i32, homological_degree : usize) -> QuasiInverse {
    //     let qi_pivots = self.image_deg_zero[degree].get();
    //     QuasiInverse {
            
    //     }
    // }
}
